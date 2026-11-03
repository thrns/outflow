#pragma once

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

}  // namespace outflow::mic_probe
