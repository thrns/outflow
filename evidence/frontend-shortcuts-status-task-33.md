# Frontend Shortcuts Status — Task 33

**Result:** Implemented and verified for the browser preview. This is the user-requested frontend Task 33; it is separate from repository task T-033 (toggle shortcut recording), which remains unchecked.

| Field | Value |
|---|---|
| Feature / flow | F-11 / UF-09 |
| Acceptance | AC-11 remains incomplete; this preview check does not satisfy native accessibility acceptance |
| Branch / base | `codex/task23-moonshine-build-scope` / `a8b92fc4cc683abce23bcb528d62e0ce8457a5b1` |
| Platform | macOS 26.6.2, arm64; local Vite preview in the ChatGPT in-app browser |
| Result commit | Uncommitted; no commit created |

## Files changed for this task

- `src/app/App.tsx` — render the existing `SettingsStatusMessage` for Shortcuts.
- `src/app/App.test.tsx` — add focused wording, semantics, and section-switch integration assertions; update the existing section availability test.
- `README.md` — document the preview limitation, run/manual checks, troubleshooting, and evidence link.
- `evidence/frontend-shortcuts-status-task-33.md` — this record.

`App.tsx`, `App.test.tsx`, and `README.md` already contained uncommitted work at task start. That work was preserved. `SettingsStatusMessage.tsx`, `SettingsSection.tsx`, and the shared status-message CSS were also already present in the working tree and were not changed for this task.

## Implementation

Shortcuts reuses `SettingsStatusMessage` with the title **“Keyboard shortcuts unavailable”** and this explanation:

> Global keyboard shortcuts are not connected in this development preview. No system-wide shortcuts are registered or monitored. Shortcut configuration will become available after native integration.

The Shortcuts section retains its existing heading and description. The component API, Microphone message, CSS, settings state, and navigation behavior were not changed. No recorder, native hook, permission claim, registration, persistence, or interactive control was added.

## Automated verification

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 52 tests across 6 files (50 existing tests preserved; 2 added) |
| `npm run lint` | PASS |
| `npm run build` | PASS — Vite production build completed |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 unit tests; doc tests also passed (0 present) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |

No actual Moonshine CMake configuration was attempted. The Python suite includes synthetic and mocked CMake-runner tests; those do not establish that Moonshine configures or builds. The requested Cargo workspace contains only `core` and `settings` crates and has no Moonshine CMake integration.

## Visual and interaction checks

- Inspected Settings → Shortcuts at approximately 320px and 900px. The title and full explanation were visible and wrapped; no horizontal overflow was observed (`scrollWidth` equaled viewport width at both sizes).
- Confirmed all six Settings tabs remained in the accessibility tree at both widths. The Shortcuts message used the same computed background, border, radius, padding, and top spacing as the Microphone message.
- Confirmed the message contained no controls, alert role, or live region. Its heading and explanatory paragraph appeared as ordinary accessible text.
- Used the Settings arrow-key navigation from Microphone to Shortcuts; Shortcuts received focus. Home and Settings navigation moved focus to each destination page heading.
- Turned Launch at Login on, navigated Shortcuts → Home → Settings, and confirmed the in-memory value remained On.
- Browser console error list was empty.
- Literal 200% browser zoom: **NOT RUN**. The in-app browser exposed viewport sizing but not browser-level zoom; viewport resizing was not treated as zoom verification.
- Screen-reader behavior: **NOT RUN**. The browser accessibility tree was inspected, but no VoiceOver or other screen-reader session was available for a spoken interaction check.

## Limitations and status

Global shortcuts remain unavailable in this development preview. Keyboard permissions and native shortcut capability are not checked. There is no recording, native registration, or shortcut persistence. The repository task T-033 and settings tasks T-051/T-052 remain unchecked; no acceptance criteria were marked complete. AC-11 remains incomplete.

An auxiliary unpinned `rustup` active-toolchain inspection encountered a temp-directory permission error while trying to sync the repository-pinned toolchain. Every requested Rust validation command was explicitly run with `RUSTUP_TOOLCHAIN=stable` and passed.
