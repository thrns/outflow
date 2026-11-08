# MVP-19 — Return to Setup from Settings

**Status:** Implemented in the browser-only setup preview. The contextual origin is temporary App state; no onboarding progress, completion state, or route history is persisted. No production task or acceptance criterion is completed by this presentation change.

```yaml
task_id: MVP-19
feature_ids: [F-01, F-11]
flows: [UF-01, UF-09]
acceptance: [] # Browser-only navigation preview; no production acceptance criterion is met.
owner: Codex
branch: feature/mvp-settings-return-to-setup
worktree: /private/tmp/outflow-mvp19-settings-return-to-setup
base_commit: 9936f71c02d661e9ec4240904cd4876862b1531a
base_verification: GitHub branch endpoint confirmed feature/mvp-setup-step-indicator at this exact SHA; the local base worktree matched.
result_commit: Included in the final branch commit; exact final SHA is reported in the handoff.
platform: macOS Darwin 25.6.0 arm64; Node.js v22.23.3; npm 10.9.9; Rust 1.99.1; Cargo 1.99.1; host RAM NOT_MEASURED.
review: PENDING
```

## Implementation

`src/app/App.tsx` owns one `SetupStepView | null` state value for the current explicit setup origin. The explicit Welcome, Privacy, Permissions, and Shortcut setup actions use a small callback to open Settings and select their required section. The return label comes from `SETUP_STEP_DEFINITIONS`. The neutral existing `app-navigation-button` style is used, with only a small top margin in `src/styles/global.css`.

Ordinary navigation clears the temporary origin before changing views. The return action navigates to that exact setup view through the existing heading-focus effect, which clears the origin and focuses the destination `h1`; the existing page renders its normal indicator. Switching Settings tabs does not change the origin. A fresh App mount starts with no origin. No router, history stack, dependency, storage key, completion state, browser/native API, or Rust code was added. Settings does not render the setup indicator.

The setup pages' primary Settings navigation remains on General. Welcome and Privacy permission-readiness links and the Permissions details link open Settings General with their origin available for return. Shortcut setup's explicit details action opens Settings Shortcuts and offers return to Shortcut setup.

## Automated verification

| Command | Result |
|---|---|
| `npm ci` | PASS — installed 204 locked packages; `package.json` and `package-lock.json` unchanged. |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run src/app/AppSetupSettingsReturn.test.tsx` | PASS — 1 file, 16 tests. |
| `npm run test -- --run` | PASS — 23 files, 374 tests. |
| `npm run build` | PASS — Vite transformed 45 modules; JS 297.14 kB and CSS 34.50 kB. |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS — 580 unit/integration tests and 11 doctests; no temporary-directory collision. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS — 629 tests; 1 existing skip. |
| `git diff --check` | PASS |

The focused tests cover all four explicit origins, correct Settings sections, exact return destinations, destination heading focus and step indicators, tab switching, ordinary Home/Settings actions, stale-context clearing, remount behavior, independent setup/Settings capability fixtures, fixture reset after the setup page unmounts, and Settings preference/vocabulary state continuity. Existing App tests still exercise Home unmounting, Copy Result and simulated acknowledgment discard, distinct App-owned result lifecycle IDs, Home's unavailable default after return, Try Dictation reset on navigation, and API tripwires. The navigation tripwire path now exercises contextual return and ordinary Settings navigation; no Tauri, microphone, permission, keyboard, clipboard, storage, telemetry, or network API was invoked.

The historical MVP-17/MVP-18 Rust test-count discrepancy remains unresolved: MVP-17 recorded 591 unit/integration tests, while MVP-18 recorded 580 unit/integration tests plus 11 doctests. This run independently reports 580 unit/integration tests and 11 doctests. Prior evidence was not rewritten.

## Browser inspection

The Vite server ran at `http://127.0.0.1:5174/` because port 5173 was occupied. The in-app browser's default viewport was 1280 CSS px wide. At 320 CSS px, `document.documentElement.scrollWidth` and `document.body.scrollWidth` were both 320 for Home, Welcome, Privacy, Permissions, Shortcut setup, Settings General, and Settings Shortcuts with a contextual return button.

- **Welcome:** Home → Preview setup → Review permission readiness opened Settings General with **Return to setup: Welcome**. Settings initially focused its `h1`. Tab moved to the return button; Enter returned to Welcome, focused its `h1`, and restored Step 1 of 6.
- **Permissions:** Welcome → Privacy → Permissions → View permission details in Settings opened General with **Return to setup: Permissions**. Switching to the Shortcuts tab kept the action visible; activation returned to the Permissions `h1` and Step 3 of 6.
- **Shortcut setup:** View shortcut details in Settings opened Shortcuts with **Return to setup: Shortcuts**. Activation returned to the Shortcut setup `h1` and Step 4 of 6.
- **Stale context:** Settings → Home → Settings ended on General with no return button. The Home and Settings headings received focus on each transition.
- **Refresh:** Reloading Settings while **Return to setup: Shortcuts** was available returned the preview to Home; reopening Settings selected General and showed no return action.
- Desktop screenshot inspection showed the return button below Settings and visually neutral. The Settings view had exactly one `h1`.

| Manual check | Result | Detail |
|---|---|---|
| Desktop layout at 1280 CSS px | PASS | Settings return action visually inspected; no horizontal overflow. |
| Reflow at 320 CSS px | PASS | All seven measured routes matched the viewport width. |
| Keyboard navigation and activation | PASS | Tab reached the return button after the focused Settings `h1`; Enter returned and focused the origin `h1`. |
| Genuine 200% browser zoom | NOT_RUN | Tried `Meta+Plus`; the in-app browser remained at width 1280 and visual viewport scale 1. It exposes viewport sizing but no genuine zoom control. |
| Reduced-motion emulation | NOT_RUN | Browser controls expose viewport and visibility only; no `prefers-reduced-motion` emulation was available. |
| Spoken screen-reader test | NOT_RUN | No spoken VoiceOver/Narrator/Orca session was run. Accessibility-tree inspection was not counted as spoken testing. |
| Native permission, microphone, shortcut, tray, insertion, clipboard, installer, offline, platform, and release checks | NOT_RUN | This change only alters browser-preview navigation. |

## Remaining status

T-010 remains blocked. T-011, T-041, T-050, T-051, T-054, T-055, the relevant native/accessibility work, and release gates remain open. No production task or acceptance checkbox changed. The 4 GB hardware, app RSS, installed bytes, native behavior, and production accessibility gates were not measured or validated here. The final remote branch SHA is reported in the final handoff.
