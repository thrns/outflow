# Frontend Task 32 — SettingsStatusMessage evidence

```yaml
task_id: frontend-task-32
feature_ids: [F-11]
flows: [UF-09]
acceptance: [AC-11] # Partial frontend preview work only; AC-11 is NOT COMPLETE.
owner: coordinator in existing working tree
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: uncommitted changes on codex/task23-moonshine-build-scope
platform: macOS 26.6.2; local Vite preview in the Codex in-app browser
implementation: Added a reusable static Settings message and used it in the Microphone preview.
doc_updates: README.md Task 32 section; this evidence file
review: PENDING
```

## Changes

- Created `src/app/SettingsStatusMessage.tsx` with `title: string` and `children: ReactNode` props. It renders an `h3` and explanatory paragraph without live-region/alert semantics or controls.
- Created `src/app/SettingsStatusMessage.test.tsx` with six focused component tests.
- Updated the Microphone branch in `src/app/App.tsx` to explain that selection is not connected in this preview, no audio is captured, device availability and permissions have not been checked, and input selection needs future native integration.
- Added neutral, token-based styling to `src/styles/global.css` with wrapping at narrow widths.
- Updated `src/app/App.test.tsx` to assert the Microphone message and preserve the other sections' existing placeholders.
- Updated `README.md` with component props, static-versus-live semantics, commands, visual steps, and troubleshooting.
- The existing Home preview already says microphone access is not requested and no recording session starts; that Home copy remains unchanged. The Microphone message adds the device/permission-check limitations and future-integration context in the relevant Settings panel.

The working tree already contained unrelated uncommitted work before this task, including earlier edits to `App.tsx`, `App.test.tsx`, `global.css`, and `README.md`. Those changes were preserved. No dependency, lockfile, Rust setting, or native microphone functionality was added. `docs/TASKS.md` and AC-11 completion status were not changed.

## Automated verification

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 50 tests total; 43 existing frontend tests retained and 7 task tests added (6 component, 1 integration) |
| `npm run lint` | PASS |
| `npm run build` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests (2 core, 70 settings) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |
| `/Users/bixxie/.agents/skills/impeccable/scripts/impeccable detect --json src/app/SettingsStatusMessage.tsx src/app/App.tsx src/styles/global.css` | PASS — no findings |

The first frontend test run had one test-only failure because Testing Library did not match a sentence spanning an inline `<strong>` node as one text node. The assertion was adjusted to inspect the paragraph's combined text. The final full suite passed 50/50.

The Rust workspace contains only the existing core and settings crates and no `build.rs`; the listed Cargo checks did not invoke the blocked Moonshine CMake configure task.

## Visual and accessibility checks

Development server: `npm run dev -- --host 127.0.0.1` — PASS at `http://127.0.0.1:5173/`.

- **320×800 CSS viewport:** status message and title remained visible and wrapped; `documentElement.scrollWidth` equaled `innerWidth` at 320px. Panel width was 288px and message width was 246px.
- **900×800 CSS viewport:** message fit within the existing Settings panel; `documentElement.scrollWidth` equaled `innerWidth` at 900px. Panel width was 628px and message width was 572px.
- **Default browser viewport after reset:** 897px wide; no horizontal overflow.
- Browser console error list was empty after rendering and navigation. No permission prompt or recording state appeared.
- The browser accessibility tree exposed the Microphone section, level-three message title, and complete explanation. No alert or live-region semantics were present.
- Keyboard checks covered Settings tab navigation, Home/Settings navigation with destination heading focus, Space activation of Launch at Login, and preservation of its in-memory On state across Settings section navigation.
- All six Settings section tabs remained available. Existing automated integration tests cover the untouched placeholder sections and General behavior.
- **200% browser zoom: NOT RUN.** The content-only in-app browser did not respond to its zoom shortcut; viewport resizing was not treated as equivalent to zoom.
- **Screen-reader session: NOT RUN.** Accessibility-tree text was inspected, but VoiceOver/Narrator/Orca was not run.

## Limitations

This remains a frontend development preview. The app does not connect native microphone selection, capture audio, check device availability, or request/check microphone permissions. AC-11 and native OS accessibility/focus behavior remain unverified.
