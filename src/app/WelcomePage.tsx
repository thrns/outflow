import AppHeader, { type AppPage } from "./AppHeader";
import { PageContainer } from "./PageContainer";

type WelcomePageProps = {
  onNavigate: (page: AppPage) => void;
  headingRef: (heading: HTMLHeadingElement | null) => void;
};

export default function WelcomePage({
  onNavigate,
  headingRef,
}: WelcomePageProps) {
  return (
    <main>
      <PageContainer>
        <AppHeader onNavigate={onNavigate} />
        <div className="home-heading-group welcome-heading-group">
          <h1 id="welcome-heading" ref={headingRef} tabIndex={-1}>
            Welcome to Outflow
          </h1>
          <p className="home-description">
            An introduction to offline desktop dictation and setup.
          </p>
        </div>

        <div className="welcome-content">
          <section
            aria-labelledby="welcome-privacy-heading"
            className="home-status-card welcome-principles"
          >
            <h2 id="welcome-privacy-heading">Designed to keep speech local</h2>
            <p className="welcome-principles-message">
              Speech stays on this device. No account. No transcript history.
            </p>
            <p className="welcome-copy">
              That is the intended privacy design for the planned MVP: speech
              processing is meant to happen on-device, without an account or
              saved transcript history.
            </p>
          </section>

          <p className="welcome-copy">
            Actual dictation requires a functioning Outflow desktop application
            and microphone permission. This browser preview cannot record audio
            or check permissions.
          </p>

          <section
            aria-labelledby="welcome-preview-heading"
            className="home-status-card welcome-preview-note"
          >
            <h2 id="welcome-preview-heading">Development preview</h2>
            <p className="welcome-copy">
              This screen introduces planned setup; it does not prove that
              offline speech processing is complete or indicate that dictation
              is ready. No setup progress is saved here.
            </p>
          </section>

          <div aria-label="Welcome actions" className="welcome-actions">
            <button
              className="app-navigation-button"
              onClick={() => onNavigate("home")}
              type="button"
            >
              Back to Home
            </button>
            <button
              className="home-open-settings-button"
              onClick={() => onNavigate("settings")}
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
