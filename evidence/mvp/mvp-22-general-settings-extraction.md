# MVP-22 — General Settings panel extraction

```yaml
task_id: MVP-22
feature_ids: [F-02, F-11, F-12]
flows: [UF-02, UF-09]
acceptance: [AC-02, AC-11, AC-12]
owner: Codex coordinator
branch: feature/mvp-general-settings-extraction
worktree: /private/tmp/outflow-mvp22-general-settings-extraction
base_commit: 7ee7317a1f0c2acb636e5393054a4c1573864740
implementation_commit: e0d316860c947ef38397ac4976a85c64ceee8691
platform: macOS 26.6.2 arm64; Node 22.23.3; npm 10.9.9; Rust 1.99.1; browser-based frontend preview
result: PASS for this scoped, behavior-preserving frontend refactor; production acceptance remains open
review: PENDING
```

## Base verification and implementation

The GitHub repository API confirmed that the live `feature/mvp-settings-navigation-extraction` branch and requested commit both resolve to `7ee7317a1f0c2acb636e5393054a4c1573864740`. The cached `origin` reference and clean MVP-21 worktree resolve to the same commit. Direct `git ls-remote` could not resolve `github.com`; the GitHub API read supplied the independent live confirmation.

Created the requested feature branch in its own worktree at that base. The original checkout remained untouched, including its pre-existing untracked `....` path and `scripts/__pycache__/` directory.

`GeneralSettingsPanel` accepts `runAtLogin: boolean` and `onRunAtLoginChange(checked: boolean)`. It owns no preference state. `App` still initializes `startup.run_at_login` to `false`, handles the callback, and retains the value across Settings and Home navigation. A fresh App mount resets it. The panel reuses `SettingsRow`, `SettingsSwitch`, and `PermissionReadinessChecklist`, inside the existing `SettingsSection`.

The Launch at Login copy, current On/Off text, title and description IDs, `aria-labelledby`/`aria-describedby` references, CSS classes, preview disclaimer, checklist, and DOM order are preserved. The six Settings tabs, their order, selected-tab/panel relationship, heading focus, and setup return navigation remain App-owned. The checklist fixture remains local to its conditional General panel and resets when General unmounts. No preference schema, persistence, Tauri/native bridge, OS startup behavior, microphone, permission, clipboard, telemetry, or network integration was added.

Changed files:

- `src/app/App.tsx`
- `src/app/GeneralSettingsPanel.tsx`
- `src/app/GeneralSettingsPanel.test.tsx`
- `README.md`
- `docs/TASKS.md` (one MVP-22 history note; no production task or acceptance checkbox changed)
- `evidence/mvp/mvp-22-general-settings-extraction.md`

## Automated validation

All commands were run in the feature worktree. Results:

| Command | Result |
|---|---|
| `npm ci` | PASS; installed 204 packages from the existing lockfile |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run src/app/GeneralSettingsPanel.test.tsx` | PASS; 1 file, 15 tests |
| `npm run test -- --run` | PASS; 25 files, 403 tests |
| `npm run build` | PASS; Vite transformed 47 modules |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS; 580 unit/integration tests and 11 doctests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS; 629 tests, 1 skipped |
| `git diff --check` | PASS |
| `git diff --cached --check` | PASS before the implementation commit |

The complete frontend suite retains the existing App integration coverage for General defaults, Settings navigation and keyboard behavior, preference retention between Settings sections and Home/Settings, fresh-mount reset, all four contextual Return to setup origins, primary Settings re-entry, independent Settings/setup capability fixtures, volatile Home Copy Result cleanup, and unique App-owned Copy Result lifecycle IDs. No existing test was removed or weakened.

The historical MVP-17–MVP-21 Rust test-count discrepancy remains unresolved. This run independently reports 580 unit/integration tests and 11 doctests; it does not establish the cause, and no earlier evidence was rewritten.

## Manual browser verification

Ran `npm run dev -- --host 127.0.0.1` and inspected the local preview.

- **Home → Settings → General:** General was selected; the six tabs remained in the existing order; Launch at Login showed Off; the permission checklist appeared below the row with all five capability values “Not checked”. The switch, copy, and checklist looked consistent with the existing layout.
- **Switch and state:** mouse activation changed Off to On immediately. Space activation changed the value and kept focus on the switch. Existing automated coverage confirms a repeated Space keydown does not toggle twice.
- **General → Privacy → General:** Launch at Login remained On. After selecting a checklist scenario, leaving and reopening General reset the fixture to “Permission readiness has not been checked.”
- **General → Home → Settings:** General was selected, the Settings heading received focus, and Launch at Login remained On.
- **Refresh:** after re-opening Settings, Launch at Login reset to Off.
- **Setup return:** Home → Welcome → Privacy → Permissions → View permission details in Settings opened General with **Return to setup: Permissions**. Return restored the Permissions heading, its focus, and Step 3 of 6.
- **Desktop layout:** browser viewport 1280×720 CSS px; `documentElement.scrollWidth` and `body.scrollWidth` were both 1280 px; `.settings-layout` measured 896 px wide.
- **Narrow layout:** browser viewport 320×800 CSS px; `documentElement.scrollWidth` and `body.scrollWidth` were both 320 px; `.settings-layout` measured 288 px; document scroll height was 2810 px. The row and checklist wrapped vertically, with no horizontal overflow or clipped text/control observed.

## Checks not run and remaining blockers

- **NOT_RUN — genuine 200% browser zoom.** Cmd+plus did not change the in-app browser zoom. The browser exposed viewport override controls but no zoom control; native Brave access was not approved, so the genuine zoom check could not be completed.
- **NOT_RUN — reduced-motion emulation.** No browser media-emulation control was available.
- **NOT_RUN — spoken screen-reader verification.** The browser accessibility tree was inspectable, but no spoken screen-reader session was available.
- **NOT_RUN — physical 4 GB hardware and release gates.** The browser preview does not validate native macOS/Windows/Linux behavior, OS startup behavior, native permissions, microphone or clipboard behavior, offline installation/network egress, package size, aggregate process RSS, latency, recognition quality, signing, or release accessibility gates.

T-010 remains blocked. T-011, T-041, T-050, T-051, T-054, T-055, AC-11, and the relevant native, accessibility, and release gates remain open. The MVP-22 note in `docs/TASKS.md` records preview verification only and changes no production task or acceptance status.
