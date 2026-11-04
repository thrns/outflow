#pragma once

#include "audio_pipeline.hpp"

#include <moonshine-c-api.h>

#include <atomic>
#include <cstdint>
#include <filesystem>
#include <functional>
#include <string>
#include <thread>

namespace outflow::mic_probe {

enum class WorkerEventKind : std::uint8_t { Ready, Partial, Final, Cancelled, Error };
enum class AudioSourceKind : std::uint8_t { Microphone, TimedFixture };

struct SessionMetrics {
  AudioSourceKind source = AudioSourceKind::Microphone;
  double model_load_ms = 0.0;
  double first_frame_after_capture_start_ms = 0.0;
  double first_partial_after_first_frame_ms = 0.0;
  double mean_partial_cadence_ms = 0.0;
  double audio_duration_seconds = 0.0;
  double capture_duration_ms = 0.0;
  double stop_to_final_ms = 0.0;
  double total_session_ms = 0.0;
  double microphone_release_latency_ms = 0.0;
  double process_cpu_ms = 0.0;
  std::uint64_t partial_update_count = 0;
  std::uint64_t queue_overflow_count = 0;
  std::uint64_t peak_rss_bytes = 0;
  std::uint64_t native_thread_count = 0;
  std::uint64_t queue_maximum_depth = 0;
};

struct WorkerEvent {
  WorkerEventKind kind = WorkerEventKind::Error;
  std::uint64_t session_id = 0;
  std::string text;
  std::string error_message;
  std::int32_t native_error_code = 0;
  SessionMetrics metrics{};
};

struct MoonshineApi {
  decltype(&moonshine_load_transcriber_from_files) load =
      &moonshine_load_transcriber_from_files;
  decltype(&moonshine_create_stream) create_stream = &moonshine_create_stream;
  decltype(&moonshine_start_stream) start_stream = &moonshine_start_stream;
  decltype(&moonshine_transcribe_add_audio_to_stream) add_audio =
      &moonshine_transcribe_add_audio_to_stream;
  decltype(&moonshine_transcribe_stream) transcribe_stream =
      &moonshine_transcribe_stream;
  decltype(&moonshine_stop_stream) stop_stream = &moonshine_stop_stream;
  decltype(&moonshine_free_stream) free_stream = &moonshine_free_stream;
  decltype(&moonshine_free_transcriber) free_transcriber =
      &moonshine_free_transcriber;
};

using WorkerEventHandler = std::function<void(WorkerEvent)>;
using ModelVerifier = std::function<bool(const std::filesystem::path&, std::string&)>;

class SessionWorker final {
 public:
  SessionWorker(std::uint64_t session_id, std::filesystem::path model_directory,
                AudioSourceKind source, WorkerEventHandler event_handler,
                MoonshineApi api = {}, ModelVerifier verifier = {},
                std::uint64_t partial_poll_interval_ns = 250'000'000ULL);
  ~SessionWorker();
  SessionWorker(const SessionWorker&) = delete;
  SessionWorker& operator=(const SessionWorker&) = delete;

  bool start() noexcept;
  void set_capture_start_time(std::uint64_t monotonic_ns) noexcept;
  bool request_finalize(std::uint64_t stop_request_ns,
                        std::uint64_t microphone_release_ns) noexcept;
  void request_cancel() noexcept;
  void request_failure(std::int32_t code, std::string message) noexcept;
  void join() noexcept;
  BoundedAudioQueue& audio_queue() noexcept;
  bool is_joinable() const noexcept;

 private:
  void run() noexcept;
  void emit(WorkerEvent event) noexcept;

  const std::uint64_t session_id_;
  const std::filesystem::path model_directory_;
  const AudioSourceKind source_;
  WorkerEventHandler event_handler_;
  MoonshineApi api_;
  ModelVerifier verifier_;
  const std::uint64_t partial_poll_interval_ns_;
  BoundedAudioQueue audio_queue_;
  std::thread thread_;
  std::atomic<bool> started_{false};
  std::atomic<bool> cancel_requested_{false};
  std::atomic<bool> finalize_requested_{false};
  std::atomic<std::int32_t> external_error_code_{0};
  std::atomic<std::uint64_t> capture_start_ns_{0};
  std::atomic<std::uint64_t> stop_request_ns_{0};
  std::atomic<std::uint64_t> microphone_release_ns_{0};
  std::string external_error_message_;
};

std::uint64_t monotonic_time_ns() noexcept;

}  // namespace outflow::mic_probe
