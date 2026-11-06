// Development-only batch evaluator for verified offline benchmark fixtures.
// Standard output is a private pipe protocol containing raw hypotheses; the
// Python caller must capture it in memory and persist only aggregate metrics.
#include "moonshine-c-api.h"

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <iterator>
#include <limits.h>
#include <optional>
#include <string>
#include <sys/resource.h>
#include <utility>
#include <vector>
#include <dlfcn.h>
#include <mach-o/dyld.h>

struct WavData {
  int sample_rate = 0;
  std::vector<float> samples;
};

struct SessionResult {
  std::string hypothesis;
  size_t partial_calls = 0;
  bool partial_nonempty = false;
  uint64_t final_lines = 0;
  double recognition_ms = 0.0;
  double stop_to_final_ms = 0.0;
  double real_time_factor = 0.0;
  std::optional<double> cpu_ms;
};

static uint16_t le16(const std::vector<uint8_t>& bytes, size_t offset) {
  return static_cast<uint16_t>(bytes[offset]) |
         (static_cast<uint16_t>(bytes[offset + 1]) << 8);
}

static uint32_t le32(const std::vector<uint8_t>& bytes, size_t offset) {
  return static_cast<uint32_t>(bytes[offset]) |
         (static_cast<uint32_t>(bytes[offset + 1]) << 8) |
         (static_cast<uint32_t>(bytes[offset + 2]) << 16) |
         (static_cast<uint32_t>(bytes[offset + 3]) << 24);
}

static bool read_pcm16_mono_wav(const char* path, WavData& output) {
  std::ifstream input(path, std::ios::binary);
  if (!input) return false;
  constexpr size_t kMaxWavBytes = 64U * 1024U * 1024U;
  input.seekg(0, std::ios::end);
  const std::streamoff file_size = input.tellg();
  if (file_size < 44 || static_cast<uint64_t>(file_size) > kMaxWavBytes) return false;
  input.seekg(0, std::ios::beg);
  std::vector<uint8_t> bytes(static_cast<size_t>(file_size));
  input.read(reinterpret_cast<char*>(bytes.data()), file_size);
  if (!input || bytes.size() > kMaxWavBytes ||
      std::string(reinterpret_cast<const char*>(bytes.data()), 4) != "RIFF" ||
      std::string(reinterpret_cast<const char*>(bytes.data() + 8), 4) != "WAVE") {
    return false;
  }

  uint16_t format = 0;
  uint16_t channels = 0;
  uint16_t bits = 0;
  uint32_t sample_rate = 0;
  size_t data_offset = 0;
  size_t data_size = 0;
  for (size_t offset = 12; offset + 8 <= bytes.size();) {
    const uint32_t chunk_size = le32(bytes, offset + 4);
    const size_t payload = offset + 8;
    if (chunk_size > bytes.size() - payload) return false;
    const std::string chunk_id(reinterpret_cast<const char*>(bytes.data() + offset), 4);
    if (chunk_id == "fmt " && chunk_size >= 16) {
      format = le16(bytes, payload);
      channels = le16(bytes, payload + 2);
      sample_rate = le32(bytes, payload + 4);
      bits = le16(bytes, payload + 14);
    } else if (chunk_id == "data") {
      data_offset = payload;
      data_size = chunk_size;
    }
    const size_t padded_size = static_cast<size_t>(chunk_size) + (chunk_size & 1U);
    if (padded_size > bytes.size() - payload) return false;
    offset = payload + padded_size;
  }

  if (format != 1 || channels != 1 || bits != 16 || sample_rate == 0 ||
      data_size == 0 || data_size % 2 != 0 || data_offset > bytes.size() ||
      data_size > bytes.size() - data_offset) {
    return false;
  }
  const double seconds = static_cast<double>(data_size / 2) / sample_rate;
  if (seconds <= 0.0 || seconds > 120.0) return false;

  output.sample_rate = static_cast<int>(sample_rate);
  output.samples.reserve(data_size / 2);
  for (size_t offset = data_offset; offset < data_offset + data_size; offset += 2) {
    const int16_t sample = static_cast<int16_t>(le16(bytes, offset));
    output.samples.push_back(static_cast<float>(sample) / 32768.0f);
  }
  return true;
}

static bool has_nonempty_line(const transcript_t* transcript) {
  if (transcript == nullptr ||
      (transcript->line_count > 0 && transcript->lines == nullptr)) {
    return false;
  }
  for (uint64_t index = 0; index < transcript->line_count; ++index) {
    const char* text = transcript->lines[index].text;
    if (text != nullptr && *text != '\0') return true;
  }
  return false;
}

