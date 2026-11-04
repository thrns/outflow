#include "audio_pipeline.hpp"
#include "copy_result_policy.hpp"
#include "fixture_producer.hpp"
#include "permission_policy.hpp"
#include "session_state.hpp"
#include "wav_fixture.hpp"

#include <array>
#include <cassert>
#include <chrono>
#include <cmath>
#include <condition_variable>
#include <cstdint>
#include <mutex>
#include <iostream>
#include <memory>
#include <filesystem>
#include <fstream>
#include <thread>
#include <vector>
#include <unistd.h>

using namespace outflow::mic_probe;

namespace {

void test_float_downmix_and_rate_conversion() {
  const std::array<float, 6> left{0.0F, 0.25F, 0.5F, 0.75F, 1.0F, 0.5F};
  const std::array<float, 6> right{0.0F, -0.25F, -0.5F, -0.75F, -1.0F, -0.5F};
  const AudioBufferView buffers[]{{left.data(), sizeof(left)},
                                  {right.data(), sizeof(right)}};
  auto block = std::make_unique<AudioBlock>();
  const auto copied = copy_pcm_to_block(
      *block, AudioFormat{48'000, 2, PcmEncoding::Float32, false}, buffers, 2, 6);
  assert(copied == AudioCopyError::None);
  std::vector<float> converted;
  converted.reserve(kMaxCaptureFrames * 2 + 2);
  MonoResampler resampler;
  assert(resampler.process(*block, converted));
  assert(converted.size() == 2);
  assert(std::abs(converted[0]) < 1e-6F);
  assert(std::abs(converted[1]) < 1e-6F);
  assert(resampler.source_rate_hz() == 48'000);
}

void test_int16_interleaved_conversion() {
  const std::array<std::int16_t, 4> pcm{-32768, 32767, 16384, -16384};
  const AudioBufferView buffer{pcm.data(), sizeof(pcm)};
  auto block = std::make_unique<AudioBlock>();
  assert(copy_pcm_to_block(*block,
                           AudioFormat{16'000, 2, PcmEncoding::SignedInt16, true},
                           &buffer, 1, 2) == AudioCopyError::None);
  assert(std::abs(block->samples[0] + 1.0F) < 1e-6F);
  assert(block->samples[1] > 0.999F);
  assert(std::abs(block->samples[2] - 0.5F) < 1e-6F);
  assert(std::abs(block->samples[3] + 0.5F) < 1e-6F);
}

void test_bad_audio_buffers_fail_without_partial_acceptance() {
  const std::array<std::int16_t, 2> pcm{0, 0};
  const AudioBufferView short_buffer{pcm.data(), 2};
  auto block = std::make_unique<AudioBlock>();
  assert(copy_pcm_to_block(*block,
                           AudioFormat{16'000, 1, PcmEncoding::SignedInt16, false},
                           &short_buffer, 1, 2) == AudioCopyError::ShortBuffer);
  assert(copy_pcm_to_block(*block,
                           AudioFormat{0, 1, PcmEncoding::Float32, false},
                           &short_buffer, 1, 2) == AudioCopyError::InvalidFormat);
  assert(copy_pcm_to_block(*block,
                           AudioFormat{48'000, 9, PcmEncoding::Float32, false},
                           &short_buffer, 1, 2) ==
         AudioCopyError::UnsupportedDimensions);
}

void test_bounded_queue_overflow_and_wakeup() {
  auto queue = std::make_unique<BoundedAudioQueue>();
  for (std::size_t index = 0; index < kAudioQueueSlots; ++index) {
    auto* block = queue->begin_write();
    assert(block != nullptr);
    block->frame_count = static_cast<std::uint32_t>(index + 1);
    queue->publish_write();
  }
  assert(queue->depth() == kAudioQueueSlots);
  assert(queue->maximum_depth() == kAudioQueueSlots);
  assert(queue->begin_write() == nullptr);
  assert(queue->overflowed());
  assert(queue->overflow_count() == 1);
  for (std::size_t index = 0; index < kAudioQueueSlots; ++index) {
    const auto* block = queue->begin_read();
    assert(block != nullptr);
    assert(block->frame_count == index + 1);
    queue->release_read();
  }
  assert(queue->depth() == 0);

  auto wake_queue = std::make_unique<BoundedAudioQueue>();
  std::atomic<bool> woke{false};
  std::atomic<bool> waiter_started{false};
  const auto observed = wake_queue->wake_generation();
  std::thread waiter([&, observed] {
    waiter_started.store(true, std::memory_order_release);
    wake_queue->wait_for_wake(observed);
    woke.store(true, std::memory_order_release);
  });
  while (!waiter_started.load(std::memory_order_acquire)) std::this_thread::yield();
  auto* block = wake_queue->begin_write();
  assert(block != nullptr);
  wake_queue->publish_write();
  waiter.join();
  assert(woke.load(std::memory_order_acquire));
}

void test_large_capture_callback_is_split_into_bounded_queue_blocks() {
  std::vector<float> samples(kMaxCaptureFrames + 7);
  for (std::size_t index = 0; index < samples.size(); ++index) {
    samples[index] = static_cast<float>(index) / static_cast<float>(samples.size());
  }
  const AudioBufferView buffer{samples.data(), samples.size() * sizeof(float)};
  BoundedAudioQueue queue;
  bool overflow = false;
  const auto callback_time = 10'000'000ULL;
  assert(copy_pcm_frames_to_queue(
             queue, AudioFormat{48'000, 1, PcmEncoding::Float32, true},
             &buffer, 1, static_cast<std::uint32_t>(samples.size()),
             callback_time, overflow) == AudioCopyError::None);
  assert(!overflow && !queue.overflowed());
  assert(queue.depth() == 2);
  const auto* first = queue.begin_read();
  assert(first != nullptr && first->frame_count == kMaxCaptureFrames);
  assert(std::abs(first->samples[0] - samples[0]) < 1e-6F);
  assert(std::abs(first->samples[kMaxCaptureFrames - 1] -
                  samples[kMaxCaptureFrames - 1]) < 1e-6F);
  queue.release_read();
  const auto* second = queue.begin_read();
  assert(second != nullptr && second->frame_count == 7);
  assert(second->captured_at_monotonic_ns == callback_time +
         audio_time_nanoseconds(kMaxCaptureFrames, 48'000));
  assert(std::abs(second->samples[0] - samples[kMaxCaptureFrames]) < 1e-6F);
  queue.release_read();

  BoundedAudioQueue nearly_full;
  for (std::size_t index = 0; index < kAudioQueueSlots - 1; ++index) {
    auto* block = nearly_full.begin_write();
    assert(block != nullptr);
    block->frame_count = 1;
    nearly_full.publish_write();
  }
  std::vector<float> two_blocks(kMaxCaptureFrames + 1, 0.25F);
  const AudioBufferView overflow_buffer{two_blocks.data(),
                                        two_blocks.size() * sizeof(float)};
  assert(copy_pcm_frames_to_queue(
             nearly_full, AudioFormat{16'000, 1, PcmEncoding::Float32, true},
             &overflow_buffer, 1,
             static_cast<std::uint32_t>(two_blocks.size()), callback_time,
             overflow) == AudioCopyError::None);
  assert(overflow && nearly_full.overflowed());
  assert(nearly_full.depth() == kAudioQueueSlots);
  assert(nearly_full.overflow_count() == 1);
}

void test_resampler_preserves_chunk_boundary_and_rejects_rate_change() {
  std::vector<float> converted;
  converted.reserve(kMaxCaptureFrames * 2 + 2);
  MonoResampler resampler;
  auto first = std::make_unique<AudioBlock>();
  first->format = AudioFormat{48'000, 1, PcmEncoding::Float32, true};
  first->frame_count = 3;
  first->samples[0] = 0.0F;
  first->samples[1] = 0.25F;
  first->samples[2] = 0.5F;
  assert(resampler.process(*first, converted));
  assert(converted.size() == 1);

  auto second = std::make_unique<AudioBlock>();
  second->format = first->format;
  second->frame_count = 3;
  second->samples[0] = 0.75F;
  second->samples[1] = 1.0F;
  second->samples[2] = 0.5F;
  assert(resampler.process(*second, converted));
  assert(converted.size() == 1);
  assert(std::abs(converted[0] - 0.75F) < 1e-6F);

  second->format.sample_rate_hz = 44'100;
  assert(!resampler.process(*second, converted));
  second->format.sample_rate_hz = 48'000;
  second->format.channels = 2;
  assert(!resampler.process(*second, converted));
}

void test_session_transitions_and_one_hundred_cycles() {
  SessionStateMachine state;
  assert(!state.listening(1));
  for (std::uint64_t session = 1; session <= 100; ++session) {
    assert(state.begin(session));
    assert(state.state() == SessionState::Preparing);
    assert(!state.begin(session + 1));
    assert(state.listening(session));
    assert(!state.listening(session));
    if (session % 3 == 0) {
      assert(state.cancel(session));
      assert(!state.cancel(session));
    } else if (session % 3 == 1) {
      assert(state.finalize(session));
      assert(!state.finalize(session));
      assert(state.ready(session));
    } else {
      assert(state.fail(session));
    }
    const auto stale = session + 10'000;
    assert(!state.accepts(stale));
    assert(!state.ready(stale));
    assert(state.dismiss());
    assert(state.state() == SessionState::Idle);
  }
}

void test_cancelled_session_rejects_late_events_and_restarts_after_dismiss() {
  SessionStateMachine state;
  assert(state.begin(7));
  assert(state.listening(7));
  assert(state.cancel(7));
  assert(!state.listening(7));
  assert(!state.finalize(7));
  assert(!state.ready(7));
  assert(!state.fail(7));
  assert(state.dismiss());
  assert(!state.accepts(7));
  assert(state.begin(8));
  assert(state.listening(8));
  assert(state.session_id() == 8);
}

void test_timed_fixture_clock() {
  const auto twelve_seconds = audio_time_nanoseconds(192'000, 16'000);
  assert(twelve_seconds == 12'000'000'000ULL);
  assert(audio_time_nanoseconds(0, 16'000) == 0);
  assert(audio_time_nanoseconds(100, 0) == 0);
}

void test_timed_fixture_producer_supports_two_sessions() {
  WavFixture source;
  source.sample_rate_hz = 16'000;
  source.mono_samples.resize(1'600, 0.1F);
  BoundedAudioQueue queue;
  TimedFixtureProducer producer;
  for (int session = 0; session < 2; ++session) {
    std::mutex mutex;
    std::condition_variable changed;
    bool completed = false;
    std::string error;
    assert(producer.start(source, 1, queue, [&] {
      {
        std::lock_guard lock(mutex);
        completed = true;
      }
      changed.notify_one();
    }, error));
    {
      std::unique_lock lock(mutex);
      assert(changed.wait_for(lock, std::chrono::seconds(2), [&] {
        return completed;
      }));
    }
    producer.stop();
    while (queue.begin_read() != nullptr) queue.release_read();
    assert(queue.depth() == 0);
  }
  assert(source.mono_samples.size() == 1'600);
}

void test_microphone_permission_decisions() {
  assert(action_for_permission(MicrophonePermission::Authorized) ==
         PermissionAction::BeginCapture);
  assert(action_for_permission(MicrophonePermission::NotDetermined) ==
         PermissionAction::RequestPermission);
  assert(action_for_permission(MicrophonePermission::Denied) ==
         PermissionAction::OpenSystemSettings);
  assert(action_for_permission(MicrophonePermission::Restricted) ==
         PermissionAction::OpenSystemSettings);
  assert(action_for_permission(MicrophonePermission::Unknown) ==
         PermissionAction::OpenSystemSettings);

  SessionStateMachine state;
  assert(state.begin(41));
  assert(is_current_permission_request(3, 3, true, false, 41, state));
  assert(permission_grant_is_authorized(true, MicrophonePermission::Authorized));
  assert(!permission_grant_is_authorized(false, MicrophonePermission::Authorized));
  assert(!permission_grant_is_authorized(true, MicrophonePermission::Denied));
  assert(!permission_grant_is_authorized(true, MicrophonePermission::Restricted));
  assert(!is_current_permission_request(2, 3, true, false, 41, state));
  assert(!is_current_permission_request(3, 3, false, false, 41, state));
  assert(!is_current_permission_request(3, 3, true, true, 41, state));
  assert(!is_current_permission_request(3, 3, true, false, 42, state));
  assert(state.cancel(41));
  assert(!is_current_permission_request(3, 3, true, false, 41, state));
  assert(state.dismiss());
  assert(state.begin(43));
  assert(!is_current_permission_request(3, 3, true, false, 41, state));
}

void test_explicit_copy_result_policy_and_exact_text() {
  std::size_t write_requests = 0;
  std::string written_text;
  const auto fake_writer = [&](std::string_view text) {
    ++write_requests;
    written_text.assign(text.data(), text.size());
    return true;
  };
  const std::string final_text = "Café — line one!\nSecond line 😀…";

  // Merely receiving a final result does not invoke the injected writer.
  assert(write_requests == 0);
  assert(copy_result_eligible(SessionState::ResultReady, 71, 71, final_text));

  for (const auto state : {SessionState::Idle, SessionState::Preparing,
                           SessionState::Listening, SessionState::Finalizing,
                           SessionState::Cancelled, SessionState::Error}) {
    assert(!copy_result_eligible(state, 71, 71, final_text));
    assert(request_copy_result(state, 71, 71, final_text, fake_writer) ==
           CopyResultAttempt::Rejected);
  }
  assert(!copy_result_eligible(SessionState::ResultReady, 71, 71, {}));
  assert(request_copy_result(SessionState::ResultReady, 71, 71, {}, fake_writer) ==
         CopyResultAttempt::Rejected);

  // A visible no-speech message is display-only: empty transcript data is
  // passed to the policy, so the placeholder can never reach the writer.
  const std::string placeholder = "No speech was recognized in this session.";
  const std::string empty_final_text;
  assert(!copy_result_eligible(SessionState::ResultReady, 71, 71,
                               empty_final_text));
  assert(request_copy_result(SessionState::ResultReady, 71, 71,
                             empty_final_text, fake_writer) ==
         CopyResultAttempt::Rejected);
  assert(written_text != placeholder);

  assert(!copy_result_eligible(SessionState::ResultReady, 72, 71, final_text));
  assert(request_copy_result(SessionState::ResultReady, 72, 71, final_text,
                             fake_writer) == CopyResultAttempt::Rejected);
  assert(write_requests == 0);

  // The explicit action makes one request and preserves all UTF-8 bytes,
  // including punctuation, a newline, and a multi-byte emoji.
  assert(request_copy_result(SessionState::ResultReady, 71, 71, final_text,
                             fake_writer) == CopyResultAttempt::Succeeded);
  assert(write_requests == 1);
  assert(written_text == final_text);

  // A failed writer does not alter the session-owned result or eligibility.
  const auto failed_writer = [&](std::string_view text) {
    ++write_requests;
    written_text.assign(text.data(), text.size());
    return false;
  };
  assert(request_copy_result(SessionState::ResultReady, 71, 71, final_text,
                             failed_writer) == CopyResultAttempt::Failed);
  assert(write_requests == 2);
  assert(written_text == final_text);
  assert(copy_result_eligible(SessionState::ResultReady, 71, 71, final_text));

  // Cancel, Dismiss/new-session identity, and errors all disable the action.
  assert(!copy_result_eligible(SessionState::Cancelled, 71, 71, final_text));
  assert(!copy_result_eligible(SessionState::Idle, 72, 0, final_text));
  assert(!copy_result_eligible(SessionState::Error, 71, 71, final_text));
  assert(write_requests == 2);
}

void test_bounded_pcm16_wav_fixture_reader() {
  std::vector<std::uint8_t> bytes{
      'R','I','F','F', 40,0,0,0, 'W','A','V','E',
      'f','m','t',' ', 16,0,0,0, 1,0, 1,0, 0x80,0x3e,0,0,
      0,0x7d,0,0, 2,0, 16,0,
      'd','a','t','a', 4,0,0,0, 0,0x40, 0,0x80};
  const auto path = std::filesystem::temp_directory_path() /
                    ("outflow-mic-probe-" + std::to_string(::getpid()) + ".wav");
  {
    std::ofstream output(path, std::ios::binary | std::ios::trunc);
    output.write(reinterpret_cast<const char*>(bytes.data()),
                 static_cast<std::streamsize>(bytes.size()));
    assert(output.good());
  }
  WavFixture fixture;
  std::string error;
  assert(load_pcm16_mono_wav(path, fixture, error));
  assert(fixture.sample_rate_hz == 16'000);
  assert(fixture.mono_samples.size() == 2);
  assert(std::abs(fixture.mono_samples[0] - 0.5F) < 1e-6F);
  assert(std::abs(fixture.mono_samples[1] + 1.0F) < 1e-6F);

  bytes.resize(32);
  {
    std::ofstream output(path, std::ios::binary | std::ios::trunc);
    output.write(reinterpret_cast<const char*>(bytes.data()),
                 static_cast<std::streamsize>(bytes.size()));
  }
  assert(!load_pcm16_mono_wav(path, fixture, error));
  std::error_code ignored;
  (void)std::filesystem::remove(path, ignored);
}

}  // namespace

int main() {
  test_float_downmix_and_rate_conversion();
  test_int16_interleaved_conversion();
  test_bad_audio_buffers_fail_without_partial_acceptance();
  test_bounded_queue_overflow_and_wakeup();
  test_large_capture_callback_is_split_into_bounded_queue_blocks();
  test_resampler_preserves_chunk_boundary_and_rejects_rate_change();
  test_session_transitions_and_one_hundred_cycles();
  test_cancelled_session_rejects_late_events_and_restarts_after_dismiss();
  test_timed_fixture_clock();
  test_timed_fixture_producer_supports_two_sessions();
  test_microphone_permission_decisions();
  test_explicit_copy_result_policy_and_exact_text();
  test_bounded_pcm16_wav_fixture_reader();
  std::cout << "mic probe core tests passed: 13 groups; 100 state cycles; "
               "permission generation and restart guards; 2 fixture starts\n";
}
