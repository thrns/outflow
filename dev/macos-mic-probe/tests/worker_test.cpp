#include "session_worker.hpp"
#include "session_state.hpp"

#include <atomic>
#include <cassert>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <iostream>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

using namespace outflow::mic_probe;

namespace {

std::atomic<int> load_calls{0};
std::atomic<int> create_calls{0};
std::atomic<int> start_calls{0};
std::atomic<int> add_calls{0};
std::atomic<int> transcribe_calls{0};
std::atomic<int> stop_calls{0};
std::atomic<int> free_stream_calls{0};
std::atomic<int> free_transcriber_calls{0};
std::atomic<std::int32_t> load_result{4};
transcript_line_t fake_line{};
transcript_t fake_transcript{&fake_line, 1};

class BlockingGate final {
 public:
  void block_call() {
    std::unique_lock lock(mutex_);
    entered_ = true;
    changed_.notify_all();
    changed_.wait(lock, [&] { return released_; });
  }

  void wait_until_entered() {
    std::unique_lock lock(mutex_);
    const bool entered = changed_.wait_for(lock, std::chrono::seconds(5), [&] {
      return entered_;
    });
    assert(entered);
  }

  void release() {
    {
      std::lock_guard lock(mutex_);
      released_ = true;
    }
    changed_.notify_all();
  }

