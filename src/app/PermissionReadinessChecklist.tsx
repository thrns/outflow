import { useState } from "react";
import type {
  InputDeviceAvailability,
  MicrophonePermissionStatus,
} from "./MicrophoneCapabilityPanel";
import type {
  HoldReleaseCapability,
  ShortcutRegistrationStatus,
} from "./ShortcutCapabilityPanel";

export type TextInsertionCapabilityStatus =
  | "unknown"
  | "available"
  | "denied"
  | "needs_user_action"
  | "not_supported";

type PermissionScenarioId =
  | "default"
  | "microphone-denied"
  | "microphone-no-device"
  | "shortcut-conflict"
  | "release-unsupported"
  | "insertion-denied"
  | "insertion-limited"
  | "hypothetical-available";

type PermissionScenario = {
  label: string;
  microphonePermission: MicrophonePermissionStatus;
  inputDevice: InputDeviceAvailability;
  shortcutRegistration: ShortcutRegistrationStatus;
  shortcutRelease: HoldReleaseCapability;
  textInsertion: TextInsertionCapabilityStatus;
};

const scenarios: Record<PermissionScenarioId, PermissionScenario> = {
  default: {
    label: "All unknown (default)",
    microphonePermission: "unknown",
    inputDevice: "unknown",
    shortcutRegistration: "unknown",
    shortcutRelease: "unknown",
    textInsertion: "unknown",
  },
  "microphone-denied": {
    label: "Microphone permission denied",
    microphonePermission: "denied",
    inputDevice: "unknown",
    shortcutRegistration: "unknown",
    shortcutRelease: "unknown",
    textInsertion: "unknown",
  },
  "microphone-no-device": {
    label: "Microphone available, no input device",
    microphonePermission: "available",
    inputDevice: "none",
    shortcutRegistration: "unknown",
    shortcutRelease: "unknown",
    textInsertion: "unknown",
  },
  "shortcut-conflict": {
    label: "Shortcut conflict",
    microphonePermission: "unknown",
    inputDevice: "unknown",
    shortcutRegistration: "conflict",
    shortcutRelease: "unknown",
    textInsertion: "unknown",
  },
  "release-unsupported": {
    label: "Shortcut release events unsupported",
    microphonePermission: "unknown",
    inputDevice: "unknown",
    shortcutRegistration: "unknown",
    shortcutRelease: "unsupported",
    textInsertion: "unknown",
  },
  "insertion-denied": {
    label: "Text insertion permission denied",
    microphonePermission: "unknown",
    inputDevice: "unknown",
    shortcutRegistration: "unknown",
    shortcutRelease: "unknown",
    textInsertion: "denied",
  },
  "insertion-limited": {
    label: "Text insertion unsupported or limited",
    microphonePermission: "unknown",
    inputDevice: "unknown",
    shortcutRegistration: "unknown",
    shortcutRelease: "unknown",
    textInsertion: "not_supported",
  },
  "hypothetical-available": {
    label: "All capabilities hypothetically available",
    microphonePermission: "available",
    inputDevice: "available",
    shortcutRegistration: "registered",
    shortcutRelease: "supported",
    textInsertion: "available",
  },
};

const microphonePermissionLabels: Record<MicrophonePermissionStatus, string> = {
  unknown: "Not checked",
  available: "Available",
  denied: "Denied",
  needs_user_action: "Action required",
  not_supported: "Not supported",
};

const inputDeviceLabels: Record<InputDeviceAvailability, string> = {
  unknown: "Not checked",
  available: "Device detected",
  none: "No available input device",
  disconnected: "Disconnected or unavailable",
};

const shortcutRegistrationLabels: Record<ShortcutRegistrationStatus, string> = {
  unknown: "Not checked",
  available: "Available, not registered",
  registered: "Registered",
  permission_unavailable: "Authorization unavailable",
  conflict: "Conflict detected",
  failed: "Registration failed",
  unsupported: "Not supported",
};

const shortcutReleaseLabels: Record<HoldReleaseCapability, string> = {
  unknown: "Not checked",
  supported: "Supported",
  unsupported: "Unsupported",
};

const textInsertionLabels: Record<TextInsertionCapabilityStatus, string> = {
  unknown: "Not checked",
  available: "Available",
  denied: "Denied",
  needs_user_action: "Action required",
  not_supported: "Not supported or limited",
};

function fixtureLabel(label: string, simulated: boolean) {
  return simulated ? `${label} (simulated)` : label;
}

function microphoneGuidance(scenario: PermissionScenario) {
  if (scenario.microphonePermission === "denied") {
    return "Microphone permission is required for actual recording. Grant access in operating-system settings, then verify it through native integration; this preview cannot open settings.";
  }

  if (scenario.inputDevice === "none") {
    return "Connect or enable an input device in operating-system audio settings. Permission and device availability are separate, and this preview cannot enumerate devices.";
  }

  if (scenario.microphonePermission === "available" && scenario.inputDevice === "available") {
    return "This fixture marks permission and a device as hypothetically available. Neither was checked by the operating system, and no recording has started.";
  }

  return "Microphone permission is required for actual recording. Permission and input-device availability have not been checked by this browser preview.";
}

function shortcutGuidance(scenario: PermissionScenario) {
  if (scenario.shortcutRegistration === "conflict") {
    return "Another assignment may be needed to resolve this shortcut conflict. A conflict is different from authorization denial; no shortcut registration was attempted.";
  }

  if (scenario.shortcutRelease === "unsupported") {
    return "Push-to-talk needs reliable matching press-and-release events, which are unsupported in this fixture. Toggle might be an alternative only after native shortcut registration and microphone readiness are confirmed.";
  }

  if (scenario.shortcutRelease === "unknown") {
    return "Unknown release support does not mean push-to-talk works. Confirm reliable press-and-release events through native integration; a timeout cannot stand in for a release. No shortcut is registered by this preview.";
  }

  if (scenario.shortcutRegistration === "registered") {
    return "This fixture is hypothetical. Registration and release events are not operating-system observations, and microphone readiness is separate.";
  }

  return "Global shortcut registration and matching press-and-release capability have not been checked by this browser preview.";
}

