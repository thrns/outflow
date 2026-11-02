import type { ReactNode } from "react";

type SettingsSectionProps = {
  id: string;
  headingId?: string;
  labelledBy?: string;
  title: string;
  description?: string;
  children: ReactNode;
};

export function SettingsSection({
  id,
  headingId,
  labelledBy,
  title,
  description,
  children,
}: SettingsSectionProps) {
  const resolvedHeadingId = headingId ?? `${id}-heading`;

  return (
    <section
      aria-labelledby={labelledBy ?? resolvedHeadingId}
      className="settings-panel"
      id={id}
      role="tabpanel"
      tabIndex={0}
    >
      <div className="settings-panel-heading">
        <h2 id={resolvedHeadingId}>{title}</h2>
        {description ? <p>{description}</p> : null}
      </div>
      {children}
    </section>
  );
}
