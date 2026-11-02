export type CleanupMode = "clean" | "verbatim";

type TextCleanupModeSelectorProps = {
  value: CleanupMode;
  onChange: (mode: CleanupMode) => void;
};

const cleanDescriptionId = "text-cleanup-clean-description";
const cleanTitleId = "text-cleanup-clean-title";
const verbatimDescriptionId = "text-cleanup-verbatim-description";
const verbatimTitleId = "text-cleanup-verbatim-title";
const previewNoteId = "text-cleanup-mode-preview";

/**
 * Lets the user preview the planned cleanup preference without connecting it
 * to the speech or text-processing pipeline.
 */
export default function TextCleanupModeSelector({
  value,
  onChange,
}: TextCleanupModeSelectorProps) {
  return (
    <div className="text-cleanup-mode">
      <fieldset
        aria-describedby={previewNoteId}
        className="text-cleanup-mode-fieldset"
      >
        <legend className="text-cleanup-mode-legend">Text cleanup mode</legend>
        <div className="text-cleanup-mode-options">
          <label
            className={`text-cleanup-mode-option${
              value === "clean" ? " is-selected" : ""
            }`}
            htmlFor="text-cleanup-mode-clean"
          >
            <input
              aria-describedby={`${cleanDescriptionId} ${previewNoteId}`}
              aria-labelledby={cleanTitleId}
              checked={value === "clean"}
              id="text-cleanup-mode-clean"
              name="text-cleanup-mode"
              onChange={() => onChange("clean")}
              type="radio"
              value="clean"
            />
            <span className="text-cleanup-mode-option-copy">
              <span className="text-cleanup-mode-option-title" id={cleanTitleId}>
                Clean
              </span>
              <span
                className="text-cleanup-mode-option-description"
                id={cleanDescriptionId}
              >
                Intended to remove supported filler words and apply deterministic
                text cleanup.
              </span>
            </span>
          </label>
          <label
            className={`text-cleanup-mode-option${
              value === "verbatim" ? " is-selected" : ""
            }`}
            htmlFor="text-cleanup-mode-verbatim"
          >
            <input
              aria-describedby={`${verbatimDescriptionId} ${previewNoteId}`}
              aria-labelledby={verbatimTitleId}
              checked={value === "verbatim"}
              id="text-cleanup-mode-verbatim"
              name="text-cleanup-mode"
              onChange={() => onChange("verbatim")}
              type="radio"
              value="verbatim"
            />
            <span className="text-cleanup-mode-option-copy">
              <span
                className="text-cleanup-mode-option-title"
                id={verbatimTitleId}
              >
                Verbatim
              </span>
              <span
                className="text-cleanup-mode-option-description"
                id={verbatimDescriptionId}
              >
                Intended to preserve filler words rather than removing them.
              </span>
            </span>
          </label>
        </div>
      </fieldset>
      <p
        className="settings-preview-note text-cleanup-mode-preview"
        id={previewNoteId}
        role="note"
      >
        Mode selection is available in this development preview. It is not yet
        connected to dictation or text processing.
      </p>
    </div>
  );
}
