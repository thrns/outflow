#include "fixture_producer.hpp"
#include "model_verifier.hpp"
#include "session_worker.hpp"
#include "wav_fixture.hpp"

#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <mutex>
#include <optional>
#include <string>
#include <thread>

using namespace outflow::mic_probe;

namespace {

class ResultWaiter final {
 public:
  void add(WorkerEvent event) {
    {
      std::lock_guard lock(mutex_);
      if (event.kind == WorkerEventKind::Final ||
          event.kind == WorkerEventKind::Error ||
          event.kind == WorkerEventKind::Cancelled) {
        terminal_ = std::move(event);
      }
      if (event.kind == WorkerEventKind::Partial && !event.text.empty()) {
        saw_nonempty_partial_ = true;
      }
    }
    changed_.notify_all();
  }

  bool wait_ready() {
    std::unique_lock lock(mutex_);
    return changed_.wait_for(lock, std::chrono::minutes(3), [this] {
      return ready_ || terminal_.has_value();
    }) && ready_;
  }

  void mark_ready() {
    {
      std::lock_guard lock(mutex_);
      ready_ = true;
    }
    changed_.notify_all();
  }

  bool wait_terminal(WorkerEvent& event) {
    std::unique_lock lock(mutex_);
    if (!changed_.wait_for(lock, std::chrono::minutes(3), [this] {
          return terminal_.has_value();
        })) {
      return false;
    }
    event = *terminal_;
    return true;
  }

  bool saw_nonempty_partial() const {
    std::lock_guard lock(mutex_);
    return saw_nonempty_partial_;
  }

 private:
  mutable std::mutex mutex_;
  std::condition_variable changed_;
  bool ready_ = false;
  bool saw_nonempty_partial_ = false;
  std::optional<WorkerEvent> terminal_;
};

}  // namespace

int main(int argc, char** argv) {
  if (argc != 4) {
    std::cerr << "usage: mic_probe_fixture_harness <model-dir> <wav> <seconds>\n";
    return 2;
  }
  std::uint32_t seconds = 0;
  try {
    seconds = static_cast<std::uint32_t>(std::stoul(argv[3]));
  } catch (...) {
    std::cerr << "fixture duration must be an integer from 1 to 120 seconds\n";
    return 2;
  }
  if (seconds == 0 || seconds > 120) {
    std::cerr << "fixture duration must be an integer from 1 to 120 seconds\n";
    return 2;
  }

  WavFixture fixture;
  std::string error;
  if (!load_pcm16_mono_wav(argv[2], fixture, error)) {
    std::cerr << "fixture load failed: " << error << '\n';
    return 1;
  }
  const std::uint64_t session_id = 1;
  ResultWaiter events;
  SessionWorker worker(session_id, argv[1], AudioSourceKind::TimedFixture,
                       [&events](WorkerEvent event) {
                         if (event.kind == WorkerEventKind::Ready) {
                           events.mark_ready();
                         }
                         events.add(std::move(event));
                       });
  if (!worker.start() || !events.wait_ready()) {
    worker.request_cancel();
    worker.join();
    WorkerEvent terminal;
    if (events.wait_terminal(terminal) && terminal.kind == WorkerEventKind::Error) {
      std::cerr << "native model initialization failed (code "
                << terminal.native_error_code << ")\n";
    } else {
      std::cerr << "native worker did not become ready\n";
    }
    return 1;
  }

  TimedFixtureProducer producer;
  const auto capture_start_ns = monotonic_time_ns();
  worker.set_capture_start_time(capture_start_ns);
  const auto fixture_rate = fixture.sample_rate_hz;
  if (!producer.start(std::move(fixture), seconds, worker.audio_queue(), [&worker] {
        const auto stop_ns = monotonic_time_ns();
        (void)worker.request_finalize(stop_ns, stop_ns);
      }, error)) {
    worker.request_failure(-107, error);
  }

  WorkerEvent terminal;
  if (!events.wait_terminal(terminal)) {
    producer.stop();
    worker.request_cancel();
    worker.join();
    std::cerr << "native worker timed out during paced fixture recognition\n";
    return 1;
  }
  producer.stop();
  worker.join();
  if (terminal.kind != WorkerEventKind::Final) {
    std::cerr << "native streaming fixture failed (code "
              << terminal.native_error_code << ")\n";
    return 1;
  }

  const auto& metrics = terminal.metrics;
  std::cout.setf(std::ios::fixed);
  std::cout.precision(3);
  std::cout << "{\"schema_version\":1,\"source\":\"timed_fixture\","
            << "\"fixture_sample_rate_hz\":" << fixture_rate << ','
            << "\"fixture_paced_seconds\":" << seconds << ','
            << "\"partial_nonempty\":"
            << (events.saw_nonempty_partial() ? "true" : "false") << ','
            << "\"final_nonempty\":"
            << (terminal.text.empty() ? "false" : "true") << ','
            << "\"model_load_ms\":" << metrics.model_load_ms << ','
            << "\"first_partial_after_first_frame_ms\":"
            << metrics.first_partial_after_first_frame_ms << ','
            << "\"mean_partial_cadence_ms\":"
            << metrics.mean_partial_cadence_ms << ','
            << "\"audio_duration_seconds\":" << metrics.audio_duration_seconds << ','
            << "\"capture_duration_ms\":" << metrics.capture_duration_ms << ','
            << "\"stop_to_final_ms\":" << metrics.stop_to_final_ms << ','
            << "\"total_session_ms\":" << metrics.total_session_ms << ','
            << "\"process_cpu_ms\":" << metrics.process_cpu_ms << ','
            << "\"partial_update_count\":" << metrics.partial_update_count << ','
            << "\"queue_overflow_count\":" << metrics.queue_overflow_count << ','
            << "\"queue_maximum_depth\":" << metrics.queue_maximum_depth << ','
            << "\"native_process_peak_rss_bytes\":" << metrics.peak_rss_bytes << ','
            << "\"native_thread_count\":" << metrics.native_thread_count << "}\n";
  return terminal.text.empty() || !events.saw_nonempty_partial() ? 3 : 0;
}
