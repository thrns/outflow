# Frontend Task 34 — Text cleanup mode selector

## Scope and status

Implemented a frontend-only Clean / Verbatim selector in Settings → Text & Vocabulary. This request is the frontend Task 34 preview; it does not complete PRD acceptance criteria AC-07, AC-08, or AC-11, and does not mark project tasks T-035, T-038, or T-053 complete. The cleanup mode does not call backend commands or affect speech recognition, text processing, vocabulary storage, or persistence.

The mode is the `CleanupMode` union (`"clean" | "verbatim"`) in `src/app/TextCleanupModeSelector.tsx`, with React state owned by `App`. It starts as `"clean"`, survives section and Home/Settings navigation while the app remains mounted, and returns to Clean after a full page reload. Native labeled radios share one `name`; a fieldset and legend name the group, and each option exposes its intended behavior and preview limitation to assistive technology.

## Files changed for this request

- `src/app/App.tsx` — app-owned in-memory mode state and Text & Vocabulary selector composition; the section description remains intact.
- `src/app/TextCleanupModeSelector.tsx` — new controlled native radio group and explicit preview note.
- `src/styles/global.css` — compact option rows, semantic token colors, wrapping, selected state, and visible focus treatment.
- `src/app/App.test.tsx` — 10 focused integration tests; existing Launch at Login and six-section coverage retained.
- `README.md` — concise feature, behavior, test, manual-check, and limitation notes.
- `evidence/frontend-text-cleanup-mode-task-34.md` — this record.

`App.tsx`, `App.test.tsx`, `global.css`, and `README.md` already contained uncommitted changes before this request; those changes were preserved. The current branch is `codex/task23-moonshine-build-scope`; the pre-existing HEAD was `a8b92fc4cc683abce23bcb528d62e0ce8457a5b1`. No commit was created.

## Verification

Environment: macOS/Darwin arm64, Node `v22.23.3`, npm `10.9.9`, Rust/Cargo `1.99.1`. The local Vite preview ran at `http://localhost:5173/`.

| Command or check | Result |
|---|---|
| `npm run test -- --run src/app/App.test.tsx` | PASS — 35 App integration tests |
| Baseline `npm run test -- --run` before this change | PASS — 52 tests |
| `npm run typecheck` | PASS |
| `npm run test -- --run` after this change | PASS — 62 tests across 6 files (10 added) |
| `npm run lint` | PASS |
| `npm run build` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |
| `git diff --check` | PASS after implementation and documentation updates |

## Visual and interaction checks

- Opened the local preview, selected Text & Vocabulary, and confirmed the selector is beneath the existing section heading with the original “Set text cleanup preferences and custom terms.” description.
- Confirmed Clean is initially checked; mouse selection changed to Verbatim and back to Clean.
- Pressed ArrowDown on Clean in the browser: Verbatim became checked and received visible focus. The selected radio dot and row treatment distinguish selection without relying on color alone.
- Confirmed Verbatim survives navigation to Privacy and back, and Home → Settings → Text & Vocabulary. Reloaded the page and confirmed Clean was selected.
- Verified Launch at Login still toggles and all six Settings sections remain available. Visually inspected the existing Shortcuts status section.
- At the available approximately 900px viewport, measured `window.innerWidth = 897` and document/body `scrollWidth = 897`; no horizontal overflow. Browser console error log was empty.
- 320px viewport resize: **NOT RUN** — viewport resizing was unavailable through the active browser-control interface; no narrow-viewport substitute was used.
- Literal 200% browser zoom: **NOT RUN**.
- Screen-reader testing: **NOT RUN**.

## Limitations

Clean and Verbatim state is only an in-memory preview. It does not run deterministic cleanup, remove or preserve actual filler words, call native or backend code, persist preferences, or provide vocabulary CRUD. The mode selection resets on page refresh. Physical platform, screen-reader, narrow viewport, and zoom checks remain outstanding.
