#include "permission_policy.hpp"

namespace outflow::mic_probe {

PermissionAction action_for_permission(MicrophonePermission permission) noexcept {
  switch (permission) {
    case MicrophonePermission::Authorized:
      return PermissionAction::BeginCapture;
    case MicrophonePermission::NotDetermined:
      return PermissionAction::RequestPermission;
    case MicrophonePermission::Denied:
    case MicrophonePermission::Restricted:
    case MicrophonePermission::Unknown:
      return PermissionAction::OpenSystemSettings;
  }
  return PermissionAction::OpenSystemSettings;
}

const char* permission_name(MicrophonePermission permission) noexcept {
  switch (permission) {
    case MicrophonePermission::Unknown: return "Unknown";
    case MicrophonePermission::NotDetermined: return "Not requested";
    case MicrophonePermission::Authorized: return "Granted";
    case MicrophonePermission::Denied: return "Denied";
    case MicrophonePermission::Restricted: return "Restricted";
  }
  return "Unknown";
}

bool is_current_permission_request(std::uint64_t callback_generation,
                                   std::uint64_t current_generation,
                                   bool request_pending, bool closing,
                                   std::uint64_t session_id,
                                   const SessionStateMachine& state) noexcept {
  return request_pending && !closing && callback_generation != 0 &&
         callback_generation == current_generation &&
         state.accepts(session_id) && state.state() == SessionState::Preparing;
}

bool permission_grant_is_authorized(
    bool completion_granted,
    MicrophonePermission current_authorization) noexcept {
  return completion_granted &&
         current_authorization == MicrophonePermission::Authorized;
}

}  // namespace outflow::mic_probe
