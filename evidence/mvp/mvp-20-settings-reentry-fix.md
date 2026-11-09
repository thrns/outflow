# MVP-20 — Primary Settings re-entry

```yaml
task_id: MVP-20
feature_ids: [F-01, F-11]
flows: [UF-01, UF-09]
acceptance: [] # Browser-preview navigation only; no production acceptance criterion is met.
owner: Codex
branch: feature/mvp-settings-reentry-fix
worktree: /private/tmp/outflow-mvp20-settings-reentry-fix
base_commit: c068906f4e9923de8bff6f82b2141dfe602d3a40
base_verification: GitHub refs/heads/feature/mvp-settings-return-to-setup returned the exact supplied SHA via git ls-remote.
result_commit: Included in the branch commit; exact final SHA is reported in the handoff.
platform: macOS 26.6.2 arm64; Node.js v22.23.3; npm 10.9.9; Rust 1.99.1; host RAM NOT_MEASURED.
review: ACCEPTED
```

## Issue and change

When Settings was already the active view, `navigateTo("settings")` returned before selecting General, clearing the temporary setup-return view, or moving focus to the Settings heading. The same-view branch now performs those three actions for Settings, then returns without changing `activeView`. Home same-view behavior is unchanged. The App remains mounted, so Settings preferences and vocabulary preview state stay in memory.

The four explicit setup actions still set their origin and destination: Welcome, Privacy, and Permissions open General; Shortcut setup opens Shortcuts. Returning still focuses the originating setup heading and shows its existing step indicator. Settings tab changes alone still preserve the temporary return context. No routing, dependency, CSS, Rust, persistence, or production-task/acceptance change was made.

## Regression tests

`src/app/AppSetupSettingsReturn.test.tsx` adds coverage for primary Settings re-entry from Shortcuts and Privacy, heading focus, clearing Welcome and Shortcut return context, no return-context restoration after tab changes, and preference plus saved vocabulary-preview value retention. Existing parameterized tests continue to cover all four explicit destinations and exact setup returns.

`src/app/App.test.tsx` extends the existing browser-only API tripwire sequence to change to Shortcuts and invoke primary Settings again while Settings is active. The full suite also retains the Home Copy Result lifecycle test, which verifies Home's volatile result/acknowledgment is discarded on navigation and later result lifecycle IDs remain unique.

Before the handler fix, the focused regression command failed as expected: 5 failures and 17 passes out of 22 tests. The captured run output is preserved in [`mvp-20-pre-fix-focused-test.log`](mvp-20-pre-fix-focused-test.log); terminal whitespace in rendered button text and the extra final blank line were normalized for `git diff --check`. It showed General remaining unselected, contextual return buttons remaining visible, and preview state assertions stopping at the incorrect selected section. After the fix, all 22 focused tests passed.

## Automated verification

| Command | Result |
|---|---|
| `npm ci` | PASS — installed 204 locked packages; package manifests and lockfile unchanged. |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run src/app/AppSetupSettingsReturn.test.tsx` | PASS — 1 file, 22 tests. |
| `npm run test -- --run` | PASS — 23 files, 380 tests. |
| `npm run build` | PASS — Vite transformed 45 modules; JavaScript 297.20 kB and CSS 34.50 kB. |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS — 591 unit/integration tests and 11 doctests. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS — 629 tests; 1 skipped. |
| `git diff --check` | PASS |

The navigation API tripwire test passed with forbidden-call sentinels installed for Tauri, microphone, permission, keyboard-listener, clipboard, storage, telemetry, and network access. The existing Home Copy Result test passed in the full frontend run. No native, microphone, permission, global keyboard, clipboard, storage, telemetry, or network API was invoked by this change.

## Browser observations

`npm run dev -- --host 127.0.0.1` started Vite 8.3.5 at `http://127.0.0.1:5174/` because port 5173 was occupied.

- **Home → Settings → Shortcuts → primary Settings:** General became selected and the accessibility state reported focus on the Settings `h1`.
- **Welcome → Privacy → Permissions → Shortcut setup → View shortcut details in Settings → primary Settings:** the explicit action opened Shortcuts with **Return to setup: Shortcuts**. Primary Settings then selected General, focused the Settings `h1`, and removed the return action.
- **Explicit return:** the normal walkthrough was reopened; its explicit Shortcut action still opened Shortcuts with the valid return action. Activating it returned to Shortcut setup, focused its `h1`, and restored **Step 4 of 6 · Shortcuts**.
- **Keyboard:** from the selected Shortcuts tab, Shift+Tab focused the primary Settings button; Enter selected General and focused the Settings `h1`.
- **Desktop:** 1280 × 900 CSS px Settings with the contextual return button was visually inspected; content stayed within the viewport.
- **320 CSS px:** contextual Shortcuts and cleared General Settings layouts were visually inspected. Text and controls wrapped within the viewport with no apparent horizontal clipping or overflow. The browser UI did not expose a DOM `scrollWidth` readout, so this is a visual check rather than a numeric measurement.

| Manual check | Result | Detail |
|---|---|---|
| Desktop layout at 1280 × 900 CSS px | PASS | Contextual Settings layout inspected. |
| Reflow at 320 CSS px | PASS (visual) | No horizontal clipping apparent in contextual Shortcuts or General after clearing context; no numeric `scrollWidth` measurement. |
| Keyboard focus and activation | PASS | Shift+Tab reached primary Settings; Enter returned to General and focused the Settings heading. |
| Genuine 200% browser zoom | NOT_RUN | Tried the browser zoom keyboard shortcut; the in-app browser did not expose a zoom control or a verifiable 200% state. |
| Reduced-motion emulation | NOT_RUN | Browser exposed viewport and visibility controls only; no motion-preference emulation was available. |
| Spoken screen-reader check | NOT_RUN | No spoken VoiceOver/Narrator/Orca session was available or run. |
| Native/platform/release checks | NOT_RUN | No native integration, OS permissions, installer, hardware/resource, offline, or release behavior was changed or verified. |

## Remaining status

This is browser-preview evidence only. T-010 remains blocked; T-011, T-041, T-050, T-051, T-054, T-055, production tasks, acceptance criteria, native/platform/accessibility work, and release gates remain open. No 4 GB hardware, process RSS, installed-size, recognition-quality, latency, or release metric was measured. The task ledger checkboxes were left unchanged.