static std::string collect_final_text(const transcript_t* transcript) {
  std::string result;
  if (transcript == nullptr ||
      (transcript->line_count > 0 && transcript->lines == nullptr)) {
    return result;
  }
  for (uint64_t index = 0; index < transcript->line_count; ++index) {
    const char* text = transcript->lines[index].text;
    if (text == nullptr || *text == '\0') continue;
    if (!result.empty()) result.push_back(' ');
    result.append(text);
  }
  return result;
}

static double timeval_seconds(const timeval& value) {
  return static_cast<double>(value.tv_sec) + static_cast<double>(value.tv_usec) / 1'000'000.0;
}

static std::optional<double> process_cpu_seconds() {
  struct rusage usage {};
  if (getrusage(RUSAGE_SELF, &usage) != 0) return std::nullopt;
  return timeval_seconds(usage.ru_utime) + timeval_seconds(usage.ru_stime);
}

static bool run_session(int32_t transcriber, const WavData& wav, SessionResult& result) {
  const auto session_started = std::chrono::steady_clock::now();
  const std::optional<double> cpu_started = process_cpu_seconds();
  const int32_t stream = moonshine_create_stream(transcriber, 0);
  if (stream < 0) return false;
  bool started = false;
  auto release = [&]() {
    if (started) (void)moonshine_stop_stream(transcriber, stream);
    (void)moonshine_free_stream(transcriber, stream);
  };

  const int32_t start_error = moonshine_start_stream(transcriber, stream);
  if (start_error != MOONSHINE_ERROR_NONE) {
    release();
    return false;
  }
  started = true;

  const size_t chunk_samples = std::max<size_t>(1, static_cast<size_t>(wav.sample_rate) * 3 / 10);
  size_t next_poll = static_cast<size_t>(wav.sample_rate);
  for (size_t offset = 0; offset < wav.samples.size();) {
    const size_t count = std::min(chunk_samples, wav.samples.size() - offset);
    const int32_t add_error = moonshine_transcribe_add_audio_to_stream(
        transcriber, stream, wav.samples.data() + offset, count, wav.sample_rate, 0);
    if (add_error != MOONSHINE_ERROR_NONE) {
      release();
      return false;
    }
    offset += count;
    if (offset >= next_poll) {
      transcript_t* partial = nullptr;
      const int32_t poll_error = moonshine_transcribe_stream(
          transcriber, stream, MOONSHINE_FLAG_FORCE_UPDATE, &partial);
      if (poll_error != MOONSHINE_ERROR_NONE || partial == nullptr) {
        release();
        return false;
      }
      ++result.partial_calls;
      // Consume each library-owned transcript before the next C API call; the
      // pinned C API keeps it valid only until the next transcribe operation.
      result.partial_nonempty = result.partial_nonempty || has_nonempty_line(partial);
      next_poll = offset + static_cast<size_t>(wav.sample_rate);
    }
  }

  const int32_t stop_error = moonshine_stop_stream(transcriber, stream);
  const auto stopped_at = std::chrono::steady_clock::now();
  started = false;
  if (stop_error != MOONSHINE_ERROR_NONE) {
    release();
    return false;
  }
  transcript_t* final_transcript = nullptr;
  const int32_t final_error = moonshine_transcribe_stream(transcriber, stream, 0, &final_transcript);
  const auto finalized_at = std::chrono::steady_clock::now();
  if (final_error != MOONSHINE_ERROR_NONE || final_transcript == nullptr) {
    release();
    return false;
  }

  result.hypothesis = collect_final_text(final_transcript);
  result.final_lines = final_transcript->line_count;
  result.recognition_ms = std::chrono::duration<double, std::milli>(
                              finalized_at - session_started).count();
  result.stop_to_final_ms = std::chrono::duration<double, std::milli>(
                                finalized_at - stopped_at).count();
  const double audio_seconds = static_cast<double>(wav.samples.size()) / wav.sample_rate;
  result.real_time_factor = (result.recognition_ms / 1000.0) / audio_seconds;
  const std::optional<double> cpu_finished = process_cpu_seconds();
  if (cpu_started.has_value() && cpu_finished.has_value()) {
    result.cpu_ms = (*cpu_finished - *cpu_started) * 1000.0;
  }
  (void)moonshine_free_stream(transcriber, stream);
  return true;
}

