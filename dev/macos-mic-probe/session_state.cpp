#include "session_state.hpp"

namespace outflow::mic_probe {

bool SessionStateMachine::begin(std::uint64_t session_id) noexcept {
  if (state_ != SessionState::Idle || session_id == 0) return false;
  session_id_ = session_id;
  state_ = SessionState::Preparing;
  return true;
}

bool SessionStateMachine::listening(std::uint64_t session_id) noexcept {
  if (!accepts(session_id) || state_ != SessionState::Preparing) return false;
  state_ = SessionState::Listening;
  return true;
}

bool SessionStateMachine::finalize(std::uint64_t session_id) noexcept {
  if (!accepts(session_id) || state_ != SessionState::Listening) return false;
  state_ = SessionState::Finalizing;
  return true;
}

bool SessionStateMachine::ready(std::uint64_t session_id) noexcept {
  if (!accepts(session_id) || state_ != SessionState::Finalizing) return false;
  state_ = SessionState::ResultReady;
  return true;
}

bool SessionStateMachine::cancel(std::uint64_t session_id) noexcept {
  if (!accepts(session_id) ||
      (state_ != SessionState::Preparing && state_ != SessionState::Listening &&
       state_ != SessionState::Finalizing)) {
    return false;
  }
  state_ = SessionState::Cancelled;
  return true;
}

bool SessionStateMachine::fail(std::uint64_t session_id) noexcept {
  if (!accepts(session_id) ||
      (state_ != SessionState::Preparing && state_ != SessionState::Listening &&
       state_ != SessionState::Finalizing)) {
    return false;
  }
  state_ = SessionState::Error;
  return true;
}

bool SessionStateMachine::dismiss() noexcept {
  if (state_ == SessionState::Preparing || state_ == SessionState::Listening ||
      state_ == SessionState::Finalizing) {
    return false;
  }
  state_ = SessionState::Idle;
  session_id_ = 0;
  return true;
}

SessionState SessionStateMachine::state() const noexcept { return state_; }

std::uint64_t SessionStateMachine::session_id() const noexcept {
  return session_id_;
}

bool SessionStateMachine::accepts(std::uint64_t session_id) const noexcept {
  return session_id != 0 && session_id == session_id_;
}

const char* session_state_name(SessionState state) noexcept {
  switch (state) {
    case SessionState::Idle: return "Idle";
    case SessionState::Preparing: return "Preparing";
    case SessionState::Listening: return "Listening";
    case SessionState::Finalizing: return "Finalizing";
    case SessionState::ResultReady: return "Result ready";
    case SessionState::Cancelled: return "Cancelled";
    case SessionState::Error: return "Error";
  }
  return "Error";
}

}  // namespace outflow::mic_probe
