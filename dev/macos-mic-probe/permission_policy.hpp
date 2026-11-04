#pragma once

#include "session_state.hpp"

#include <cstdint>

namespace outflow::mic_probe {

enum class MicrophonePermission : std::uint8_t {
  Unknown,
  NotDetermined,
  Authorized,
  Denied,
  Restricted,
};

enum class PermissionAction : std::uint8_t {
  BeginCapture,
  RequestPermission,
  OpenSystemSettings,
};

PermissionAction action_for_permission(MicrophonePermission permission) noexcept;
const char* permission_name(MicrophonePermission permission) noexcept;

// A delayed TCC completion may act only on the still-pending Preparing session
// that initiated it. Generation invalidation also covers window close/exit,
// where the session state may otherwise remain observable during teardown.
bool is_current_permission_request(std::uint64_t callback_generation,
                                   std::uint64_t current_generation,
                                   bool request_pending, bool closing,
                                   std::uint64_t session_id,
                                   const SessionStateMachine& state) noexcept;
bool permission_grant_is_authorized(bool completion_granted,
                                    MicrophonePermission current_authorization) noexcept;

}  // namespace outflow::mic_probe
