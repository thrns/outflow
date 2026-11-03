#include "fixture_producer.hpp"

#include "session_worker.hpp"

#include <algorithm>
#include <chrono>
#include <condition_variable>
#include <cstring>
#include <mutex>
#include <utility>

namespace outflow::mic_probe {
namespace {

constexpr std::size_t kFixtureChunkSamples = 1'600;

}  // namespace

TimedFixtureProducer::~TimedFixtureProducer() { stop(); }

bool TimedFixtureProducer::start(WavFixture fixture,
                                std::uint32_t maximum_seconds,
                                BoundedAudioQueue& queue,
                                CompletionHandler completed,
                                std::string& error) noexcept {
  if (thread_.joinable() || fixture.sample_rate_hz < 8'000 ||
      fixture.sample_rate_hz > 192'000 || fixture.mono_samples.empty() ||
      maximum_seconds > 120) {
    error = "timed fixture settings are invalid or a replay is already active";
    return false;
  }
  const std::uint64_t requested_limit = maximum_seconds == 0
      ? fixture.mono_samples.size()
      : static_cast<std::uint64_t>(fixture.sample_rate_hz) * maximum_seconds;
  const auto sample_limit = std::min<std::uint64_t>(requested_limit,
                                                   fixture.mono_samples.size());
  if (sample_limit == 0) {
    error = "timed fixture has no samples to replay";
    return false;
  }
  stop_requested_.store(false, std::memory_order_release);
  running_.store(true, std::memory_order_release);
  started_at_ns_.store(monotonic_time_ns(), std::memory_order_release);
  try {
    thread_ = std::thread(&TimedFixtureProducer::run, this, std::move(fixture),
                          sample_limit, std::ref(queue), std::move(completed));
    return true;
  } catch (...) {
    running_.store(false, std::memory_order_release);
    error = "timed fixture producer could not start";
    return false;
  }
}

void TimedFixtureProducer::run(WavFixture fixture, std::uint64_t sample_limit,
                               BoundedAudioQueue& queue,
                               CompletionHandler completed) noexcept {
  const auto start = std::chrono::steady_clock::now();
  for (std::uint64_t offset = 0; offset < sample_limit;) {
    const auto deadline = start + std::chrono::nanoseconds(
        audio_time_nanoseconds(offset, fixture.sample_rate_hz));
    {
      std::unique_lock lock(wait_mutex_);
      if (wake_.wait_until(lock, deadline, [this] {
            return stop_requested_.load(std::memory_order_acquire);
          })) {
        break;
      }
    }
    if (stop_requested_.load(std::memory_order_acquire)) break;
    const auto count = static_cast<std::size_t>(std::min<std::uint64_t>(
        kFixtureChunkSamples, sample_limit - offset));
    auto* block = queue.begin_write();
    if (block == nullptr) break;
    block->format = AudioFormat{fixture.sample_rate_hz, 1,
                                PcmEncoding::Float32, true};
    block->frame_count = static_cast<std::uint32_t>(count);
    block->captured_at_monotonic_ns = monotonic_time_ns();
    std::memcpy(block->samples.data(),
                fixture.mono_samples.data() + static_cast<std::size_t>(offset),
                count * sizeof(float));
    queue.publish_write();
    offset += count;
  }
  running_.store(false, std::memory_order_release);
  if (!stop_requested_.load(std::memory_order_acquire) && completed) {
    try {
      completed();
    } catch (...) {
    }
  }
}

void TimedFixtureProducer::stop() noexcept {
  stop_requested_.store(true, std::memory_order_release);
  wake_.notify_one();
  if (thread_.joinable()) {
    // The producer waits interruptibly on each real-time deadline so a user
    // Stop or Cancel never waits for the rest of a long fixture.
    thread_.join();
  }
  running_.store(false, std::memory_order_release);
}

bool TimedFixtureProducer::running() const noexcept {
  return running_.load(std::memory_order_acquire);
}

std::uint64_t TimedFixtureProducer::started_at_ns() const noexcept {
  return started_at_ns_.load(std::memory_order_acquire);
}

}  // namespace outflow::mic_probe
