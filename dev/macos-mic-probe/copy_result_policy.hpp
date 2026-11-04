#pragma once

#include "session_state.hpp"

#include <cstdint>
#include <string_view>
#include <utility>

namespace outflow::mic_probe {

enum class CopyResultAttempt : std::uint8_t {
  RejectedBySessionPolicy,
  Succeeded,
  Failed,
};

/// Returns whether an accepted session owns nonempty text that may be copied.
///
/// The session IDs bind the finalized text to the active result. Callers must
/// pass the actual transcript separately from any display-only empty-result
/// placeholder. Partial text is never eligible.
inline bool copy_result_eligible(SessionState state,
                                 std::uint64_t accepted_session_id,
                                 std::uint64_t result_session_id,
                                 std::string_view final_text) noexcept {
  return state == SessionState::ResultReady && accepted_session_id != 0 &&
         result_session_id == accepted_session_id && !final_text.empty();
}

/// Requests one clipboard write only when the accepted final result is ready.
///
/// `clipboard_writer` is called exactly once for an eligible request and is
/// never called for an ineligible state. The writer's success value represents
/// the operating system's result; this policy does not mutate or clear text on
/// either success or failure.
template <typename ClipboardWriter>
CopyResultAttempt request_copy_result(SessionState state,
                                      std::uint64_t accepted_session_id,
                                      std::uint64_t result_session_id,
                                      std::string_view final_text,
                                      ClipboardWriter&& clipboard_writer) {
  if (!copy_result_eligible(state, accepted_session_id, result_session_id,
                            final_text)) {
    return CopyResultAttempt::RejectedBySessionPolicy;
  }
  return std::forward<ClipboardWriter>(clipboard_writer)(final_text)
             ? CopyResultAttempt::Succeeded
             : CopyResultAttempt::Failed;
}

}  // namespace outflow::mic_probe
