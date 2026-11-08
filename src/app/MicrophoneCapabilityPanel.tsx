import type { ReactNode } from "react";

export type MicrophonePermissionStatus =
  | "unknown"
  | "available"
  | "denied"
  | "needs_user_action"
  | "not_supported";

export type InputDeviceAvailability =
  | "unknown"
  | "available"
  | "none"
  | "disconnected";

export type MicrophoneStatusOrigin = "preview" | "simulated" | "native";

export type NativeMicrophoneAdapter = {
  connected: boolean;
  checkAgain?: () => void;
  openSystemSettings?: () => void;
};

const permissionLabels: Record<MicrophonePermissionStatus, string> = {
  unknown: "Not checked",
  available: "Available",
  denied: "Denied",
  needs_user_action: "Action required",
  not_supported: "Not supported",
};

const deviceLabels: Record<InputDeviceAvailability, string> = {
  unknown: "Not checked",
  available: "Device detected",
  none: "No available input device",
  disconnected: "Disconnected or unavailable",
};

const permissionGuidance: Record<MicrophonePermissionStatus, string> = {
  unknown:
    "Operating-system microphone permission has not been checked in this preview.",
  available:
    "Permission is reported as available. This does not confirm that an input device is present or that recording has started.",
  denied:
    "Microphone access is off. Outflow needs microphone permission to capture speech after you intentionally start dictation. Recording remains unavailable until permission is granted.",
  needs_user_action:
    "Complete the required microphone permission step in your operating-system settings, return to Outflow, then check again when native integration is connected.",
  not_supported:
    "The reported microphone capability is not supported on this platform. Outflow cannot use a microphone through this capability.",
};

const deviceGuidance: Record<InputDeviceAvailability, string> = {
  unknown:
    "Input-device availability has not been checked. This preview does not enumerate devices.",
  available:
    "An input device is reported as present. This status alone does not make recording ready.",
  none:
    "No input device is available. Connect or enable a microphone in your operating-system audio settings, then check again when native integration is connected.",
  disconnected:
    "The input device is disconnected or unavailable. Reconnect or enable an input in your operating-system audio settings, then check again when native integration is connected.",
};

type MicrophoneCapabilityPanelProps = {
  permissionStatus?: MicrophonePermissionStatus;
  inputDeviceAvailability?: InputDeviceAvailability;
  statusOrigin?: MicrophoneStatusOrigin;
  nativeAdapter?: NativeMicrophoneAdapter;
  children?: ReactNode;
};

function statusSummary(
  permission: MicrophonePermissionStatus,
  origin: MicrophoneStatusOrigin,
) {
  const summary: Record<MicrophonePermissionStatus, string> = {
    unknown: "Microphone status has not been checked.",
    available: "Microphone permission is available.",
    denied: "Microphone access is off.",
    needs_user_action: "Microphone permission needs your action.",
    not_supported: "Microphone capability is not supported.",
  };

  return origin === "simulated"
    ? `Simulated scenario: ${summary[permission]}`
    : summary[permission];
}

function nextStep(
  permission: MicrophonePermissionStatus,
  inputDevice: InputDeviceAvailability,
  origin: MicrophoneStatusOrigin,
) {
  if (permission === "not_supported") {
    return "Microphone use is unavailable through this capability. Use another supported platform when one is available.";
  }

  if (permission === "denied") {
    return "Allow microphone access in operating-system settings, return to Outflow, then check again.";
  }

  if (permission === "needs_user_action") {
    return "Complete the requested permission step in operating-system settings, return to Outflow, then check again.";
  }

  if (inputDevice === "none") {
    return "Connect or enable an input microphone in operating-system audio settings. This preview cannot enumerate or select devices.";
  }

  if (inputDevice === "disconnected") {
    return "Reconnect or enable an input microphone in operating-system audio settings, then recheck through native integration.";
  }

  if (permission === "unknown" || inputDevice === "unknown") {
    return "Review microphone permission and input availability in operating-system settings. Outflow can check them only after native integration is connected.";
  }

  if (origin !== "native") {
    return "These values are not live readiness checks. Native microphone integration and an intentional dictation action are still required before audio can start.";
  }

  return "Start dictation intentionally from the app's native session controls. This panel does not start recording.";
}

