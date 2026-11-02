import { type KeyboardEvent } from "react";

type SettingsSwitchProps = {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  labelledBy: string;
  describedBy?: string;
};

/** A controlled button switch for an immediately applied settings preference. */
export function SettingsSwitch({
  checked,
  onCheckedChange,
  labelledBy,
  describedBy,
}: SettingsSwitchProps) {
  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.key !== " ") return;

    // The native button emits a click on Space keyup; toggling on keydown keeps
    // the existing preview response immediate while preventing a second toggle.
    event.preventDefault();
    if (!event.repeat) onCheckedChange(!checked);
  }

  return (
    <button
      aria-checked={checked}
      aria-describedby={describedBy}
      aria-labelledby={labelledBy}
      className="settings-switch"
      onClick={() => onCheckedChange(!checked)}
      onKeyDown={handleKeyDown}
      role="switch"
      type="button"
    >
      <span aria-hidden="true" className="settings-switch-track">
        <span className="settings-switch-thumb" />
      </span>
    </button>
  );
}
