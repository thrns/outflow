# Frontend Task 30 — Reusable Settings row

```yaml
task_id: Task 30 (frontend SettingsRow extraction)
feature_ids: [F-11]
flows: [UF-09]
acceptance: [AC-11]
owner: Codex in the existing codex/task23-moonshine-build-scope worktree
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: UNCOMMITTED
platform: macOS 26.6.2 arm64; Node.js 22.23.3; npm 10.9.3; Rust 1.99.1 stable; Python 3.13.13
implementation: Extracted the Launch at Login copy and row layout into src/app/SettingsRow.tsx; its controlled preview state remains in App.tsx.
metrics: NOT_MEASURED (resource/release gates are outside this frontend-only task)
doc_updates: README.md Task 30 section; evidence/frontend-settings-row-task-30.md
risks: Narrow viewport, literal 200% zoom, and browser console inspection were unavailable in the in-app browser. AC-11 remains incomplete.
review: PENDING
```

## Implementation

- Added `src/app/SettingsRow.tsx` with a typed title, required title ID, optional description and description ID, and `ReactNode` control children.
- The row owns the visible title/description structure and layout classes. It exposes IDs supplied by the caller so non-labelable controls can reference the visible copy with ARIA; it does not wrap controls in `<label>` or own setting state.
- General Settings now uses `SettingsRow` for Launch at Login. The existing title, description, state, switch, preview note, control appearance, keyboard handler, and ARIA name/description references are unchanged. `App.tsx` still owns the preview boolean, which survives Home/Settings and Settings-tab navigation and resets after refresh.
- Moved the existing row CSS selectors to reusable `.settings-row`, `.settings-row-copy`, `.settings-row-title`, and `.settings-row-description` names without changing their declarations or tokens. The existing `.settings-field-control` remains the control layout.
- Added six focused tests in `src/app/SettingsRow.test.tsx` for title, optional description, children, omission, group/control semantics, and explicit accessible name/description association.
- Updated the README frontend guidance with component purpose, state and accessibility ownership, run/test/visual-check instructions, and troubleshooting.
- No dependencies, lockfiles, Rust Settings code, or AC-11 task status were changed.

## Verification

Frontend commands, from the repository root:

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 34 tests total (28 existing + 6 new) |
| `npm run lint` | PASS |
| `npm run build` | PASS |

Rust commands, using the installed stable toolchain:

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 72 tests (2 core + 70 settings) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |

Python command:

| Command | Result |
|---|---|
| `python3 -m unittest discover -s scripts/tests -v` | PASS; 226 tests |

`git diff --check` passed. The Impeccable detector returned no findings for the changed UI targets.

## Visual and interaction checks

Started the preview with `npm run dev` at `http://localhost:5173/` and inspected the in-app browser at 899 × 675 px (approximately the requested 900 px width).

- General Settings showed the existing title, description, Off state, and preview note with the row aligned as before; the capture showed no horizontal overflow at the available width.
- Mouse click changed the switch Off → On. Space changed it On → Off while focused. The keyboard-focused switch showed the existing visible outline.
- The value remained On across Home → Settings navigation and switching away from General and back. After a full page reload it returned to Off.
- The accessibility tree exposed all six Settings tabs; Settings section navigation reached Text & Vocabulary and About & Diagnostics. Existing integration tests cover the remaining sections and navigation behaviors.
- Browser console errors: NOT RUN; the in-app browser surface did not expose a console log view.
- 320 px viewport: NOT RUN; no viewport-size override was available in the selected in-app browser.
- Literal 200% browser zoom: NOT RUN; this was not available through the selected browser controls. A narrow-width check is not substituted for zoom.

The visual check used the active local preview and did not retain a screenshot artifact. Automated DOM/component tests do not replace native screen-reader or platform accessibility testing.

## Limitations and handoff

This task verifies the frontend preview only. Native Settings integration, platform screen-reader testing, 320 px reflow, and 200% browser zoom remain unverified. **AC-11 remains incomplete**; this evidence does not change `docs/TASKS.md` status.

The working tree already contained unrelated uncommitted changes before this task began. They were preserved. Task 30 contributions are limited to the SettingsRow extraction, focused tests, README addition, and this evidence file. Nothing was staged or committed.
