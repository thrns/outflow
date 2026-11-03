#pragma once

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace outflow::mic_probe {

struct WavFixture {
  std::uint32_t sample_rate_hz = 0;
  std::vector<float> mono_samples;
};

bool load_pcm16_mono_wav(const std::filesystem::path& path, WavFixture& fixture,
                         std::string& error) noexcept;

}  // namespace outflow::mic_probe
