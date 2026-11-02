# Task 37 — Clear vocabulary draft

## Scope and result

- **Goal:** Clear the volatile custom vocabulary draft and its synthetic preview from Settings → Text & Vocabulary.
- **Requirements:** F-08, UF-06; AC-08, AC-11, T-038, T-039, and T-053 remain incomplete.
- **Branch/base:** `codex/task23-moonshine-build-scope`, base `a8b92fc4cc683abce23bcb528d62e0ce8457a5b1`; result is uncommitted.
- **Platform:** macOS 26.6.2, arm64; Node 22.23.3, Rust stable 1.99.1, Python 3.13.13. RAM not measured; it is not relevant to this frontend-only change.
- The worktree was already dirty before Task 37. Existing work was preserved; no files were staged or committed.

## Files created or modified for this task

- `src/app/VocabularyEntryForm.tsx` — adds a controlled clear callback, native disabled Clear draft button, touched-state reset, and focus return to Spoken phrase.
- `src/app/App.tsx` — resets the existing controlled draft to `{ spoken: "", written: "" }`; the same object remains the source for the preview.
- `src/app/VocabularyEntryForm.test.tsx` — adds 10 focused clear-action tests.
- `src/app/App.test.tsx` — adds 2 integration tests for preview, cleanup mode, Launch at Login, section selection, navigation, and immediate re-entry.
- `src/styles/global.css` — uses the existing neutral outlined navigation button style and adds a visible native-disabled state.
- `README.md` — documents the feature, verification steps, and troubleshooting.
- `evidence/frontend-vocabulary-clear-task-37.md` — this record.

No dependency or lockfile changes were made. The Clear draft callback only updates React state. This frontend preview form has no connection to settings storage, so the action does not call a backend, access storage, log vocabulary, or delete saved vocabulary. It clears only this unsaved in-memory draft.

## Automated verification

All requested commands passed:

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run src/app/VocabularyEntryForm.test.tsx src/app/App.test.tsx` | PASS — 68 tests across 2 files |
| `npm run test -- --run` | PASS — 109 tests across 8 files; 97 existing plus 12 new |
| `npm run lint` | PASS |
| `npm run build` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |

## Visual and interaction verification

Ran `npm run dev` (Vite selected `http://localhost:5174/` because port 5173 was already occupied) and verified the live preview in Settings → Text & Vocabulary.

- Clear draft starts disabled, enables when either field contains text, and remains enabled for invalid content.
- A valid pair displays the existing synthetic example. Enter, Space, and pointer activation clear both fields and restore the preview's empty guidance.
- A 129-character value shows its existing validation error; clearing removes both field errors, resets the untouched presentation, disables Clear draft, and focuses Spoken phrase.
- A new draft can be entered immediately after clearing.
- At 320px and 900px viewport widths, `document.documentElement.scrollWidth` equaled `innerWidth` (320 and 900 respectively); no horizontal overflow was observed.
- Verbatim remained selected after clearing and navigating. Launch at Login remained On across section and Home/Settings navigation. All six Settings tabs remained available, and Home/Settings navigation worked.
- Browser console warnings/errors: none observed.
- Literal 200% browser zoom: **NOT RUN**; this browser setup exposed viewport sizing but no working zoom control.
- VoiceOver/screen-reader session: **NOT RUN**. The accessibility tree exposed the native button name, disabled state, form labels, and validation alerts.

## Remaining scope

AC-08, AC-11, T-038, T-039, and T-053 remain incomplete. This change does not implement vocabulary CRUD, saved vocabulary, deletion, export, or replacement processing.
