import { useEffect, useId, useRef, useState, type FormEvent } from "react";
import { MAX_VOCABULARY_PREVIEW_ENTRIES } from "./VocabularyEntryForm";
import {
  getVocabularyValidationMessage,
  validateVocabularyValue,
  type VocabularyDraft,
} from "./VocabularyEntryForm";

export type VocabularyPreviewEntry = {
  id: number;
  spoken: string;
  written: string;
};

type VocabularyPreviewListProps = {
  entries: VocabularyPreviewEntry[];
  onRemove: (id: number) => void;
  onSave: (id: number, value: VocabularyDraft) => void;
};

type VocabularyEntryEditorProps = {
  entry: VocabularyPreviewEntry;
  onSave: (value: VocabularyDraft) => void;
  onCancel: () => void;
};

/** Edits one preview entry in memory and focuses its first field on open. */
function VocabularyEntryEditor({
  entry,
  onSave,
  onCancel,
}: VocabularyEntryEditorProps) {
  const idPrefix = useId();
  const spokenInput = useRef<HTMLInputElement>(null);
  const writtenInput = useRef<HTMLInputElement>(null);
  const [value, setValue] = useState<VocabularyDraft>({
    spoken: entry.spoken,
    written: entry.written,
  });
  const [touched, setTouched] = useState({ spoken: false, written: false });
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
  const spokenId = `${idPrefix}-edit-spoken`;
  const spokenDescriptionId = `${spokenId}-description`;
  const spokenErrorId = `${spokenId}-error`;
  const writtenId = `${idPrefix}-edit-written`;
  const writtenDescriptionId = `${writtenId}-description`;
  const writtenErrorId = `${writtenId}-error`;

  useEffect(() => {
    spokenInput.current?.focus();
  }, []);

  function saveChanges(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setTouched({ spoken: true, written: true });

    if (validateVocabularyValue(value.spoken)) {
      spokenInput.current?.focus();
      return;
    }
    if (validateVocabularyValue(value.written)) {
      writtenInput.current?.focus();
      return;
    }

    onSave(value);
  }

  return (
    <form
      aria-label={`Edit vocabulary entry: ${entry.spoken}`}
      className="vocabulary-entry-form vocabulary-preview-entry-editor"
      noValidate
      onSubmit={saveChanges}
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
          onBlur={() =>
            setTouched((current) => ({ ...current, spoken: true }))
          }
          onChange={(event) => {
            const spoken = event.currentTarget.value;
            setValue((current) => ({
              ...current,
              spoken,
            }));
          }}
          placeholder="north parkade"
          ref={spokenInput}
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
          onBlur={() =>
            setTouched((current) => ({ ...current, written: true }))
          }
          onChange={(event) => {
            const written = event.currentTarget.value;
            setValue((current) => ({
              ...current,
              written,
            }));
          }}
          placeholder="North Parkade"
          ref={writtenInput}
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
          className="home-open-settings-button vocabulary-save-button"
          type="submit"
        >
          Save changes
        </button>
        <button
          className="app-navigation-button vocabulary-cancel-edit-button"
          onClick={onCancel}
          type="button"
        >
          Cancel editing
        </button>
      </div>
    </form>
  );
}

/** Displays the page-session vocabulary preview without persisting or applying entries. */
export default function VocabularyPreviewList({
  entries,
  onRemove,
  onSave,
}: VocabularyPreviewListProps) {
  const listId = useId();
  const [editingEntryId, setEditingEntryId] = useState<number | null>(null);
  const editButtonRefs = useRef(new Map<number, HTMLButtonElement>());
  const focusAfterEditRef = useRef<number | null>(null);
  const count = entries.length;
  const editingMessageId = `${listId}-editing-message`;

  useEffect(() => {
    if (editingEntryId !== null || focusAfterEditRef.current === null) return;

    const entryId = focusAfterEditRef.current;
    focusAfterEditRef.current = null;
    editButtonRefs.current.get(entryId)?.focus();
  }, [editingEntryId]);

  function finishEditing(entryId: number) {
    focusAfterEditRef.current = entryId;
    setEditingEntryId(null);
  }

  return (
    <section
      aria-labelledby="vocabulary-preview-list-heading"
      className="vocabulary-preview-list"
    >
      <div className="vocabulary-preview-list-header">
        <h4 id="vocabulary-preview-list-heading">Vocabulary preview list</h4>
        <p aria-live="polite" className="vocabulary-preview-count">
          {count} of {MAX_VOCABULARY_PREVIEW_ENTRIES} entries
        </p>
      </div>

      {count === 0 ? (
        <p className="vocabulary-preview-empty" role="status">
          No vocabulary entries yet in this development preview. Add a valid
          pair above to see it here.
        </p>
      ) : (
        <>
          {editingEntryId !== null ? (
            <p
              className="vocabulary-preview-editing-note"
              id={editingMessageId}
              role="status"
            >
              Save or cancel the current edit before editing another entry.
            </p>
          ) : null}
          <ul
            aria-label="Vocabulary entries in preview"
            className="vocabulary-preview-entries"
          >
            {entries.map((entry, index) => {
              const isEditing = editingEntryId === entry.id;

              return (
                <li className="vocabulary-preview-entry" key={entry.id}>
                  {isEditing ? (
                    <VocabularyEntryEditor
                      entry={entry}
                      onCancel={() => finishEditing(entry.id)}
                      onSave={(value) => {
                        onSave(entry.id, value);
                        finishEditing(entry.id);
                      }}
                    />
                  ) : (
                    <>
                      <div className="vocabulary-preview-entry-values">
                        <p>
                          <span className="vocabulary-preview-entry-label">
                            Spoken phrase
                          </span>
                          <span className="vocabulary-preview-entry-value">
                            {entry.spoken}
                          </span>
                        </p>
                        <p>
                          <span className="vocabulary-preview-entry-label">
                            Written replacement
                          </span>
                          <span className="vocabulary-preview-entry-value">
                            {entry.written}
                          </span>
                        </p>
                      </div>
                      <div className="vocabulary-preview-entry-actions">
                        <button
                          aria-describedby={
                            editingEntryId !== null ? editingMessageId : undefined
                          }
                          aria-label={`Edit vocabulary entry ${index + 1}: ${entry.spoken}`}
                          className="app-navigation-button vocabulary-edit-button"
                          disabled={editingEntryId !== null}
                          onClick={() => setEditingEntryId(entry.id)}
                          ref={(element) => {
                            if (element) {
                              editButtonRefs.current.set(entry.id, element);
                            } else {
                              editButtonRefs.current.delete(entry.id);
                            }
                          }}
                          type="button"
                        >
                          Edit
                        </button>
                        <button
                          aria-label={`Remove vocabulary entry ${index + 1}: ${entry.spoken}`}
                          className="app-navigation-button vocabulary-remove-button"
                          onClick={() => onRemove(entry.id)}
                          type="button"
                        >
                          Remove
                        </button>
                      </div>
                    </>
                  )}
                </li>
              );
            })}
          </ul>
        </>
      )}
    </section>
  );
}
