#pragma once

#include "session_state.hpp"

#include <cstdint>
#include <string_view>

namespace outflow::mic_probe {

// Fixed, non-personal UTF-8 fixture. Keep the sample in one source definition
// so displayed and tested bytes cannot drift apart.
inline constexpr std::string_view kSyntheticCopyDiagnosticSample =
    "OUTFLOW-SYNTHETIC-COPY-START\n"
    "Café — plain-text clipboard check! 😀\n"
    "OUTFLOW-SYNTHETIC-COPY-END";

/// Establishes a ResultReady session without audio or recognition.
///
/// Call only from the validated --synthetic-copy-diagnostic startup branch.
/// Normal microphone and timed-fixture paths do not call this helper.
inline bool establish_synthetic_copy_result(SessionStateMachine& state,
                                            std::uint64_t session_id) noexcept {
  if (state.state() != SessionState::Idle || session_id == 0) return false;
  return state.begin(session_id) && state.listening(session_id) &&
         state.finalize(session_id) && state.ready(session_id);
}

}  // namespace outflow::mic_probe
