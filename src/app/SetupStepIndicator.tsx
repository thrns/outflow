export const SETUP_STEP_DEFINITIONS = [
  { view: "welcome", label: "Welcome" },
  { view: "privacy", label: "Privacy" },
  { view: "permissions", label: "Permissions" },
  { view: "shortcut-setup", label: "Shortcuts" },
  { view: "try-dictation", label: "Try Dictation" },
  { view: "tray-status", label: "Tray and App Status" },
] as const;

export type SetupStepView = (typeof SETUP_STEP_DEFINITIONS)[number]["view"];

type SetupStepIndicatorProps = {
  view: SetupStepView;
};

/** Shows the current informational preview position without tracking completion. */
export default function SetupStepIndicator({ view }: SetupStepIndicatorProps) {
  const stepIndex = SETUP_STEP_DEFINITIONS.findIndex(
    (step) => step.view === view,
  );
  const step = SETUP_STEP_DEFINITIONS[stepIndex];

  if (!step) return null;

  return (
    <p className="setup-step-indicator">
      Setup preview · Step {stepIndex + 1} of {SETUP_STEP_DEFINITIONS.length} · {step.label}
    </p>
  );
}
