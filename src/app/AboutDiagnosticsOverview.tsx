import { SettingsStatusMessage } from "./SettingsStatusMessage";

/**
 * Describes intended About, recovery, diagnostics, and update behavior without
 * inspecting runtime state or connecting to native services.
 */
export default function AboutDiagnosticsOverview() {
  return (
    <div className="about-diagnostics-overview privacy-overview">
      <section
        aria-labelledby="about-overview-heading"
        className="about-diagnostics-section privacy-overview-section"
      >
        <h3 id="about-overview-heading">About</h3>
        <p>
          <strong>Outflow</strong>
        </p>
        <p>
          Outflow is an offline-first desktop dictation application currently
          under development.
        </p>
        <p className="settings-preview-note" role="note">
          This interface is a development preview. Native desktop release
          metadata is not available here.
        </p>
      </section>

      <section
        aria-labelledby="about-troubleshooting-heading"
        className="about-diagnostics-section privacy-overview-section"
      >
        <h3 id="about-troubleshooting-heading">Troubleshooting</h3>
        <p>
          The finished application is intended to provide understandable error
          codes and recovery guidance for problems such as:
        </p>
        <ul>
          <li>Microphone permission is denied.</li>
          <li>An input device is disconnected.</li>
          <li>The bundled speech model cannot load or fails an integrity check.</li>
          <li>Available resources or memory limits prevent a session.</li>
          <li>The recording buffer overflows.</li>
          <li>Text insertion is uncertain or unsupported for the current target.</li>
        </ul>
        <p>
          These are planned recovery capabilities, not problems detected in
          this development preview.
        </p>
      </section>

      <section
        aria-labelledby="about-diagnostics-heading"
        className="about-diagnostics-section privacy-overview-section"
      >
        <h3 id="about-diagnostics-heading">Diagnostics</h3>
        <p>A future user-initiated diagnostic export may include:</p>
        <ul>
          <li>Application and runtime version information.</li>
          <li>Operating system and build information.</li>
          <li>Device capability results.</li>
          <li>Anonymized error codes.</li>
          <li>Timing and aggregate performance metrics.</li>
          <li>Model integrity-check results.</li>
        </ul>
        <p>Diagnostic exports must exclude:</p>
        <ul>
          <li>Recorded audio.</li>
          <li>Dictated transcripts.</li>
          <li>Custom vocabulary values.</li>
          <li>Clipboard content.</li>
        </ul>
        <SettingsStatusMessage title="Diagnostics export unavailable">
          Diagnostic collection and export are not connected in this
          development preview. No diagnostic report can be generated from this
          page.
        </SettingsStatusMessage>
      </section>

      <section
        aria-labelledby="about-updates-heading"
        className="about-diagnostics-section privacy-overview-section"
      >
        <h3 id="about-updates-heading">Application updates</h3>
        <p>
          The planned update check requires an explicit user action and may
          connect to the vendor&apos;s release host. Updates must use verified,
          signed full-application packages that include the bundled speech
          model. Ordinary offline dictation must not require an update check,
          and the update process must provide a safe failure or recovery path.
        </p>
        <p>Update checking is not implemented in this development preview.</p>
      </section>
    </div>
  );
}
