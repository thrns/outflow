#include "audio_pipeline.hpp"
#include "copy_result_policy.hpp"
#include "final_result_parity.hpp"
#include "fixture_producer.hpp"
#include "launch_policy.hpp"
#include "permission_policy.hpp"
#include "session_state.hpp"
#include "synthetic_copy_diagnostic.hpp"
#include "verification_identity.hpp"
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

bool valid_utf8(std::string_view value) {
  const auto* bytes = reinterpret_cast<const unsigned char*>(value.data());
  std::size_t index = 0;
  while (index < value.size()) {
    const auto lead = bytes[index];
    std::size_t length = 0;
    if (lead <= 0x7f) {
      length = 1;
    } else if (lead >= 0xc2 && lead <= 0xdf) {
      length = 2;
    } else if (lead >= 0xe0 && lead <= 0xef) {
      length = 3;
    } else if (lead >= 0xf0 && lead <= 0xf4) {
      length = 4;
    } else {
      return false;
    }
    if (index + length > value.size()) return false;
    for (std::size_t continuation = 1; continuation < length; ++continuation) {
      if ((bytes[index + continuation] & 0xc0) != 0x80) return false;
    }
    if ((lead == 0xe0 && bytes[index + 1] < 0xa0) ||
        (lead == 0xed && bytes[index + 1] >= 0xa0) ||
        (lead == 0xf0 && bytes[index + 1] < 0x90) ||
        (lead == 0xf4 && bytes[index + 1] >= 0x90)) {
      return false;
    }
    index += length;
  }
  return true;
}

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
           CopyResultAttempt::RejectedBySessionPolicy);
  }
  assert(!copy_result_eligible(SessionState::ResultReady, 71, 71, {}));
  assert(request_copy_result(SessionState::ResultReady, 71, 71, {}, fake_writer) ==
         CopyResultAttempt::RejectedBySessionPolicy);

  // A visible no-speech message is display-only: empty transcript data is
  // passed to the policy, so the placeholder can never reach the writer.
  const std::string placeholder = "No speech was recognized in this session.";
  const std::string empty_final_text;
  assert(!copy_result_eligible(SessionState::ResultReady, 71, 71,
                               empty_final_text));
  assert(request_copy_result(SessionState::ResultReady, 71, 71,
                             empty_final_text, fake_writer) ==
         CopyResultAttempt::RejectedBySessionPolicy);
  assert(written_text != placeholder);

  assert(!copy_result_eligible(SessionState::ResultReady, 72, 71, final_text));
  assert(request_copy_result(SessionState::ResultReady, 72, 71, final_text,
                             fake_writer) == CopyResultAttempt::RejectedBySessionPolicy);
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
  assert(final_text == "Café — line one!\nSecond line 😀…");
  assert(copy_result_eligible(SessionState::ResultReady, 71, 71, final_text));

  // Cancel, Dismiss/new-session identity, and errors all disable the action.
  assert(!copy_result_eligible(SessionState::Cancelled, 71, 71, final_text));
  assert(!copy_result_eligible(SessionState::Idle, 72, 0, final_text));
  assert(!copy_result_eligible(SessionState::Error, 71, 71, final_text));
  assert(write_requests == 2);
}

