#import <AVFoundation/AVFoundation.h>
#import <AudioToolbox/AudioToolbox.h>
#import <Foundation/Foundation.h>

#include "macos_capture.hpp"

#include "session_worker.hpp"

#include <array>
#include <atomic>
#include <thread>

namespace outflow::mic_probe {

struct MacMicrophoneCapture::Impl final {
  AVAudioEngine* __strong engine = nil;
  id __strong configuration_observer = nil;
  BoundedAudioQueue* queue = nullptr;
  MicrophoneFormatInfo input_format{};
  std::atomic<bool> is_active{false};
  std::atomic<std::uint32_t> callbacks_in_flight{0};
  std::uint64_t capture_started_ns = 0;
  bool engine_started = false;

  bool discover_format(const AVAudioFormat* format, std::string& error) noexcept {
    if (format == nil || format.streamDescription == nullptr ||
        format.channelCount == 0 || format.channelCount > kMaxCaptureChannels ||
        format.sampleRate < 8'000.0 || format.sampleRate > 192'000.0) {
      error = "No supported microphone input format is available";
      return false;
    }
    const auto* asbd = format.streamDescription;
    input_format.sample_rate_hz = static_cast<std::uint32_t>(format.sampleRate + 0.5);
    input_format.channels = static_cast<std::uint8_t>(format.channelCount);
    input_format.interleaved = (asbd->mFormatFlags & kAudioFormatFlagIsNonInterleaved) == 0;
    if ((asbd->mFormatFlags & kAudioFormatFlagIsFloat) != 0 &&
        asbd->mBitsPerChannel == 32) {
      input_format.encoding = PcmEncoding::Float32;
    } else if ((asbd->mFormatFlags & kAudioFormatFlagIsSignedInteger) != 0 &&
               asbd->mBitsPerChannel == 16) {
      input_format.encoding = PcmEncoding::SignedInt16;
    } else if ((asbd->mFormatFlags & kAudioFormatFlagIsSignedInteger) != 0 &&
               asbd->mBitsPerChannel == 32) {
      input_format.encoding = PcmEncoding::SignedInt32;
    } else if (asbd->mBitsPerChannel == 8) {
      input_format.encoding = PcmEncoding::UnsignedInt8;
    } else {
      error = "Microphone uses a PCM format the probe cannot convert";
      return false;
    }
    return true;
  }

  void handle_configuration_change() noexcept {
    if (is_active.load(std::memory_order_acquire) && queue != nullptr) {
      queue->set_producer_error(-106);
    }
  }

  void input_callback(AVAudioPCMBuffer* buffer) noexcept {
    callbacks_in_flight.fetch_add(1, std::memory_order_acq_rel);
    if (!is_active.load(std::memory_order_acquire) || queue == nullptr || buffer == nil) {
      callbacks_in_flight.fetch_sub(1, std::memory_order_release);
      return;
    }
    // AVAudioEngine can deliver an empty callback while the route is starting
    // or changing; it carries no samples and is not a malformed PCM block.
    if (buffer.frameLength == 0) {
      callbacks_in_flight.fetch_sub(1, std::memory_order_release);
      return;
    }
    const auto* format = buffer.format.streamDescription;
    if (format == nullptr || buffer.format.channelCount != input_format.channels ||
        static_cast<std::uint32_t>(buffer.format.sampleRate + 0.5) !=
            input_format.sample_rate_hz) {
      queue->set_producer_error(-105);
      callbacks_in_flight.fetch_sub(1, std::memory_order_release);
      return;
    }
    const AudioBufferList* audio_list = buffer.audioBufferList;
    if (audio_list == nullptr || audio_list->mNumberBuffers == 0 ||
        audio_list->mNumberBuffers > kMaxCaptureChannels) {
      queue->set_producer_error(-109);
      callbacks_in_flight.fetch_sub(1, std::memory_order_release);
      return;
    }
    std::array<AudioBufferView, kMaxCaptureChannels> views{};
    for (std::size_t index = 0; index < audio_list->mNumberBuffers; ++index) {
      views[index] = AudioBufferView{audio_list->mBuffers[index].mData,
                                     audio_list->mBuffers[index].mDataByteSize};
    }
    const AudioFormat format_info{input_format.sample_rate_hz, input_format.channels,
                                  input_format.encoding, input_format.interleaved};
    bool queue_overflow = false;
    const auto copied = copy_pcm_frames_to_queue(
        *queue, format_info, views.data(), audio_list->mNumberBuffers,
        static_cast<std::uint32_t>(buffer.frameLength), monotonic_time_ns(),
        queue_overflow);
    if (queue_overflow) {
      if (!queue->overflowed()) queue->set_producer_error(-116);
      callbacks_in_flight.fetch_sub(1, std::memory_order_release);
      return;
    }
    if (copied != AudioCopyError::None) {
      queue->set_producer_error(-110 - static_cast<std::int32_t>(copied));
      callbacks_in_flight.fetch_sub(1, std::memory_order_release);
      return;
    }
    callbacks_in_flight.fetch_sub(1, std::memory_order_release);
  }

