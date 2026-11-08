import { useState } from "react";

const SYNTHETIC_EXAMPLE_TEXT = "So I met North Parkade at three.";

/**
 * A development-only, local presentation fixture. It has no connection to
 * recording, recognition, insertion, clipboard, persistence, or Rust sessions.
 */
export default function SyntheticDictationExample() {
  const [isVisible, setIsVisible] = useState(false);
  const [announcement, setAnnouncement] = useState("");

  function showExample() {
    if (isVisible) return;

    setIsVisible(true);
    setAnnouncement("Synthetic example is shown.");
  }

  function clearExample() {
    if (!isVisible) return;

    setIsVisible(false);
    setAnnouncement("Synthetic example cleared.");
  }

  return (
    <section
      aria-labelledby="synthetic-dictation-example-heading"
      className="synthetic-dictation-example"
    >
      <h2 id="synthetic-dictation-example-heading">Fixed text demonstration</h2>
      <p className="synthetic-example-disclaimer">
        <strong>
          Synthetic example — no speech was recorded or transcribed.
        </strong>
      </p>

      <div className="synthetic-example-actions">
        <button
          className="home-open-settings-button"
          onClick={showExample}
          type="button"
        >
          Show example text
        </button>
        <button
          className="app-navigation-button"
          onClick={clearExample}
          type="button"
        >
          Clear example
        </button>
      </div>

      {isVisible && (
        <div className="synthetic-example-result">
          <label htmlFor="synthetic-example-text">
            Example text (read-only, selectable)
          </label>
          <textarea
            aria-readonly="true"
            className="synthetic-example-text"
            id="synthetic-example-text"
            readOnly
            rows={2}
            spellCheck={false}
            value={SYNTHETIC_EXAMPLE_TEXT}
            wrap="soft"
          />
        </div>
      )}

      <p aria-atomic="true" aria-live="polite" className="synthetic-example-announcement" role="status">
        {announcement}
      </p>
    </section>
  );
}
