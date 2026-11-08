# MVP-18 — Setup Step Indicator

**Status:** Implemented as a read-only position label across the six existing browser setup-preview screens. No production task or acceptance criterion is completed by this UI preview.

```yaml
task_id: MVP-18
feature_ids: [F-01]
flows: [UF-01]
acceptance: [] # The browser-only indicator does not meet AC-01 or another production criterion.
owner: Codex
branch: feature/mvp-setup-step-indicator
worktree: /private/tmp/outflow-mvp-setup-step-indicator
base_commit: ca223d69e759ce7190fb063fb365fecdc815f81a
base_verification: GitHub confirmed the live feature/mvp-tray-status-preview tip matched this SHA; the requested new branch name was unused.
result_commit: Included in the final branch commit; exact head SHA is reported in the handoff.
platform: macOS Darwin arm64; Node.js v22.23.3; npm 10.9.9; Rust 1.99.1; Cargo 1.99.1; host model and RAM NOT_MEASURED.
review: PENDING
```

## Implementation

Added `src/app/SetupStepIndicator.tsx` with one ordered, typed static definition for Welcome, Privacy, Permissions, Shortcuts, Try Dictation, and Tray and App Status. The view identity passed by each page selects its label; its array position supplies the step number. The component renders an ordinary paragraph before the page `h1`. It adds no navigation controls, progress role, success/completion state, live announcement, focus target, or persistent state. `src/styles/global.css` uses the existing `--muted-foreground` token and permits safe wrapping.

The component is included in `WelcomePage.tsx`, `PrivacyStepPage.tsx`, `PermissionsStepPage.tsx`, `ShortcutSetupStepPage.tsx`, `TryDictationStepPage.tsx`, and `TrayStatusStepPage.tsx`. `App.tsx`, `AppHeader.tsx`, route identities, navigation, page headings, existing actions, and native coordinator code were not changed. Home and Settings do not render the indicator.

## Automated verification

| Command | Result |
|---|---|
| `npm ci` | PASS — 204 locked packages installed; package manifests and lockfile unchanged. |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run src/app/SetupStepIndicator.test.tsx` | PASS — 1 file, 3 tests. |
| `npm run test -- --run` | PASS — 22 files, 358 tests. |
| `npm run build` | PASS — Vite transformed 45 modules; JavaScript 296.73 kB and CSS 34.46 kB. |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS — 580 unit/integration tests and 11 doctests. No temporary-directory collision occurred; a quiet rerun confirmed counts. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS — 629 tests; 1 existing skip. |
| `git diff --check` | PASS |

The focused Vitest suite asserts the six unique ordered identities and labels, renders the exact number and label on every setup page, checks one `h1` and one indicator per screen, verifies forward/backward navigation and heading focus, rejects interactive or progress/completion semantics, and confirms Home and Settings omit the indicator. The full existing suite also exercises the App navigation, Settings General versus explicit Shortcuts destination, Home unmount and volatile Copy Result/acknowledgment cleanup, unique App-owned result IDs, setup/Settings fixture independence, synthetic Try Dictation reset, and browser-only API tripwires.

## Browser inspection

The Vite server ran at `http://127.0.0.1:5174/` because port 5173 was already occupied. All six pages were visited in sequence at 1280 × 900 and 320 × 800 CSS pixels. Full-page desktop and viewport-sized narrow screenshots were visually inspected.

- At 1280 px, every screen showed the expected number, label, and unchanged page heading. The active element was the destination heading on each forward transition. `document.documentElement.clientWidth` and `scrollWidth` were both 1280 on all six screens.
- At 320 px, each indicator stayed within the 288 px content width. The Tray and App Status label wrapped to two lines; the other five fit on one line. `document.documentElement.scrollWidth` and `document.body.scrollWidth` both remained 320 on every screen.
- Keyboard activation of **Back to Try Dictation** from Tray and App Status focused `try-dictation-heading`; the indicator was not a keyboard stop.
- Home and Settings were the only primary navigation items in the inspected screens. The existing automated tests cover ordinary Settings opening General and the explicit Shortcut details action opening Shortcuts.

## Accessibility and native limitations

| Manual check | Result | Detail |
|---|---|---|
| Responsive reflow at 320 CSS px | PASS | All six screens inspected; no horizontal page overflow. |
| Keyboard focus handoff | PASS | Enter activation returned focus to the destination `h1`, not the indicator. |
| Genuine 200% browser zoom | NOT_RUN | A `Meta+Plus` attempt in the DOM-only tab timed out; the in-app browser exposes viewport sizing but no real zoom control. |
| Reduced-motion emulation | NOT_RUN | The browser has no emulation control for `prefers-reduced-motion`. |
| Spoken screen-reader verification | NOT_RUN | No spoken VoiceOver/NVDA/Orca test was performed. DOM/accessibility-tree inspection was not counted as spoken testing. |
| Native OS, permission, microphone, shortcut, insertion, clipboard, tray, offline installer, and release gates | NOT_RUN | This milestone changes only the browser presentation; T-010 remains blocked. |

## Review findings and outstanding work

No navigation, focus, volatile-state, privacy, or operation regression was found. The live remote base SHA was independently confirmed through the connected GitHub source. The workspace remained on macOS arm64; physical 4 GB hardware, installed bytes, aggregate app RSS, production accessibility, native platform behavior, and release gates were not measured here.

The existing MVP-17 evidence reports 591 unit/integration tests, while the current locked workspace run passed 580 unit/integration tests and 11 doctests. MVP-18 changes no Rust source or tests, so this historical count mismatch is recorded for later reconciliation and its prior evidence was left unchanged.

T-010 remains blocked. T-011, T-041, T-050, T-051, T-054, T-055, all related native checks, and all release gates remain open. No production task or acceptance checkbox was changed.
