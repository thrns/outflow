#pragma once

#include <cstdint>

namespace outflow::mic_probe {

enum class SessionState : std::uint8_t {
  Idle,
  Preparing,
  Listening,
  Finalizing,
  ResultReady,
  Cancelled,
  Error,
};

class SessionStateMachine final {
 public:
  bool begin(std::uint64_t session_id) noexcept;
  bool listening(std::uint64_t session_id) noexcept;
  bool finalize(std::uint64_t session_id) noexcept;
  bool ready(std::uint64_t session_id) noexcept;
  bool cancel(std::uint64_t session_id) noexcept;
  bool fail(std::uint64_t session_id) noexcept;
  bool dismiss() noexcept;

  SessionState state() const noexcept;
  std::uint64_t session_id() const noexcept;
  bool accepts(std::uint64_t session_id) const noexcept;

 private:
  SessionState state_ = SessionState::Idle;
  std::uint64_t session_id_ = 0;
};

const char* session_state_name(SessionState state) noexcept;

}  // namespace outflow::mic_probe
