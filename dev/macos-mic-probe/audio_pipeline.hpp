#pragma once

#include <array>
#include <atomic>
#include <cstddef>
#include <cstdint>
#include <vector>

namespace outflow::mic_probe {

inline constexpr std::uint32_t kModelSampleRateHz = 16'000;
inline constexpr std::size_t kMaxCaptureFrames = 4'096;
inline constexpr std::size_t kMaxCaptureChannels = 8;
inline constexpr std::size_t kAudioQueueSlots = 16;
inline constexpr std::size_t kMaxCallbackFrames =
    kMaxCaptureFrames * kAudioQueueSlots;

enum class PcmEncoding : std::uint8_t { Float32, SignedInt16, SignedInt32, UnsignedInt8 };

struct AudioFormat {
  std::uint32_t sample_rate_hz = 0;
  std::uint8_t channels = 0;
  PcmEncoding encoding = PcmEncoding::Float32;
  bool interleaved = false;
};

struct AudioBufferView {
  const void* data = nullptr;
  std::size_t byte_length = 0;
};

struct AudioBlock {
  AudioFormat format{};
  std::uint32_t frame_count = 0;
  std::uint64_t captured_at_monotonic_ns = 0;
  // Fixed storage keeps the microphone callback allocation-free and bounds the
  // amount of unprocessed audio even when the inference worker falls behind.
  std::array<float, kMaxCaptureFrames * kMaxCaptureChannels> samples{};
};

enum class AudioCopyError : std::uint8_t {
  None,
  InvalidFormat,
  UnsupportedDimensions,
  MissingBuffer,
  ShortBuffer,
  NonFiniteSample,
};

class BoundedAudioQueue;

AudioCopyError copy_pcm_to_block(AudioBlock& destination, AudioFormat format,
                                 const AudioBufferView* buffers,
                                 std::size_t buffer_count,
                                 std::uint32_t frame_count) noexcept;

// Copy one platform callback into one or more fixed queue blocks. The callback
// work is bounded to kMaxCallbackFrames and never allocates; queue exhaustion
// is reported separately because BoundedAudioQueue also latches overflow.
AudioCopyError copy_pcm_frames_to_queue(BoundedAudioQueue& queue,
                                        AudioFormat format,
                                        const AudioBufferView* buffers,
                                        std::size_t buffer_count,
                                        std::uint32_t frame_count,
                                        std::uint64_t callback_time_ns,
                                        bool& queue_overflow) noexcept;

class BoundedAudioQueue final {
 public:
  AudioBlock* begin_write() noexcept;
  void cancel_write() noexcept;
  void publish_write() noexcept;
  const AudioBlock* begin_read() noexcept;
  void release_read() noexcept;

  void wake() noexcept;
  void wait_for_wake(std::uint64_t observed_generation) noexcept;
  std::uint64_t wake_generation() const noexcept;
  std::size_t depth() const noexcept;
  std::size_t maximum_depth() const noexcept;
  std::uint64_t overflow_count() const noexcept;
  bool overflowed() const noexcept;
  void set_producer_error(std::int32_t code) noexcept;
  std::int32_t producer_error() const noexcept;
  void clear_after_producer_stops() noexcept;

 private:
  std::array<AudioBlock, kAudioQueueSlots> slots_{};
  alignas(64) std::atomic<std::uint64_t> write_sequence_{0};
  alignas(64) std::atomic<std::uint64_t> read_sequence_{0};
  std::atomic<std::uint64_t> wake_generation_{0};
  std::atomic<std::uint64_t> overflow_count_{0};
  std::atomic<std::size_t> maximum_depth_{0};
  std::atomic<bool> overflowed_{false};
  std::atomic<std::int32_t> producer_error_{0};
  std::uint64_t reserved_write_sequence_ = 0;
  bool write_reserved_ = false;
  bool read_reserved_ = false;
};

class MonoResampler final {
 public:
  bool process(const AudioBlock& input, std::vector<float>& output) noexcept;
  void reset() noexcept;
  std::uint32_t source_rate_hz() const noexcept;

 private:
  bool initialized_ = false;
  bool have_previous_ = false;
  std::uint32_t source_rate_hz_ = 0;
  std::uint8_t channel_count_ = 0;
  std::uint64_t total_input_frames_ = 0;
  double next_output_position_ = 0.0;
  float previous_mono_sample_ = 0.0F;
};

std::uint64_t audio_time_nanoseconds(std::uint64_t sample_count,
                                     std::uint32_t sample_rate_hz) noexcept;

}  // namespace outflow::mic_probe
