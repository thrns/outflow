#include "wav_fixture.hpp"

#include <array>
#include <fstream>
#include <limits>

namespace outflow::mic_probe {
namespace {

constexpr std::uint64_t kMaximumFixtureBytes = 16 * 1024 * 1024;
constexpr std::uint64_t kMaximumFixtureSeconds = 120;

std::uint16_t read_u16(const std::array<std::uint8_t, 16>& bytes,
                       std::size_t offset) noexcept {
  return static_cast<std::uint16_t>(bytes[offset]) |
         (static_cast<std::uint16_t>(bytes[offset + 1]) << 8);
}

std::uint32_t read_u32(const std::array<std::uint8_t, 16>& bytes,
                       std::size_t offset) noexcept {
  return static_cast<std::uint32_t>(bytes[offset]) |
         (static_cast<std::uint32_t>(bytes[offset + 1]) << 8) |
         (static_cast<std::uint32_t>(bytes[offset + 2]) << 16) |
         (static_cast<std::uint32_t>(bytes[offset + 3]) << 24);
}

}  // namespace

bool load_pcm16_mono_wav(const std::filesystem::path& path, WavFixture& fixture,
                         std::string& error) noexcept {
  try {
    const auto file_size = std::filesystem::file_size(path);
    if (file_size < 44 || file_size > kMaximumFixtureBytes) {
      error = "fixture size is outside the 44-byte to 16-MiB limit";
      return false;
    }
    std::ifstream input(path, std::ios::binary);
    if (!input) {
      error = "fixture could not be opened";
      return false;
    }
    std::array<std::uint8_t, 12> header{};
    input.read(reinterpret_cast<char*>(header.data()), header.size());
    if (!input || std::string(reinterpret_cast<const char*>(header.data()), 4) != "RIFF" ||
        std::string(reinterpret_cast<const char*>(header.data() + 8), 4) != "WAVE") {
      error = "fixture is not a RIFF/WAVE file";
      return false;
    }

    std::uint16_t audio_format = 0;
    std::uint16_t channels = 0;
    std::uint16_t bits_per_sample = 0;
    std::uint32_t sample_rate = 0;
    std::vector<std::uint8_t> audio_bytes;
    bool found_format = false;
    bool found_data = false;
    while (input && !found_data) {
      std::array<std::uint8_t, 8> chunk_header{};
      input.read(reinterpret_cast<char*>(chunk_header.data()), chunk_header.size());
      if (!input) break;
      const std::string chunk_id(reinterpret_cast<const char*>(chunk_header.data()), 4);
      const std::uint32_t chunk_size =
          static_cast<std::uint32_t>(chunk_header[4]) |
          (static_cast<std::uint32_t>(chunk_header[5]) << 8) |
          (static_cast<std::uint32_t>(chunk_header[6]) << 16) |
          (static_cast<std::uint32_t>(chunk_header[7]) << 24);
      const auto current_position = input.tellg();
      if (current_position < 0 ||
          static_cast<std::uint64_t>(current_position) + chunk_size > file_size) {
        error = "fixture contains a truncated chunk";
        return false;
      }
      if (chunk_id == "fmt ") {
        if (chunk_size < 16 || chunk_size > 4096) {
          error = "fixture format chunk is unsupported";
          return false;
        }
        std::array<std::uint8_t, 16> format{};
        input.read(reinterpret_cast<char*>(format.data()), format.size());
        if (!input) {
          error = "fixture format chunk is truncated";
          return false;
        }
        audio_format = read_u16(format, 0);
        channels = read_u16(format, 2);
        sample_rate = read_u32(format, 4);
        bits_per_sample = read_u16(format, 14);
        input.seekg(static_cast<std::streamoff>(chunk_size - format.size()),
                    std::ios::cur);
        found_format = static_cast<bool>(input);
      } else if (chunk_id == "data") {
        if (chunk_size == 0 || chunk_size > kMaximumFixtureBytes) {
          error = "fixture audio data is empty or exceeds 16 MiB";
          return false;
        }
        audio_bytes.resize(chunk_size);
        input.read(reinterpret_cast<char*>(audio_bytes.data()), chunk_size);
        if (!input) {
          error = "fixture audio data is truncated";
          return false;
        }
        found_data = true;
      } else {
        input.seekg(chunk_size, std::ios::cur);
      }
      if ((chunk_size & 1U) != 0) input.seekg(1, std::ios::cur);
    }
    if (!found_format || !found_data || audio_format != 1 || channels != 1 ||
        bits_per_sample != 16 || sample_rate < 8'000 || sample_rate > 192'000 ||
        (audio_bytes.size() & 1U) != 0) {
      error = "fixture must be mono PCM16 WAV at 8–192 kHz";
      return false;
    }
    const std::uint64_t frame_count = audio_bytes.size() / 2;
    if (frame_count == 0 || frame_count >
                                 static_cast<std::uint64_t>(sample_rate) *
                                     kMaximumFixtureSeconds) {
      error = "fixture duration must be between 0 and 120 seconds";
      return false;
    }
    WavFixture loaded;
    loaded.sample_rate_hz = sample_rate;
    loaded.mono_samples.reserve(static_cast<std::size_t>(frame_count));
    for (std::size_t index = 0; index < audio_bytes.size(); index += 2) {
      const std::uint16_t bits = static_cast<std::uint16_t>(audio_bytes[index]) |
                                 (static_cast<std::uint16_t>(audio_bytes[index + 1])
                                  << 8);
      const auto sample = static_cast<std::int16_t>(bits);
      loaded.mono_samples.push_back(static_cast<float>(sample) / 32768.0F);
    }
    fixture = std::move(loaded);
    return true;
  } catch (const std::exception&) {
    error = "fixture could not be read within the supported bounds";
    return false;
  }
}

}  // namespace outflow::mic_probe
