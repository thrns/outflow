export type ShortcutRegistrationStatus =
  | "unknown"
  | "available"
  | "registered"
  | "permission_unavailable"
  | "conflict"
  | "failed"
  | "unsupported";

export type HoldReleaseCapability = "unknown" | "supported" | "unsupported";

export type RecordingMode = "push_to_talk" | "toggle";

export type RecordingReadiness = "unknown" | "ready";

export type ShortcutStatusOrigin = "preview" | "simulated" | "native";

export type NativeShortcutAdapter = {
  connected: boolean;
  checkStatus?: () => void;
};

type ShortcutCapabilityPanelProps = {
  registrationStatus?: ShortcutRegistrationStatus;
  holdReleaseCapability?: HoldReleaseCapability;
  recordingReadiness?: RecordingReadiness;
  statusOrigin?: ShortcutStatusOrigin;
  nativeAdapter?: NativeShortcutAdapter;
};

const registrationLabels: Record<ShortcutRegistrationStatus, string> = {
  unknown: "Not checked",
  available: "Available — not registered",
  registered: "Registered",
  permission_unavailable: "Permission or authorization unavailable",
  conflict: "Conflict detected",
  failed: "Registration failed",
  unsupported: "Unsupported",
};

const releaseLabels: Record<HoldReleaseCapability, string> = {
  unknown: "Not checked",
  supported: "Supported",
  unsupported: "Unsupported",
};

function isNativeRegistrationVerified(
  registrationStatus: ShortcutRegistrationStatus,
  statusOrigin: ShortcutStatusOrigin,
) {
  return registrationStatus === "registered" && statusOrigin === "native";
}

function registrationGuidance(
  registrationStatus: ShortcutRegistrationStatus,
  statusOrigin: ShortcutStatusOrigin,
) {
  if (statusOrigin !== "native") {
    if (registrationStatus === "unknown") {
      return "Shortcut registration has not been checked. This browser preview has no operating-system shortcut connection.";
    }

    return "This is a simulated capability value. The browser preview has not registered or monitored a global shortcut.";
  }

  switch (registrationStatus) {
    case "unknown":
      return "Registration has not been checked. Use Check shortcut status when a native status adapter is available.";
    case "available":
      return "The native adapter reports that global shortcuts are available, but no shortcut is registered yet.";
    case "registered":
      return "Native integration reports a registered shortcut. This does not confirm microphone permission or recording readiness.";
    case "permission_unavailable":
      return "Grant the required shortcut authorization in operating-system settings, then check status again. Permission is separate from shortcut conflicts.";
    case "conflict":
      return "Another application or shortcut owns the requested combination. Choose a different assignment when shortcut configuration is available, or resolve the other application's conflict.";
    case "failed":
      return "Registration did not complete. Check the operating-system status and retry only through the native app when that recovery action is available.";
    case "unsupported":
      return "This environment cannot provide the required global shortcut capability. Use the app's tray or menu control if the native app makes one available.";
  }
}

function statusAnnouncement(
  registrationStatus: ShortcutRegistrationStatus,
  holdReleaseCapability: HoldReleaseCapability,
  statusOrigin: ShortcutStatusOrigin,
) {
  if (statusOrigin !== "native") {
    if (registrationStatus === "unknown") {
      return "Global keyboard shortcuts are not connected.";
    }

    return `Simulated scenario: global shortcut registration is ${registrationLabels[registrationStatus].toLowerCase()}; matching press-and-release events are ${releaseLabels[holdReleaseCapability].toLowerCase()}. Global keyboard shortcuts are not connected in this preview.`;
  }

  return `Global shortcut registration: ${registrationLabels[registrationStatus]}. Matching press-and-release events: ${releaseLabels[holdReleaseCapability]}.`;
}

