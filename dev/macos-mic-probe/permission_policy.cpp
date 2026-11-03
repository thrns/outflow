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

}  // namespace outflow::mic_probe
