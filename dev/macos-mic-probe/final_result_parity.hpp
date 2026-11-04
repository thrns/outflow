#pragma once

#include "verification_identity.hpp"

#include <cstddef>
#include <optional>
#include <string_view>

namespace outflow::mic_probe {

enum class FinalResultParity : unsigned char {
  Unavailable,
  Match,
  Mismatch,
};

/// Enables the parity diagnostic only for an identified timed-fixture run.
/// Ordinary microphone and synthetic diagnostic sessions keep their existing
/// behavior and do not expose this development-only check.
inline bool final_result_parity_enabled(
    bool timed_fixture_mode, bool synthetic_copy_diagnostic,
    std::optional<std::string_view> verification_id) noexcept {
  return timed_fixture_mode && !synthetic_copy_diagnostic &&
         verification_id.has_value() &&
         is_valid_verification_id(*verification_id);
}

/// Returns whether `text` contains well-formed UTF-8 without transforming it.
inline bool is_well_formed_utf8(std::string_view text) noexcept {
  const auto* bytes = reinterpret_cast<const unsigned char*>(text.data());
  std::size_t index = 0;
  while (index < text.size()) {
    const auto lead = bytes[index];
    if (lead <= 0x7f) {
      ++index;
      continue;
    }

    if (lead >= 0xc2 && lead <= 0xdf) {
      if (index + 1 >= text.size() || (bytes[index + 1] & 0xc0) != 0x80) {
        return false;
      }
      index += 2;
      continue;
    }

    if (lead >= 0xe0 && lead <= 0xef) {
      if (index + 2 >= text.size()) return false;
      const auto second = bytes[index + 1];
      const auto third = bytes[index + 2];
      if ((third & 0xc0) != 0x80 ||
          (lead == 0xe0 ? second < 0xa0 || second > 0xbf
                        : lead == 0xed ? second < 0x80 || second > 0x9f
                                       : (second & 0xc0) != 0x80)) {
        return false;
      }
      index += 3;
      continue;
    }

    if (lead >= 0xf0 && lead <= 0xf4) {
      if (index + 3 >= text.size()) return false;
      const auto second = bytes[index + 1];
      const auto third = bytes[index + 2];
      const auto fourth = bytes[index + 3];
      const bool valid_second =
          lead == 0xf0 ? second >= 0x90 && second <= 0xbf
          : lead == 0xf4 ? second >= 0x80 && second <= 0x8f
                         : (second & 0xc0) == 0x80;
      if (!valid_second || (third & 0xc0) != 0x80 ||
          (fourth & 0xc0) != 0x80) {
        return false;
      }
      index += 4;
      continue;
    }

    return false;
  }
  return true;
}

/// Compares complete UTF-8 byte sequences exactly, without normalization,
/// trimming, case folding, newline conversion, hashing, or content reporting.
/// A missing display value or malformed UTF-8 on either side is unavailable.
inline FinalResultParity compare_final_result_parity(
    std::string_view session_owned_utf8,
    std::optional<std::string_view> displayed_utf8) noexcept {
  if (!displayed_utf8.has_value() ||
      !is_well_formed_utf8(session_owned_utf8) ||
      !is_well_formed_utf8(*displayed_utf8)) {
    return FinalResultParity::Unavailable;
  }
  return session_owned_utf8 == *displayed_utf8 ? FinalResultParity::Match
                                                : FinalResultParity::Mismatch;
}

}  // namespace outflow::mic_probe
