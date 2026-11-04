#pragma once

#include <cstdint>
#include <cstddef>
#include <string>
#include <string_view>

namespace outflow::mic_probe {

inline constexpr std::size_t kMaximumVerificationIdLength = 32;

/// Accepts short, printable ASCII identifiers suitable for a window title.
///
/// The explicit alphabet prevents terminal controls, newlines, bidirectional
/// controls, and arbitrary text from entering the development diagnostic.
inline bool is_valid_verification_id(std::string_view value) noexcept {
  if (value.empty() || value.size() > kMaximumVerificationIdLength) return false;
  for (const unsigned char character : value) {
    const bool upper = character >= 'A' && character <= 'Z';
    const bool lower = character >= 'a' && character <= 'z';
    const bool digit = character >= '0' && character <= '9';
    if (!upper && !lower && !digit && character != '-') return false;
  }
  return true;
}

/// Validates occurrences of the optional CLI flag before AppKit is initialized.
inline bool verification_id_arguments_valid(int argument_count,
                                           const char* const arguments[]) noexcept {
  bool seen = false;
  for (int index = 1; index < argument_count; ++index) {
    if (arguments[index] == nullptr ||
        std::string_view(arguments[index]) != "--verification-id") {
      continue;
    }
    if (seen || index + 1 >= argument_count || arguments[index + 1] == nullptr ||
        !is_valid_verification_id(arguments[index + 1])) {
      return false;
    }
    seen = true;
    ++index;
  }
  return true;
}

/// Formats only validated verification identity and the current process ID.
inline std::string verification_diagnostic(std::string_view value,
                                           std::uint32_t process_id) {
  if (!is_valid_verification_id(value)) return {};
  return "Verification ID: " + std::string(value) + " — PID " +
         std::to_string(process_id);
}

}  // namespace outflow::mic_probe
