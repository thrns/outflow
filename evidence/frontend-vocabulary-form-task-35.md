# Task 35 — Custom vocabulary entry form preview

```yaml
task_id: frontend-task-35
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08, AC-11]
owner: /root on the existing worktree
branch: codex/task23-moonshine-build-scope
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: uncommitted
platform: macOS 26.6.2 arm64; model and RAM were not independently observable in this task environment
review: PENDING
```

## Implementation

Added `src/app/VocabularyEntryForm.tsx` with controlled `VocabularyDraft` props, native labeled text inputs, helper descriptions, and blur-triggered inline validation. The validator counts values with JavaScript code-point iteration, rejects lone surrogate code points, checks empty and whitespace-only content, and enforces a 128 Unicode scalar value maximum. It leaves capitalization, spacing, combining sequences, and other valid input unchanged.

`src/app/App.tsx` owns the draft in React state. Section changes and Home/Settings navigation preserve it; a fresh mount or browser refresh resets it. The Text & Vocabulary panel retains Clean/Verbatim and shows the preview-only explanation. No vocabulary data is persisted or applied to dictated text.

## Verification

| Command or check | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 82 tests across 7 files; all 62 baseline cases pass (two placeholder assertions updated), with 20 added |
| `npm run lint` | PASS |
| `npm run build` | PASS — Vite production build completed |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |
| `git diff --check` | PASS |

The frontend suite covers accessible names and descriptions, empty initial values, controlled typing, preserved spaces/capitalization, valid accented text, whitespace rejection, 128/129 ASCII and emoji boundaries, combining scalar counts, unpaired surrogates, linked inline errors, error clearing, length/Unicode error text, and the absence of Save/Delete/Import actions. App integration tests cover the new form, draft preservation across both navigation paths, and fresh-mount reset. Existing Clean/Verbatim, Launch at Login, and six-section coverage remains passing. Two preexisting assertions that expected the old “controls will be added later” placeholder were updated to assert the new form.

## Manual visual checks

- Started Vite with `npm run dev -- --host 127.0.0.1` and opened `http://127.0.0.1:5173/`.
- Inspected the Text & Vocabulary panel at 320px and 900px. At both sizes the document width matched the viewport (320px and 900px); no horizontal overflow was observed. Fields and long helper/error text wrap within the panel.
- Typed a spoken phrase and pasted a written replacement. Blurring whitespace-only content displayed its inline error; correcting it cleared the error. Keyboard focus was visible on the written field.
- Confirmed values survive Settings-section changes and Home/Settings navigation, then refreshed and confirmed both fields were empty.
- Confirmed Clean/Verbatim switching, all six Settings tabs, and Launch at Login on/off preview behavior.
- Browser console warnings/errors: none observed.
- Screen-reader testing: NOT RUN. Literal 200% browser zoom: NOT RUN; viewport reflow checks are not a substitute.

## Scope and remaining work

This preview does not save, edit, delete, import, export, or process vocabulary. It does not implement duplicate detection, the 500-entry collection cap, word-boundary behavior, or native integration. It does not store transcripts or recording data. No dependency, lockfile, Rust, or `docs/TASKS.md` changes were made. AC-08, AC-11, T-038, T-039, and T-053 remain incomplete.
