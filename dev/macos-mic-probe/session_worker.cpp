#include "session_worker.hpp"

#include "model_verifier.hpp"

#include <mach/mach.h>
#include <sys/resource.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstring>
#include <limits>
#include <memory>
#include <sstream>
#include <utility>
#include <vector>

namespace outflow::mic_probe {
namespace {

using Clock = std::chrono::steady_clock;

void secure_zero(void* data, std::size_t byte_count) noexcept {
  auto* bytes = static_cast<volatile unsigned char*>(data);
  for (std::size_t index = 0; index < byte_count; ++index) bytes[index] = 0;
}

void clear_sensitive_text(std::string& text) noexcept {
  if (!text.empty()) secure_zero(text.data(), text.size());
  text.clear();
}

void clear_audio_samples(std::vector<float>& samples) noexcept {
  if (!samples.empty()) secure_zero(samples.data(), samples.size() * sizeof(float));
  samples.clear();
}

struct TextScrubber final {
  std::string& text;
  ~TextScrubber() { clear_sensitive_text(text); }
};

struct AudioScrubber final {
  std::vector<float>& samples;
  ~AudioScrubber() { clear_audio_samples(samples); }
};

double elapsed_ms(std::uint64_t start_ns, std::uint64_t end_ns) noexcept {
  if (start_ns == 0 || end_ns < start_ns) return 0.0;
  return static_cast<double>(end_ns - start_ns) / 1'000'000.0;
}

double usage_cpu_ms(const rusage& usage) noexcept {
  return (static_cast<double>(usage.ru_utime.tv_sec) * 1'000.0 +
          static_cast<double>(usage.ru_utime.tv_usec) / 1'000.0 +
          static_cast<double>(usage.ru_stime.tv_sec) * 1'000.0 +
          static_cast<double>(usage.ru_stime.tv_usec) / 1'000.0);
}

std::uint64_t current_peak_rss_bytes() noexcept {
  rusage usage{};
  if (getrusage(RUSAGE_SELF, &usage) != 0 || usage.ru_maxrss < 0) return 0;
  // macOS reports ru_maxrss in bytes; Linux reports KiB, but this is a macOS
  // only development target and the value is labeled as the native process.
  return static_cast<std::uint64_t>(usage.ru_maxrss);
}

std::uint64_t current_thread_count() noexcept {
  thread_act_array_t threads = nullptr;
  mach_msg_type_number_t count = 0;
  if (task_threads(mach_task_self(), &threads, &count) != KERN_SUCCESS) return 0;
  const auto result = static_cast<std::uint64_t>(count);
  if (threads != nullptr) {
    vm_deallocate(mach_task_self(), reinterpret_cast<vm_address_t>(threads),
                  static_cast<vm_size_t>(count) * sizeof(thread_t));
  }
  return result;
}

bool copy_transcript(const transcript_t* transcript, std::string& text) {
  text.clear();
  if (transcript == nullptr) return true;
  if (transcript->line_count > 10'000 ||
      (transcript->line_count != 0 && transcript->lines == nullptr)) {
    return false;
  }
  constexpr std::size_t kMaxTranscriptBytes = 1024 * 1024;
  for (std::uint64_t index = 0; index < transcript->line_count; ++index) {
    const char* line = transcript->lines[index].text;
    if (line == nullptr) continue;
    const auto separator_bytes = text.empty() ? 0U : 1U;
    if (text.size() + separator_bytes > kMaxTranscriptBytes) return false;
    const auto remaining = kMaxTranscriptBytes - text.size() - separator_bytes;
    const auto length = strnlen(line, remaining + 1);
    if (length > remaining) return false;
    if (length == 0) continue;
    if (!text.empty()) text.push_back('\n');
    text.append(line, length);
  }
  return true;
}

struct NativeHandles {
  MoonshineApi& api;
  std::int32_t transcriber = -1;
  std::int32_t stream = -1;
  bool stream_started = false;
  bool stream_stopped = false;

  void release() noexcept {
    if (stream >= 0) {
      if (stream_started && !stream_stopped) {
        (void)api.stop_stream(transcriber, stream);
        stream_stopped = true;
      }
      (void)api.free_stream(transcriber, stream);
      stream = -1;
      stream_started = false;
    }
    if (transcriber >= 0) {
      api.free_transcriber(transcriber);
      transcriber = -1;
    }
  }

  ~NativeHandles() { release(); }
};

}  // namespace

std::uint64_t monotonic_time_ns() noexcept {
  return static_cast<std::uint64_t>(
      std::chrono::duration_cast<std::chrono::nanoseconds>(
          Clock::now().time_since_epoch())
          .count());
}

SessionWorker::SessionWorker(std::uint64_t session_id,
                             std::filesystem::path model_directory,
                             AudioSourceKind source,
                             WorkerEventHandler event_handler, MoonshineApi api,
                             ModelVerifier verifier,
                             std::uint64_t partial_poll_interval_ns)
    : session_id_(session_id),
      model_directory_(std::move(model_directory)),
      source_(source),
      event_handler_(std::move(event_handler)),
      api_(api),
      verifier_(std::move(verifier)),
      partial_poll_interval_ns_(partial_poll_interval_ns) {
  if (!verifier_) verifier_ = verify_model_directory;
}

SessionWorker::~SessionWorker() {
  request_cancel();
  join();
}

bool SessionWorker::start() noexcept {
  bool expected = false;
  if (!started_.compare_exchange_strong(expected, true, std::memory_order_acq_rel)) {
    return false;
  }
  try {
    thread_ = std::thread(&SessionWorker::run, this);
    return true;
  } catch (...) {
    started_.store(false, std::memory_order_release);
    return false;
  }
}

void SessionWorker::set_capture_start_time(std::uint64_t monotonic_ns) noexcept {
  capture_start_ns_.store(monotonic_ns, std::memory_order_release);
}

bool SessionWorker::request_finalize(std::uint64_t stop_request_ns,
                                     std::uint64_t microphone_release_ns) noexcept {
  if (cancel_requested_.load(std::memory_order_acquire) ||
      finalize_requested_.load(std::memory_order_acquire)) return false;
  stop_request_ns_.store(stop_request_ns, std::memory_order_release);
  microphone_release_ns_.store(microphone_release_ns, std::memory_order_release);
  finalize_requested_.store(true, std::memory_order_release);
  audio_queue_.wake();
  return true;
}

void SessionWorker::request_cancel() noexcept {
  cancel_requested_.store(true, std::memory_order_release);
  audio_queue_.wake();
}

void SessionWorker::request_failure(std::int32_t code,
                                    std::string message) noexcept {
  external_error_message_ = std::move(message);
  external_error_code_.store(code == 0 ? -1 : code, std::memory_order_release);
  request_cancel();
}

void SessionWorker::join() noexcept {
  if (thread_.joinable()) {
    try {
      thread_.join();
    } catch (...) {
    }
  }
  audio_queue_.clear_after_producer_stops();
}

BoundedAudioQueue& SessionWorker::audio_queue() noexcept { return audio_queue_; }

bool SessionWorker::is_joinable() const noexcept { return thread_.joinable(); }

void SessionWorker::emit(WorkerEvent event) noexcept {
  if (!event_handler_) return;
  try {
    event_handler_(std::move(event));
  } catch (...) {
  }
}

void SessionWorker::run() noexcept {
  const auto session_started_ns = monotonic_time_ns();
  rusage cpu_start{};
  (void)getrusage(RUSAGE_SELF, &cpu_start);
  NativeHandles native{api_};
  SessionMetrics metrics{};
  metrics.source = source_;
  std::string error_message;
  std::int32_t error_code = 0;

  try {
    const auto model_started_ns = monotonic_time_ns();
    if (cancel_requested_.load(std::memory_order_acquire)) {
      WorkerEvent cancelled;
      cancelled.kind = WorkerEventKind::Cancelled;
      cancelled.session_id = session_id_;
      emit(std::move(cancelled));
      return;
    }
    if (!verifier_(model_directory_, error_message)) {
      error_code = -100;
      if (error_message.empty()) error_message = "model asset verification failed";
    } else if (!cancel_requested_.load(std::memory_order_acquire)) {
      const moonshine_option_t options[] = {
          {"identify_speakers", "false"},
          {"ort_providers", "CPU"},
      };
      native.transcriber = api_.load(model_directory_.c_str(),
                                     MOONSHINE_MODEL_ARCH_SMALL_STREAMING,
                                     options, 2, MOONSHINE_HEADER_VERSION);
      metrics.model_load_ms = elapsed_ms(model_started_ns, monotonic_time_ns());
      if (native.transcriber < 0) {
        error_code = native.transcriber;
        error_message = "Moonshine model initialization failed (native error " +
                        std::to_string(error_code) + ")";
      }
    }
    // A cancellation that arrives while the verifier or model loader is
    // running wins over a load error. Loading is not interruptible, but a
    // completed transcriber is immediately released and never opens capture.
    if (cancel_requested_.load(std::memory_order_acquire)) {
      native.release();
      WorkerEvent cancelled;
      cancelled.kind = WorkerEventKind::Cancelled;
      cancelled.session_id = session_id_;
      emit(std::move(cancelled));
      return;
    }
    if (error_code != 0) {
      native.release();
      emit(WorkerEvent{WorkerEventKind::Error, session_id_, {}, error_message,
                       error_code, metrics});
      return;
    }
    if (error_code == 0) {
      native.stream = api_.create_stream(native.transcriber, 0);
      if (native.stream < 0) {
        error_code = native.stream;
        error_message = "Moonshine stream creation failed (native error " +
                        std::to_string(error_code) + ")";
      }
    }
    if (cancel_requested_.load(std::memory_order_acquire)) {
      native.release();
      WorkerEvent cancelled;
      cancelled.kind = WorkerEventKind::Cancelled;
      cancelled.session_id = session_id_;
      emit(std::move(cancelled));
      return;
    }
    if (error_code == 0) {
      error_code = api_.start_stream(native.transcriber, native.stream);
      if (error_code != MOONSHINE_ERROR_NONE) {
        error_message = "Moonshine stream start failed (native error " +
                        std::to_string(error_code) + ")";
      } else {
        native.stream_started = true;
      }
    }
    if (cancel_requested_.load(std::memory_order_acquire)) {
      native.release();
      WorkerEvent cancelled;
      cancelled.kind = WorkerEventKind::Cancelled;
      cancelled.session_id = session_id_;
      emit(std::move(cancelled));
      return;
    }
    if (error_code != 0) {
      native.release();
      emit(WorkerEvent{WorkerEventKind::Error, session_id_, {}, error_message,
                       error_code, metrics});
      return;
    }

    WorkerEvent ready;
    ready.kind = WorkerEventKind::Ready;
    ready.session_id = session_id_;
    emit(std::move(ready));
    MonoResampler resampler;
    std::string latest_text;
    TextScrubber latest_text_scrubber{latest_text};
    std::vector<float> converted;
    AudioScrubber converted_scrubber{converted};
    converted.reserve(kMaxCaptureFrames * 2 + 2);
    std::uint64_t input_samples = 0;
    std::uint64_t first_frame_ns = 0;
    std::uint64_t first_partial_ns = 0;
    std::uint64_t previous_partial_ns = 0;
    double cadence_sum_ms = 0.0;
    std::uint64_t last_poll_ns = monotonic_time_ns();

    while (true) {
      if (external_error_code_.load(std::memory_order_acquire) != 0) {
        error_code = external_error_code_.load(std::memory_order_acquire);
        error_message = external_error_message_;
        break;
      }
      if (cancel_requested_.load(std::memory_order_acquire)) break;
      if (audio_queue_.overflowed()) {
        error_code = -101;
        error_message = "Audio queue overflow; capture was stopped safely";
        break;
      }
      if (audio_queue_.producer_error() != 0) {
        error_code = audio_queue_.producer_error();
        error_message = error_code == -106
            ? "Microphone input changed or disconnected; recording was stopped"
            : error_code == -105
                ? "Microphone stream format changed after input was opened"
                : error_code == -109
                    ? "Microphone callback returned an unsupported buffer layout"
                    : error_code <= -111 && error_code >= -115
                        ? "Microphone PCM buffer was rejected by conversion checks (reason " +
                              std::to_string(-110 - error_code) + ")"
                        : "Audio capture reported an unsupported or invalid input format";
        break;
      }

      bool consumed_audio = false;
      while (!cancel_requested_.load(std::memory_order_acquire)) {
        const auto* block = audio_queue_.begin_read();
        if (block == nullptr) break;
        consumed_audio = true;
        if (block->frame_count == 0 || !resampler.process(*block, converted)) {
          audio_queue_.release_read();
          error_code = -102;
          error_message = "Microphone format changed or could not be converted";
          break;
        }
        if (first_frame_ns == 0) {
          first_frame_ns = block->captured_at_monotonic_ns;
          metrics.first_frame_after_capture_start_ms = elapsed_ms(
              capture_start_ns_.load(std::memory_order_acquire), first_frame_ns);
        }
        input_samples += block->frame_count;
        const auto add_error = api_.add_audio(
            native.transcriber, native.stream, converted.data(), converted.size(),
            kModelSampleRateHz, 0);
        audio_queue_.release_read();
        if (add_error != MOONSHINE_ERROR_NONE) {
          error_code = add_error;
          error_message = "Moonshine rejected microphone audio (native error " +
                          std::to_string(error_code) + ")";
          break;
        }
        if (cancel_requested_.load(std::memory_order_acquire)) break;
        const auto now_ns = monotonic_time_ns();
        if (now_ns - last_poll_ns >= partial_poll_interval_ns_) {
          transcript_t* partial = nullptr;
          const auto poll_error = api_.transcribe_stream(
              native.transcriber, native.stream, 0, &partial);
          if (cancel_requested_.load(std::memory_order_acquire)) break;
          if (poll_error != MOONSHINE_ERROR_NONE) {
            error_code = poll_error;
            error_message = "Moonshine partial recognition failed (native error " +
                            std::to_string(error_code) + ")";
            break;
          }
          std::string snapshot;
          TextScrubber snapshot_scrubber{snapshot};
          if (!copy_transcript(partial, snapshot)) {
            error_code = -103;
            error_message = "Moonshine returned an invalid or oversized transcript";
            break;
          }
          if (cancel_requested_.load(std::memory_order_acquire)) break;
          if (!snapshot.empty() && snapshot != latest_text) {
            const auto partial_ns = monotonic_time_ns();
            if (first_partial_ns == 0) {
              first_partial_ns = partial_ns;
              metrics.first_partial_after_first_frame_ms =
                  elapsed_ms(first_frame_ns, partial_ns);
            }
            if (previous_partial_ns != 0) {
              cadence_sum_ms += elapsed_ms(previous_partial_ns, partial_ns);
            }
            previous_partial_ns = partial_ns;
            latest_text = snapshot;
            ++metrics.partial_update_count;
            emit(WorkerEvent{WorkerEventKind::Partial, session_id_,
                             std::move(snapshot), {}, 0, metrics});
          }
          last_poll_ns = now_ns;
        }
      }
      if (error_code != 0 || cancel_requested_.load(std::memory_order_acquire)) break;
      if (finalize_requested_.load(std::memory_order_acquire) &&
          audio_queue_.depth() == 0) {
        break;
      }
      if (consumed_audio) continue;
      const auto observed = audio_queue_.wake_generation();
      if (audio_queue_.depth() == 0 &&
          !cancel_requested_.load(std::memory_order_acquire) &&
          !finalize_requested_.load(std::memory_order_acquire) &&
          !audio_queue_.overflowed()) {
        audio_queue_.wait_for_wake(observed);
      }
    }

    if (external_error_code_.load(std::memory_order_acquire) != 0) {
      error_code = external_error_code_.load(std::memory_order_acquire);
      error_message = external_error_message_;
    }
    if (cancel_requested_.load(std::memory_order_acquire) && error_code == 0) {
      native.release();
      clear_sensitive_text(latest_text);
      WorkerEvent cancelled;
      cancelled.kind = WorkerEventKind::Cancelled;
      cancelled.session_id = session_id_;
      emit(std::move(cancelled));
      return;
    }
    if (error_code != 0) {
      native.release();
      clear_sensitive_text(latest_text);
      WorkerEvent failed;
      failed.kind = WorkerEventKind::Error;
      failed.session_id = session_id_;
      failed.error_message = error_message;
      failed.native_error_code = error_code;
      emit(std::move(failed));
      return;
    }

    const auto stop_error = api_.stop_stream(native.transcriber, native.stream);
    native.stream_stopped = true;
    if (stop_error != MOONSHINE_ERROR_NONE) {
      error_code = stop_error;
      error_message = "Moonshine stream stop failed (native error " +
                      std::to_string(error_code) + ")";
    }
    transcript_t* final_transcript = nullptr;
    if (error_code == 0) {
      error_code = api_.transcribe_stream(native.transcriber, native.stream, 0,
                                          &final_transcript);
      if (error_code != MOONSHINE_ERROR_NONE) {
        error_message = "Moonshine final recognition failed (native error " +
                        std::to_string(error_code) + ")";
      }
    }
    std::string final_text;
    TextScrubber final_text_scrubber{final_text};
    if (error_code == 0 && !copy_transcript(final_transcript, final_text)) {
      error_code = -103;
      error_message = "Moonshine returned an invalid or oversized final transcript";
    }
    const auto final_ready_ns = monotonic_time_ns();
    if (cancel_requested_.load(std::memory_order_acquire)) {
      clear_sensitive_text(final_text);
      clear_sensitive_text(latest_text);
      native.release();
      WorkerEvent cancelled;
      cancelled.kind = WorkerEventKind::Cancelled;
      cancelled.session_id = session_id_;
      emit(std::move(cancelled));
      return;
    }
    if (error_code != 0) {
      native.release();
      WorkerEvent failed;
      failed.kind = WorkerEventKind::Error;
      failed.session_id = session_id_;
      failed.error_message = error_message;
      failed.native_error_code = error_code;
      emit(std::move(failed));
      return;
    }

    const auto stop_ns = stop_request_ns_.load(std::memory_order_acquire);
    const auto release_ns = microphone_release_ns_.load(std::memory_order_acquire);
    metrics.audio_duration_seconds =
        static_cast<double>(input_samples) / std::max<std::uint32_t>(
                                                resampler.source_rate_hz(), 1);
    metrics.capture_duration_ms = elapsed_ms(
        capture_start_ns_.load(std::memory_order_acquire), stop_ns);
    metrics.stop_to_final_ms = elapsed_ms(stop_ns, final_ready_ns);
    metrics.total_session_ms = elapsed_ms(session_started_ns, final_ready_ns);
    metrics.microphone_release_latency_ms = elapsed_ms(stop_ns, release_ns);
    metrics.queue_overflow_count = audio_queue_.overflow_count();
    metrics.queue_maximum_depth = audio_queue_.maximum_depth();
    if (metrics.partial_update_count > 1) {
      metrics.mean_partial_cadence_ms =
          cadence_sum_ms / static_cast<double>(metrics.partial_update_count - 1);
    }
    metrics.peak_rss_bytes = current_peak_rss_bytes();
    metrics.native_thread_count = current_thread_count();
    rusage cpu_end{};
    if (getrusage(RUSAGE_SELF, &cpu_end) == 0) {
      metrics.process_cpu_ms = usage_cpu_ms(cpu_end) - usage_cpu_ms(cpu_start);
    }
    native.release();
    clear_sensitive_text(latest_text);
    WorkerEvent result;
    result.kind = WorkerEventKind::Final;
    result.session_id = session_id_;
    result.text = std::move(final_text);
    result.metrics = metrics;
    emit(std::move(result));
  } catch (const std::exception&) {
    native.release();
    WorkerEvent failed;
    failed.kind = WorkerEventKind::Error;
    failed.session_id = session_id_;
    failed.error_message = "Native recognition worker failed safely";
    failed.native_error_code = -104;
    emit(std::move(failed));
  } catch (...) {
    native.release();
    WorkerEvent failed;
    failed.kind = WorkerEventKind::Error;
    failed.session_id = session_id_;
    failed.error_message = "Native recognition worker failed safely";
    failed.native_error_code = -104;
    emit(std::move(failed));
  }
}

}  // namespace outflow::mic_probe
