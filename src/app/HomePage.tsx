import AppHeader, { type AppPage } from "./AppHeader";
import { PageContainer } from "./PageContainer";

type HomePageProps = {
  onNavigate: (page: AppPage) => void;
  headingRef: (heading: HTMLHeadingElement | null) => void;
};

export default function HomePage({ onNavigate, headingRef }: HomePageProps) {
  return (
    <main>
      <PageContainer>
        <AppHeader currentPage="home" onNavigate={onNavigate} />
        <div className="home-heading-group">
          <h1 id="home-heading" ref={headingRef} tabIndex={-1}>
            Home
          </h1>
          <p className="home-description">
            Outflow is designed for offline desktop dictation.
          </p>
        </div>

        <div className="home-content">
          <section
            aria-labelledby="home-status-heading"
            className="home-status-card"
            role="status"
          >
            <p className="home-status-label">Development preview</p>
            <h2 id="home-status-heading">
              Native dictation is not connected
            </h2>
            <p className="home-status-description">
              This preview does not request microphone access or start a
              recording session. No speech model is connected.
            </p>
          </section>

          <section
            aria-labelledby="getting-started-heading"
            className="home-getting-started-card"
          >
            <div>
              <p className="home-section-label">Getting started</p>
              <h2 id="getting-started-heading">Dictation that stays local</h2>
              <p className="home-getting-started-description">
                The planned desktop app will let you start dictation with a
                shortcut, process speech offline, then use finished text in a
                safe target or copy it when insertion is unavailable. These
                features are not available in this preview.
              </p>
            </div>
            <button
              className="home-open-settings-button"
              onClick={() => onNavigate("settings")}
              type="button"
            >
              Open Settings
            </button>
          </section>
        </div>
      </PageContainer>
    </main>
  );
}
