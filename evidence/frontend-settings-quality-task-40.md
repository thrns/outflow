# Frontend Settings Quality — Task 40

> This frontend QA task is separate from repository T-040 (ASR/cleanup endurance). The task ledger and AC-11 status were not changed.

```yaml
task_id: frontend-settings-quality-task-40
related_features: [F-11, F-12, F-13]
related_flows: [UF-09, UF-10, UF-11, UF-12]
acceptance: [AC-11]
owner: Codex / codex/task23-moonshine-build-scope
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: uncommitted
platform: macOS 26.6.2 arm64; ChatGPT in-app browser; explicit CSS viewport overrides
implementation: Corrected the active Settings tab to panel accessible-name relationship and strengthened existing integration coverage.
metrics: 320x900 and 900x900 CSS-pixel viewport checks; no horizontal overflow observed.
doc_updates: README.md Task 40 section; this evidence report; task ledger unchanged
review: PENDING
```

## Scope and finding

Reviewed Home and all six Settings sections: General, Microphone, Shortcuts, Text & Vocabulary, Privacy, and About & Diagnostics. The current implementation is a frontend preview; checks below do not establish native settings, privacy protections, diagnostics, updates, or AC-11 completion.

The active tab had `aria-controls="settings-panel"`, but the panel's `aria-labelledby` pointed to its inner heading (`settings-panel-heading`) instead of the selected tab. The rendered DOM showed this one-way relationship for all six sections. Reproduction: open Settings, select each tab, and compare the active tab's `id` with the active panel's `aria-labelledby`. Before the fix, the existing integration test failed with expected `settings-tab-general` and received `settings-panel-heading` (42 passed, 1 failed in `App.test.tsx`).

`SettingsSection` now accepts an optional `labelledBy` ID, and `App.tsx` supplies the selected tab ID. Its visible H2 remains in place. The existing Settings integration test now checks all six selected tabs, their `aria-controls` target, the panel's reciprocal `aria-labelledby`, one active panel, and unique IDs. The targeted test failed before the correction and passed afterward.

## Verification matrix

| Area | Check | Result | Evidence |
|---|---|---|---|
| General | OFF default; mouse and Space switch activation; name/description; navigation retention; refresh reset | PASS | Live browser interaction; `SettingsSwitch.test.tsx`; `App.test.tsx` |
| Microphone | Unavailable native setup is explicit; no audio/device/permission status is fabricated | PASS | Live accessibility tree and panel copy; `App.test.tsx` |
| Shortcuts | Global registration is explicitly unavailable; no working shortcut control is shown | PASS | Live accessibility tree and panel copy; `App.test.tsx` |
| Text & Vocabulary | Clean default; native radio arrow behavior; field labels; Unicode sample and preview; linked whitespace error; Clear draft reset/focus; mode and draft preservation; refresh reset | PASS | Live browser interaction; `VocabularyEntryForm.test.tsx`, `VocabularyDraftPreview.test.tsx`, `App.test.tsx` |
| Privacy | Product requirements are distinguished from protections not yet implemented and verified | PASS | Live accessibility tree and content inspection; `PrivacyOverview.test.tsx` |
| About & Diagnostics | No operational export/update controls or fabricated runtime metadata; unavailability is explicit | PASS | Live accessibility tree and content inspection; `AboutDiagnosticsOverview.test.tsx` |
| Navigation | Home/Settings keyboard navigation; destination heading focus; all six arrow-key tabs and wrap; active semantics; one main landmark; panel/tab relationships; no trap observed | PASS | Keyboard walkthrough and browser accessibility tree; updated `App.test.tsx` |
| Responsive | Home and all six panels at 320x900 and 900x900 CSS px; gutters, wrapping, scrolling, and horizontal overflow | PASS | Browser viewport measurements and screenshots; `scrollWidth` equaled `clientWidth` for every view; no panel child exceeded its panel at 320px |
| Focus | Visible heading focus after keyboard navigation; switch focus remains visible near the 320px viewport edge | PASS | Keyboard walkthrough and screenshots |
| Accessibility | Literal 200% browser zoom; text-spacing override; reduced-motion emulation; forced-colors/high-contrast emulation; alternate light color scheme; VoiceOver/screen-reader operation | NOT RUN | In-app browser exposed viewport resizing but no zoom or preference-emulation control; actual screen-reader testing was unavailable |

### Manual interaction notes

- The browser accessibility tree exposed one `main` landmark on Home and Settings, six named tabs, one selected tab, and one active panel. After the fix, each selected tab points to the panel and the panel names that tab.
- Tab moved from the selected tab to the panel and onward without a keyboard trap. Keyboard navigation from Home to Settings and back focused the destination H1 with a visible outline. Down-arrow movement traversed all six tabs and wrapped from About & Diagnostics to General.
- Launch at Login started Off. Mouse click and Space changed the switch; it retained its value through Home/Settings navigation and returned Off after refresh.
- Clean was initially selected. Right-arrow from Clean selected Verbatim. The `café` / `Café` draft rendered as synthetic example text. A whitespace-only spoken phrase exposed `aria-invalid`, an associated description/error ID, and a `role="alert"`; clearing removed the error, emptied both values, disabled Clear draft, restored the empty preview, returned focus to Spoken phrase, and retained Verbatim.
- Vocabulary values and Verbatim survived section switching and Home/Settings navigation. A refresh returned the draft to empty and cleanup mode to Clean.
- Privacy copy calls the protections product requirements and says native paths are not yet verified. About & Diagnostics says no report can be generated and update checking is not implemented. Microphone and Shortcuts expose no native action controls.
- The current browser preference queries matched `prefers-reduced-motion: reduce`, `forced-colors: active`, and `prefers-contrast: more` as false; these preferences and the alternate light color scheme were not emulated. An attempted zoom shortcut did not change browser scale, so literal 200% zoom remains NOT RUN.
- No WCAG 2.2 AA conformance claim is made. Accessibility-tree inspection is not a substitute for VoiceOver or other assistive-technology testing.

## Automated verification

Environment: macOS 26.6.2 arm64; Node.js 22.23.3, npm 10.9.9, Rust 1.99.1 stable, Python 3.13.13; Vite 8.3.5 at `http://127.0.0.1:5175/` (ports 5173 and 5174 were already occupied).

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 10 files, 136 tests (the existing integration test was strengthened; total unchanged) |
| `npm run lint` | PASS |
| `npm run build` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests (2 core, 70 settings) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |
| `git diff --check` | PASS — rerun after final report and README edits |

The browser console log query returned no warnings or errors. The blocked Moonshine CMake configuration was not attempted.

## Files and remaining work

Task 40 touched `src/app/SettingsSection.tsx`, `src/app/App.tsx`, `src/app/App.test.tsx`, `README.md`, and this report. No dependency, lockfile, Rust, task-ledger, or unrelated ASR/benchmark file was changed for this task. The worktree already contained unrelated uncommitted changes before the audit; they remain unstaged and preserved.

The Settings page remains a frontend preview. Native microphone selection, global shortcuts, persisted settings/vocabulary, privacy enforcement, diagnostics export, signed updates, and native webview lifecycle were not tested or implemented here. AC-11 and release/platform acceptance remain incomplete. Literal 200% zoom, text-spacing, reduced-motion and high-contrast emulation, and screen-reader testing remain outstanding.
