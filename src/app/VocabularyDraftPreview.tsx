import {
  validateVocabularyValue,
  type VocabularyDraft,
} from "./VocabularyEntryForm";

type VocabularyDraftPreviewProps = {
  value: VocabularyDraft;
};

/**
 * Shows a synthetic, display-only illustration for the current in-memory
 * vocabulary draft. The fixed sentence is composed from the two validated
 * phrases; it does not inspect transcripts or perform replacement processing.
 */
export default function VocabularyDraftPreview({
  value,
}: VocabularyDraftPreviewProps) {
  const spokenValidation = validateVocabularyValue(value.spoken);
  const writtenValidation = validateVocabularyValue(value.written);
  const isEmpty =
    spokenValidation === "required" && writtenValidation === "required";
  const isValid = spokenValidation === null && writtenValidation === null;

  let message: string | null = null;

  if (isEmpty) {
    message =
      "Enter a spoken phrase and written replacement to see an illustrative example.";
  } else if (spokenValidation === "required" || writtenValidation === "required") {
    message =
      "Enter both a spoken phrase and written replacement to see the illustrative example.";
  } else if (!isValid) {
    message = "Correct both values to see the illustrative example.";
  }

  return (
    <>
      <div className="vocabulary-draft-preview">
        <h4 className="vocabulary-draft-preview-heading">
          Illustrative example
        </h4>

        {isValid ? (
          <div className="vocabulary-draft-preview-examples">
            <div>
              <p className="vocabulary-draft-preview-label">
                Synthetic spoken sentence
              </p>
              <p className="vocabulary-draft-preview-text">
                I mentioned “{value.spoken}” today.
              </p>
            </div>
            <div>
              <p className="vocabulary-draft-preview-label">
                Intended written sentence
              </p>
              <p className="vocabulary-draft-preview-text">
                I mentioned “{value.written}” today.
              </p>
            </div>
          </div>
        ) : (
          <p className="vocabulary-draft-preview-message">{message}</p>
        )}
      </div>

      <p className="vocabulary-draft-preview-limitation">
        This is a synthetic illustration of your draft. Entries in the
        development preview list are not persisted or applied to actual speech.
      </p>
    </>
  );
}