function modeAvailability(
  mode: RecordingMode,
  registrationStatus: ShortcutRegistrationStatus,
  holdReleaseCapability: HoldReleaseCapability,
  recordingReadiness: RecordingReadiness,
  statusOrigin: ShortcutStatusOrigin,
) {
  if (mode === "push_to_talk") {
    if (holdReleaseCapability === "unsupported") {
      return {
        state: "Unavailable",
        description:
          "Push-to-talk requires reliable matching key-down and key-up events. They are unsupported here, so this mode cannot be used. A timeout must never stand in for a key release.",
      };
    }
  }

  if (registrationStatus === "conflict") {
    return {
      state: "Unavailable — conflict",
      description:
        "This is a shortcut conflict, not a permission denial. A different assignment or resolution of the other application's shortcut is needed.",
    };
  }

  if (registrationStatus === "permission_unavailable") {
    return {
      state: "Unavailable — authorization",
      description:
        "Global shortcut authorization is unavailable. Follow the operating-system permission guidance before trying again.",
    };
  }

  if (registrationStatus === "unsupported") {
    return {
      state: "Unavailable — unsupported",
      description:
        "Global shortcut registration is unsupported. A native tray or menu action may be an alternative when available; this browser preview has no recording control.",
    };
  }

  if (registrationStatus === "failed") {
    return {
      state: "Unavailable — registration failed",
      description:
        "The shortcut could not be registered. Use native recovery guidance before retrying; no retry or recording action is available in this preview.",
    };
  }

  if (registrationStatus === "available") {
    return {
      state: mode === "toggle" && holdReleaseCapability === "unsupported"
        ? "Possible fallback — not operational"
        : "Not operational",
      description:
        mode === "toggle" && holdReleaseCapability === "unsupported"
          ? "Toggle ignores key release, so it is a possible alternative to push-to-talk. It remains a possible fallback pending actual native registration and microphone readiness."
          : "The platform capability is reported as available, but a shortcut is not registered and recording readiness has not been verified.",
    };
  }

  if (mode === "push_to_talk" && holdReleaseCapability === "unknown") {
    return {
      state: "Not verified",
      description:
        "Push-to-talk is unavailable until reliable matching key-down and key-up events are confirmed.",
    };
  }

  if (
    isNativeRegistrationVerified(registrationStatus, statusOrigin) &&
    recordingReadiness === "ready"
  ) {
    return {
      state: "Operational",
      description:
        mode === "push_to_talk"
          ? "Native registration, release-event support, and recording readiness are reported as verified. Hold the configured shortcut to record; release to finalize."
          : "Native registration and recording readiness are reported as verified. Activate the configured shortcut to begin, then activate it again to stop. Key release is ignored.",
    };
  }

  if (isNativeRegistrationVerified(registrationStatus, statusOrigin)) {
    return {
      state: "Not operational — readiness not verified",
      description:
        "Native shortcut registration is verified, but microphone permission and recording readiness have not been verified.",
    };
  }

  return {
    state: "Not operational",
    description:
      "Native shortcut registration and microphone readiness are not verified. No recording mode is operational in this preview.",
  };
}

/**
 * Presents shortcut and recording capabilities without observing keyboard,
 * microphone, permission, or operating-system state. Native statuses are
 * explicit presentation inputs for a future adapter; this component does not
 * register shortcuts or start a recording session.
 */
