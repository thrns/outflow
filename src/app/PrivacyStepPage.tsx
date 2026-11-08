import AppHeader, { type AppView } from "./AppHeader";
import { PageContainer } from "./PageContainer";
import PrivacyOverview from "./PrivacyOverview";
import SetupStepIndicator from "./SetupStepIndicator";

type PrivacyStepPageProps = {
  onNavigate: (view: AppView) => void;
  onOpenSettings: () => void;
  headingRef: (heading: HTMLHeadingElement | null) => void;
};

/**
 * Presents the shared privacy requirements as a transient setup-preview step.
 * Navigation and disclosure do not inspect or persist user data.
 */
export default function PrivacyStepPage({
  onNavigate,
  onOpenSettings,
  headingRef,
}: PrivacyStepPageProps) {
  return (
    <main>
      <PageContainer>
        <AppHeader onNavigate={onNavigate} />
        <div className="home-heading-group">
          <SetupStepIndicator view="privacy" />
          <h1 id="privacy-step-heading" ref={headingRef} tabIndex={-1}>
            Your privacy with Outflow
          </h1>
          <p className="home-description">
            What the planned product requires and what this browser preview
            cannot verify.
          </p>
        </div>

        <div className="privacy-step-content">
          <section
            aria-labelledby="privacy-step-requirements-heading"
            className="privacy-step-requirements"
          >
            <h2 id="privacy-step-requirements-heading">
              Planned privacy requirements
            </h2>
            <PrivacyOverview setupDisclosure />
          </section>

          <div aria-label="Privacy step actions" className="welcome-actions">
            <button
              className="app-navigation-button"
              onClick={() => onNavigate("welcome")}
              type="button"
            >
              Back to Welcome
            </button>
            <button
              className="home-open-settings-button"
              onClick={() => onNavigate("permissions")}
              type="button"
            >
              Continue to permissions
            </button>
            <button
              className="app-navigation-button"
              onClick={onOpenSettings}
              type="button"
            >
              Review permission readiness
            </button>
          </div>
        </div>
      </PageContainer>
    </main>
  );
}
