#pragma once

#include "audio_pipeline.hpp"
#include "wav_fixture.hpp"

#include <atomic>
#include <cstdint>
#include <functional>
#include <condition_variable>
#include <mutex>
#include <string>
#include <thread>

namespace outflow::mic_probe {

class TimedFixtureProducer final {
 public:
  using CompletionHandler = std::function<void()>;

  TimedFixtureProducer() = default;
  ~TimedFixtureProducer();
  TimedFixtureProducer(const TimedFixtureProducer&) = delete;
  TimedFixtureProducer& operator=(const TimedFixtureProducer&) = delete;

  bool start(WavFixture fixture, std::uint32_t maximum_seconds,
             BoundedAudioQueue& queue, CompletionHandler completed,
             std::string& error) noexcept;
  void stop() noexcept;
  bool running() const noexcept;
  std::uint64_t started_at_ns() const noexcept;

 private:
  void run(WavFixture fixture, std::uint64_t sample_limit,
           BoundedAudioQueue& queue, CompletionHandler completed) noexcept;

  std::thread thread_;
  std::mutex wait_mutex_;
  std::condition_variable wake_;
  std::atomic<bool> stop_requested_{false};
  std::atomic<bool> running_{false};
  std::atomic<std::uint64_t> started_at_ns_{0};
};

}  // namespace outflow::mic_probe