export function ShortcutCapabilityPanel({
  registrationStatus = "unknown",
  holdReleaseCapability = "unknown",
  recordingReadiness = "unknown",
  statusOrigin = "preview",
  nativeAdapter,
}: ShortcutCapabilityPanelProps) {
  const canCheckStatus =
    nativeAdapter?.connected === true &&
    typeof nativeAdapter.checkStatus === "function";
  const pttAvailability = modeAvailability(
    "push_to_talk",
    registrationStatus,
    holdReleaseCapability,
    recordingReadiness,
    statusOrigin,
  );
  const toggleAvailability = modeAvailability(
    "toggle",
    registrationStatus,
    holdReleaseCapability,
    recordingReadiness,
    statusOrigin,
  );
  const announcement = statusAnnouncement(
    registrationStatus,
    holdReleaseCapability,
    statusOrigin,
  );

  return (
    <section
      aria-labelledby="shortcut-capability-heading"
      className="shortcut-capability-panel"
    >
      <h3 id="shortcut-capability-heading">Global shortcut capability</h3>
      <p
        aria-atomic="true"
        aria-live="polite"
        className="shortcut-status-announcement"
        role="status"
      >
        {announcement}
      </p>

      {statusOrigin === "simulated" ? (
        <p className="shortcut-simulation-label" role="note">
          Simulated scenario — these values are fixed presentation examples, not
          observations from an operating system.
        </p>
      ) : null}

      <dl className="shortcut-status-grid">
        <div className="shortcut-status-item">
          <dt>Global shortcut registration</dt>
          <dd>{registrationLabels[registrationStatus]}</dd>
        </div>
        <div className="shortcut-status-item">
          <dt>Matching press-and-release events</dt>
          <dd>{releaseLabels[holdReleaseCapability]}</dd>
        </div>
        <div className="shortcut-status-item">
          <dt>Recording readiness, including microphone permission</dt>
          <dd>
            {statusOrigin === "native" && recordingReadiness === "ready"
              ? "Verified ready by native adapter"
              : "Not checked"}
          </dd>
        </div>
        <div className="shortcut-status-item">
          <dt>Status source</dt>
          <dd>
            {statusOrigin === "native"
              ? "Native adapter"
              : statusOrigin === "simulated"
                ? "Simulated"
                : "Browser preview — not verified"}
          </dd>
        </div>
      </dl>

      {statusOrigin !== "native" ? (
        <p className="shortcut-preview-note" role="note">
          No shortcut is currently registered by this preview. No recording
          mode is operational. Shortcut availability, release-event support,
          microphone permission, and recording readiness are separate facts.
        </p>
      ) : null}

      <section aria-labelledby="shortcut-recording-modes-heading">
        <h4 id="shortcut-recording-modes-heading">Recording modes</h4>
        <div className="shortcut-mode-grid">
          <article
            aria-labelledby="push-to-talk-mode-heading"
            className="shortcut-mode-card"
          >
            <h5 id="push-to-talk-mode-heading">Push-to-talk</h5>
            <p>
              Hold the configured shortcut to record; release to finalize.
            </p>
            <p>
              This mode requires reliable matching key-down and key-up events.
            </p>
            <p className="shortcut-mode-status">
              <strong>{pttAvailability.state}.</strong>{" "}
              {pttAvailability.description}
            </p>
          </article>
          <article
            aria-labelledby="toggle-mode-heading"
            className="shortcut-mode-card"
          >
            <h5 id="toggle-mode-heading">Toggle</h5>
            <p>
              Activate the configured shortcut to begin; activate it again to
              stop. Key release is ignored for recording control.
            </p>
            <p className="shortcut-mode-status">
              <strong>{toggleAvailability.state}.</strong>{" "}
              {toggleAvailability.description}
            </p>
          </article>
        </div>
      </section>

      <div className="shortcut-guidance">
        <h4>Shortcut status and recovery</h4>
        <p>{registrationGuidance(registrationStatus, statusOrigin)}</p>
        <p>
          A registered shortcut alone does not establish microphone permission
          or recording readiness. This panel does not request microphone access
          or start recording.
        </p>
      </div>

      <div className="shortcut-actions">
        <button
          aria-describedby="shortcut-check-status-help"
          disabled={!canCheckStatus}
          onClick={() => nativeAdapter?.checkStatus?.()}
          type="button"
        >
          Check shortcut status
        </button>
      </div>
      <div className="shortcut-action-help">
        <p id="shortcut-check-status-help">
          {canCheckStatus
            ? "This action checks status only after you activate it; it does not start recording."
            : "Checking is unavailable in the browser preview because no connected native shortcut status callback is provided."}
        </p>
      </div>
    </section>
  );
}