static void write_json_string(const std::string& value) {
  static constexpr char hex[] = "0123456789abcdef";
  std::cout << '"';
  for (unsigned char character : value) {
    switch (character) {
      case '"': std::cout << "\\\""; break;
      case '\\': std::cout << "\\\\"; break;
      case '\b': std::cout << "\\b"; break;
      case '\f': std::cout << "\\f"; break;
      case '\n': std::cout << "\\n"; break;
      case '\r': std::cout << "\\r"; break;
      case '\t': std::cout << "\\t"; break;
      default:
        if (character < 0x20) {
          std::cout << "\\u00" << hex[(character >> 4) & 0x0f] << hex[character & 0x0f];
        } else {
          std::cout << static_cast<char>(character);
        }
    }
  }
  std::cout << '"';
}

static bool parse_model_architecture(const char* value, int32_t& architecture) {
  if (value == nullptr || *value == '\0') return false;
  char* end = nullptr;
  const long parsed = std::strtol(value, &end, 10);
  if (end == value || *end != '\0' || parsed < 0 || parsed > INT32_MAX) return false;
  if (parsed != MOONSHINE_MODEL_ARCH_SMALL_STREAMING &&
      parsed != MOONSHINE_MODEL_ARCH_TINY_STREAMING) {
    return false;
  }
  architecture = static_cast<int32_t>(parsed);
  return true;
}

static int32_t load_transcriber(const char* model_directory, int32_t architecture) {
  // The pinned C API documents CPU as its default. Select it explicitly so
  // benchmark records prove the accelerator-free path was requested.
  const moonshine_option_t options[] = {
      {"identify_speakers", "false"},
      {"ort_providers", "CPU"},
  };
  return moonshine_load_transcriber_from_files(
      model_directory,
      architecture,
      options,
      2,
      MOONSHINE_HEADER_VERSION);
}

static std::string resolved_path(const char* path) {
  if (path == nullptr) return {};
  char buffer[PATH_MAX];
  if (realpath(path, buffer) == nullptr) return {};
  return std::string(buffer);
}

static int run_runtime_identity(const char* executable_path) {
#if defined(__arm64__) || defined(__aarch64__)
  constexpr const char* architecture = "arm64";
#elif defined(__x86_64__)
  constexpr const char* architecture = "x86_64";
#else
  constexpr const char* architecture = "unsupported";
#endif
  Dl_info api_info{};
  if (dladdr(reinterpret_cast<const void*>(&moonshine_get_version), &api_info) == 0) {
    std::cerr << "Moonshine C API symbol identity unavailable\n";
    return 1;
  }
  const std::string runner = resolved_path(executable_path);
  const std::string api_path = resolved_path(api_info.dli_fname);
  if (runner.empty() || api_path.empty()) {
    std::cerr << "Native runtime path resolution failed\n";
    return 1;
  }
  std::vector<std::string> images;
  const uint32_t image_count = _dyld_image_count();
  images.reserve(image_count);
  for (uint32_t index = 0; index < image_count; ++index) {
    const char* image_name = _dyld_get_image_name(index);
    std::string path = resolved_path(image_name);
    if (path.empty() && image_name != nullptr) {
      const std::string reported(image_name);
      // macOS maps many Apple system libraries from the dyld shared cache;
      // their logical paths do not resolve to standalone files on disk.
      // Keep those system image names for allowlist checks, but require every
      // non-system image to resolve to an actual file.
      if (reported.rfind("/usr/lib/", 0) == 0 ||
          reported.rfind("/System/Library/", 0) == 0) {
        path = reported;
      }
    }
    if (path.empty()) {
      std::cerr << "A loaded native image path could not be resolved\n";
      return 1;
    }
    images.push_back(path);
  }
  std::cout << "{\"schema_version\":1,\"architecture\":\"" << architecture
            << "\",\"pointer_bits\":" << sizeof(void*) * 8
            << ",\"c_api_version\":" << moonshine_get_version()
            << ",\"runner_path\":";
  write_json_string(runner);
  std::cout << ",\"moonshine_api_path\":";
  write_json_string(api_path);
  std::cout << ",\"loaded_images\":[";
  for (size_t index = 0; index < images.size(); ++index) {
    if (index != 0) std::cout << ',';
    write_json_string(images[index]);
  }
  std::cout << "]}\n";
  return std::cout.good() ? 0 : 1;
}

