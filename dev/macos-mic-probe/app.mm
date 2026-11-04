#import <AVFoundation/AVFoundation.h>
#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>

#include "copy_result_policy.hpp"
#include "fixture_producer.hpp"
#include "macos_capture.hpp"
#include "model_verifier.hpp"
#include "permission_policy.hpp"
#include "session_state.hpp"
#include "session_worker.hpp"
#include "wav_fixture.hpp"

#include <fcntl.h>
#include <dlfcn.h>
#include <mach-o/dyld.h>
#include <unistd.h>

#include <algorithm>
#include <charconv>
#include <cstdlib>
#include <filesystem>
#include <iostream>
#include <limits.h>
#include <memory>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>
#include <utility>

using namespace outflow::mic_probe;

namespace {

struct LaunchOptions {
  std::filesystem::path model_directory;
  std::filesystem::path fixture_path;
  std::filesystem::path metrics_path;
  std::uint32_t fixture_seconds = 12;
  std::uint32_t auto_cancel_after_ms = 0;
  bool fixture_mode = false;
  bool auto_exit_after_final = false;
  bool auto_exit_after_cancel = false;
};

std::optional<std::uint32_t> parse_unsigned(const char* value) {
  if (value == nullptr || *value == '\0') return std::nullopt;
  std::uint32_t parsed = 0;
  const char* end = value + std::char_traits<char>::length(value);
  const auto result = std::from_chars(value, end, parsed);
  if (result.ec != std::errc{} || result.ptr != end) return std::nullopt;
  return parsed;
}

LaunchOptions read_launch_options() {
  LaunchOptions result;
  NSArray<NSString*>* arguments = NSProcessInfo.processInfo.arguments;
  for (NSUInteger index = 1; index < arguments.count; ++index) {
    NSString* argument = arguments[index];
    auto next_value = [&]() -> NSString* {
      if (index + 1 >= arguments.count) return nil;
      return arguments[++index];
    };
    if ([argument isEqualToString:@"--model-dir"]) {
      NSString* value = next_value();
      if (value != nil) result.model_directory = value.fileSystemRepresentation;
    } else if ([argument isEqualToString:@"--timed-fixture"]) {
      NSString* value = next_value();
      if (value != nil) {
        result.fixture_path = value.fileSystemRepresentation;
        result.fixture_mode = true;
      }
    } else if ([argument isEqualToString:@"--fixture-seconds"]) {
      NSString* value = next_value();
      if (const auto parsed = parse_unsigned(value.UTF8String)) {
        result.fixture_seconds = std::min<std::uint32_t>(*parsed, 120);
      }
    } else if ([argument isEqualToString:@"--metrics-file"]) {
      NSString* value = next_value();
      if (value != nil) result.metrics_path = value.fileSystemRepresentation;
    } else if ([argument isEqualToString:@"--auto-cancel-after-ms"]) {
      NSString* value = next_value();
      if (const auto parsed = parse_unsigned(value.UTF8String)) {
        result.auto_cancel_after_ms = std::min<std::uint32_t>(*parsed, 120'000);
      }
    } else if ([argument isEqualToString:@"--auto-exit-after-final"]) {
      result.auto_exit_after_final = true;
    } else if ([argument isEqualToString:@"--auto-exit-after-cancel"]) {
      result.auto_exit_after_cancel = true;
    }
  }
  return result;
}

NSString* ns_string(const std::string& value) {
  NSString* result = [[NSString alloc] initWithBytes:value.data()
                                              length:value.size()
                                            encoding:NSUTF8StringEncoding];
  return result != nil ? result : @"Transcript text was not valid UTF-8.";
}

double elapsed_milliseconds(std::uint64_t start_ns,
                            std::uint64_t end_ns) noexcept {
  if (start_ns == 0 || end_ns < start_ns) return 0.0;
  return static_cast<double>(end_ns - start_ns) / 1'000'000.0;
}

void clear_sensitive_text(std::string& text) noexcept {
  auto* bytes = reinterpret_cast<volatile unsigned char*>(text.data());
  for (std::size_t index = 0; index < text.size(); ++index) bytes[index] = 0;
  text.clear();
}

MicrophonePermission microphone_permission() {
  switch ([AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio]) {
    case AVAuthorizationStatusAuthorized:
      return MicrophonePermission::Authorized;
    case AVAuthorizationStatusDenied:
      return MicrophonePermission::Denied;
    case AVAuthorizationStatusRestricted:
      return MicrophonePermission::Restricted;
    case AVAuthorizationStatusNotDetermined:
      return MicrophonePermission::NotDetermined;
  }
  return MicrophonePermission::Unknown;
}

NSString* permission_status_text() {
  switch (microphone_permission()) {
    case MicrophonePermission::Authorized:
      return @"Microphone permission: Granted";
    case MicrophonePermission::Denied:
      return @"Microphone permission: Denied — allow access in System Settings.";
    case MicrophonePermission::Restricted:
      return @"Microphone permission: Restricted by macOS.";
    case MicrophonePermission::NotDetermined:
      return @"Microphone permission: Not requested — Start will ask.";
    case MicrophonePermission::Unknown:
      return @"Microphone permission: Unknown";
  }
  return @"Microphone permission: Unknown";
}

std::string metrics_json(const SessionMetrics& metrics) {
  std::ostringstream output;
  output.setf(std::ios::fixed);
  output.precision(3);
  output << "{\"schema_version\":1,\"source\":\""
         << (metrics.source == AudioSourceKind::Microphone ? "microphone" : "timed_fixture")
         << "\",\"model_load_ms\":" << metrics.model_load_ms
         << ",\"first_frame_after_capture_start_ms\":"
         << metrics.first_frame_after_capture_start_ms
         << ",\"first_partial_after_first_frame_ms\":"
         << metrics.first_partial_after_first_frame_ms
         << ",\"mean_partial_cadence_ms\":" << metrics.mean_partial_cadence_ms
         << ",\"audio_duration_seconds\":" << metrics.audio_duration_seconds
         << ",\"capture_duration_ms\":" << metrics.capture_duration_ms
         << ",\"stop_to_final_ms\":" << metrics.stop_to_final_ms
         << ",\"total_session_ms\":" << metrics.total_session_ms
         << ",\"microphone_release_latency_ms\":"
         << metrics.microphone_release_latency_ms
         << ",\"process_cpu_ms\":" << metrics.process_cpu_ms
         << ",\"partial_update_count\":" << metrics.partial_update_count
         << ",\"queue_overflow_count\":" << metrics.queue_overflow_count
         << ",\"queue_maximum_depth\":" << metrics.queue_maximum_depth
         << ",\"native_process_peak_rss_bytes\":" << metrics.peak_rss_bytes
         << ",\"native_thread_count\":" << metrics.native_thread_count << "}\n";
  return output.str();
}

bool write_metrics_exclusive(const std::filesystem::path& path,
                             const SessionMetrics& metrics) noexcept {
  if (path.empty()) return true;
  const auto json = metrics_json(metrics);
  const int descriptor = ::open(path.c_str(), O_WRONLY | O_CREAT | O_EXCL, 0600);
  if (descriptor < 0) return false;
  std::size_t offset = 0;
  while (offset < json.size()) {
    const auto written = ::write(descriptor, json.data() + offset, json.size() - offset);
    if (written <= 0) {
      (void)::close(descriptor);
      return false;
    }
    offset += static_cast<std::size_t>(written);
  }
  return ::close(descriptor) == 0;
}

bool write_clipboard_text(std::string_view text) {
  // Convert before touching the system pasteboard. A single writeObjects:
  // call reports the OS result; no clear/declare pre-write is performed, and
  // NSPasteboard does not promise transactional rollback.
  NSString* value = [[NSString alloc] initWithBytes:text.data()
                                             length:text.size()
                                           encoding:NSUTF8StringEncoding];
  if (value == nil) return false;
  return [[NSPasteboard generalPasteboard] writeObjects:@[value]];
}

NSString* clipboard_write_failure_message() {
  // This reads only macOS's per-app access policy, never pasteboard contents.
  switch ([NSPasteboard generalPasteboard].accessBehavior) {
    case NSPasteboardAccessBehaviorAlwaysDeny:
      return @"macOS blocks clipboard access for this app. Check System Settings, or select the text above and press ⌘C.";
    case NSPasteboardAccessBehaviorAsk:
      return @"macOS did not allow the clipboard write. Check System Settings, or select the text above and press ⌘C.";
    case NSPasteboardAccessBehaviorDefault:
    case NSPasteboardAccessBehaviorAlwaysAllow:
      return @"Copy failed. Select the text above, then press ⌘C to copy it manually.";
  }
  return @"Copy failed. Select the text above, then press ⌘C to copy it manually.";
}

bool print_runtime_identity(const char* executable_path) {
  char resolved_executable[PATH_MAX]{};
  if (executable_path == nullptr ||
      ::realpath(executable_path, resolved_executable) == nullptr) {
    return false;
  }
  Dl_info moonshine_info{};
  const auto* api_address = reinterpret_cast<const void*>(
      reinterpret_cast<std::uintptr_t>(&moonshine_get_version));
  if (::dladdr(api_address, &moonshine_info) == 0 ||
      moonshine_info.dli_fname == nullptr) {
    return false;
  }
#if defined(__arm64__)
  constexpr const char* architecture = "arm64";
#elif defined(__x86_64__)
  constexpr const char* architecture = "x86_64";
#else
  constexpr const char* architecture = "unsupported";
#endif
  std::cout << "{\"schema_version\":1,\"architecture\":\"" << architecture
            << "\",\"pointer_bits\":" << (sizeof(void*) * 8)
            << ",\"c_api_version\":" << moonshine_get_version()
            << ",\"runner_path\":\"" << resolved_executable
            << "\",\"moonshine_api_path\":\"" << moonshine_info.dli_fname
            << "\",\"loaded_images\":[";
  const auto image_count = _dyld_image_count();
  for (std::uint32_t index = 0; index < image_count; ++index) {
    const char* image_path = _dyld_get_image_name(index);
    if (image_path == nullptr) continue;
    if (index != 0) std::cout << ',';
    std::cout << '"' << image_path << '"';
  }
  std::cout << "]}\n";
  return std::cout.good();
}

NSButton* make_button(NSString* title, id target, SEL action) {
  NSButton* button = [NSButton buttonWithTitle:title target:target action:action];
  button.bezelStyle = NSBezelStyleRounded;
  button.translatesAutoresizingMaskIntoConstraints = NO;
  button.accessibilityLabel = title;
  return button;
}

NSTextField* make_label(NSString* text, CGFloat font_size, BOOL bold) {
  NSTextField* label = [NSTextField labelWithString:text];
  label.translatesAutoresizingMaskIntoConstraints = NO;
  label.font = bold ? [NSFont boldSystemFontOfSize:font_size]
                    : [NSFont systemFontOfSize:font_size];
  label.lineBreakMode = NSLineBreakByWordWrapping;
  label.maximumNumberOfLines = 0;
  return label;
}

struct PreviewViews {
  NSScrollView* __strong scroll;
  NSTextView* __strong text;
};

PreviewViews make_preview() {
  NSScrollView* scroll = [[NSScrollView alloc] initWithFrame:NSZeroRect];
  scroll.translatesAutoresizingMaskIntoConstraints = NO;
  scroll.hasVerticalScroller = YES;
  scroll.borderType = NSBezelBorder;
  NSTextView* view = [[NSTextView alloc] initWithFrame:NSZeroRect];
  view.editable = NO;
  view.selectable = YES;
  view.richText = NO;
  view.usesFindBar = NO;
  view.font = [NSFont systemFontOfSize:13.0];
  view.textContainerInset = NSMakeSize(8, 8);
  view.accessibilityLabel = @"Temporary recognition preview";
  scroll.documentView = view;
  return PreviewViews{scroll, view};
}

}  // namespace

@interface MicProbeAppDelegate : NSObject <NSApplicationDelegate, NSWindowDelegate> {
 @private
  LaunchOptions _launch;
  NSWindow* __strong _window;
  NSTextField* __strong _permission_label;
  NSTextField* __strong _state_label;
  NSTextField* __strong _partial_label;
  NSTextField* __strong _final_label;
  NSTextField* __strong _metrics_label;
  NSTextField* __strong _copy_warning_label;
  NSTextField* __strong _copy_status_label;
  NSTextView* __strong _partial_text;
  NSTextView* __strong _final_text;
  NSButton* __strong _start_button;
  NSButton* __strong _stop_button;
  NSButton* __strong _cancel_button;
  NSButton* __strong _dismiss_button;
  NSButton* __strong _copy_result_button;
  NSButton* __strong _settings_button;
  std::string _copy_result_text;
  std::uint64_t _copy_result_session_id;
  SessionStateMachine _state;
  std::uint64_t _next_session_id;
  std::unique_ptr<SessionWorker> _worker;
  std::unique_ptr<MacMicrophoneCapture> _capture;
  std::unique_ptr<TimedFixtureProducer> _fixture_producer;
  WavFixture _fixture;
  bool _fixture_loaded;
  bool _closing;
  bool _permission_request_pending;
  std::uint64_t _permission_request_generation;
  std::uint64_t _stop_request_ns;
  std::uint64_t _microphone_release_ns;
}
- (void)startPressed:(id)sender;
- (void)stopPressed:(id)sender;
- (void)cancelPressed:(id)sender;
- (void)dismissPressed:(id)sender;
- (void)copyResultPressed:(id)sender;
- (void)settingsPressed:(id)sender;
- (void)clearFinalResult;
- (std::uint64_t)prepareSession;
- (void)startWorkerForSession:(std::uint64_t)session_id;
- (void)invalidatePermissionRequest;
- (void)handleWorkerEvent:(WorkerEvent)event;
@end

@implementation MicProbeAppDelegate

- (instancetype)init {
  self = [super init];
  if (self != nil) {
    _launch = read_launch_options();
    _next_session_id = 1;
    _fixture_loaded = false;
    _closing = false;
    _permission_request_pending = false;
    _permission_request_generation = 0;
    _stop_request_ns = 0;
    _microphone_release_ns = 0;
    _copy_result_session_id = 0;
  }
  return self;
}

- (void)applicationDidFinishLaunching:(NSNotification*)notification {
  (void)notification;
  [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
  NSRect frame = NSMakeRect(0, 0, 650, 570);
  _window = [[NSWindow alloc] initWithContentRect:frame
                                        styleMask:(NSWindowStyleMaskTitled |
                                                   NSWindowStyleMaskClosable |
                                                   NSWindowStyleMaskMiniaturizable |
                                                   NSWindowStyleMaskResizable)
                                          backing:NSBackingStoreBuffered
                                            defer:NO];
  _window.title = @"Outflow Mic Probe — Development Only";
  _window.minSize = NSMakeSize(560, 520);
  _window.delegate = self;

  NSView* content = _window.contentView;
  NSTextField* title = make_label(@"Local microphone → Moonshine", 20.0, YES);
  NSTextField* subtitle = make_label(
      @"Development feasibility probe. Audio and preview text stay in memory.",
      12.0, NO);
  _permission_label = make_label(permission_status_text(), 12.0, NO);
  _state_label = make_label(@"State: Idle", 13.0, YES);
  _partial_label = make_label(@"Partial", 12.0, YES);
  _final_label = make_label(@"Final result", 12.0, YES);
  _metrics_label = make_label(@"No session measurements yet.", 11.0, NO);
  _metrics_label.textColor = NSColor.secondaryLabelColor;
  _copy_warning_label = make_label(
      @"Copied text may be retained by clipboard managers or synced to other devices.",
      12.0, NO);
  _copy_warning_label.accessibilityLabel = @"Clipboard privacy warning";
  _copy_status_label = make_label(@"", 12.0, NO);
  _copy_status_label.accessibilityLabel = @"Copy Result status";
  const auto partial_preview = make_preview();
  const auto final_preview = make_preview();
  NSScrollView* partial_scroll = partial_preview.scroll;
  NSScrollView* final_scroll = final_preview.scroll;
  _partial_text = partial_preview.text;
  _final_text = final_preview.text;
  NSString* start_title = _launch.fixture_mode ? @"Start timed fixture" : @"Start recording";
  _start_button = make_button(start_title, self, @selector(startPressed:));
  _stop_button = make_button(@"Stop", self, @selector(stopPressed:));
  _cancel_button = make_button(@"Cancel", self, @selector(cancelPressed:));
  _dismiss_button = make_button(@"Dismiss result", self, @selector(dismissPressed:));
  _copy_result_button = make_button(@"Copy Result", self,
                                    @selector(copyResultPressed:));
  _copy_result_button.keyEquivalent = @"\r";
  _copy_result_button.accessibilityHelp =
      @"After a nonempty final result appears, activate this button or press Return to copy it.";
  [_copy_result_button setContentHuggingPriority:NSLayoutPriorityRequired
                                     forOrientation:NSLayoutConstraintOrientationHorizontal];
  _settings_button = make_button(@"Microphone Settings", self,
                                 @selector(settingsPressed:));

  NSStackView* copy_row = [[NSStackView alloc] initWithFrame:NSZeroRect];
  [copy_row addArrangedSubview:_copy_result_button];
  [copy_row addArrangedSubview:_copy_status_label];
  copy_row.orientation = NSUserInterfaceLayoutOrientationHorizontal;
  copy_row.alignment = NSLayoutAttributeCenterY;
  copy_row.distribution = NSStackViewDistributionFill;
  copy_row.spacing = 12.0;
  copy_row.translatesAutoresizingMaskIntoConstraints = NO;

  // Make the copy action part of the explicit key-view path even when the
  // system's general Full Keyboard Access preference is disabled.
  [_partial_text setNextKeyView:_final_text];
  [_final_text setNextKeyView:_copy_result_button];
  [_copy_result_button setNextKeyView:_start_button];
  [_start_button setNextKeyView:_stop_button];
  [_stop_button setNextKeyView:_cancel_button];
  [_cancel_button setNextKeyView:_dismiss_button];
  [_dismiss_button setNextKeyView:_settings_button];
  [_settings_button setNextKeyView:_partial_text];

  NSArray<NSView*>* controls = @[_start_button, _stop_button, _cancel_button,
                                 _dismiss_button, _settings_button];
  NSStackView* button_row = [[NSStackView alloc] initWithFrame:NSZeroRect];
  for (NSView* control in controls) [button_row addArrangedSubview:control];
  button_row.orientation = NSUserInterfaceLayoutOrientationHorizontal;
  button_row.alignment = NSLayoutAttributeCenterY;
  button_row.distribution = NSStackViewDistributionFillEqually;
  button_row.spacing = 8.0;
  button_row.translatesAutoresizingMaskIntoConstraints = NO;

  for (NSView* view in @[title, subtitle, _permission_label, _state_label,
                         _partial_label, partial_scroll, _final_label, final_scroll,
                         copy_row, _copy_warning_label, _metrics_label, button_row]) {
    [content addSubview:view];
  }
  [NSLayoutConstraint activateConstraints:@[
    [title.leadingAnchor constraintEqualToAnchor:content.leadingAnchor constant:24],
    [title.trailingAnchor constraintEqualToAnchor:content.trailingAnchor constant:-24],
    [title.topAnchor constraintEqualToAnchor:content.topAnchor constant:20],
    [subtitle.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [subtitle.trailingAnchor constraintEqualToAnchor:title.trailingAnchor],
    [subtitle.topAnchor constraintEqualToAnchor:title.bottomAnchor constant:4],
    [_permission_label.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [_permission_label.trailingAnchor constraintEqualToAnchor:title.trailingAnchor],
    [_permission_label.topAnchor constraintEqualToAnchor:subtitle.bottomAnchor constant:10],
    [_state_label.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [_state_label.trailingAnchor constraintEqualToAnchor:title.trailingAnchor],
    [_state_label.topAnchor constraintEqualToAnchor:_permission_label.bottomAnchor constant:8],
    [_partial_label.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [_partial_label.topAnchor constraintEqualToAnchor:_state_label.bottomAnchor constant:14],
    [partial_scroll.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [partial_scroll.trailingAnchor constraintEqualToAnchor:title.trailingAnchor],
    [partial_scroll.topAnchor constraintEqualToAnchor:_partial_label.bottomAnchor constant:4],
    [partial_scroll.heightAnchor constraintEqualToConstant:86],
    [_final_label.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [_final_label.topAnchor constraintEqualToAnchor:partial_scroll.bottomAnchor constant:12],
    [final_scroll.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [final_scroll.trailingAnchor constraintEqualToAnchor:title.trailingAnchor],
    [final_scroll.topAnchor constraintEqualToAnchor:_final_label.bottomAnchor constant:4],
    [final_scroll.heightAnchor constraintGreaterThanOrEqualToConstant:74],
    [copy_row.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [copy_row.trailingAnchor constraintEqualToAnchor:title.trailingAnchor],
    [copy_row.topAnchor constraintEqualToAnchor:final_scroll.bottomAnchor constant:8],
    [_copy_warning_label.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [_copy_warning_label.trailingAnchor constraintEqualToAnchor:title.trailingAnchor],
    [_copy_warning_label.topAnchor constraintEqualToAnchor:copy_row.bottomAnchor constant:4],
    [_metrics_label.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [_metrics_label.trailingAnchor constraintEqualToAnchor:title.trailingAnchor],
    [_metrics_label.topAnchor constraintEqualToAnchor:_copy_warning_label.bottomAnchor constant:8],
    [_metrics_label.bottomAnchor constraintLessThanOrEqualToAnchor:button_row.topAnchor constant:-8],
    [button_row.leadingAnchor constraintEqualToAnchor:title.leadingAnchor],
    [button_row.trailingAnchor constraintEqualToAnchor:title.trailingAnchor],
    [button_row.bottomAnchor constraintEqualToAnchor:content.bottomAnchor constant:-18],
    [final_scroll.bottomAnchor constraintLessThanOrEqualToAnchor:_metrics_label.topAnchor constant:-8],
  ]];
  if (_launch.fixture_mode) {
    _permission_label.stringValue = @"Microphone: not used — this mode replays a local fixture in real time.";
    std::string error;
    if (!load_pcm16_mono_wav(_launch.fixture_path, _fixture, error)) {
      _state_label.stringValue = [NSString stringWithFormat:@"State: Error — %@",
                                  ns_string(error)];
    } else {
      _fixture_loaded = true;
    }
  }
  if (_launch.model_directory.empty()) {
    _state_label.stringValue = @"State: Error — launch with --model-dir <local-model-directory>.";
  }
  [self refreshControls];
  [_window center];
  [_window makeKeyAndOrderFront:nil];
  [NSApp activateIgnoringOtherApps:YES];
}

- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication*)sender {
  (void)sender;
  return YES;
}

- (void)applicationWillTerminate:(NSNotification*)notification {
  (void)notification;
  _closing = true;
  [self invalidatePermissionRequest];
  [self stopSourcesForShutdown];
  if (_worker) {
    _worker->request_cancel();
    _worker->join();
    _worker.reset();
  }
  _partial_text.string = @"";
  [self clearFinalResult];
}

- (BOOL)windowShouldClose:(NSWindow*)sender {
  (void)sender;
  _closing = true;
  [self invalidatePermissionRequest];
  [self cancelActiveSessionForDismissal];
  return YES;
}

- (void)windowWillClose:(NSNotification*)notification {
  (void)notification;
  _closing = true;
  [self invalidatePermissionRequest];
  [self cancelActiveSessionForDismissal];
  _partial_text.string = @"";
  [self clearFinalResult];
}

- (void)startPressed:(id)sender {
  (void)sender;
  if (_closing || _state.state() != SessionState::Idle ||
      _launch.model_directory.empty()) return;
  if (_launch.fixture_mode) {
    if (!_fixture_loaded) {
      _state_label.stringValue = @"State: Error — timed fixture could not be loaded.";
      return;
    }
    [self beginSession];
    return;
  }
  const auto permission = microphone_permission();
  const auto action = action_for_permission(permission);
  if (action == PermissionAction::BeginCapture) {
    [self beginSession];
    return;
  }
  if (action == PermissionAction::RequestPermission) {
    const auto session_id = [self prepareSession];
    if (session_id == 0) return;
    _permission_request_pending = true;
    ++_permission_request_generation;
    if (_permission_request_generation == 0) ++_permission_request_generation;
    const auto request_generation = _permission_request_generation;
    _state_label.stringValue =
        @"State: Preparing — waiting for microphone permission. Cancel to discard this request.";
    [self refreshControls];
    __weak MicProbeAppDelegate* weak_self = self;
    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio
                             completionHandler:^(BOOL granted) {
      dispatch_async(dispatch_get_main_queue(), ^{
        MicProbeAppDelegate* strong_self = weak_self;
        if (strong_self == nil ||
            !is_current_permission_request(
                request_generation, strong_self->_permission_request_generation,
                strong_self->_permission_request_pending, strong_self->_closing,
                session_id, strong_self->_state)) return;

        strong_self->_permission_request_pending = false;
        ++strong_self->_permission_request_generation;
        strong_self->_permission_label.stringValue = permission_status_text();
        if (permission_grant_is_authorized(granted, microphone_permission())) {
          [strong_self startWorkerForSession:session_id];
        } else {
          (void)strong_self->_state.fail(session_id);
          strong_self->_metrics_label.stringValue = @"No audio was captured.";
          strong_self->_state_label.stringValue = granted
              ? @"State: Error — microphone authorization changed before capture could start. Verify access in System Settings, then dismiss and retry."
              : @"State: Error — microphone permission was denied. Use System Settings, then dismiss and retry.";
          [strong_self refreshControls];
        }
      });
    }];
    return;
  }
  _permission_label.stringValue = permission_status_text();
  _state_label.stringValue = @"State: Idle — allow microphone access in System Settings, then retry.";
  [self refreshControls];
}

- (void)stopPressed:(id)sender {
  (void)sender;
  const auto id = _state.session_id();
  if (!_state.finalize(id)) return;
  _stop_request_ns = monotonic_time_ns();
  if (_fixture_producer) {
    _fixture_producer->stop();
    _microphone_release_ns = monotonic_time_ns();
  } else if (_capture) {
    _microphone_release_ns = _capture->stop();
  } else {
    _microphone_release_ns = monotonic_time_ns();
  }
  if (_worker) {
    (void)_worker->request_finalize(_stop_request_ns, _microphone_release_ns);
  }
  _state_label.stringValue = @"State: Finalizing — microphone is closed.";
  [self refreshControls];
}

- (void)cancelPressed:(id)sender {
  (void)sender;
  const auto id = _state.session_id();
  if (!_state.cancel(id)) return;
  const auto cancel_requested_ns = monotonic_time_ns();
  const bool microphone_was_active = _capture != nullptr && _capture->active();
  std::size_t queue_depth_at_cancel = 0;
  std::size_t queue_maximum_depth = 0;
  std::uint64_t queue_overflow_count = 0;
  [self invalidatePermissionRequest];
  [self stopSourcesForShutdown];
  if (_worker) {
    auto& queue = _worker->audio_queue();
    queue_depth_at_cancel = queue.depth();
    queue_maximum_depth = queue.maximum_depth();
    queue_overflow_count = queue.overflow_count();
    _worker->request_cancel();
    // The microphone tap is stopped above before join scrubs the worker-owned
    // queue and releases Moonshine handles. Dismiss remains disabled until this
    // synchronous cleanup completes.
    _worker->join();
    _worker.reset();
  }
  const auto cleanup_complete_ns = monotonic_time_ns();
  _partial_text.string = @"";
  [self clearFinalResult];
  if (_launch.fixture_mode) {
    _metrics_label.stringValue = [NSString stringWithFormat:
        @"Cancelled. Temporary preview cleared. Fixture stopped; worker cleanup complete. Queue at cancel %llu/16, max %llu/16, overflow %llu. No cancellation metrics file was written.",
        static_cast<unsigned long long>(queue_depth_at_cancel),
        static_cast<unsigned long long>(queue_maximum_depth),
        static_cast<unsigned long long>(queue_overflow_count)];
  } else if (microphone_was_active && _microphone_release_ns >= cancel_requested_ns) {
    _metrics_label.stringValue = [NSString stringWithFormat:
        @"Cancelled. Temporary preview cleared. Mic release %.1f ms; worker cleanup %.1f ms; queue at cancel %llu/16, max %llu/16, overflow %llu. No cancellation metrics file was written.",
        elapsed_milliseconds(cancel_requested_ns, _microphone_release_ns),
        elapsed_milliseconds(cancel_requested_ns, cleanup_complete_ns),
        static_cast<unsigned long long>(queue_depth_at_cancel),
        static_cast<unsigned long long>(queue_maximum_depth),
        static_cast<unsigned long long>(queue_overflow_count)];
  } else {
    _metrics_label.stringValue = [NSString stringWithFormat:
        @"Cancelled. Temporary preview cleared. Microphone was not open; worker cleanup complete. Queue at cancel %llu/16, max %llu/16, overflow %llu. No cancellation metrics file was written.",
        static_cast<unsigned long long>(queue_depth_at_cancel),
        static_cast<unsigned long long>(queue_maximum_depth),
        static_cast<unsigned long long>(queue_overflow_count)];
  }
  _state_label.stringValue = @"State: Cancelled — microphone released; result discarded.";
  [self refreshControls];
}

- (void)dismissPressed:(id)sender {
  (void)sender;
  const auto current_state = _state.state();
  if (current_state != SessionState::ResultReady &&
      current_state != SessionState::Cancelled &&
      current_state != SessionState::Error) return;
  [self stopSourcesForShutdown];
  if (_worker) {
    _worker->join();
    _worker.reset();
  }
  _partial_text.string = @"";
  [self clearFinalResult];
  _metrics_label.stringValue = @"No session measurements yet.";
  if (_state.dismiss()) {
    _state_label.stringValue = @"State: Idle";
  }
  [self refreshControls];
}

- (void)copyResultPressed:(id)sender {
  (void)sender;
  const auto outcome = request_copy_result(
      _state.state(), _state.session_id(), _copy_result_session_id,
      _copy_result_text, write_clipboard_text);
  switch (outcome) {
    case CopyResultAttempt::Rejected:
      return;
    case CopyResultAttempt::Succeeded:
      _copy_status_label.stringValue =
          @"Copied. Paste manually where you want the text.";
      break;
    case CopyResultAttempt::Failed:
      _copy_status_label.stringValue = clipboard_write_failure_message();
      break;
  }
}

- (void)settingsPressed:(id)sender {
  (void)sender;
  NSURL* url = [NSURL URLWithString:
      @"x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"];
  if (url != nil) [[NSWorkspace sharedWorkspace] openURL:url];
}

- (void)beginSession {
  const auto session_id = [self prepareSession];
  if (session_id == 0) return;
  [self startWorkerForSession:session_id];
}

- (std::uint64_t)prepareSession {
  if (_closing || _state.state() != SessionState::Idle ||
      _launch.model_directory.empty()) return 0;
  const auto session_id = _next_session_id++;
  if (!_state.begin(session_id)) return 0;
  _partial_text.string = @"";
  [self clearFinalResult];
  _metrics_label.stringValue = @"Verifying local model files and preparing one CPU stream…";
  _state_label.stringValue = @"State: Preparing";
  _stop_request_ns = 0;
  _microphone_release_ns = 0;
  [self refreshControls];
  _capture.reset();
  _fixture_producer.reset();
  _worker.reset();
  return session_id;
}

- (void)startWorkerForSession:(std::uint64_t)session_id {
  if (_closing || !_state.accepts(session_id) ||
      _state.state() != SessionState::Preparing || _permission_request_pending) return;
  __weak MicProbeAppDelegate* weak_self = self;
  const auto source = _launch.fixture_mode ? AudioSourceKind::TimedFixture
                                          : AudioSourceKind::Microphone;
  _worker = std::make_unique<SessionWorker>(
      session_id, _launch.model_directory, source,
      [weak_self](WorkerEvent event) mutable {
        auto shared_event = std::make_shared<WorkerEvent>(std::move(event));
        dispatch_async(dispatch_get_main_queue(), ^{
          MicProbeAppDelegate* strong_self = weak_self;
          if (strong_self != nil) {
            [strong_self handleWorkerEvent:std::move(*shared_event)];
          }
          // This queued block may outlive the window during shutdown. Scrub any
          // text that was not consumed by a live delegate as well.
          clear_sensitive_text(shared_event->text);
          clear_sensitive_text(shared_event->error_message);
        });
      });
  if (!_worker->start()) {
    (void)_state.fail(session_id);
    _state_label.stringValue = @"State: Error — native worker thread could not start.";
    _metrics_label.stringValue = @"No audio was captured.";
    [self refreshControls];
  }
}

- (void)invalidatePermissionRequest {
  _permission_request_pending = false;
  ++_permission_request_generation;
  if (_permission_request_generation == 0) ++_permission_request_generation;
}

- (void)beginAudioSourceForSession:(std::uint64_t)session_id {
  if (!_state.accepts(session_id) || _state.state() != SessionState::Preparing ||
      !_worker) return;
  std::string error;
  if (_launch.fixture_mode) {
    _fixture_producer = std::make_unique<TimedFixtureProducer>();
    _worker->set_capture_start_time(monotonic_time_ns());
    const auto fixture_rate = _fixture.sample_rate_hz;
    __weak MicProbeAppDelegate* weak_self = self;
    if (!_fixture_producer->start(
            _fixture, _launch.fixture_seconds, _worker->audio_queue(),
            [weak_self, session_id] {
              dispatch_async(dispatch_get_main_queue(), ^{
                MicProbeAppDelegate* strong_self = weak_self;
                if (strong_self != nil && strong_self->_state.accepts(session_id) &&
                    strong_self->_state.state() == SessionState::Listening) {
                  [strong_self stopPressed:nil];
                }
              });
            }, error)) {
      _worker->request_failure(-107, error);
      (void)_state.fail(session_id);
      _state_label.stringValue = @"State: Error — timed fixture could not start.";
      [self refreshControls];
      return;
    }
    if (!_state.listening(session_id)) {
      _fixture_producer->stop();
      _worker->request_cancel();
      return;
    }
    _state_label.stringValue = [NSString stringWithFormat:
        @"State: Listening — timed fixture replay (%u Hz); this is not microphone audio.",
        fixture_rate];
    _metrics_label.stringValue = @"Real-time-paced fixture. Inference runs on a separate worker.";
    [self refreshControls];
    if (_launch.auto_cancel_after_ms > 0) {
      dispatch_after(dispatch_time(DISPATCH_TIME_NOW,
                                   static_cast<int64_t>(_launch.auto_cancel_after_ms) * NSEC_PER_MSEC),
                     dispatch_get_main_queue(), ^{
        if (self->_state.session_id() == session_id &&
            self->_state.state() == SessionState::Listening) {
          [self cancelPressed:nil];
        }
      });
    }
    return;
  }

  _capture = std::make_unique<MacMicrophoneCapture>();
  _worker->set_capture_start_time(monotonic_time_ns());
  if (!_capture->start(_worker->audio_queue(), error)) {
    _worker->request_failure(-108, error);
    (void)_state.fail(session_id);
    _state_label.stringValue = [NSString stringWithFormat:@"State: Error — %@", ns_string(error)];
    _metrics_label.stringValue = @"No microphone audio was accepted.";
    [self refreshControls];
    return;
  }
  const auto info = _capture->format();
  _worker->set_capture_start_time(_capture->capture_started_at_ns());
  if (!_state.listening(session_id)) {
    (void)_capture->stop();
    _worker->request_cancel();
    return;
  }
  _state_label.stringValue = [NSString stringWithFormat:
      @"State: Listening — microphone open · %u Hz · %u channel(s).",
      info.sample_rate_hz, info.channels];
  _metrics_label.stringValue = @"Capture callback copies bounded PCM; native inference runs off the audio thread.";
  [self refreshControls];
  if (_launch.auto_cancel_after_ms > 0) {
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW,
                                 static_cast<int64_t>(_launch.auto_cancel_after_ms) * NSEC_PER_MSEC),
                   dispatch_get_main_queue(), ^{
      if (self->_state.session_id() == session_id &&
          self->_state.state() == SessionState::Listening) {
        [self cancelPressed:nil];
      }
    });
  }
}

- (void)handleWorkerEvent:(WorkerEvent)event {
  if (_closing || !_state.accepts(event.session_id)) {
    clear_sensitive_text(event.text);
    event.error_message.clear();
    return;
  }
  switch (event.kind) {
    case WorkerEventKind::Ready:
      if (_state.state() == SessionState::Preparing) {
        [self beginAudioSourceForSession:event.session_id];
      }
      break;
    case WorkerEventKind::Partial:
      if (_state.state() == SessionState::Listening) {
        _partial_text.string = ns_string(event.text);
      }
      clear_sensitive_text(event.text);
      break;
    case WorkerEventKind::Final: {
      if (!_state.ready(event.session_id)) {
        clear_sensitive_text(event.text);
        return;
      }
      _copy_result_text = std::move(event.text);
      _copy_result_session_id = event.session_id;
      _final_text.string = _copy_result_text.empty()
          ? @"No speech was recognized in this session."
          : ns_string(_copy_result_text);
      clear_sensitive_text(event.text);
      _copy_status_label.stringValue = _copy_result_text.empty()
          ? @"No final text is available to copy."
          : @"";
      const bool metrics_saved = write_metrics_exclusive(_launch.metrics_path,
                                                         event.metrics);
      _metrics_label.stringValue = [NSString stringWithFormat:
          @"%s · model %.0f ms · first frame %.0f ms · first partial %.0f ms · "
          "stop→final %.0f ms · mic release %.1f ms · queue max %llu/16 · RSS %llu MB · %llu threads%@",
          event.metrics.source == AudioSourceKind::Microphone ? "Microphone" : "Timed fixture",
          event.metrics.model_load_ms,
          event.metrics.first_frame_after_capture_start_ms,
          event.metrics.first_partial_after_first_frame_ms,
          event.metrics.stop_to_final_ms,
          event.metrics.microphone_release_latency_ms,
          static_cast<unsigned long long>(event.metrics.queue_maximum_depth),
          static_cast<unsigned long long>(event.metrics.peak_rss_bytes / 1'000'000),
          static_cast<unsigned long long>(event.metrics.native_thread_count),
          _launch.metrics_path.empty() ? @"" : (metrics_saved ? @" · metrics saved" : @" · metrics save failed")];
      _state_label.stringValue = @"State: Result ready — microphone released.";
      _capture.reset();
      if (_worker) {
        _worker->join();
        _worker.reset();
      }
      [self refreshControls];
      if (_launch.auto_exit_after_final) {
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 1 * NSEC_PER_SEC),
                       dispatch_get_main_queue(), ^{ [NSApp terminate:nil]; });
      }
      break;
    }
    case WorkerEventKind::Cancelled:
      if (_state.state() != SessionState::Preparing &&
          _state.state() != SessionState::Listening &&
          _state.state() != SessionState::Finalizing &&
          _state.state() != SessionState::Cancelled) {
        clear_sensitive_text(event.text);
        event.error_message.clear();
        break;
      }
      [self stopSourcesForShutdown];
      _partial_text.string = @"";
      [self clearFinalResult];
      if (_state.state() != SessionState::Cancelled) (void)_state.cancel(event.session_id);
      if (_worker) {
        _worker->join();
        _worker.reset();
      }
      _state_label.stringValue = @"State: Cancelled — microphone released; result discarded.";
      [self refreshControls];
      if (_launch.auto_exit_after_cancel) {
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 1 * NSEC_PER_SEC),
                       dispatch_get_main_queue(), ^{ [NSApp terminate:nil]; });
      }
      break;
    case WorkerEventKind::Error:
      if (_state.state() != SessionState::Preparing &&
          _state.state() != SessionState::Listening &&
          _state.state() != SessionState::Finalizing) {
        clear_sensitive_text(event.text);
        event.error_message.clear();
        break;
      }
      [self stopSourcesForShutdown];
      _partial_text.string = @"";
      [self clearFinalResult];
      if (_state.state() != SessionState::Error) (void)_state.fail(event.session_id);
      if (_worker) {
        _worker->join();
        _worker.reset();
      }
      _state_label.stringValue = [NSString stringWithFormat:@"State: Error — %@",
                                  ns_string(event.error_message)];
      _metrics_label.stringValue = [NSString stringWithFormat:
          @"No result retained. Native error code: %d.", event.native_error_code];
      [self refreshControls];
      break;
  }
}

- (void)refreshControls {
  const auto state = _state.state();
  const bool active = state == SessionState::Preparing || state == SessionState::Listening ||
                      state == SessionState::Finalizing;
  const bool terminal = state == SessionState::ResultReady || state == SessionState::Cancelled ||
                        state == SessionState::Error;
  _start_button.enabled = state == SessionState::Idle && !_launch.model_directory.empty() &&
                          (!_launch.fixture_mode || _fixture_loaded);
  _stop_button.enabled = state == SessionState::Listening;
  _cancel_button.enabled = active;
  _dismiss_button.enabled = terminal;
  _copy_result_button.enabled = copy_result_eligible(
      state, _state.session_id(), _copy_result_session_id, _copy_result_text);
  _settings_button.enabled = !_launch.fixture_mode;
  if (!_launch.fixture_mode) _permission_label.stringValue = permission_status_text();
}

- (void)stopSourcesForShutdown {
  if (_fixture_producer) {
    _fixture_producer->stop();
    _fixture_producer.reset();
  }
  if (_capture) {
    _microphone_release_ns = _capture->stop();
    _capture.reset();
  }
}

- (void)clearFinalResult {
  clear_sensitive_text(_copy_result_text);
  _copy_result_session_id = 0;
  _final_text.string = @"";
  _copy_status_label.stringValue = @"";
}

- (void)cancelActiveSessionForDismissal {
  [self stopSourcesForShutdown];
  if (_worker) {
    _worker->request_cancel();
    _worker->join();
    _worker.reset();
  }
  if (_state.state() == SessionState::Preparing ||
      _state.state() == SessionState::Listening ||
      _state.state() == SessionState::Finalizing) {
    (void)_state.cancel(_state.session_id());
  }
  _partial_text.string = @"";
  [self clearFinalResult];
}

@end

int main(int argc, const char* argv[]) {
  for (int index = 1; index < argc; ++index) {
    if (std::string_view(argv[index]) == "--runtime-identity") {
      return print_runtime_identity(argv[0]) ? 0 : 1;
    }
  }
  @autoreleasepool {
    NSApplication* application = [NSApplication sharedApplication];
    MicProbeAppDelegate* delegate = [[MicProbeAppDelegate alloc] init];
    application.delegate = delegate;
    [application run];
  }
  return 0;
}
