// Local integration smoke for a caller-supplied Moonshine build, model bundle,
// and speech fixture. It prints status/metrics only, never recognized text.
#include "moonshine-c-api.h"

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <iomanip>
#include <fstream>
#include <iostream>
#include <iterator>
#include <optional>
#include <string>
#include <sys/resource.h>
#include <vector>

struct WavData {
  int sample_rate = 0;
  std::vector<float> samples;
};

static uint16_t le16(const std::vector<uint8_t>& b, size_t p) {
  return static_cast<uint16_t>(b[p]) | (static_cast<uint16_t>(b[p + 1]) << 8);
}
static uint32_t le32(const std::vector<uint8_t>& b, size_t p) {
  return static_cast<uint32_t>(b[p]) |
         (static_cast<uint32_t>(b[p + 1]) << 8) |
         (static_cast<uint32_t>(b[p + 2]) << 16) |
         (static_cast<uint32_t>(b[p + 3]) << 24);
}

static bool read_pcm16_mono_wav(const char* path, WavData& out) {
  std::ifstream in(path, std::ios::binary);
  if (!in) return false;
  std::vector<uint8_t> b((std::istreambuf_iterator<char>(in)), {});
  if (b.size() < 44 || std::string(reinterpret_cast<char*>(b.data()), 4) != "RIFF" ||
      std::string(reinterpret_cast<char*>(b.data() + 8), 4) != "WAVE") return false;
  uint16_t format = 0, channels = 0, bits = 0;
  uint32_t rate = 0;
  size_t data_offset = 0, data_size = 0;
  for (size_t p = 12; p + 8 <= b.size();) {
    const uint32_t chunk_size = le32(b, p + 4);
    const size_t payload = p + 8;
    if (payload + chunk_size > b.size()) return false;
    const std::string id(reinterpret_cast<char*>(b.data() + p), 4);
    if (id == "fmt " && chunk_size >= 16) {
      format = le16(b, payload);
      channels = le16(b, payload + 2);
      rate = le32(b, payload + 4);
      bits = le16(b, payload + 14);
    } else if (id == "data") {
      data_offset = payload;
      data_size = chunk_size;
    }
    p = payload + chunk_size + (chunk_size & 1U);
  }
  if (format != 1 || channels != 1 || bits != 16 || rate == 0 || data_size == 0 ||
      data_size % 2 != 0 || data_offset + data_size > b.size()) return false;
  out.sample_rate = static_cast<int>(rate);
  out.samples.reserve(data_size / 2);
  for (size_t p = data_offset; p < data_offset + data_size; p += 2) {
    const int16_t sample = static_cast<int16_t>(le16(b, p));
    out.samples.push_back(static_cast<float>(sample) / 32768.0f);
  }
  return true;
}

static bool has_nonempty_line(const transcript_t* transcript) {
  if (transcript == nullptr ||
      (transcript->line_count > 0 && transcript->lines == nullptr)) return false;
  for (uint64_t i = 0; i < transcript->line_count; ++i) {
    const char* text = transcript->lines[i].text;
    if (text != nullptr && *text != '\0') return true;
  }
  return false;
}

static std::optional<double> process_cpu_seconds() {
  struct rusage usage {};
  if (getrusage(RUSAGE_SELF, &usage) != 0) return std::nullopt;
  return static_cast<double>(usage.ru_utime.tv_sec) +
         static_cast<double>(usage.ru_utime.tv_usec) / 1'000'000.0 +
         static_cast<double>(usage.ru_stime.tv_sec) +
         static_cast<double>(usage.ru_stime.tv_usec) / 1'000'000.0;
}