static int run_load_probe(int argc, char** argv) {
  if (argc != 4 && argc != 5) {
    std::cerr << "load probe usage: moonshine_asr_batch --load-probe MODEL_DIR REPEATS [ARCHITECTURE]\n";
    return 2;
  }
  int32_t architecture = MOONSHINE_MODEL_ARCH_SMALL_STREAMING;
  if (argc == 5 && !parse_model_architecture(argv[4], architecture)) {
    std::cerr << "load probe architecture must be Small Streaming (4) or Tiny Streaming (2)\n";
    return 2;
  }
  char* end = nullptr;
  const long repetitions = std::strtol(argv[3], &end, 10);
  if (end == argv[3] || *end != '\0' || repetitions < 2 || repetitions > 10) {
    std::cerr << "load probe repetitions must be an integer from 2 through 10\n";
    return 2;
  }
  std::vector<double> load_ms;
  for (long index = 0; index < repetitions; ++index) {
    const auto started = std::chrono::steady_clock::now();
    const int32_t transcriber = load_transcriber(argv[2], architecture);
    const auto finished = std::chrono::steady_clock::now();
    if (transcriber < 0) {
      std::cerr << "Moonshine model load failed\n";
      return 1;
    }
    load_ms.push_back(std::chrono::duration<double, std::milli>(finished - started).count());
    moonshine_free_transcriber(transcriber);
  }
  std::cout << "{\"schema_version\":1,\"execution_provider\":\"CPUExecutionProvider\",\"architecture\":"
            << architecture << ",\"header_version\":" << MOONSHINE_HEADER_VERSION
            << ",\"model_load_ms\":[";
  for (size_t index = 0; index < load_ms.size(); ++index) {
    if (index != 0) std::cout << ',';
    std::cout << std::fixed << std::setprecision(3) << load_ms[index];
  }
  std::cout << "]}\n";
  return 0;
}

int main(int argc, char** argv) {
  if (argc == 2 && std::string(argv[1]) == "--runtime-identity") {
    return run_runtime_identity(argv[0]);
  }
  if (argc >= 2 && std::string(argv[1]) == "--load-probe") return run_load_probe(argc, argv);
  int32_t architecture = MOONSHINE_MODEL_ARCH_SMALL_STREAMING;
  int first_wav = 2;
  int model_directory_index = 1;
  if (argc >= 2 && std::string(argv[1]) == "--model-arch") {
    if (argc < 5 || !parse_model_architecture(argv[2], architecture)) {
      std::cerr << "usage: moonshine_asr_batch --model-arch {2|4} MODEL_DIR WAV_PATH...\n";
      return 2;
    }
    model_directory_index = 3;
    first_wav = 4;
  }
  if (argc < first_wav + 1) {
    std::cerr << "usage: moonshine_asr_batch [--model-arch {2|4}] MODEL_DIR WAV_PATH...\n";
    return 2;
  }

  const auto model_started = std::chrono::steady_clock::now();
  const int32_t transcriber = load_transcriber(argv[model_directory_index], architecture);
  const auto model_finished = std::chrono::steady_clock::now();
  if (transcriber < 0) {
    std::cerr << "Moonshine model load failed\n";
    return 1;
  }
  const double model_load_ms = std::chrono::duration<double, std::milli>(
                                   model_finished - model_started).count();

  std::vector<SessionResult> results;
  results.reserve(static_cast<size_t>(argc - first_wav));
  for (int index = first_wav; index < argc; ++index) {
    WavData wav;
    SessionResult result;
    if (!read_pcm16_mono_wav(argv[index], wav) || !run_session(transcriber, wav, result)) {
      moonshine_free_transcriber(transcriber);
      std::cerr << "invalid benchmark WAV or Moonshine inference failed\n";
      return 1;
    }
    results.push_back(std::move(result));
  }
  moonshine_free_transcriber(transcriber);

  std::cout << "{\"schema_version\":1,\"execution_provider\":\"CPUExecutionProvider\",\"architecture\":"
            << architecture << ",\"header_version\":"
            << MOONSHINE_HEADER_VERSION << ",\"model_load_ms\":"
            << std::fixed << std::setprecision(3) << model_load_ms << ",\"outputs\":[";
  for (size_t index = 0; index < results.size(); ++index) {
    if (index != 0) std::cout << ',';
    const SessionResult& result = results[index];
    std::cout << "{\"index\":" << index << ",\"hypothesis\":";
    write_json_string(result.hypothesis);
    std::cout << ",\"partial_calls\":" << result.partial_calls
              << ",\"partial_nonempty\":" << (result.partial_nonempty ? "true" : "false")
              << ",\"final_lines\":" << result.final_lines
              << ",\"recognition_ms\":" << result.recognition_ms
              << ",\"stop_to_final_ms\":" << result.stop_to_final_ms
              << ",\"real_time_factor\":" << result.real_time_factor
              << ",\"cpu_ms\":";
    if (result.cpu_ms.has_value()) std::cout << *result.cpu_ms;
    else std::cout << "null";
    std::cout << '}';
  }
  std::cout << "]}\n";
  return std::cout.good() ? 0 : 1;
}
