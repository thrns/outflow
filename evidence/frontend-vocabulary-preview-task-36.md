# Task 36 — Live vocabulary draft preview

```yaml
task_id: frontend-task-36
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08, AC-11]
owner: /root on the existing worktree
branch: codex/task23-moonshine-build-scope
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: uncommitted
platform: macOS 26.6.2 arm64; physical RAM not measured
review: PENDING
```

## Implementation

Added `src/app/VocabularyDraftPreview.tsx` and its tests. The component accepts the existing `VocabularyDraft` as a controlled `value` prop, has no draft state or side effects, and reuses `validateVocabularyValue` from `VocabularyEntryForm`. It composes two fixed synthetic sentences directly from the spoken and written fields. React renders each value as text; no transcript search, replacement logic, normalization, storage, backend call, or network request is involved.

The empty draft shows a neutral prompt. A missing value asks for both fields, while whitespace-only, overlength, and invalid-Unicode drafts hide the complete example until corrected. The preview shows a static explanation that the entry is not saved and speech or replacement is not performed. `App` passes its existing draft state, preserving section and Home/Settings navigation behavior and the fresh-mount/refresh reset. Clean/Verbatim and Launch at Login remain unchanged.

## Files changed for this request

- `src/app/VocabularyDraftPreview.tsx` — controlled synthetic preview and limitation copy.
- `src/app/VocabularyDraftPreview.test.tsx` — 14 tests covering empty/partial/valid/invalid values, updates, Unicode, literal markup, and absence of Save/Apply controls.
- `src/app/App.tsx` — inserts the preview after the existing form and before its empty/preview explanation.
- `src/app/App.test.tsx` — preview placement, updates, validation, navigation preservation, and refresh reset assertions; existing cleanup-mode, Launch at Login, and six-section coverage remains.
- `src/styles/global.css` — neutral bordered preview surface, labels, spacing, and wrapping.
- `README.md` — Task 36 behavior, run/test, manual checks, and limitations.
- `evidence/frontend-vocabulary-preview-task-36.md` — this record.

`README.md`, `src/app/App.tsx`, `src/app/App.test.tsx`, and `src/styles/global.css` already had uncommitted changes before this task; those changes were preserved. No dependency, lockfile, Rust, or centralized `docs/TASKS.md` changes were made.

## Verification

Environment: macOS 26.6.2 arm64, Node `v22.23.3`, npm `10.9.9`, Rust/Cargo `1.99.1`, Python `3.13.13`.

| Command or check | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 97 tests across 8 files; 82 existing frontend tests remain, with 15 added |
| `npm run lint` | PASS |
| `npm run build` | PASS — Vite production build completed |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |
| `/Users/bixxie/.agents/skills/impeccable/scripts/impeccable detect --json src/app/VocabularyDraftPreview.tsx src/app/App.tsx src/styles/global.css` | PASS — no findings |
| `git diff --check` | PASS after implementation and documentation updates |

## Manual visual and interaction checks

- Started Vite with `npm run dev -- --host 127.0.0.1`; port 5173 was occupied, so the preview opened at `http://127.0.0.1:5174/`.
- Inspected Settings → Text & Vocabulary at 900px and 320px. The preview panel fits the narrow layout and sentence/disclaimer text wraps without visible horizontal overflow.
- Confirmed the empty state, partial-field message, valid `north parkade` → `North Parkade` synthetic pair, immediate field updates, invalid whitespace hiding the example, and correction restoring it.
- Confirmed emoji, combining marks, and HTML-like values display literally; no Save or Apply control is present.
- Changed the selected mode to Verbatim, edited the draft, and confirmed the mode stayed selected. Launch at Login toggled on and off. General, Microphone, Shortcuts, Text & Vocabulary, Privacy, and About & Diagnostics all remained reachable.
- Confirmed the draft and example survive Settings-section and Home/Settings navigation. After refresh both fields were empty and the empty preview returned.
- Used ArrowUp to move between Settings tabs; visible focus appeared on the selected tab. The preview itself adds no keyboard stops or live-region semantics.
- Browser console error log: empty.
- Literal 200% browser zoom: **NOT RUN**.
- Screen-reader testing: **NOT RUN**.

## Scope and remaining work

Only synthetic test phrases were entered during manual review. Draft values remain in React memory for the current mount; no user vocabulary or transcript is persisted or logged by this feature. The example is synthetic and display-only; it does not implement vocabulary CRUD, word-boundary matching, speech recognition, or transcript text replacement. Resource gates were not measured. AC-08, AC-11, T-038, T-039, and T-053 remain incomplete.