void test_development_verification_identity_is_bounded_and_title_safe() {
  assert(is_valid_verification_id("79I-COPY-TEST-01"));
  assert(is_valid_verification_id("a"));
  assert(is_valid_verification_id(std::string(kMaximumVerificationIdLength, 'Z')));
  assert(!is_valid_verification_id(""));
  assert(!is_valid_verification_id(
      std::string(kMaximumVerificationIdLength + 1, 'A')));
  assert(!is_valid_verification_id("has space"));
  assert(!is_valid_verification_id("bad_identifier"));
  assert(!is_valid_verification_id("line\nbreak"));
  assert(!is_valid_verification_id("tab\tvalue"));
  assert(!is_valid_verification_id("control\x1bvalue"));
  assert(!is_valid_verification_id("Unicode-😀"));

  const char* no_id_arguments[]{"probe"};
  const char* valid_id_arguments[]{"probe", "--verification-id", "79I-COPY-TEST-01"};
  const char* missing_id_arguments[]{"probe", "--verification-id"};
  const char* empty_id_arguments[]{"probe", "--verification-id", ""};
  const char* oversized_id_arguments[]{
      "probe", "--verification-id", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"};
  const char* control_id_arguments[]{"probe", "--verification-id", "bad\nID"};
  const char* repeated_id_arguments[]{
      "probe", "--verification-id", "first", "--verification-id", "second"};
  assert(verification_id_arguments_valid(1, no_id_arguments));
  assert(verification_id_arguments_valid(3, valid_id_arguments));
  assert(!verification_id_arguments_valid(2, missing_id_arguments));
  assert(!verification_id_arguments_valid(3, empty_id_arguments));
  assert(!verification_id_arguments_valid(3, oversized_id_arguments));
  assert(!verification_id_arguments_valid(3, control_id_arguments));
  assert(!verification_id_arguments_valid(5, repeated_id_arguments));

  const auto diagnostic = verification_diagnostic("79I-COPY-TEST-01", 12345);
  assert(diagnostic == "Verification ID: 79I-COPY-TEST-01 — PID 12345");
  assert(diagnostic.find('\n') == std::string::npos);
  assert(diagnostic.find('\r') == std::string::npos);
  assert(verification_diagnostic("bad\nID", 12345).empty());
}

void test_synthetic_copy_diagnostic_uses_a_fixed_session_and_shared_policy() {
  const auto sample = kSyntheticCopyDiagnosticSample;
  const auto same_sample = kSyntheticCopyDiagnosticSample;
  assert(sample.data() == same_sample.data());
  assert(sample == same_sample);
  assert(valid_utf8(sample));
  assert(sample.find("OUTFLOW-SYNTHETIC-COPY-START\n") == 0);
  assert(sample.find("Café") != std::string_view::npos);
  assert(sample.find("plain-text clipboard check!") != std::string_view::npos);
  assert(sample.find("😀") != std::string_view::npos);
  assert(sample.find("\nOUTFLOW-SYNTHETIC-COPY-END") != std::string_view::npos);
  std::size_t newline_count = 0;
  for (const char character : sample) {
    if (character == '\n') ++newline_count;
    assert(character != '\r');
  }
  assert(newline_count == 2);

  assert(probe_input_mode(false, false) == ProbeInputMode::Microphone);
  assert(probe_input_mode(true, false) == ProbeInputMode::TimedFixture);
  assert(probe_input_mode(false, true) == ProbeInputMode::SyntheticCopyDiagnostic);
  assert(probe_input_mode(true, true) == ProbeInputMode::Invalid);

  const char* normal_arguments[]{"probe"};
  const char* missing_id_arguments[]{"probe", "--synthetic-copy-diagnostic"};
  const char* valid_arguments[]{"probe", "--verification-id", "79J-SYNTH-01",
                                "--synthetic-copy-diagnostic"};
  const char* invalid_id_arguments[]{"probe", "--verification-id", "bad id",
                                     "--synthetic-copy-diagnostic"};
  const char* fixture_conflict_arguments[]{"probe", "--verification-id",
                                           "79J-SYNTH-01", "--synthetic-copy-diagnostic",
                                           "--timed-fixture", "/tmp/fixed.wav"};
  const char* microphone_conflict_arguments[]{"probe", "--verification-id",
                                              "79J-SYNTH-01", "--synthetic-copy-diagnostic",
                                              "--microphone-capture"};
  assert(probe_launch_arguments_valid(1, normal_arguments));
  assert(!probe_launch_arguments_valid(2, missing_id_arguments));
  assert(probe_launch_arguments_valid(4, valid_arguments));
  assert(!probe_launch_arguments_valid(4, invalid_id_arguments));
  assert(!probe_launch_arguments_valid(6, fixture_conflict_arguments));
  assert(!probe_launch_arguments_valid(5, microphone_conflict_arguments));

  SessionStateMachine session;
  assert(!establish_synthetic_copy_result(session, 0));
  assert(establish_synthetic_copy_result(session, 79));
  assert(session.state() == SessionState::ResultReady);
  assert(session.session_id() == 79);

  std::string result(sample);
  std::uint64_t result_session_id = session.session_id();
  std::size_t injected_writer_calls = 0;
  std::string written_sample;
  const auto injected_writer = [&](std::string_view text) {
    ++injected_writer_calls;
    written_sample.assign(text.data(), text.size());
    return true;
  };
  assert(copy_result_eligible(session.state(), session.session_id(),
                              result_session_id, result));
  assert(injected_writer_calls == 0);
  assert(request_copy_result(session.state(), session.session_id(),
                             result_session_id, result, injected_writer) ==
         CopyResultAttempt::Succeeded);
  assert(injected_writer_calls == 1);
  assert(written_sample == sample);

  assert(request_copy_result(SessionState::ResultReady, 80, result_session_id,
                             result, injected_writer) ==
         CopyResultAttempt::RejectedBySessionPolicy);
  assert(injected_writer_calls == 1);
  assert(session.dismiss());
  result.clear();
  result_session_id = 0;
  assert(!copy_result_eligible(session.state(), session.session_id(),
                               result_session_id, result));
  assert(request_copy_result(session.state(), session.session_id(),
                             result_session_id, result, injected_writer) ==
         CopyResultAttempt::RejectedBySessionPolicy);
  assert(injected_writer_calls == 1);
}

void test_final_result_parity_is_exact_and_copy_fails_closed() {
  assert(final_result_parity_enabled(true, false, "79K-REAL-COPY-01"));
  assert(!final_result_parity_enabled(false, false, "79K-REAL-COPY-01"));
  assert(!final_result_parity_enabled(true, true, "79K-REAL-COPY-01"));
  assert(!final_result_parity_enabled(true, false, std::nullopt));
  assert(!final_result_parity_enabled(true, false, "invalid id"));

  assert(compare_final_result_parity("plain ASCII", "plain ASCII") ==
         FinalResultParity::Match);
  const std::string unicode = "Café — Moonshine 😀 🧪";
  assert(compare_final_result_parity(unicode, unicode) ==
         FinalResultParity::Match);
  const std::string multiline = "First line\nSecond line\n";
  assert(compare_final_result_parity(multiline, multiline) ==
         FinalResultParity::Match);
  assert(compare_final_result_parity("a character", "a caractere") ==
         FinalResultParity::Mismatch);
  assert(compare_final_result_parity("two words", "two  words") ==
         FinalResultParity::Mismatch);
  assert(compare_final_result_parity("first\nsecond", "first\r\nsecond") ==
         FinalResultParity::Mismatch);

  // Composed U+00E9 and decomposed U+0065 U+0301 stay byte-distinct.
  const std::string composed = "\xC3\xA9";
  const std::string decomposed = "e\xCC\x81";
  assert(compare_final_result_parity(composed, decomposed) ==
         FinalResultParity::Mismatch);

  const std::string invalid_utf8("\xC0\xAF", 2);
  assert(compare_final_result_parity(invalid_utf8, invalid_utf8) ==
         FinalResultParity::Unavailable);
  assert(compare_final_result_parity("valid", invalid_utf8) ==
         FinalResultParity::Unavailable);
  assert(compare_final_result_parity("valid", std::nullopt) ==
         FinalResultParity::Unavailable);

  std::size_t writer_calls = 0;
  const auto counting_writer = [&](std::string_view) {
    ++writer_calls;
    return true;
  };
  const std::string accepted_text = "fixture final";
  assert(request_copy_result(SessionState::ResultReady, 81, 80, accepted_text,
                             true, FinalResultParity::Match,
                             counting_writer) ==
         CopyResultAttempt::RejectedBySessionPolicy);
  assert(writer_calls == 0);
  assert(request_copy_result(SessionState::ResultReady, 81, 81, accepted_text,
                             true, FinalResultParity::Mismatch,
                             counting_writer) ==
         CopyResultAttempt::RejectedByFinalResultParity);
  assert(request_copy_result(SessionState::ResultReady, 81, 81, accepted_text,
                             true, FinalResultParity::Unavailable,
                             counting_writer) ==
         CopyResultAttempt::RejectedByFinalResultParity);
  assert(writer_calls == 0);
  assert(copy_result_eligible(SessionState::ResultReady, 81, 81, accepted_text,
                              true, FinalResultParity::Match));
  assert(!copy_result_eligible(SessionState::ResultReady, 81, 81, accepted_text,
                               true, FinalResultParity::Mismatch));
  assert(request_copy_result(SessionState::ResultReady, 81, 81, accepted_text,
                             true, FinalResultParity::Match,
                             counting_writer) == CopyResultAttempt::Succeeded);
  assert(writer_calls == 1);

  // Cancel rejects the session before parity can authorize a writer. Dismiss
  // returns to Idle, where a retained-looking stale result is also ineligible.
  SessionStateMachine session;
  assert(session.begin(82));
  assert(session.listening(82));
  assert(session.cancel(82));
  assert(request_copy_result(session.state(), session.session_id(), 82,
                             accepted_text, true, FinalResultParity::Match,
                             counting_writer) ==
         CopyResultAttempt::RejectedBySessionPolicy);
  assert(writer_calls == 1);
  assert(session.dismiss());
  assert(request_copy_result(session.state(), session.session_id(), 82,
                             accepted_text, true, FinalResultParity::Match,
                             counting_writer) ==
         CopyResultAttempt::RejectedBySessionPolicy);
  assert(writer_calls == 1);
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
  test_development_verification_identity_is_bounded_and_title_safe();
  test_synthetic_copy_diagnostic_uses_a_fixed_session_and_shared_policy();
  test_final_result_parity_is_exact_and_copy_fails_closed();
  test_bounded_pcm16_wav_fixture_reader();
  std::cout << "mic probe core tests passed: 16 groups; 100 state cycles; "
               "permission generation and restart guards; 2 fixture starts\n";
}
