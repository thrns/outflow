import { PermissionReadinessChecklist } from "./PermissionReadinessChecklist";
import { SettingsRow } from "./SettingsRow";
import { SettingsSwitch } from "./SettingsSwitch";

type GeneralSettingsPanelProps = {
  runAtLogin: boolean;
  onRunAtLoginChange: (checked: boolean) => void;
};

/** Renders General preferences while leaving their value and lifetime to App. */
export function GeneralSettingsPanel({
  runAtLogin,
  onRunAtLoginChange,
}: GeneralSettingsPanelProps) {
  return (
    <div className="settings-preferences">
      <SettingsRow
        title="Launch at Login"
        titleId="launch-at-login-label"
        description="Start Outflow automatically when you sign in to your computer."
        descriptionId="launch-at-login-description"
      >
        <div className="settings-field-control">
          <span className="settings-switch-state">
            {runAtLogin ? "On" : "Off"}
          </span>
          <SettingsSwitch
            checked={runAtLogin}
            onCheckedChange={onRunAtLoginChange}
            labelledBy="launch-at-login-label"
            describedBy="launch-at-login-description launch-at-login-preview-note"
          />
        </div>
      </SettingsRow>
      <p
        className="settings-preview-note"
        id="launch-at-login-preview-note"
        role="note"
      >
        Preview only — this setting does not change your computer&apos;s startup
        behavior yet.
      </p>
      <PermissionReadinessChecklist />
    </div>
  );
}
