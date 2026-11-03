#include "audio_pipeline.hpp"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <limits>

namespace outflow::mic_probe {
namespace {

std::size_t bytes_per_sample(PcmEncoding encoding) noexcept {
  switch (encoding) {
    case PcmEncoding::Float32:
    case PcmEncoding::SignedInt32:
      return 4;
    case PcmEncoding::SignedInt16:
      return 2;
    case PcmEncoding::UnsignedInt8:
      return 1;
  }
  return 0;
}

bool read_sample(const std::byte* bytes, PcmEncoding encoding,
                 float& sample) noexcept {
  switch (encoding) {
    case PcmEncoding::Float32: {
      float value = 0.0F;
      std::memcpy(&value, bytes, sizeof(value));
      if (!std::isfinite(value)) return false;
      sample = std::clamp(value, -1.0F, 1.0F);
      return true;
    }
    case PcmEncoding::SignedInt16: {
      std::int16_t value = 0;
      std::memcpy(&value, bytes, sizeof(value));
      sample = static_cast<float>(value) / 32768.0F;
      return true;
    }
    case PcmEncoding::SignedInt32: {
      std::int32_t value = 0;
      std::memcpy(&value, bytes, sizeof(value));
      sample = static_cast<float>(static_cast<double>(value) / 2147483648.0);
      return true;
    }
    case PcmEncoding::UnsignedInt8: {
      const auto value = std::to_integer<std::uint8_t>(*bytes);
      sample = (static_cast<float>(value) - 128.0F) / 128.0F;
      return true;
    }
  }
  return false;
}

float mono_at(const AudioBlock& input, std::size_t frame) noexcept {
  double sum = 0.0;
  const auto base = frame * input.format.channels;
  for (std::size_t channel = 0; channel < input.format.channels; ++channel) {
    sum += input.samples[base + channel];
  }
  return static_cast<float>(sum / input.format.channels);
}

}  // namespace

AudioCopyError copy_pcm_to_block(AudioBlock& destination, AudioFormat format,
                                 const AudioBufferView* buffers,
                                 std::size_t buffer_count,
                                 std::uint32_t frame_count) noexcept {
  if (format.sample_rate_hz == 0 || format.channels == 0 || buffers == nullptr ||
      frame_count == 0) {
    return AudioCopyError::InvalidFormat;
  }
  if (format.channels > kMaxCaptureChannels || frame_count > kMaxCaptureFrames) {
    return AudioCopyError::UnsupportedDimensions;
  }
  const std::size_t sample_bytes = bytes_per_sample(format.encoding);
  if (sample_bytes == 0) return AudioCopyError::InvalidFormat;
  const std::size_t expected_buffers = format.interleaved ? 1 : format.channels;
  if (buffer_count != expected_buffers) return AudioCopyError::InvalidFormat;

  for (std::size_t index = 0; index < buffer_count; ++index) {
    if (buffers[index].data == nullptr) return AudioCopyError::MissingBuffer;
  }
  const std::size_t required_bytes = static_cast<std::size_t>(frame_count) *
                                     (format.interleaved ? format.channels : 1U) *
                                     sample_bytes;
  for (std::size_t index = 0; index < buffer_count; ++index) {
    if (buffers[index].byte_length < required_bytes) {
      return AudioCopyError::ShortBuffer;
    }
  }

  auto* output = destination.samples.data();
  for (std::size_t frame = 0; frame < frame_count; ++frame) {
    for (std::size_t channel = 0; channel < format.channels; ++channel) {
      const auto buffer_index = format.interleaved ? 0 : channel;
      const auto sample_index = format.interleaved
                                    ? frame * format.channels + channel
                                    : frame;
      const auto* bytes = static_cast<const std::byte*>(buffers[buffer_index].data) +
                          sample_index * sample_bytes;
      if (!read_sample(bytes, format.encoding,
                       output[frame * format.channels + channel])) {
        return AudioCopyError::NonFiniteSample;
      }
    }
  }
  destination.format = format;
  destination.frame_count = frame_count;
  return AudioCopyError::None;
}

AudioCopyError copy_pcm_frames_to_queue(BoundedAudioQueue& queue,
                                        AudioFormat format,
                                        const AudioBufferView* buffers,
                                        std::size_t buffer_count,
                                        std::uint32_t frame_count,
                                        std::uint64_t callback_time_ns,
                                        bool& queue_overflow) noexcept {
  queue_overflow = false;
  if (format.sample_rate_hz == 0 || format.channels == 0 || buffers == nullptr ||
      frame_count == 0) {
    return AudioCopyError::InvalidFormat;
  }
  if (format.channels > kMaxCaptureChannels ||
      frame_count > kMaxCallbackFrames) {
    return AudioCopyError::UnsupportedDimensions;
  }
  const auto sample_bytes = bytes_per_sample(format.encoding);
  if (sample_bytes == 0) return AudioCopyError::InvalidFormat;
  const std::size_t expected_buffers = format.interleaved ? 1 : format.channels;
  if (buffer_count != expected_buffers) return AudioCopyError::InvalidFormat;
  const std::size_t required_bytes = static_cast<std::size_t>(frame_count) *
                                     (format.interleaved ? format.channels : 1U) *
                                     sample_bytes;
  for (std::size_t index = 0; index < buffer_count; ++index) {
    if (buffers[index].data == nullptr) return AudioCopyError::MissingBuffer;
    if (buffers[index].byte_length < required_bytes) {
      return AudioCopyError::ShortBuffer;
    }
  }

  std::array<AudioBufferView, kMaxCaptureChannels> sliced_buffers{};
  for (std::uint32_t offset = 0; offset < frame_count;) {
    const auto block_frames = static_cast<std::uint32_t>(std::min<std::size_t>(
        kMaxCaptureFrames, static_cast<std::size_t>(frame_count - offset)));
    for (std::size_t index = 0; index < buffer_count; ++index) {
      const auto frame_stride = format.interleaved
                                    ? static_cast<std::size_t>(format.channels) * sample_bytes
                                    : sample_bytes;
      const auto byte_offset = static_cast<std::size_t>(offset) * frame_stride;
      sliced_buffers[index] = AudioBufferView{
          static_cast<const std::byte*>(buffers[index].data) + byte_offset,
          buffers[index].byte_length - byte_offset};
    }

    AudioBlock* block = queue.begin_write();
    if (block == nullptr) {
      queue_overflow = true;
      return AudioCopyError::None;
    }
    const auto copied = copy_pcm_to_block(*block, format, sliced_buffers.data(),
                                          buffer_count, block_frames);
    if (copied != AudioCopyError::None) {
      queue.cancel_write();
      return copied;
    }
    block->captured_at_monotonic_ns = callback_time_ns +
        audio_time_nanoseconds(offset, format.sample_rate_hz);
    queue.publish_write();
    offset += block_frames;
  }
  return AudioCopyError::None;
}

AudioBlock* BoundedAudioQueue::begin_write() noexcept {
  if (write_reserved_) return nullptr;
  const auto write = write_sequence_.load(std::memory_order_relaxed);
  const auto read = read_sequence_.load(std::memory_order_acquire);
  if (write - read >= kAudioQueueSlots) {
    overflow_count_.fetch_add(1, std::memory_order_relaxed);
    overflowed_.store(true, std::memory_order_release);
    wake();
    return nullptr;
  }
  reserved_write_sequence_ = write;
  write_reserved_ = true;
  return &slots_[write % kAudioQueueSlots];
}

void BoundedAudioQueue::cancel_write() noexcept { write_reserved_ = false; }

void BoundedAudioQueue::publish_write() noexcept {
  if (!write_reserved_) return;
  write_sequence_.store(reserved_write_sequence_ + 1, std::memory_order_release);
  write_reserved_ = false;
  const auto current_depth = static_cast<std::size_t>(
      reserved_write_sequence_ + 1 - read_sequence_.load(std::memory_order_acquire));
  auto maximum = maximum_depth_.load(std::memory_order_relaxed);
  while (current_depth > maximum &&
         !maximum_depth_.compare_exchange_weak(maximum, current_depth,
                                               std::memory_order_relaxed)) {
  }
  wake();
}

const AudioBlock* BoundedAudioQueue::begin_read() noexcept {
  if (read_reserved_) return nullptr;
  const auto read = read_sequence_.load(std::memory_order_relaxed);
  if (read == write_sequence_.load(std::memory_order_acquire)) return nullptr;
  read_reserved_ = true;
  return &slots_[read % kAudioQueueSlots];
}

void BoundedAudioQueue::release_read() noexcept {
  if (!read_reserved_) return;
  read_sequence_.fetch_add(1, std::memory_order_release);
  read_reserved_ = false;
}

void BoundedAudioQueue::wake() noexcept {
  wake_generation_.fetch_add(1, std::memory_order_release);
  wake_generation_.notify_one();
}

void BoundedAudioQueue::wait_for_wake(std::uint64_t observed_generation) noexcept {
  wake_generation_.wait(observed_generation, std::memory_order_acquire);
}

std::uint64_t BoundedAudioQueue::wake_generation() const noexcept {
  return wake_generation_.load(std::memory_order_acquire);
}

std::size_t BoundedAudioQueue::depth() const noexcept {
  const auto write = write_sequence_.load(std::memory_order_acquire);
  const auto read = read_sequence_.load(std::memory_order_acquire);
  return static_cast<std::size_t>(write - read);
}

std::size_t BoundedAudioQueue::maximum_depth() const noexcept {
  return maximum_depth_.load(std::memory_order_relaxed);
}

std::uint64_t BoundedAudioQueue::overflow_count() const noexcept {
  return overflow_count_.load(std::memory_order_relaxed);
}

bool BoundedAudioQueue::overflowed() const noexcept {
  return overflowed_.load(std::memory_order_acquire);
}

void BoundedAudioQueue::set_producer_error(std::int32_t code) noexcept {
  producer_error_.store(code, std::memory_order_release);
  wake();
}

std::int32_t BoundedAudioQueue::producer_error() const noexcept {
  return producer_error_.load(std::memory_order_acquire);
}

void BoundedAudioQueue::clear_after_producer_stops() noexcept {
  for (auto& block : slots_) {
    std::fill(block.samples.begin(), block.samples.end(), 0.0F);
    block.frame_count = 0;
    block.captured_at_monotonic_ns = 0;
  }
  const auto write = write_sequence_.load(std::memory_order_acquire);
  read_sequence_.store(write, std::memory_order_release);
}

bool MonoResampler::process(const AudioBlock& input,
                            std::vector<float>& output) noexcept {
  if (input.frame_count == 0 || input.format.sample_rate_hz < 8'000 ||
      input.format.sample_rate_hz > 192'000 ||
      input.format.channels == 0 || input.format.channels > kMaxCaptureChannels ||
      input.frame_count > kMaxCaptureFrames) {
    return false;
  }
  if (!initialized_) {
    source_rate_hz_ = input.format.sample_rate_hz;
    channel_count_ = input.format.channels;
    initialized_ = true;
  } else if (source_rate_hz_ != input.format.sample_rate_hz ||
             channel_count_ != input.format.channels) {
    // A device format change needs a clean stream; never reinterpret queued
    // samples with a new clock or silently stretch speech.
    return false;
  }
  output.clear();
  if (output.capacity() < kMaxCaptureFrames * 2 + 2) return false;
  const double step = static_cast<double>(source_rate_hz_) / kModelSampleRateHz;
  for (std::size_t frame = 0; frame < input.frame_count; ++frame) {
    const float current = mono_at(input, frame);
    const std::uint64_t global_position = total_input_frames_ + frame;
    if (!have_previous_) {
      output.push_back(current);
      next_output_position_ = step;
      previous_mono_sample_ = current;
      have_previous_ = true;
      continue;
    }
    const double interval_start = static_cast<double>(global_position - 1);
    const double interval_end = static_cast<double>(global_position);
    while (next_output_position_ <= interval_end) {
      const double fraction = std::clamp(next_output_position_ - interval_start, 0.0, 1.0);
      output.push_back(static_cast<float>(
          previous_mono_sample_ * (1.0 - fraction) + current * fraction));
      next_output_position_ += step;
    }
    previous_mono_sample_ = current;
  }
  total_input_frames_ += input.frame_count;
  return true;
}

void MonoResampler::reset() noexcept {
  initialized_ = false;
  have_previous_ = false;
  source_rate_hz_ = 0;
  channel_count_ = 0;
  total_input_frames_ = 0;
  next_output_position_ = 0.0;
  previous_mono_sample_ = 0.0F;
}

std::uint32_t MonoResampler::source_rate_hz() const noexcept {
  return source_rate_hz_;
}

std::uint64_t audio_time_nanoseconds(std::uint64_t sample_count,
                                     std::uint32_t sample_rate_hz) noexcept {
  if (sample_rate_hz == 0) return 0;
  const long double nanoseconds =
      static_cast<long double>(sample_count) * 1'000'000'000.0L / sample_rate_hz;
  if (nanoseconds >= std::ldexp(1.0L, 64)) {
    return std::numeric_limits<std::uint64_t>::max();
  }
  return static_cast<std::uint64_t>(nanoseconds + 0.5L);
}

}  // namespace outflow::mic_probe
