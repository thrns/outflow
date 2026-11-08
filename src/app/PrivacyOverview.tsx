import { SettingsStatusMessage } from "./SettingsStatusMessage";

type PrivacyOverviewProps = {
  /** Adds setup-specific wording while leaving Settings → Privacy unchanged. */
  setupDisclosure?: boolean;
};

/**
 * Presents Outflow's privacy requirements and the limits of the current UI preview.
 * This component is informational only; it does not inspect or change app data.
 */
export default function PrivacyOverview({
  setupDisclosure = false,
}: PrivacyOverviewProps) {
  return (
    <div className="privacy-overview">
      <SettingsStatusMessage title="Privacy implementation in progress">
        {setupDisclosure ? (
          <>
            This page describes Outflow&apos;s privacy requirements. Native
            recording, complete offline operation, clipboard integration,
            network isolation, and privacy safeguards have not yet been fully
            implemented and verified. This browser preview does not implement
            microphone recording or prove those privacy guarantees.
          </>
        ) : (
          <>
            This page describes Outflow&apos;s privacy requirements. Native
            recording, offline speech recognition, clipboard handling, network
            isolation, and data-retention safeguards have not yet been fully
            implemented and verified. This frontend preview does not implement
            microphone recording.
          </>
        )}
      </SettingsStatusMessage>

      <div className="privacy-overview-sections">
        <section
          aria-labelledby="privacy-offline-heading"
          className="privacy-overview-section"
        >
          <h3 id="privacy-offline-heading">Offline processing</h3>
          <p>
            Outflow is designed to run speech recognition and text cleanup
            locally. The finished application is intended to work without an
            Internet connection, an account, or cloud inference. Its speech
            model is intended to ship with the installer instead of being
            downloaded during ordinary use.
          </p>
          <p>
            These are product requirements, not verified behavior in this
            frontend preview. Speech recognition and offline operation have not
            been delivered here.
          </p>
          {setupDisclosure ? (
            <p>
              Dictated speech is not intended to be sent to a cloud
              transcription service.
            </p>
          ) : null}
        </section>

        <section
          aria-labelledby="privacy-retention-heading"
          className="privacy-overview-section"
        >
          <h3 id="privacy-retention-heading">
            What Outflow is designed not to retain
          </h3>
          <p>The product requirements prohibit retaining or tracking:</p>
          <ul>
            <li>Saved audio recordings.</li>
            <li>Transcript history or persistent transcript caches.</li>
            <li>Usage tracking or telemetry.</li>
            <li>Logs or crash reports containing dictated content.</li>
          </ul>
          <p>
            The native recording, recognition, logging, and crash-reporting
            paths have not yet been fully implemented and privacy-tested.
          </p>
        </section>

        <section
          aria-labelledby="privacy-storage-heading"
          className="privacy-overview-section"
        >
          <h3 id="privacy-storage-heading">What may be stored</h3>
          <p>
            Application settings may be stored locally. User-created vocabulary
            entries may eventually be stored locally when that feature is
            implemented. Recording audio and transcript history must not be
            stored.
          </p>
          <p>
            In this preview, Launch at Login and Clean/Verbatim are in-memory
            settings, and the vocabulary draft is not persisted. Refreshing the
            page resets these preview values. They are not connected to
            settings storage or dictation.
          </p>
        </section>

        <section
          aria-labelledby="privacy-boundaries-heading"
          className="privacy-overview-section"
        >
          <h3 id="privacy-boundaries-heading">
            Clipboard, operating system, and updates
          </h3>
          <p>
            Clipboard copying is intended to happen only after an explicit user
            action. The destination application, operating system swap, and
            clipboard managers may retain content outside Outflow&apos;s control.
            Outflow cannot guarantee cryptographic erasure of data from system
            memory.
          </p>
          {setupDisclosure ? (
            <p>
              If you explicitly choose Copy Result, the intended behavior is to
              place that text on the system clipboard. The operating system and
              clipboard managers may retain it. Outflow cannot promise
              cryptographic erasure from memory or operating-system swap.
            </p>
          ) : null}
          <p>
            The intended update mechanism is an optional, explicit,
            user-initiated check for signed full-application updates. Normal
            dictation must not require update checks or runtime model downloads.
            Native clipboard behavior and signed updates are not implemented in
            this preview.
          </p>
          {setupDisclosure ? (
            <p>
              Normal dictation must not require Internet access. A future signed
              full-app update check is separate and may connect only after the
              user initiates it.
            </p>
          ) : null}
        </section>
      </div>
    </div>
  );
}