  std::uint64_t stop() noexcept {
    is_active.store(false, std::memory_order_release);
    if (engine != nil) {
      @try {
        [engine.inputNode removeTapOnBus:0];
      } @catch (__unused NSException* exception) {
      }
      if (engine_started) {
        [engine stop];
        engine_started = false;
      }
    }
    if (configuration_observer != nil) {
      [[NSNotificationCenter defaultCenter] removeObserver:configuration_observer];
      configuration_observer = nil;
    }
    while (callbacks_in_flight.load(std::memory_order_acquire) != 0) {
      std::this_thread::yield();
    }
    queue = nullptr;
    engine = nil;
    return monotonic_time_ns();
  }

  ~Impl() { (void)stop(); }
};

MacMicrophoneCapture::MacMicrophoneCapture() : impl_(std::make_shared<Impl>()) {}

MacMicrophoneCapture::~MacMicrophoneCapture() {
  if (impl_) (void)impl_->stop();
}

bool MacMicrophoneCapture::start(BoundedAudioQueue& queue,
                                 std::string& error) noexcept {
  if (!impl_ || impl_->is_active.load(std::memory_order_acquire)) {
    error = "Microphone capture is already active";
    return false;
  }
  @try {
    impl_->engine = [[AVAudioEngine alloc] init];
    AVAudioInputNode* input = impl_->engine.inputNode;
    AVAudioFormat* format = [input inputFormatForBus:0];
    if (!impl_->discover_format(format, error)) {
      impl_->engine = nil;
      return false;
    }
    impl_->queue = &queue;
    std::weak_ptr<Impl> weak_impl = impl_;
    Impl* const callback_context = impl_.get();
    impl_->configuration_observer = [[NSNotificationCenter defaultCenter]
        addObserverForName:AVAudioEngineConfigurationChangeNotification
                    object:impl_->engine
                     queue:[NSOperationQueue mainQueue]
                usingBlock:^(__unused NSNotification* notification) {
                  if (const auto locked = weak_impl.lock()) {
                    locked->handle_configuration_change();
                  }
                }];
    impl_->is_active.store(true, std::memory_order_release);
    [input installTapOnBus:0
                bufferSize:static_cast<AVAudioFrameCount>(kMaxCaptureFrames)
                    format:nil
                     block:^(AVAudioPCMBuffer* buffer, __unused AVAudioTime* time) {
                       callback_context->input_callback(buffer);
                     }];
    [impl_->engine prepare];
    NSError* start_error = nil;
    impl_->capture_started_ns = monotonic_time_ns();
    if (![impl_->engine startAndReturnError:&start_error]) {
      error = "Microphone input could not be opened (AVAudioEngine start failed)";
      (void)impl_->stop();
      return false;
    }
    impl_->engine_started = true;
    return true;
  } @catch (NSException* exception) {
    (void)exception;
    error = "Microphone input could not be initialized";
    (void)impl_->stop();
    return false;
  }
}

std::uint64_t MacMicrophoneCapture::stop() noexcept {
  return impl_ ? impl_->stop() : monotonic_time_ns();
}

bool MacMicrophoneCapture::active() const noexcept {
  return impl_ && impl_->is_active.load(std::memory_order_acquire);
}

MicrophoneFormatInfo MacMicrophoneCapture::format() const noexcept {
  return impl_ ? impl_->input_format : MicrophoneFormatInfo{};
}

std::uint64_t MacMicrophoneCapture::capture_started_at_ns() const noexcept {
  return impl_ ? impl_->capture_started_ns : 0;
}

}  // namespace outflow::mic_probe
