import AppHeader, { type AppView } from "./AppHeader";
import { PageContainer } from "./PageContainer";
import { ShortcutCapabilityPanel } from "./ShortcutCapabilityPanel";
import { ShortcutScenarioPreview } from "./ShortcutScenarioPreview";

type ShortcutSetupStepPageProps = {
  onNavigate: (view: AppView) => void;
  onOpenShortcuts: () => void;
  headingRef: (heading: HTMLHeadingElement | null) => void;
};

/**
 * Explains planned shortcut recording modes in a transient browser preview.
 * It does not assign, capture, register, or activate a keyboard shortcut.
 */
export default function ShortcutSetupStepPage({
  onNavigate,
  onOpenShortcuts,
  headingRef,
}: ShortcutSetupStepPageProps) {
  return (
    <main>
      <PageContainer>
        <AppHeader onNavigate={onNavigate} />
        <div className="home-heading-group">
          <h1 id="shortcut-setup-heading" ref={headingRef} tabIndex={-1}>
            Choose how dictation starts
          </h1>
          <p className="home-description">
            Compare the planned recording modes and the capabilities each one
            requires.
          </p>
        </div>

        <div className="shortcut-setup-content">
          <p className="permissions-preview-disclaimer" role="note">
            Browser-only preview: no global shortcut is registered, no key
            combination is assigned or captured, and no actual recording mode
            is operational.
          </p>

          <section
            aria-labelledby="shortcut-setup-capabilities-heading"
            className="shortcut-setup-capabilities"
          >
            <h2 id="shortcut-setup-capabilities-heading">
              Planned recording modes and shortcut capability
            </h2>
            {import.meta.env.DEV ? (
              <ShortcutScenarioPreview />
            ) : (
              <ShortcutCapabilityPanel />
            )}
          </section>

          <p className="shortcut-setup-boundary" role="note">
            Shortcut conflicts and OS authorization problems are separate
            registration conditions and need different recovery. If release
            events are unsupported, toggle is only a possible fallback when
            native shortcut registration and microphone readiness both work.
          </p>

          <div aria-label="Shortcut setup actions" className="welcome-actions">
            <button
              className="app-navigation-button"
              onClick={() => onNavigate("permissions")}
              type="button"
            >
              Back to Permissions
            </button>
            <button
              className="home-open-settings-button"
              onClick={onOpenShortcuts}
              type="button"
            >
              View shortcut details in Settings
            </button>
          </div>
        </div>
      </PageContainer>
    </main>
  );
}
