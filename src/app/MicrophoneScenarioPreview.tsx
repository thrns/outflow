import { useState } from "react";
import {
  MicrophoneCapabilityPanel,
  type InputDeviceAvailability,
  type MicrophonePermissionStatus,
} from "./MicrophoneCapabilityPanel";

type Scenario = {
  id:
    | "default"
    | "available"
    | "denied"
    | "needs-user-action"
    | "unsupported"
    | "no-device"
    | "disconnected";
  label: string;
  permission: MicrophonePermissionStatus;
  inputDevice: InputDeviceAvailability;
};

const scenarios: Scenario[] = [
  {
    id: "default",
    label: "Unknown / default",
    permission: "unknown",
    inputDevice: "unknown",
  },
  {
    id: "available",
    label: "Permission available, device present (simulated)",
    permission: "available",
    inputDevice: "available",
  },
  {
    id: "denied",
    label: "Permission denied (simulated)",
    permission: "denied",
    inputDevice: "unknown",
  },
  {
    id: "needs-user-action",
    label: "User action required (simulated)",
    permission: "needs_user_action",
    inputDevice: "unknown",
  },
  {
    id: "unsupported",
    label: "Unsupported capability (simulated)",
    permission: "not_supported",
    inputDevice: "unknown",
  },
  {
    id: "no-device",
    label: "No device detected (simulated)",
    permission: "available",
    inputDevice: "none",
  },
  {
    id: "disconnected",
    label: "Device disconnected (simulated)",
    permission: "available",
    inputDevice: "disconnected",
  },
];

/** Keeps synthetic microphone states in local development-only React state. */
export function MicrophoneScenarioPreview() {
  const [selectedScenarioId, setSelectedScenarioId] =
    useState<Scenario["id"]>("default");
  const selectedScenario =
    scenarios.find((scenario) => scenario.id === selectedScenarioId) ??
    scenarios[0];
  const origin = selectedScenarioId === "default" ? "preview" : "simulated";

  return (
    <MicrophoneCapabilityPanel
      inputDeviceAvailability={selectedScenario.inputDevice}
      permissionStatus={selectedScenario.permission}
      statusOrigin={origin}
    >
      <fieldset className="microphone-scenario-inspector">
        <legend>Developer scenarios (simulated)</legend>
        <label htmlFor="microphone-scenario">Scenario</label>
        <select
          aria-describedby="microphone-scenario-help"
          id="microphone-scenario"
          onChange={(event) =>
            setSelectedScenarioId(event.currentTarget.value as Scenario["id"])
          }
          value={selectedScenarioId}
        >
          {scenarios.map((scenario) => (
            <option key={scenario.id} value={scenario.id}>
              {scenario.label}
            </option>
          ))}
        </select>
        <p id="microphone-scenario-help">
          Scenarios change only local React presentation state. They do not
          request permission, inspect devices, or start recording.
        </p>
      </fieldset>
    </MicrophoneCapabilityPanel>
  );
}
