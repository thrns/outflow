import AppHeader, { type AppView } from "./AppHeader";
import { PageContainer } from "./PageContainer";
import SyntheticDictationExample from "./SyntheticDictationExample";

type TryDictationStepPageProps = {
  onNavigate: (view: AppView) => void;
  headingRef: (heading: HTMLHeadingElement | null) => void;
};

/**
 * Explains the planned native dictation workflow without connecting any of
 * its audio, recognition, focus, insertion, or clipboard capabilities.
 */
export default function TryDictationStepPage({
  onNavigate,
  headingRef,
}: TryDictationStepPageProps) {
  return (
    <main>
      <PageContainer>
        <AppHeader onNavigate={onNavigate} />
        <div className="home-heading-group">
          <h1 id="try-dictation-heading" ref={headingRef} tabIndex={-1}>
            Try Dictation — preview
          </h1>
          <p className="home-description">
            This browser preview cannot execute any of these native steps.
            Dictation, microphone access, and destination checks are not
            connected.
          </p>
        </div>

        <div className="try-dictation-content">
          <section
            aria-labelledby="dictation-workflow-heading"
            className="try-dictation-workflow"
          >
            <h2 id="dictation-workflow-heading">
              How dictation is intended to work
            </h2>
            <ol className="try-dictation-stages">
              <li>
                <strong>Start.</strong> Begin dictation with a registered
                shortcut or supported native control.
              </li>
              <li>
                <strong>Speak.</strong> Speak while the microphone is open and
                speech is processed locally.
              </li>
              <li>
                <strong>Stop.</strong> Stop recording and finalize the result.
              </li>
              <li>
                <strong>Review.</strong> Verify the destination before inserting
                text, or offer explicit Copy Result when safe insertion is
                unavailable.
              </li>
            </ol>
          </section>

          <p className="try-dictation-safety-note" role="note">
            Protected controls and unexpected focus changes block insertion. If
            an insertion outcome is uncertain, stop and require review; never
            retry automatically or send text to an unsafe destination.
          </p>

          {import.meta.env.DEV ? (
            <SyntheticDictationExample />
          ) : (
            <p className="permissions-preview-disclaimer" role="note">
              Native dictation is not connected. This example is available only
              in the development preview.
            </p>
          )}

          <div aria-label="Try Dictation actions" className="welcome-actions">
            <button
              className="app-navigation-button"
              onClick={() => onNavigate("shortcut-setup")}
              type="button"
            >
              Back to Shortcuts
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
