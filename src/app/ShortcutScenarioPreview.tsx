import { useState } from "react";
import {
  ShortcutCapabilityPanel,
  type HoldReleaseCapability,
  type ShortcutRegistrationStatus,
} from "./ShortcutCapabilityPanel";

type Scenario = {
  id:
    | "default"
    | "available-release-supported"
    | "available-release-unsupported"
    | "conflict"
    | "permission-unavailable"
    | "unsupported"
    | "registration-failed";
  label: string;
  registration: ShortcutRegistrationStatus;
  release: HoldReleaseCapability;
};

const scenarios: Scenario[] = [
  {
    id: "default",
    label: "Unknown / default (simulated)",
    registration: "unknown",
    release: "unknown",
  },
  {
    id: "available-release-supported",
    label: "Registration available, release supported (simulated)",
    registration: "available",
    release: "supported",
  },
  {
    id: "available-release-unsupported",
    label: "Registration available, release unsupported (simulated)",
    registration: "available",
    release: "unsupported",
  },
  {
    id: "conflict",
    label: "Registration conflict (simulated)",
    registration: "conflict",
    release: "unknown",
  },
  {
    id: "permission-unavailable",
    label: "Permission / authorization unavailable (simulated)",
    registration: "permission_unavailable",
    release: "unknown",
  },
  {
    id: "unsupported",
    label: "Shortcut capability unsupported (simulated)",
    registration: "unsupported",
    release: "unknown",
  },
  {
    id: "registration-failed",
    label: "Registration failed (simulated)",
    registration: "failed",
    release: "unknown",
  },
];

/** Keeps fixed shortcut fixtures in local development-only React state. */
export function ShortcutScenarioPreview() {
  const [selectedScenarioId, setSelectedScenarioId] =
    useState<Scenario["id"]>("default");
  const selectedScenario =
    scenarios.find((scenario) => scenario.id === selectedScenarioId) ??
    scenarios[0];

  return (
    <>
      <ShortcutCapabilityPanel
        holdReleaseCapability={selectedScenario.release}
        registrationStatus={selectedScenario.registration}
        statusOrigin="simulated"
      />
      <fieldset className="shortcut-scenario-inspector">
        <legend>Developer scenarios (simulated)</legend>
        <label htmlFor="shortcut-scenario">Scenario</label>
        <select
          aria-describedby="shortcut-scenario-help"
          id="shortcut-scenario"
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
        <p id="shortcut-scenario-help">
          Each fixed scenario is simulated. Selection changes only local React
          presentation state; no keys are observed, no shortcut is registered,
          and no microphone or recording action runs.
        </p>
      </fieldset>
    </>
  );
}
