# MVP-13 — Privacy explanation step preview

```yaml
task_id: MVP-13-privacy-step-preview
feature_ids: [F-01, F-02, F-12]
flows: [UF-01, UF-02, UF-10]
acceptance: [AC-01, AC-02, AC-12]
owner: Codex / feature/mvp-privacy-step-preview
base_commit: 0e288b3703ae96c60ccfd3d22c520f70827cbca8
result_commit: ec73d3107a56d778d797c3a1b26da6fae740888b
platform: macOS 26.6.2, arm64; RAM NOT_MEASURED (sysctl denied by sandbox)
implementation: Browser-only transient Privacy setup view reusing PrivacyOverview; no native integration or persisted setup state.
commands:
  - {command: 'npm ci', result: PASS, notes: 'Installed 204 packages from the lockfile.'}
  - {command: 'npm run typecheck', result: PASS, notes: 'No TypeScript errors.'}
  - {command: 'npm run lint', result: PASS, notes: 'ESLint completed without findings.'}
  - {command: 'npm run test -- --run', result: PASS, notes: '19 files; 299 tests passed.'}
  - {command: 'npm run build', result: PASS, notes: 'Typecheck and Vite production build completed.'}
  - {command: 'npm run dev -- --host 127.0.0.1', result: PASS, notes: 'Started local preview for manual inspection; stopped afterward.'}
  - {command: 'cargo fmt --all -- --check', result: PASS, notes: 'Workspace formatted.'}
  - {command: 'cargo check --workspace --locked', result: PASS, notes: 'Workspace checked.'}
  - {command: 'cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'cargo test --workspace --locked', result: PASS, notes: 'All workspace unit, integration, and doctest suites passed; no temporary-directory collision observed.'}
  - {command: "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v", result: PASS, notes: '629 tests passed; 1 skipped.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors.'}
metrics: NOT_MEASURED; product resource, offline-operation, recognition, and native privacy gates are outside this browser-only preview.
doc_updates: README.md MVP-13 run/navigation/troubleshooting; docs/TASKS.md preview note only; production task checkboxes unchanged.
risks: 'Desktop layout and both navigation paths visually inspected in the IAB. Genuine 320 CSS px, 200% browser zoom, and screen-reader inspection are NOT_RUN: the IAB exposed no viewport or zoom control, Meta+Plus/Meta+Equal produced no visible zoom change, and access to the installed Brave Browser was denied by the computer-use gate. No native recording, complete offline test, clipboard operation, or release privacy guarantee was tested.'
review: ACCEPTED (scoped implementation and documentation reviewed; production acceptance and release gates remain open)
```

## Behavior implemented

- Added **Review privacy** to Welcome without removing **Back to Home** or **Review permission readiness**.
- Added the transient Privacy step with one focusable `h1`, shared Settings → Privacy content, **Back to Welcome**, and **Review permission readiness**.
- Kept Home and Settings as the only primary navigation destinations. Heading focus follows each app-view change; the checklist action opens Settings → General.
- Clarified that local speech recognition, cleanup, offline use, no account, no cloud transcription, and no audio/transcript history are product requirements rather than verified browser-preview behavior.
- Explained the allowed local settings/vocabulary boundary, explicit Copy Result clipboard exposure, possible OS/clipboard-manager retention, no cryptographic-erasure promise for memory or swap, and the separately user-initiated signed full-app update check.
- Kept Home conditionally unmounted through Welcome and Privacy. Tests verify volatile Copy Result and simulated acknowledgment state are discarded, Home returns to its unavailable default, and the App-owned lifecycle counter assigns a distinct ID to the next result.
- No consent, telemetry, permissions, native integration, setup-completion/progress storage, or additional dependency was introduced. Existing Settings → Privacy tests and content checks remain unchanged and pass.

## Verification boundaries

The manual IAB inspection showed the full page reading order, desktop composition, destination heading focus, visible keyboard focus, keyboard Tab navigation, and Enter activation of **Review permission readiness**. Both requested routes were inspected: **Home → Welcome → Privacy → Back to Welcome** and **Home → Welcome → Privacy → Settings → General**. The existing checklist rendered with its unknown defaults.

The IAB accessibility tree was inspected, but this is not a screen-reader test. Responsive behavior at 320 CSS px and genuine 200% browser zoom remain **NOT_RUN**; desktop inspection is not a substitute. No browser clipboard was read or written.

This evidence does not establish AC-01, AC-02, AC-11, AC-12 or release readiness. **T-010 remains blocked. T-011, T-041, T-054, T-055, and all native, platform, privacy, resource, and release gates remain open.**
