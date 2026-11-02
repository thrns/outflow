import type { ReactNode } from "react";

type SettingsStatusMessageProps = {
  title: string;
  children: ReactNode;
};

/** Renders static explanatory copy without announcing it as a live status. */
export function SettingsStatusMessage({
  title,
  children,
}: SettingsStatusMessageProps) {
  return (
    <div className="settings-status-message">
      <h3 className="settings-status-message-title">{title}</h3>
      <p className="settings-status-message-description">{children}</p>
    </div>
  );
}
