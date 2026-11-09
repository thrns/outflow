# MVP-21 — Settings navigation extraction

```yaml
task_id: MVP-21
feature_ids: [F-11]
flows: [UF-09]
acceptance: [AC-11]
owner: Codex coordinator
branch: feature/mvp-settings-navigation-extraction
worktree: /private/tmp/outflow-mvp21-settings-navigation-extraction
base_commit: 3e99bcfffd9ead0c24e94154bdec31c2f7d49bef
implementation_commit: 9d5444c91e23aefe1af7224b19abf2c9ed9ae999
platform: macOS 26.6.2 arm64; browser-based frontend preview; physical 4 GB hardware gate NOT_RUN
result: PASS for this scoped frontend refactor; native/release acceptance remains open
review: PENDING
```

## Implementation

Moved the existing ordered six-section definition and its tablist markup, tab refs, and local keyboard handler into `SettingsNavigation`. The component accepts `selectedSection` and `onSelectSection`; it does not own the selected value. `SETTINGS_SECTIONS` and the inferred `SettingsSectionId` type are exported for the `App` panel title, description, and routing types.

`App` still owns `activeView`, `settingsReturnView`, `selectedSection`, Settings preference and vocabulary preview state, Copy Result lifecycle identity, setup navigation, and heading focus. `SettingsSection`, `AppHeader`, global CSS, and existing integration tests were not modified. No dependencies, native integrations, settings persistence, production task checkboxes, or acceptance status changed.

Changed files:

- `src/app/SettingsNavigation.tsx`
- `src/app/SettingsNavigation.test.tsx`
- `src/app/App.tsx`
- `README.md`
- `docs/TASKS.md`
- `evidence/mvp/mvp-21-settings-navigation-extraction.md`

## Automated validation

All listed commands were run from the feature worktree. Results:

| Command | Result |
|---|---|
| `npm ci` | PASS; installed 204 packages from the existing lockfile |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run src/app/SettingsNavigation.test.tsx` | PASS; 1 file, 8 tests |
| `npm run test -- --run` | PASS; 24 files, 388 tests |
| `npm run build` | PASS; Vite transformed 46 modules |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS; 580 unit/integration tests, 11 doctests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS; 629 tests run, 1 skipped |
| `git diff --check` | PASS; no whitespace errors in the working-tree diff |
| `git diff --cached --check` | PASS; no whitespace errors in the final staged diff before the evidence commit |

The Rust unit/integration and doctest totals are reported separately. The MVP-17–MVP-20 historical test-count discrepancy remains open; no earlier evidence was changed to reconcile it.

## Manual browser verification

- Opened **Home → Settings** and inspected the six sections in order: General, Microphone, Shortcuts, Text & Vocabulary, Privacy, About & Diagnostics. The panel title, description, and content changed with the selected tab.
- Ran the keyboard sequence with visible focus: Home → General; ArrowDown through all six tabs and wrap to General; ArrowUp wrap to About & Diagnostics and reverse; Home to General; End to About & Diagnostics. Each step left focus on the selected tab.
- From Shortcuts, clicking primary Settings reset selection to General and moved focus to the Settings heading.
- Walked **Home → Welcome → Privacy → Permissions → Shortcuts → View shortcut details in Settings → Return to setup: Shortcuts**. Settings opened with Shortcuts selected and Settings heading focus; Return restored Step 4 of 6 and focused the Shortcut setup heading.
- Inspected desktop and 320 CSS px browser views, including all six sections at 320 CSS px. The vertical tab navigation and panel wrapped in the existing layout; no horizontal overflow was visible.
- Existing integration tests remained enabled and passed, including ordinary General and explicit Shortcuts destinations, same-view Settings reset, all four contextual return origins, preference/vocabulary retention, fixture independence, volatile Copy Result cleanup, Try Dictation remount reset, and browser API tripwires.

## Not run and remaining blockers

- **NOT_RUN — genuine 200% browser zoom.** The in-app browser exposes viewport sizing and visibility controls, not browser zoom. A page-level `super+plus` attempt did not provide browser-level zoom; opening native Brave for the zoom check was blocked by the computer-use approval gate.
- **NOT_RUN — reduced-motion emulation.** The in-app browser provides no media-emulation control. The existing CSS reduced-motion rule was not changed.
- **NOT_RUN — spoken screen-reader check.** No screen-reader session was available for the browser preview.
- Physical 4 GB hardware, actual accessibility sign-off, native OS behavior, offline installation, package size/RSS/latency/quality, and release gates were not evaluated by this UI refactor. They remain open; T-010 remains blocked. AC-11 and production task checkboxes remain open.
