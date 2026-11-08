---
task_id: MVP-08
feature_ids: [F-10, F-11, F-12]
flows: [UF-07, UF-08, UF-09]
acceptance: [AC-09, AC-10, AC-11]
owner: Codex / feature/mvp-copy-lifecycle-ui follow-up
base_commit: 548c9cd871385703562521f80c92abc47faca1d8
result_commit: 8cd6393b38baba5f634899231c7047f6508c6233
platform: macOS 26.6.2 / arm64; Node v22.23.3; npm 10.9.9; Rust 1.99.1; CPU model and RAM not verified in this task
implementation: App owns a checked, volatile monotonic lifecycle counter and passes a narrow allocator callback to Home. Home result/acknowledgment state still unmounts when Settings opens. IDs are correlation metadata only and are unique only while the App root remains mounted. No runtime dependency, lockfile, Rust, Tauri, clipboard, microphone, target insertion, or persistence changes.
commands:
  - {command: 'npm ci', result: PASS, notes: '204 packages installed; package.json and package-lock.json unchanged'}
  - {command: 'npm run typecheck', result: PASS, notes: 'TypeScript native compiler'}
  - {command: 'npm run lint', result: PASS, notes: 'ESLint'}
  - {command: 'npm run test -- --run', result: PASS, notes: '15 files; 208 tests passed'}
  - {command: 'npm run build', result: PASS, notes: 'Vite production build; 32 modules transformed'}
  - {command: 'npm run dev -- --host 127.0.0.1', result: PASS, notes: 'Local Vite preview served at 127.0.0.1:5173; manual Home → Settings → Home and reopen verified IDs copy-result-1 then copy-result-2'}
  - {command: 'cargo fmt --all -- --check', result: PASS, notes: 'Workspace format check'}
  - {command: 'cargo check --workspace --locked', result: PASS, notes: 'Workspace check'}
  - {command: 'cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'Workspace Clippy with warnings denied'}
  - {command: 'cargo test --workspace --locked', result: PASS, notes: '591 tests passed: 580 unit/integration tests and 11 doctests'}
  - {command: 'cargo test --workspace --locked --quiet', result: PASS, notes: '591 passed; 0 failed'}
  - {command: "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v", result: PASS, notes: '629 tests; one skipped'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors at final source/document review'}
metrics: NOT_MEASURED (physical 4 GB performance, process RSS, installed size, ASR quality, and native latency)
doc_updates: README.md MVP-08 lifecycle ownership, scope limitations, and verification results; this evidence file. Existing docs/ARCHITECTURE.md and docs/TASKS.md bridge/milestone rules remain unchanged.
risks: >-
  No native bridge or clipboard writer exists. A frontend lifecycle/request token
  is correlation metadata, never authorization; a future bridge must validate
  the Rust coordinator's actual SessionId and CopyAttemptId. IDs are not unique
  across full App-root remounts, page reloads, or application restarts because
  no persistence is authorized or implemented. T-010 remains blocked. T-046,
  T-047, T-050, T-055, AC-09, AC-10, AC-11, and release gates remain open. The
  selected in-app browser supported a 320 CSS px viewport but exposed no browser
  zoom control; 200% zoom is NOT_RUN. No screen-reader audit was performed.
review: PENDING
---

## Behavior covered

- Every result scenario receives an App-root-allocated numeric lifecycle ID. The checked counter advances only within one mounted App lifecycle; the component key and opaque request scope include the ID, while each attempt uses a separate increasing suffix. Dismiss/reopen and Home → Settings → Home receive new scopes. IDs contain no transcript, target, or vocabulary data.
- Regression proof: before the fix, the new Home → Settings → Home test failed because both results had `copy-result-1`; after the fix, the first result is `copy-result-1` and the reopened result is `copy-result-2`. The same test verifies the result is absent in Settings and both destination headings receive focus.
- A separate full React remount test confirms an active result is not restored. Existing same-mount Dismiss/reopen and CopyResultPanel stale success/failure acknowledgment tests pass.
- Failure acknowledgment keeps the exact multiline Unicode text. A new explicit request receives a different attempt ID. Success clears only the current result; Dismiss and scenario replacement clear its state and acknowledgment.
- Uncertain insertion has no Copy Result panel until the manual review action. A protected-target result remains disabled until the user checks its acknowledgment; without an adapter the browser's ordinary Copy remains disabled.
- The development-only Copy failure scenario labels the panel action **Simulate Copy request**. The mock fails the first request, succeeds on explicit retry, and uses no timer, network, browser clipboard, microphone, or native API. Production-mode Home tests omit both the scenario selector and simulated Copy controls.

## Browser inspection

- The local Vite preview was opened in the ChatGPT in-app browser. Copy failure created `copy-result-1`; Settings removed the result and focused its heading; returning Home focused the Home heading; reopening Copy failure created `copy-result-2`.
- At a 320 × 900 CSS px browser viewport, `window.innerWidth`, `document.documentElement.clientWidth`, and `scrollWidth` were all 320; no horizontal overflow was observed. The viewport override was reset afterward.
- The simulated failure retained the result and displayed its failure status; explicit retry cleared it and announced success. The preview made no browser clipboard, microphone, or Tauri call; integration tests spy on all three APIs and confirm they are not called.
- 200% browser zoom is **NOT_RUN**: the in-app browser exposed viewport sizing but no zoom control, and its keyboard shortcut attempt did not change zoom. No VoiceOver/Narrator/Orca screen-reader audit was performed, and no screenshot was committed.
- Component tests hold delayed acknowledgments to verify pending status, dismissal, and rejection of stale success/failure callbacks; the full suite reports 208 passing tests across 15 files.

## Remaining native blockers

This evidence establishes only React presentation and correlation behavior. It does not implement clipboard access, Tauri commands/events, Rust session binding, native focus validation, secure-field protection, target insertion, or OS permission flows. Simulated UI tests do not complete T-046, T-047, T-050, T-055, AC-09, AC-10, AC-11, or any release gate.
