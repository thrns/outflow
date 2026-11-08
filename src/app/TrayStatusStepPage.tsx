import AppHeader, { type AppView } from "./AppHeader";
import { PageContainer } from "./PageContainer";

type TrayStatusStepPageProps = {
  onNavigate: (view: AppView) => void;
  headingRef: (heading: HTMLHeadingElement | null) => void;
};

/**
 * Explains the planned native tray and readiness requirements without
 * connecting a tray, background process, microphone, or recording controls.
 */
export default function TrayStatusStepPage({
  onNavigate,
  headingRef,
}: TrayStatusStepPageProps) {
  return (
    <main>
      <PageContainer>
        <AppHeader onNavigate={onNavigate} />
        <div className="home-heading-group">
          <h1 id="tray-status-heading" ref={headingRef} tabIndex={-1}>
            Tray and app status — preview
          </h1>
          <p className="home-description">
            This browser preview is not connected to a native Outflow
            application.
          </p>
        </div>

        <div className="tray-status-content">
          <p className="tray-status-statement" role="note">
            <strong>System tray integration is not connected.</strong> The
            planned desktop application will use a system tray or
            platform-appropriate system menu to keep essential dictation
            controls accessible.
          </p>

          <section
            aria-labelledby="planned-tray-controls-heading"
            className="tray-status-section"
          >
            <h2 id="planned-tray-controls-heading">Planned tray functions</h2>
            <ul
              aria-labelledby="planned-tray-controls-heading"
              className="tray-status-list"
            >
              <li>
                <strong>Start / Stop.</strong> Control a dictation session when
                native recording is available.
              </li>
              <li>
                <strong>Open Settings.</strong> Open the application&apos;s
                Settings interface.
              </li>
              <li>
                <strong>Status / Permissions.</strong> Explain current
                recording readiness, capability limitations, and permission
                recovery.
              </li>
              <li>
                <strong>Quit.</strong> Safely stop active resources and exit
                the application.
              </li>
            </ul>
            <p className="tray-status-boundary">
              These are descriptions of future controls, not interactive menu
              items. This preview does not offer Start, Stop, or Quit actions.
            </p>
          </section>

          <section
            aria-labelledby="tray-readiness-heading"
            className="tray-status-section"
          >
            <h2 id="tray-readiness-heading">
              What must work before Outflow is ready
            </h2>
            <p>The planned native application must verify each of these prerequisites:</p>
            <ul aria-label="Tray readiness prerequisites" className="tray-status-list">
              <li>
                Valid bundled offline speech-model assets and working native
                recognition.
              </li>
              <li>Microphone permission and an available input device.</li>
              <li>Native recording and session lifecycle integration.</li>
              <li>
                A supported shortcut or another recording control that is
                actually implemented.
              </li>
              <li>
                Safe target handling or an explicit Copy-only fallback.
              </li>
              <li>Successful platform integration and testing.</li>
            </ul>
            <p>
              Being able to open Settings verifies only that the Settings
              interface opens; it does not prove that Outflow can record. A
              registered shortcut alone does not prove microphone permission,
              input-device availability, valid model assets, or working
              recognition.
            </p>
            <p>
              Text insertion may remain unavailable even when recording works.
              Unsupported or protected destinations must never receive forced
              insertion; the application must use an explicit Copy-only
              fallback when that is the safe option.
            </p>
            <p className="tray-status-boundary">
              The browser preview has no overall readiness result. System tray
              integration is not connected.
            </p>
          </section>

          <section
            aria-labelledby="tray-background-heading"
            className="tray-status-section"
          >
            <h2 id="tray-background-heading">
              Intended background behavior
            </h2>
            <p>
              The future desktop application must keep its native core and tray
              active when a Settings window closes, while releasing resources
              owned by that Settings window. This is a native lifecycle
              requirement; closing this browser tab does not preserve a native
              Outflow service.
            </p>
            <p>
              This browser preview does not create a background process,
              register a tray icon, control microphone resources, or manage a
              native window lifecycle.
            </p>
          </section>

          <div aria-label="Tray and app status actions" className="welcome-actions">
            <button
              className="app-navigation-button"
              onClick={() => onNavigate("try-dictation")}
              type="button"
            >
              Back to Try Dictation
            </button>
            <button
              className="home-open-settings-button"
              onClick={() => onNavigate("home")}
              type="button"
            >
              Return to Home
            </button>
          </div>
        </div>
      </PageContainer>
    </main>
  );
}