/**
 * Presents microphone capability without querying the browser or operating system.
 * The development scenarios are local presentation fixtures only; a future native
 * adapter must supply explicit status values and callbacks before actions are enabled.
 */
export function MicrophoneCapabilityPanel({
  permissionStatus,
  inputDeviceAvailability,
  statusOrigin = "preview",
  nativeAdapter,
  children,
}: MicrophoneCapabilityPanelProps) {
  const permission = permissionStatus ?? "unknown";
  const inputDevice = inputDeviceAvailability ?? "unknown";
  const origin = statusOrigin;
  const canCheckAgain =
    nativeAdapter?.connected === true &&
    typeof nativeAdapter.checkAgain === "function";
  const shouldOfferSystemSettings =
    permission === "denied" || permission === "needs_user_action";
  const canOpenSystemSettings =
    shouldOfferSystemSettings &&
    nativeAdapter?.connected === true &&
    typeof nativeAdapter.openSystemSettings === "function";
  const checkAgainHelp = canCheckAgain
    ? undefined
    : "Check again is unavailable because no connected native permission-check callback is provided.";
  const systemSettingsHelp = canOpenSystemSettings
    ? undefined
    : "System Settings cannot be opened because no connected native settings callback is provided.";

  return (
    <section
      aria-labelledby="microphone-capability-heading"
      className="microphone-capability-panel"
    >
      <h3 id="microphone-capability-heading">Microphone capability</h3>
      <p
        aria-atomic="true"
        aria-live="polite"
        className="microphone-status-announcement"
        role="status"
      >
        {statusSummary(permission, origin)}
      </p>

      {origin === "simulated" ? (
        <p className="microphone-simulation-label" role="note">
          Simulated scenario — these values are not observations from a real
          microphone or operating-system permission check.
        </p>
      ) : null}

      <dl className="microphone-status-grid">
        <div className="microphone-status-item">
          <dt>Permission status</dt>
          <dd>{permissionLabels[permission]}</dd>
        </div>
        <div className="microphone-status-item">
          <dt>Input-device availability</dt>
          <dd>{deviceLabels[inputDevice]}</dd>
        </div>
        <div className="microphone-status-item">
          <dt>Native microphone integration</dt>
          <dd>{nativeAdapter?.connected ? "Connected" : "Not connected"}</dd>
        </div>
        <div className="microphone-status-item">
          <dt>Status source</dt>
          <dd>
            {origin === "native"
              ? "Native adapter"
              : origin === "simulated"
                ? "Simulated"
                : "Preview — not verified"}
          </dd>
        </div>
      </dl>

      <div className="microphone-guidance">
        <h4>Why Outflow needs microphone access</h4>
        <p>
          Outflow needs permission to capture speech only after you intentionally
          start dictation. Opening Settings does not start recording.
        </p>
        <h4>Permission and input-device guidance</h4>
        <p>{permissionGuidance[permission]}</p>
        <p>{deviceGuidance[inputDevice]}</p>
        <h4>What you can do next</h4>
        <p>{nextStep(permission, inputDevice, origin)}</p>
      </div>

      <div className="microphone-actions">
        <button
          aria-describedby="microphone-check-again-help"
          disabled={!canCheckAgain}
          onClick={() => nativeAdapter?.checkAgain?.()}
          type="button"
        >
          Check again
        </button>
        {shouldOfferSystemSettings ? (
          <button
            aria-describedby="microphone-system-settings-help"
            disabled={!canOpenSystemSettings}
            onClick={() => nativeAdapter?.openSystemSettings?.()}
            type="button"
          >
            Open System Settings
          </button>
        ) : null}
      </div>
      <div className="microphone-action-help">
        <p id="microphone-check-again-help">
          {checkAgainHelp ??
            "Check again runs only when you activate it and a connected native callback is available."}
        </p>
        {shouldOfferSystemSettings ? (
          <p id="microphone-system-settings-help">
            {systemSettingsHelp ??
              "Open System Settings runs only when you activate it and a connected native callback is available."}
          </p>
        ) : null}
      </div>

      <p className="microphone-preview-note" role="note">
        This is a presentation preview. No audio is captured, no devices are
        enumerated, and no browser or operating-system permission check runs.
      </p>

      {children}
    </section>
  );
}
