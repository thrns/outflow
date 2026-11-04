import { useId, useRef, useState, type FormEvent } from "react";

export const MAX_VOCABULARY_PREVIEW_ENTRIES = 500;

export type VocabularyDraft = {
  spoken: string;
  written: string;
};

type VocabularyEntryFormProps = {
  value: VocabularyDraft;
  onChange: (value: VocabularyDraft) => void;
  onClearDraft: () => void;
  onAddToPreviewList: (value: VocabularyDraft) => void;
  isAtLimit: boolean;
};

export type VocabularyValidationError =
  | "required"
  | "whitespace"
  | "too-long"
  | "invalid-unicode";

const maximumScalarValues = 128;

/**
 * Validates one entry field without normalizing or changing its value.
 * Iteration counts Unicode scalar values and exposes lone UTF-16 surrogates,
 * which JavaScript's string iterator yields as individual code points.
 */
export function validateVocabularyValue(
  value: string,
): VocabularyValidationError | null {
  if (value.length === 0) return "required";
  if (value.trim().length === 0) return "whitespace";

  let scalarCount = 0;
  let containsUnpairedSurrogate = false;

  for (const character of value) {
    const codePoint = character.codePointAt(0);

    if (
      codePoint !== undefined &&
      codePoint >= 0xd800 &&
      codePoint <= 0xdfff
    ) {
      containsUnpairedSurrogate = true;
    }

    scalarCount += 1;
  }

  if (containsUnpairedSurrogate) return "invalid-unicode";
  if (scalarCount > maximumScalarValues) return "too-long";

  return null;
}

export function getVocabularyValidationMessage(
  error: VocabularyValidationError,
  fieldLabel: string,
): string {
  switch (error) {
    case "required":
      return fieldLabel === "Spoken phrase"
        ? "Enter a spoken phrase."
        : "Enter a written replacement.";
    case "whitespace":
      return "Enter at least one non-whitespace character.";
    case "too-long":
      return `Use ${maximumScalarValues} Unicode scalar values or fewer.`;
    case "invalid-unicode":
      return "This value contains invalid Unicode text.";
  }
}

/**
 * Edits an in-memory vocabulary draft. It validates text locally and has no
 * persistence or text-processing side effects.
 */
export default function VocabularyEntryForm({
  value,
  onChange,
  onClearDraft,
  onAddToPreviewList,
  isAtLimit,
}: VocabularyEntryFormProps) {
  const idPrefix = useId();
  const spokenInput = useRef<HTMLInputElement>(null);
  const writtenInput = useRef<HTMLInputElement>(null);
  const [touched, setTouched] = useState({ spoken: false, written: false });
  const spokenId = `${idPrefix}-spoken`;
  const spokenDescriptionId = `${spokenId}-description`;
  const spokenErrorId = `${spokenId}-error`;
  const writtenId = `${idPrefix}-written`;
  const writtenDescriptionId = `${writtenId}-description`;
  const writtenErrorId = `${writtenId}-error`;
  const spokenValidation = touched.spoken
    ? validateVocabularyValue(value.spoken)
    : null;
  const writtenValidation = touched.written
    ? validateVocabularyValue(value.written)
    : null;
  const spokenError = spokenValidation
    ? getVocabularyValidationMessage(spokenValidation, "Spoken phrase")
    : null;
  const writtenError = writtenValidation
    ? getVocabularyValidationMessage(writtenValidation, "Written replacement")
    : null;
  const limitMessageId = `${idPrefix}-limit-message`;

  function clearDraft() {
    setTouched({ spoken: false, written: false });
    onClearDraft();
    // Focus the stable input before the empty draft disables the clear button.
    spokenInput.current?.focus();
  }

  function addToPreviewList(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setTouched({ spoken: true, written: true });

    const spokenIssue = validateVocabularyValue(value.spoken);
    const writtenIssue = validateVocabularyValue(value.written);

    if (isAtLimit) return;
    if (spokenIssue) {
      spokenInput.current?.focus();
      return;
    }
    if (writtenIssue) {
      writtenInput.current?.focus();
      return;
    }

    onAddToPreviewList(value);
    setTouched({ spoken: false, written: false });
  }

  return (
    <form
      className="vocabulary-entry-form"
      noValidate
      onSubmit={addToPreviewList}
    >
      <div className="vocabulary-field">
        <label className="vocabulary-label" htmlFor={spokenId}>
          Spoken phrase
        </label>
        <input
          aria-describedby={
            spokenError
              ? `${spokenDescriptionId} ${spokenErrorId}`
              : spokenDescriptionId
          }
          aria-invalid={spokenError ? true : undefined}
          className="vocabulary-input"
          id={spokenId}
          ref={spokenInput}
          onBlur={() =>
            setTouched((current) => ({ ...current, spoken: true }))
          }
          onChange={(event) =>
            onChange({ ...value, spoken: event.currentTarget.value })
          }
          placeholder="north parkade"
          type="text"
          value={value.spoken}
        />
        <p className="vocabulary-field-description" id={spokenDescriptionId}>
          The word or phrase you expect to dictate. Maximum 128 Unicode scalar
          values.
        </p>
        {spokenError ? (
          <p className="vocabulary-field-error" id={spokenErrorId} role="alert">
            {spokenError}
          </p>
        ) : null}
      </div>

      <div className="vocabulary-field">
        <label className="vocabulary-label" htmlFor={writtenId}>
          Written replacement
        </label>
        <input
          aria-describedby={
            writtenError
              ? `${writtenDescriptionId} ${writtenErrorId}`
              : writtenDescriptionId
          }
          aria-invalid={writtenError ? true : undefined}
          className="vocabulary-input"
          id={writtenId}
          ref={writtenInput}
          onBlur={() =>
            setTouched((current) => ({ ...current, written: true }))
          }
          onChange={(event) =>
            onChange({ ...value, written: event.currentTarget.value })
          }
          placeholder="North Parkade"
          type="text"
          value={value.written}
        />
        <p className="vocabulary-field-description" id={writtenDescriptionId}>
          The text that should eventually appear in finalized output. Maximum
          128 Unicode scalar values.
        </p>
        {writtenError ? (
          <p className="vocabulary-field-error" id={writtenErrorId} role="alert">
            {writtenError}
          </p>
        ) : null}
      </div>

      <div className="vocabulary-entry-actions">
        <button
          aria-describedby={isAtLimit ? limitMessageId : undefined}
          className="home-open-settings-button vocabulary-add-button"
          disabled={isAtLimit}
          type="submit"
        >
          Add to preview list
        </button>
        <button
          className="app-navigation-button vocabulary-clear-button"
          disabled={value.spoken.length === 0 && value.written.length === 0}
          onClick={clearDraft}
          type="button"
        >
          Clear draft
        </button>
      </div>
      {isAtLimit ? (
        <p
          className="vocabulary-entry-limit-message"
          id={limitMessageId}
          role="status"
        >
          The preview list is full at {MAX_VOCABULARY_PREVIEW_ENTRIES} entries.
          Remove an entry before adding another.
        </p>
      ) : null}
    </form>
  );
}