static int run_session(int32_t transcriber, const WavData& wav, int session_number) {
  const auto session_started = std::chrono::steady_clock::now();
  const auto cpu_started = process_cpu_seconds();
  const int32_t stream = moonshine_create_stream(transcriber, 0);
  if (stream < 0) {
    std::cerr << "create_stream failed: " << moonshine_error_to_string(stream) << '\n';
    return 1;
  }
  bool started = false;
  auto release = [&]() {
    if (started) (void)moonshine_stop_stream(transcriber, stream);
    (void)moonshine_free_stream(transcriber, stream);
  };
  const int32_t start_error = moonshine_start_stream(transcriber, stream);
  if (start_error != MOONSHINE_ERROR_NONE) {
    std::cerr << "start_stream failed: " << moonshine_error_to_string(start_error) << '\n';
    release();
    return 1;
  }
  started = true;

  const size_t max_samples = std::min(wav.samples.size(), static_cast<size_t>(wav.sample_rate) * 12);
  const size_t chunk_samples = static_cast<size_t>(wav.sample_rate) * 3 / 10;
  size_t next_poll = static_cast<size_t>(wav.sample_rate);
  size_t partial_calls = 0;
  bool partial_nonempty = false;
  for (size_t offset = 0; offset < max_samples;) {
    const size_t count = std::min(chunk_samples, max_samples - offset);
    const int32_t add_error = moonshine_transcribe_add_audio_to_stream(
        transcriber, stream, wav.samples.data() + offset, count, wav.sample_rate, 0);
    if (add_error != MOONSHINE_ERROR_NONE) {
      std::cerr << "add_audio failed: " << moonshine_error_to_string(add_error) << '\n';
      release();
      return 1;
    }
    offset += count;
    if (offset >= next_poll) {
      transcript_t* partial = nullptr;
      const int32_t poll_error = moonshine_transcribe_stream(
          transcriber, stream, MOONSHINE_FLAG_FORCE_UPDATE, &partial);
      if (poll_error != MOONSHINE_ERROR_NONE || partial == nullptr) {
        std::cerr << "partial inference failed\n";
        release();
        return 1;
      }
      ++partial_calls;
      // The pinned C API says transcript memory is transcriber-owned and stays
      // valid until the next transcribe call. Consume this result now; there is
      // no transcript free function and retaining it across the next poll is unsafe.
      partial_nonempty = partial_nonempty || has_nonempty_line(partial);
      next_poll = offset + static_cast<size_t>(wav.sample_rate);
    }
  }

  const int32_t stop_error = moonshine_stop_stream(transcriber, stream);
  const auto stopped_at = std::chrono::steady_clock::now();
  started = false;
  if (stop_error != MOONSHINE_ERROR_NONE) {
    std::cerr << "stop_stream failed: " << moonshine_error_to_string(stop_error) << '\n';
    release();
    return 1;
  }
  transcript_t* final_transcript = nullptr;
  const int32_t final_error = moonshine_transcribe_stream(transcriber, stream, 0, &final_transcript);
  const auto finalized_at = std::chrono::steady_clock::now();
  if (final_error != MOONSHINE_ERROR_NONE || final_transcript == nullptr) {
    std::cerr << "final inference failed\n";
    release();
    return 1;
  }
  bool all_complete = final_transcript->line_count > 0 && final_transcript->lines != nullptr;
  for (uint64_t i = 0;
       final_transcript->lines != nullptr && i < final_transcript->line_count; ++i) {
    all_complete = all_complete && final_transcript->lines[i].is_complete != 0;
  }
  const bool final_nonempty = has_nonempty_line(final_transcript);
  const uint64_t final_line_count = final_transcript->line_count;
  const double session_ms = std::chrono::duration<double, std::milli>(
                                finalized_at - session_started)
                                .count();
  const double stop_to_final_ms = std::chrono::duration<double, std::milli>(
                                      finalized_at - stopped_at)
                                      .count();
  const double audio_seconds = static_cast<double>(max_samples) / wav.sample_rate;
  const double real_time_factor = (session_ms / 1000.0) / audio_seconds;
  const auto cpu_finished = process_cpu_seconds();
  const std::optional<double> cpu_ms = cpu_started.has_value() && cpu_finished.has_value()
                                           ? std::optional<double>((*cpu_finished - *cpu_started) * 1000.0)
                                           : std::nullopt;
  (void)moonshine_free_stream(transcriber, stream);
  std::cout << "SESSION " << session_number << " partial_calls=" << partial_calls
            << " partial_nonempty=" << (partial_nonempty ? "yes" : "no")
            << " final_lines=" << final_line_count
            << " final_nonempty=" << (final_nonempty ? "yes" : "no")
            << " all_final_lines_complete=" << (all_complete ? "yes" : "no")
            << std::fixed << std::setprecision(3)
            << " session_ms=" << session_ms
            << " stop_to_final_ms=" << stop_to_final_ms
            << " rtf=" << real_time_factor
            << " cpu_ms=" << (cpu_ms.has_value() ? *cpu_ms : -1.0) << '\n';
  return (partial_calls > 0 && partial_nonempty && final_nonempty && all_complete) ? 0 : 2;
}

int main(int argc, char** argv) {
  if (argc != 3) {
    std::cerr << "usage: smoke MODEL_DIR WAV_PATH\n";
    return 2;
  }
  WavData wav;
  if (!read_pcm16_mono_wav(argv[2], wav)) {
    std::cerr << "fixture is not PCM16 mono WAV\n";
    return 2;
  }
  const moonshine_option_t options[] = {
      {"identify_speakers", "false"},
      {"ort_providers", "CPU"},
  };
  const auto model_load_started = std::chrono::steady_clock::now();
  const int32_t transcriber = moonshine_load_transcriber_from_files(
      argv[1], MOONSHINE_MODEL_ARCH_SMALL_STREAMING, options, 2, MOONSHINE_HEADER_VERSION);
  const auto model_load_finished = std::chrono::steady_clock::now();
  if (transcriber < 0) {
    std::cerr << "model load failed: " << moonshine_error_to_string(transcriber) << '\n';
    return 1;
  }
  const double model_load_ms = std::chrono::duration<double, std::milli>(
                                  model_load_finished - model_load_started)
                                  .count();
  std::cout << "MODEL_LOAD_OK architecture=" << MOONSHINE_MODEL_ARCH_SMALL_STREAMING
            << " execution_provider=CPUExecutionProvider"
            << " sample_rate=" << wav.sample_rate << " sample_count=" << wav.samples.size()
            << std::fixed << std::setprecision(1) << " model_load_ms=" << model_load_ms << '\n';
  int result = run_session(transcriber, wav, 1);
  if (result == 0) result = run_session(transcriber, wav, 2);
  moonshine_free_transcriber(transcriber);
  struct rusage usage {};
  if (getrusage(RUSAGE_SELF, &usage) == 0) {
    std::cout << "SMOKE_PROCESS_MAX_RSS_BYTES=" << usage.ru_maxrss << '\n';
  }
  return result;
}
