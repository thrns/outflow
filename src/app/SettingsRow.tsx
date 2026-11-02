import type { ReactNode } from "react";

type SettingsRowProps = {
  title: string;
  titleId: string;
  description?: string;
  descriptionId?: string;
  children: ReactNode;
};

/**
 * Presents one setting's visible copy beside its control.
 *
 * IDs are supplied by the owner so controls can reference the visible title
 * and description with ARIA without assuming that every child is labelable.
 */
export function SettingsRow({
  title,
  titleId,
  description,
  descriptionId,
  children,
}: SettingsRowProps) {
  return (
    <div
      aria-labelledby={titleId}
      className="settings-row"
      role="group"
    >
      <div className="settings-row-copy">
        <p className="settings-row-title" id={titleId}>
          {title}
        </p>
        {description ? (
          <p className="settings-row-description" id={descriptionId}>
            {description}
          </p>
        ) : null}
      </div>
      {children}
    </div>
  );
}
