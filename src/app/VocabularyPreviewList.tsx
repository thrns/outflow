import { MAX_VOCABULARY_PREVIEW_ENTRIES } from "./VocabularyEntryForm";

export type VocabularyPreviewEntry = {
  id: number;
  spoken: string;
  written: string;
};

type VocabularyPreviewListProps = {
  entries: VocabularyPreviewEntry[];
  onRemove: (id: number) => void;
};

/** Displays the page-session vocabulary preview without persisting or applying entries. */
export default function VocabularyPreviewList({
  entries,
  onRemove,
}: VocabularyPreviewListProps) {
  const count = entries.length;

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
        <ul
          aria-label="Vocabulary entries in preview"
          className="vocabulary-preview-entries"
        >
          {entries.map((entry, index) => (
            <li className="vocabulary-preview-entry" key={entry.id}>
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
              <button
                aria-label={`Remove vocabulary entry ${index + 1}: ${entry.spoken}`}
                className="app-navigation-button vocabulary-remove-button"
                onClick={() => onRemove(entry.id)}
                type="button"
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
