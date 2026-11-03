#pragma once

#include "audio_pipeline.hpp"

#include <cstdint>
#include <memory>
#include <string>

namespace outflow::mic_probe {

struct MicrophoneFormatInfo {
  std::uint32_t sample_rate_hz = 0;
  std::uint8_t channels = 0;
  PcmEncoding encoding = PcmEncoding::Float32;
  bool interleaved = false;
};

class MacMicrophoneCapture final {
 public:
  MacMicrophoneCapture();
  ~MacMicrophoneCapture();
  MacMicrophoneCapture(const MacMicrophoneCapture&) = delete;
  MacMicrophoneCapture& operator=(const MacMicrophoneCapture&) = delete;

  bool start(BoundedAudioQueue& queue, std::string& error) noexcept;
  std::uint64_t stop() noexcept;
  bool active() const noexcept;
  MicrophoneFormatInfo format() const noexcept;
  std::uint64_t capture_started_at_ns() const noexcept;

 private:
  struct Impl;
  std::shared_ptr<Impl> impl_;
};

}  // namespace outflow::mic_probe
