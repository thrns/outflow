# Task 27 — Shared application header

```yaml
task_id: Task 27
feature_ids: [F-11]
flows: [UF-09]
acceptance: [AC-11]
owner: /root on the existing working tree
branch: codex/task23-moonshine-build-scope
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: uncommitted
platform: macOS 26.6.2, arm64; installed RAM NOT_MEASURED (system query denied)
implementation: Extracted the Outflow identity and Home/Settings navigation into src/app/AppHeader.tsx; App.tsx owns navigation state and heading focus; both views use the shared component.
doc_updates: README.md frontend preview instructions; this evidence file. docs/TASKS.md unchanged; AC-11 remains unverified.
review: PENDING
```

## Implementation

- Added the typed `AppHeader` interface with `currentPage` and `onNavigate` props. Its semantic header contains one labeled application navigation landmark, two native buttons, and `aria-current="page"` on the active view.
- Home and Settings render the same header. The Settings Back control is replaced by the Home button in the shared navigation. `App.tsx` retains the active view and focuses the destination `h1`; the in-memory Launch at Login value remains in the App shell.
- Added one integration test for the shared header, single application navigation landmark, and active-page state. Existing navigation, focus, content, Settings-section, and state-retention tests were reused.
- Updated the existing README frontend preview guidance with the component purpose, state ownership, visual-check steps, and navigation/focus troubleshooting. No dependency, lockfile, Settings schema, Rust implementation, or task-ledger changes.

## Automated verification

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 20 tests (19 existing, 1 added) |
| `npm run lint` | PASS |
| `npm run build` | PASS — Vite production build |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests (2 core, 70 settings) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |
| `impeccable detect --json src/app/AppHeader.tsx src/app/App.tsx src/app/HomePage.tsx src/styles/global.css` | PASS — no findings |
| `git diff --check` | PASS |

The Vite preview was started with `npm run dev -- --host 127.0.0.1` and served `http://127.0.0.1:5173/`. No Moonshine CMake configuration was attempted.

## Manual browser verification

- Inspected Home and Settings at 320×850 and 900×900. Both pages had one `.app-header`, one `Application navigation` landmark, the Outflow identity, and no horizontal overflow (`documentElement.scrollWidth` and `body.scrollWidth` matched the viewport width at both sizes).
- Confirmed `aria-current="page"` follows the active Home/Settings button, Settings has no duplicate Back control, all six Settings sections display, and Home content remains available.
- Activated Home/Settings with Enter and Space. Focus moved to the destination heading, the URL stayed at `/`, and Launch at Login remained On after leaving Settings and returning.
- Browser console error log was empty.
- **200% browser zoom: NOT RUN.** The browser did not expose a zoom control; viewport resizing was not treated as an equivalent check.

## Remaining limitations

- Native VoiceOver/NVDA/Orca screen-reader testing, forced-colors/high-contrast, and a separate reduced-motion preference run were NOT RUN. Visible keyboard focus and navigation were checked in the browser.
- This preview does not establish Tauri webview behavior or complete AC-11. Do not mark AC-11 complete based on this task.
- Physical 4 GB hardware and release memory/size gates were outside scope and NOT_MEASURED.

The starting worktree already contained uncommitted Task 25/26 changes in README, evidence, scripts, `App.tsx`, `App.test.tsx`, styles, and `HomePage.tsx`; they were preserved. The current branch remains unchanged and no files were staged or committed.