 private:
  std::mutex mutex_;
  std::condition_variable changed_;
  bool entered_ = false;
  bool released_ = false;
};

std::shared_ptr<BlockingGate> load_gate;
std::shared_ptr<BlockingGate> add_gate;
std::shared_ptr<BlockingGate> final_transcribe_gate;

void reset_fake_api() {
  load_calls = 0;
  create_calls = 0;
  start_calls = 0;
  add_calls = 0;
  transcribe_calls = 0;
  stop_calls = 0;
  free_stream_calls = 0;
  free_transcriber_calls = 0;
  load_result = 4;
  load_gate.reset();
  add_gate.reset();
  final_transcribe_gate.reset();
  fake_line = {};
  fake_line.text = "injected fixture transcript";
  fake_line.is_complete = 1;
}

std::int32_t fake_load(const char*, std::uint32_t architecture,
                       const moonshine_option_t* options, std::uint64_t count,
                       std::int32_t version) {
  ++load_calls;
  if (load_gate) load_gate->block_call();
  assert(architecture == MOONSHINE_MODEL_ARCH_SMALL_STREAMING);
  assert(version == MOONSHINE_HEADER_VERSION);
  assert(count == 2);
  assert(std::string(options[0].name) == "identify_speakers");
  assert(std::string(options[0].value) == "false");
  assert(std::string(options[1].name) == "ort_providers");
  assert(std::string(options[1].value) == "CPU");
  return load_result.load();
}

std::int32_t fake_create(std::int32_t transcriber, std::uint32_t flags) {
  ++create_calls;
  assert(transcriber == 4);
  assert(flags == 0);
  return 8;
}

std::int32_t fake_start(std::int32_t transcriber, std::int32_t stream) {
  ++start_calls;
  assert(transcriber == 4 && stream == 8);
  return MOONSHINE_ERROR_NONE;
}

std::int32_t fake_add(std::int32_t transcriber, std::int32_t stream,
                      const float* samples, std::uint64_t count,
                      std::int32_t sample_rate, std::uint32_t flags) {
  ++add_calls;
  if (add_gate) add_gate->block_call();
  assert(transcriber == 4 && stream == 8);
  assert(samples != nullptr && count > 0 && sample_rate == 16'000 && flags == 0);
  return MOONSHINE_ERROR_NONE;
}

std::int32_t fake_transcribe(std::int32_t transcriber, std::int32_t stream,
                             std::uint32_t flags, transcript_t** transcript) {
  ++transcribe_calls;
  if (stop_calls.load(std::memory_order_acquire) > 0 && final_transcribe_gate) {
    final_transcribe_gate->block_call();
  }
  assert(transcriber == 4 && stream == 8 && flags == 0);
  *transcript = &fake_transcript;
  return MOONSHINE_ERROR_NONE;
}

std::int32_t fake_stop(std::int32_t transcriber, std::int32_t stream) {
  ++stop_calls;
  assert(transcriber == 4 && stream == 8);
  return MOONSHINE_ERROR_NONE;
}

std::int32_t fake_free_stream(std::int32_t transcriber, std::int32_t stream) {
  ++free_stream_calls;
  assert(transcriber == 4 && stream == 8);
  return MOONSHINE_ERROR_NONE;
}

void fake_free_transcriber(std::int32_t transcriber) {
  ++free_transcriber_calls;
  assert(transcriber == 4);
}

MoonshineApi fake_api() {
  return MoonshineApi{fake_load, fake_create, fake_start, fake_add,
                      fake_transcribe, fake_stop, fake_free_stream,
                      fake_free_transcriber};
}

class EventCollector final {
 public:
  void add(WorkerEvent event) {
    {
      std::lock_guard lock(mutex_);
      events_.push_back(std::move(event));
    }
    changed_.notify_all();
  }

  WorkerEvent wait_for(std::uint64_t session_id, WorkerEventKind kind) {
    std::unique_lock lock(mutex_);
    const bool arrived = changed_.wait_for(lock, std::chrono::seconds(5), [&] {
      for (const auto& event : events_) {
        if (event.session_id == session_id && event.kind == kind) return true;
      }
      return false;
    });
    if (!arrived) {
      std::cerr << "worker test timed out waiting for session " << session_id
                << " event " << static_cast<int>(kind) << '\n';
    }
    assert(arrived);
    for (const auto& event : events_) {
      if (event.session_id == session_id && event.kind == kind) return event;
    }
    std::abort();
  }

  std::size_t count(std::uint64_t session_id, WorkerEventKind kind) {
    std::lock_guard lock(mutex_);
    std::size_t total = 0;
    for (const auto& event : events_) {
      if (event.session_id == session_id && event.kind == kind) ++total;
    }
    return total;
  }

 private:
  std::mutex mutex_;
  std::condition_variable changed_;
  std::vector<WorkerEvent> events_;
};

void assert_queue_is_scrubbed(BoundedAudioQueue& queue) {
  assert(queue.depth() == 0);
  // Reuse every ring slot and check that cancellation cleared the full sample
  // storage, not just the logical queue indices or frame counts.
  for (std::size_t slot = 0; slot < kAudioQueueSlots; ++slot) {
    auto* block = queue.begin_write();
    assert(block != nullptr);
    for (const float sample : block->samples) assert(sample == 0.0F);
    block->frame_count = 1;
    block->samples[0] = 0.5F;
    queue.publish_write();
  }
  queue.clear_after_producer_stops();
  assert(queue.depth() == 0);
}

void add_fixture_audio(SessionWorker& worker) {
  auto* block = worker.audio_queue().begin_write();
  assert(block != nullptr);
  block->format = AudioFormat{16'000, 1, PcmEncoding::Float32, true};
  block->frame_count = 1'600;
  block->captured_at_monotonic_ns = monotonic_time_ns();
  for (std::size_t index = 0; index < block->frame_count; ++index) {
    block->samples[index] = static_cast<float>(index % 16) / 16.0F;
  }
  worker.audio_queue().publish_write();
}

ModelVerifier verifier_always_succeeds() {
  return [](const std::filesystem::path&, std::string&) { return true; };
}

void test_load_error_is_reported_and_does_not_create_stream() {
  reset_fake_api();
  load_result = -42;
  EventCollector events;
  auto worker = std::make_unique<SessionWorker>(
      1, "/unused/model", AudioSourceKind::TimedFixture,
      [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
      verifier_always_succeeds());
  assert(worker->start());
  const auto error = events.wait_for(1, WorkerEventKind::Error);
  assert(error.native_error_code == -42);
  assert(error.error_message.find("initialization failed") != std::string::npos);
  worker->join();
  assert(load_calls == 1);
  assert(create_calls == 0);
  assert(free_transcriber_calls == 0);
}

void test_worker_stream_lifecycle_and_single_final_result() {
  reset_fake_api();
  EventCollector events;
  auto worker = std::make_unique<SessionWorker>(
      2, "/unused/model", AudioSourceKind::TimedFixture,
      [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
      verifier_always_succeeds(), 0);
  assert(worker->start());
  (void)events.wait_for(2, WorkerEventKind::Ready);
  for (int index = 0; index < 3; ++index) add_fixture_audio(*worker);
  add_fixture_audio(*worker);
  (void)events.wait_for(2, WorkerEventKind::Partial);
  const auto now = monotonic_time_ns();
  assert(worker->request_finalize(now, now + 1'000'000));
  assert(!worker->request_finalize(now, now));
  const auto final = events.wait_for(2, WorkerEventKind::Final);
  assert(final.text == "injected fixture transcript");
  worker->join();
  assert(events.count(2, WorkerEventKind::Final) == 1);
  assert(load_calls == 1 && create_calls == 1 && start_calls == 1);
  assert(add_calls == 4);
  assert(transcribe_calls >= 2);  // partial snapshots and one final drain
  assert(stop_calls == 1 && free_stream_calls == 1 && free_transcriber_calls == 1);
}

void test_cancel_during_model_load_releases_transcriber_without_opening_stream() {
  reset_fake_api();
  load_gate = std::make_shared<BlockingGate>();
  EventCollector events;
  auto worker = std::make_unique<SessionWorker>(
      3, "/unused/model", AudioSourceKind::TimedFixture,
      [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
      verifier_always_succeeds());
  assert(worker->start());
  load_gate->wait_until_entered();
  worker->request_cancel();
  load_gate->release();
  (void)events.wait_for(3, WorkerEventKind::Cancelled);
  worker->join();
  assert(events.count(3, WorkerEventKind::Final) == 0);
  assert(events.count(3, WorkerEventKind::Ready) == 0);
  assert(load_calls == 1 && create_calls == 0 && start_calls == 0);
  assert(stop_calls == 0 && free_stream_calls == 0 && free_transcriber_calls == 1);
  assert_queue_is_scrubbed(worker->audio_queue());
}

void test_cancel_immediately_after_ready_is_idempotent_and_releases_stream() {
  reset_fake_api();
  EventCollector events;
  auto worker = std::make_unique<SessionWorker>(
      6, "/unused/model", AudioSourceKind::TimedFixture,
      [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
      verifier_always_succeeds());
  assert(worker->start());
  (void)events.wait_for(6, WorkerEventKind::Ready);
  worker->request_cancel();
  worker->request_cancel();
  assert(!worker->request_finalize(monotonic_time_ns(), monotonic_time_ns()));
  (void)events.wait_for(6, WorkerEventKind::Cancelled);
  worker->join();
  assert(events.count(6, WorkerEventKind::Final) == 0);
  assert(load_calls == 1 && create_calls == 1 && start_calls == 1);
  assert(add_calls == 0 && transcribe_calls == 0);
  assert(stop_calls == 1 && free_stream_calls == 1 && free_transcriber_calls == 1);
  assert_queue_is_scrubbed(worker->audio_queue());
}

void test_cancel_while_samples_are_queued_stops_further_worker_access() {
  reset_fake_api();
  add_gate = std::make_shared<BlockingGate>();
  EventCollector events;
  auto worker = std::make_unique<SessionWorker>(
      7, "/unused/model", AudioSourceKind::TimedFixture,
      [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
      verifier_always_succeeds());
  assert(worker->start());
  (void)events.wait_for(7, WorkerEventKind::Ready);
  add_fixture_audio(*worker);
  add_gate->wait_until_entered();
  for (int index = 0; index < 5; ++index) add_fixture_audio(*worker);
  assert(worker->audio_queue().depth() == 6);
  worker->request_cancel();
  add_gate->release();
  (void)events.wait_for(7, WorkerEventKind::Cancelled);
  worker->join();
  assert(add_calls == 1);
  assert(transcribe_calls == 0 && events.count(7, WorkerEventKind::Final) == 0);
  assert(stop_calls == 1 && free_stream_calls == 1 && free_transcriber_calls == 1);
  assert_queue_is_scrubbed(worker->audio_queue());
}

void test_cancel_after_nonempty_partial_discards_final_and_scrubs_queue() {
  reset_fake_api();
  EventCollector events;
  auto worker = std::make_unique<SessionWorker>(
      8, "/unused/model", AudioSourceKind::TimedFixture,
      [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
      verifier_always_succeeds(), 0);
  assert(worker->start());
  (void)events.wait_for(8, WorkerEventKind::Ready);
  add_fixture_audio(*worker);
  const auto partial = events.wait_for(8, WorkerEventKind::Partial);
  assert(partial.text == "injected fixture transcript");
  worker->request_cancel();
  worker->request_cancel();
  assert(!worker->request_finalize(monotonic_time_ns(), monotonic_time_ns()));
  (void)events.wait_for(8, WorkerEventKind::Cancelled);
  worker->join();
  assert(events.count(8, WorkerEventKind::Final) == 0);
  assert(stop_calls == 1 && free_stream_calls == 1 && free_transcriber_calls == 1);
  assert_queue_is_scrubbed(worker->audio_queue());
}

void test_cancel_during_final_drain_rejects_uncommitted_result() {
  reset_fake_api();
  final_transcribe_gate = std::make_shared<BlockingGate>();
  EventCollector events;
  auto worker = std::make_unique<SessionWorker>(
      9, "/unused/model", AudioSourceKind::TimedFixture,
      [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
      verifier_always_succeeds());
  assert(worker->start());
  (void)events.wait_for(9, WorkerEventKind::Ready);
  const auto now = monotonic_time_ns();
  assert(worker->request_finalize(now, now));
  assert(!worker->request_finalize(now, now));
  final_transcribe_gate->wait_until_entered();
  worker->request_cancel();
  final_transcribe_gate->release();
  (void)events.wait_for(9, WorkerEventKind::Cancelled);
  worker->join();
  assert(events.count(9, WorkerEventKind::Final) == 0);
  assert(stop_calls == 1 && free_stream_calls == 1 && free_transcriber_calls == 1);
  assert_queue_is_scrubbed(worker->audio_queue());
}

void test_audio_queue_overflow_fails_closed_and_frees_stream() {
  reset_fake_api();
  EventCollector events;
  auto worker = std::make_unique<SessionWorker>(
      4, "/unused/model", AudioSourceKind::TimedFixture,
      [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
      verifier_always_succeeds());
  for (std::size_t index = 0; index < kAudioQueueSlots; ++index) {
    auto* block = worker->audio_queue().begin_write();
    assert(block != nullptr);
    block->format = AudioFormat{16'000, 1, PcmEncoding::Float32, true};
    block->frame_count = 1;
    block->samples[0] = 0.0F;
    worker->audio_queue().publish_write();
  }
  assert(worker->audio_queue().begin_write() == nullptr);
  assert(worker->audio_queue().overflowed());
  assert(worker->start());
  const auto error = events.wait_for(4, WorkerEventKind::Error);
  assert(error.native_error_code == -101);
  assert(error.error_message.find("overflow") != std::string::npos);
  worker->join();
  assert(add_calls == 0);
  assert(stop_calls == 1 && free_stream_calls == 1 && free_transcriber_calls == 1);
}

void test_disconnected_device_error_frees_native_stream() {
  reset_fake_api();
  EventCollector events;
  auto worker = std::make_unique<SessionWorker>(
      5, "/unused/model", AudioSourceKind::Microphone,
      [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
      verifier_always_succeeds());
  worker->audio_queue().set_producer_error(-106);
  assert(worker->start());
  const auto error = events.wait_for(5, WorkerEventKind::Error);
  assert(error.error_message.find("disconnected") != std::string::npos);
  worker->join();
  assert(add_calls == 0);
  assert(stop_calls == 1 && free_stream_calls == 1 && free_transcriber_calls == 1);
}

void test_one_hundred_injected_start_stop_cancel_cycles() {
  SessionStateMachine state;
  for (std::uint64_t session = 1; session <= 100; ++session) {
    reset_fake_api();
    assert(state.begin(session));
    EventCollector events;
    auto worker = std::make_unique<SessionWorker>(
        session, "/unused/model", AudioSourceKind::TimedFixture,
        [&events](WorkerEvent event) { events.add(std::move(event)); }, fake_api(),
        verifier_always_succeeds());
    assert(worker->start());
    (void)events.wait_for(session, WorkerEventKind::Ready);
    assert(state.listening(session));
    add_fixture_audio(*worker);
    if (session % 2 == 0) {
      assert(state.cancel(session));
      worker->request_cancel();
      worker->request_cancel();
      (void)events.wait_for(session, WorkerEventKind::Cancelled);
      assert(events.count(session, WorkerEventKind::Final) == 0);
    } else {
      assert(state.finalize(session));
      const auto now = monotonic_time_ns();
      assert(worker->request_finalize(now, now));
      assert(!worker->request_finalize(now, now));
      (void)events.wait_for(session, WorkerEventKind::Final);
      assert(state.ready(session));
      assert(events.count(session, WorkerEventKind::Final) == 1);
    }
    worker->join();
    assert(state.dismiss());
    assert(!state.accepts(session));
    assert_queue_is_scrubbed(worker->audio_queue());
    assert(load_calls == 1 && create_calls == 1 && start_calls == 1);
    assert(add_calls <= 1);
    assert(stop_calls == 1 && free_stream_calls == 1 && free_transcriber_calls == 1);
  }
}

}  // namespace

int main() {
  test_load_error_is_reported_and_does_not_create_stream();
  test_worker_stream_lifecycle_and_single_final_result();
  test_cancel_during_model_load_releases_transcriber_without_opening_stream();
  test_cancel_immediately_after_ready_is_idempotent_and_releases_stream();
  test_cancel_while_samples_are_queued_stops_further_worker_access();
  test_cancel_after_nonempty_partial_discards_final_and_scrubs_queue();
  test_cancel_during_final_drain_rejects_uncommitted_result();
  test_audio_queue_overflow_fails_closed_and_frees_stream();
  test_disconnected_device_error_frees_native_stream();
  test_one_hundred_injected_start_stop_cancel_cycles();
  std::cout << "mic probe worker tests passed: 9 lifecycle groups; 100 "
               "injected start/stop/cancel/dismiss cycles\n";
}
