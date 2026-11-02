# Task 26 — Home page preview evidence

This is the frontend-only Home page task requested as Task 26. It is not repository task `T-026` (CPAL mic stream lifecycle).

## Repository context

- Branch: `codex/task23-moonshine-build-scope` (preserved).
- Base commit: `a8b92fc4cc683abce23bcb528d62e0ce8457a5b1`; result remains uncommitted.
- Environment: macOS 26.6.2 arm64, Node.js v22.23.3, npm 10.9.9.
- The pre-existing README, `evidence/T-009.md`, Moonshine CMake documentation, and scripts were preserved. No dependency or lockfile changes.

## Implementation

- Added a Home page with Outflow identity, offline desktop dictation intent, a truthful development-preview status, and a Settings route.
- Added state-based Home/Settings navigation with keyboard focus moved to the new page heading. The existing Launch at Login state remains in the App shell and survives navigation.
- Preserved the existing Settings sections and preview controls. No package or lockfile changes; no Tauri, microphone, ASR, shortcut, or backend integration.
- Updated the README with frontend startup, navigation, verification, limitations, and troubleshooting instructions.

## Verification

Commands run from the repository root on 2026-10-31:

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 19 tests (16 baseline, 3 added net) |
| `npm run lint` | PASS |
| `npm run build` | PASS — Vite production build |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |

## Manual browser checks

- `npm run dev` served `http://localhost:5173/`; Home rendered on first load. Home and Settings screenshots were captured and visually inspected in the CUA browser session.
- At 320px width, `document.documentElement.scrollWidth` and `document.body.scrollWidth` were 320px. At 900px, both were 900px. At a 450px effective viewport (half the 900px desktop width), both remained 450px with the content reflowed and no horizontal overflow. The browser did not expose a working 200% zoom control; this 450px viewport check verifies the corresponding reflow width, while literal browser zoom remains **NOT RUN**.
- Keyboard Enter navigated Home → Settings → Home; keyboard Space toggled Launch at Login. Focus moved to the new page heading, and the setting remained On after returning to Settings.
- The Settings accessibility tree exposed all six existing tabs. The URL remained `http://localhost:5173/` during navigation. Browser console error log was empty.
- Reduced-motion rules cover the added button transitions and active-state transform; the browser's reduced-motion preference was not separately emulated.

## Scope and status

- `AC-11` and all Tauri-dependent acceptance criteria remain unverified for the native application. `docs/TASKS.md` was not changed.
- Physical hardware, OS-native screen-reader behavior, and literal 200% browser zoom were not tested. Resource, audio, model, and packaging gates were not part of this preview task.
- `git diff --check` passed after the evidence file and README update; final Git status is recorded in the completion report.
- Result remains uncommitted on the pre-existing branch; no files were staged.
