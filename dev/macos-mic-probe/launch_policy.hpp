#pragma once

#include "verification_identity.hpp"

#include <string_view>

namespace outflow::mic_probe {

enum class ProbeInputMode {
  Microphone,
  TimedFixture,
  SyntheticCopyDiagnostic,
  Invalid,
};

/// Keeps microphone, timed-fixture, and synthetic diagnostic paths exclusive.
inline ProbeInputMode probe_input_mode(bool fixture_mode,
                                       bool synthetic_copy_diagnostic) noexcept {
  if (fixture_mode && synthetic_copy_diagnostic) return ProbeInputMode::Invalid;
  if (synthetic_copy_diagnostic) return ProbeInputMode::SyntheticCopyDiagnostic;
  if (fixture_mode) return ProbeInputMode::TimedFixture;
  return ProbeInputMode::Microphone;
}

/// Validates synthetic mode before AppKit is initialized.
///
/// Ordinary launch remains microphone mode by default. Synthetic mode requires
/// a bounded verification ID and rejects fixture or explicit microphone-capture
/// flags so the test cannot accidentally open an input.
inline bool probe_launch_arguments_valid(int argument_count,
                                         const char* const arguments[]) noexcept {
  if (!verification_id_arguments_valid(argument_count, arguments)) return false;

  bool synthetic_seen = false;
  bool has_verification_id = false;
  bool has_timed_fixture = false;
  bool has_microphone_capture = false;
  for (int index = 1; index < argument_count; ++index) {
    if (arguments[index] == nullptr) return false;
    const std::string_view argument(arguments[index]);
    if (argument == "--verification-id") {
      has_verification_id = true;
      ++index;
    } else if (argument == "--timed-fixture") {
      has_timed_fixture = true;
      if (index + 1 < argument_count) ++index;
    } else if (argument == "--model-dir" || argument == "--fixture-seconds" ||
               argument == "--metrics-file" ||
               argument == "--auto-cancel-after-ms" ||
               argument == "--isolated-pasteboard-test") {
      if (index + 1 < argument_count) ++index;
    } else if (argument == "--synthetic-copy-diagnostic") {
      if (synthetic_seen) return false;
      synthetic_seen = true;
    } else if (argument == "--microphone-capture") {
      has_microphone_capture = true;
    }
  }

  return !synthetic_seen ||
         (has_verification_id && !has_timed_fixture && !has_microphone_capture);
}

}  // namespace outflow::mic_probe
