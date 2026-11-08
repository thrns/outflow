# MVP-15 — Shortcut setup explanation preview

```yaml
task_id: MVP-15
feature_ids: [F-02, F-03, F-04, F-11]
flows: [UF-01, UF-03, UF-04, UF-09]
acceptance: [AC-02, AC-03, AC-04, AC-11] # related only; this preview does not pass native acceptance
owner: Codex, isolated feature worktree
base_commit: a16836a6238f5d0acc29545a0b591f3c6ef3a9e4
result_commit: 5f5ce59cfd6a96bf95fce2a4ed00e4bb6e916106 # implementation commit; evidence is committed separately
platform: macOS 26.6.2 arm64; hardware model/RAM NOT_MEASURED
metrics: NOT_MEASURED (browser presentation preview)
review: PENDING
```

## Scope and implementation

This adds the transient route **Home → Welcome → Privacy → Permissions → Shortcuts**. `ShortcutSetupStepPage` uses the shared page container, header, typography, spacing, and tokens. `PermissionsStepPage` adds **Continue to shortcuts** while keeping Back to Privacy and Settings General. The new page's **Back to Permissions** returns focus to the Permissions heading. Its explicit Settings action opens Shortcuts and returns focus to the Settings heading; ordinary Settings navigation still opens General.

The page reuses `ShortcutScenarioPreview` in development and `ShortcutCapabilityPanel` with default props in production. It does not add capability types, fixture definitions, recovery logic, a shortcut service, event handler, or settings schema. The status remains unconnected and unverified by default; no key combination is shown or assigned. Unknown or unsupported release events do not enable push-to-talk. Toggle is described as a possible fallback only while native registration and microphone readiness remain unverified. Conflict and OS authorization states remain distinct.

Home stays unmounted through setup. The existing volatile Copy Result and simulated acknowledgment disappear on navigation and do not return when Home is reopened; the App-owned lifecycle counter survives and allocates distinct IDs. Setup and Settings scenarios are independent. Preference and vocabulary preview state remains App-owned. No setup progress or recording-mode preference is saved. No real shortcut, microphone, permission, Tauri, clipboard, storage, telemetry, or network API is invoked.

## Changed files

- `src/app/ShortcutSetupStepPage.tsx` — new instructional page and development/production capability presentation.
- `src/app/App.tsx` — transient view, heading focus, and explicit Shortcuts destination while retaining General as the default.
- `src/app/AppHeader.tsx` — adds `shortcut-setup` as a transient view; primary destinations remain Home and Settings.
- `src/app/PermissionsStepPage.tsx` — Continue to shortcuts action.
- `src/styles/global.css` — setup layout using existing tokens and patterns.
- `src/app/App.test.tsx` — 17 focused navigation, mode, capability, fixture, lifecycle, and API-boundary tests.
- `README.md` — setup commands, navigation, expected behavior, testing, and troubleshooting.
- `docs/TASKS.md` — conservative MVP-15 progress note and M3 preview evidence references; no task or acceptance checkbox changed.

## Validation

| Command | Result | Notes |
|---|---|---|
| `npm ci` | PASS | Installed the locked dependencies; no dependency or lockfile change. |
| `npm run typecheck` | PASS | Final run after test updates. |
| `npm run lint` | PASS | Final run after test updates. |
| `npm run test -- --run` | PASS | 19 files, 326 tests passed. One earlier run found a test assertion expecting the production status label in development; the assertion was corrected to expect the synthetic label and the full suite then passed. |
| `npm run build` | PASS | 41 modules; production JS 288.80 kB (84.27 kB gzip). |
| Production asset selector check | PASS | `rg -n 'Developer scenarios \(simulated\)|shortcut-scenario|Registration available, release supported \(simulated\)' dist/assets/*.js` found no matches. Production rendering test also confirms the selector is absent. |
| `npm run dev -- --host 127.0.0.1` | PASS | Vite served the preview for the browser inspection; process stopped afterward. |
| `cargo fmt --all -- --check` | PASS |  |
| `cargo check --workspace --locked` | PASS |  |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |  |
| `cargo test --workspace --locked` | PASS | Full workspace and doc tests passed. A quiet count run reported 591 passing tests, 0 failures. No temporary-directory collision recurred. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS | 629 tests discovered; 628 passed and 1 skipped. |
| `git diff --check` | PASS |  |

## Visual inspection

Inspected the running Vite preview in the in-app browser:

- **Desktop, 1280 CSS px:** the page has one clear heading, readable capability/status groups, two side-by-side mode cards, and visible actions. Content follows the existing setup page spacing and neutral tokens.
- **320 × 900 CSS px:** layout stacks into one column, long status text wraps, and actions fill the available width. `innerWidth`, document `scrollWidth`, and body `scrollWidth` were all 320, with no horizontal overflow.
- Followed Home → Welcome → Privacy → Permissions → Shortcuts → Back to Permissions. The Permissions heading was the active focus target after returning.
- Followed the explicit View shortcut details in Settings action. Settings was the active focus target and the Shortcuts tab was selected. Ordinary application navigation to Settings selected General.
- Keyboard Tab moved from the scenario selector to Back to Permissions; computed focus outline was solid and 2 px. The capability scenario changed only when selected as a synthetic presentation fixture. Automated tests confirmed no window/document keydown, keyup, or keypress listener was installed and no forbidden browser/native APIs were called.

### Not run

- **Genuine 200% browser zoom:** the in-app browser exposed viewport sizing but no true zoom control; 320 CSS px testing is not reported as zoom testing.
- **Reduced-motion visual emulation:** no browser preference emulation control was available. The existing reduced-motion CSS rules were inspected but not runtime-emulated.
- Screen-reader smoke test and native OS shortcut, permission, microphone, and recording checks.

## Remaining limits

This is browser-only explanatory UI. It proves no native permission, shortcut registration, microphone readiness, recording, accessibility certification, or release criterion. T-010 remains blocked. T-011, T-032, T-033, T-041, T-052, T-054, T-055, and applicable native and release acceptance criteria remain open.
