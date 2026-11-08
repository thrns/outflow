import AppHeader, { type AppView } from "./AppHeader";
import { PageContainer } from "./PageContainer";
import { PermissionReadinessChecklist } from "./PermissionReadinessChecklist";
import SetupStepIndicator from "./SetupStepIndicator";

type PermissionsStepPageProps = {
  onNavigate: (view: AppView) => void;
  onOpenSettings: () => void;
  headingRef: (heading: HTMLHeadingElement | null) => void;
};

/**
 * Explains the planned permission boundaries in a transient browser preview.
 * This page has no native permission provider and performs no capability checks.
 */
export default function PermissionsStepPage({
  onNavigate,
  onOpenSettings,
  headingRef,
}: PermissionsStepPageProps) {
  return (
    <main>
      <PageContainer>
        <AppHeader onNavigate={onNavigate} />
        <div className="home-heading-group">
          <SetupStepIndicator view="permissions" />
          <h1 id="permissions-step-heading" ref={headingRef} tabIndex={-1}>
            Permissions for Outflow
          </h1>
          <p className="home-description">
            Why the planned desktop app uses separate microphone, shortcut, and
            text-insertion capabilities.
          </p>
        </div>

        <div className="permissions-step-content">
          <p className="permissions-preview-disclaimer" role="note">
            This is an unconnected browser preview. No permission has been
            granted, checked, or requested.
          </p>

          <div className="permissions-capability-list">
            <section
              aria-labelledby="permissions-microphone-heading"
              className="home-status-card"
            >
              <h2 id="permissions-microphone-heading">Microphone</h2>
              <p>
                The planned app needs microphone permission to capture speech
                only after you intentionally start dictation. Permission and an
                available input device are separate; permission alone does not
                mean a working microphone is available.
              </p>
            </section>

            <section
              aria-labelledby="permissions-shortcuts-heading"
              className="home-status-card"
            >
              <h2 id="permissions-shortcuts-heading">Global shortcuts</h2>
              <p>
                Global shortcuts are intended to start and stop dictation.
                Push-to-talk requires reliable matching press-and-release
                events; shortcut registration alone is not enough. If release
                events are unavailable, toggle may be an alternative.
              </p>
            </section>

            <section
              aria-labelledby="permissions-insertion-heading"
              className="home-status-card"
            >
              <h2 id="permissions-insertion-heading">Text insertion</h2>
              <p>
                Text insertion is intended to place finalized text in supported
                destinations. Permission alone does not establish that a
                specific editor is safe or eligible; the destination must be
                checked before any insertion.
              </p>
              <p>
                Text-insertion permission is optional for a future Copy-only
                workflow. If insertion is unavailable, the planned fallback is
                a result panel that waits for an explicit Copy Result action;
                this browser preview has no clipboard connection and does not
                enable Copy. Microphone permission remains necessary for actual
                speech recording.
              </p>
            </section>
          </div>

          <section
            aria-labelledby="permissions-checklist-section-heading"
            className="permissions-step-checklist"
          >
            <h2 id="permissions-checklist-section-heading">
              Review the separate capability details
            </h2>
            <PermissionReadinessChecklist />
          </section>

          <div aria-label="Permissions step actions" className="welcome-actions">
            <button
              className="app-navigation-button"
              onClick={() => onNavigate("privacy")}
              type="button"
            >
              Back to Privacy
            </button>
            <button
              className="home-open-settings-button"
              onClick={onOpenSettings}
              type="button"
            >
              View permission details in Settings
            </button>
            <button
              className="home-open-settings-button"
              onClick={() => onNavigate("shortcut-setup")}
              type="button"
            >
              Continue to shortcuts
            </button>
          </div>
        </div>
      </PageContainer>
    </main>
  );
}