function insertionGuidance(status: TextInsertionCapabilityStatus) {
  if (status === "denied" || status === "needs_user_action") {
    return "Native text insertion permission is separate from microphone permission. If insertion is unavailable, the intended recovery is an explicit Copy Result action; real clipboard integration is not connected in this browser preview.";
  }

  if (status === "not_supported") {
    return "Native text insertion may not be possible on this platform. The intended recovery is an explicit Copy Result action; real clipboard integration is not connected in this browser preview. Wayland portal capabilities remain unverified here and must be treated as limited until native checks confirm them.";
  }

  if (status === "available") {
    return "This fixture marks insertion capability as hypothetically available. It does not establish that any particular destination is editable, safe, still focused, or non-protected.";
  }

  return "Text-insertion capability has not been checked. It is separate from microphone permission and is not required for a future Copy-only fallback.";
}

function CapabilityValue({
  label,
  value,
  simulated,
}: {
  label: string;
  value: string;
  simulated: boolean;
}) {
  return (
    <div className="permission-readiness-value">
      <dt>{label}</dt>
      <dd>{fixtureLabel(value, simulated)}</dd>
    </div>
  );
}

/**
 * Read-only browser presentation of the three independent native capabilities.
 * Development fixtures are local component state; they never probe or mutate
 * operating-system permissions, shortcuts, microphone, clipboard, or targets.
 */
export function PermissionReadinessChecklist() {
  const [scenarioId, setScenarioId] = useState<PermissionScenarioId>("default");
  const scenario = scenarios[scenarioId];
  const simulated = scenarioId !== "default";

  return (
    <section
      aria-labelledby="permission-readiness-heading"
      className="permission-readiness-checklist"
    >
      <header className="permission-readiness-header">
        <h3 id="permission-readiness-heading">Permission readiness</h3>
        <p>
          Microphone permission is required for actual recording. Native text
          insertion uses a separate capability and is not required for a future
          Copy-only fallback.
        </p>
      </header>

      <p
        aria-atomic="true"
        aria-live="polite"
        className="permission-readiness-announcement"
        role="status"
      >
        {simulated
          ? `Simulated scenario: ${scenario.label}. These values are independent fixtures, not operating-system observations.`
          : "Permission readiness has not been checked."}
      </p>

      {import.meta.env.DEV ? (
        <div className="permission-readiness-scenario">
          <label htmlFor="permission-readiness-scenario-select">
            Developer scenarios (simulated fixtures)
          </label>
          <select
            aria-describedby="permission-readiness-scenario-help"
            id="permission-readiness-scenario-select"
            onChange={(event) =>
              setScenarioId(event.currentTarget.value as PermissionScenarioId)
            }
            value={scenarioId}
          >
            {(Object.entries(scenarios) as [PermissionScenarioId, PermissionScenario][]).map(
              ([id, fixture]) => (
                <option key={id} value={id}>
                  {fixture.label}
                </option>
              ),
            )}
          </select>
          <p id="permission-readiness-scenario-help">
            Fixtures are local to this checklist. They do not change the
            Microphone or Shortcuts tabs and are not saved.
          </p>
        </div>
      ) : null}

      <p className="permission-readiness-preview-note" role="note">
        This browser preview cannot inspect operating-system permissions or
        native capabilities. Wayland portal support for shortcuts and insertion
        is unverified here and must be treated as limited until native checks
        confirm it.
      </p>

      {simulated ? (
        <p className="permission-readiness-simulated-note" role="note">
          Simulated values are independent fixtures, not shared operating-system
          observations. No permission was checked or granted.
        </p>
      ) : null}

      <ol aria-label="Permission capability checklist" className="permission-readiness-list">
        <li className="permission-readiness-item">
          <h4>Microphone</h4>
          <dl aria-label="Microphone status">
            <CapabilityValue
              label="Permission"
              value={microphonePermissionLabels[scenario.microphonePermission]}
              simulated={simulated}
            />
            <CapabilityValue
              label="Input-device availability"
              value={inputDeviceLabels[scenario.inputDevice]}
              simulated={simulated}
            />
          </dl>
          <p>{microphoneGuidance(scenario)}</p>
        </li>

        <li className="permission-readiness-item">
          <h4>Global shortcuts</h4>
          <dl aria-label="Global shortcut status">
            <CapabilityValue
              label="Registration"
              value={shortcutRegistrationLabels[scenario.shortcutRegistration]}
              simulated={simulated}
            />
            <CapabilityValue
              label="Press-and-release events"
              value={shortcutReleaseLabels[scenario.shortcutRelease]}
              simulated={simulated}
            />
          </dl>
          <p>{shortcutGuidance(scenario)}</p>
        </li>

        <li className="permission-readiness-item">
          <h4>Text insertion</h4>
          <dl aria-label="Text insertion status">
            <CapabilityValue
              label="Native insertion capability"
              value={textInsertionLabels[scenario.textInsertion]}
              simulated={simulated}
            />
          </dl>
          <p>{insertionGuidance(scenario.textInsertion)}</p>
          <p className="permission-readiness-target-note">
            Capability does not establish target eligibility. Before any future
            insertion, the native app must verify the specific destination is
            editable, safe, still focused, and not password-protected or
            otherwise protected. Accessibility permission alone does not make
            every field safe to insert into.
          </p>
        </li>
      </ol>
    </section>
  );
}
