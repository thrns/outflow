---
task_id: MVP-08
feature_ids: [F-10, F-11, F-12]
flows: [UF-07, UF-08, UF-09]
acceptance: [AC-09, AC-10, AC-11]
owner: Codex / feature/mvp-copy-lifecycle-ui
base_commit: 1666a6005328803d35f5dcdadffee2a317a41436
result_commit: 2fd9983fefaf726195b059ea4fe9046fd331f346
platform: macOS 26.6.2 / arm64; Node v22.23.3; npm 10.9.9; Rust 1.99.1; CPU model and RAM not verified in this task
implementation: Parent-owned checked result-lifecycle counter in Home presentation state; CopyResultPanel request scopes combine lifecycle and per-mount attempt numbers; development-only mock routes failure/retry/success through the component's request and acknowledgment props. No runtime dependency, lockfile, Rust, Tauri, clipboard, microphone, target insertion, or persistence changes.
commands:
  - {command: 'npm ci', result: PASS, notes: '204 packages installed; package.json and package-lock.json unchanged'}
  - {command: 'npm run typecheck', result: PASS, notes: 'TypeScript native compiler'}
  - {command: 'npm run lint', result: PASS, notes: 'ESLint'}
  - {command: 'npm run test -- --run', result: PASS, notes: '15 files; 206 tests passed'}
  - {command: 'npm run build', result: PASS, notes: 'Vite production build; 32 modules transformed'}
  - {command: 'npm run dev -- --host 127.0.0.1', result: PASS, notes: 'Local Vite preview served at 127.0.0.1:5173 during browser inspection'}
  - {command: 'cargo fmt --all -- --check', result: PASS, notes: 'Workspace format check'}
  - {command: 'cargo check --workspace --locked', result: PASS, notes: 'Workspace check'}
  - {command: 'cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'Workspace Clippy with warnings denied'}
  - {command: 'cargo test --workspace --locked', result: PASS, notes: '591 tests passed including doctests; a quiet summary rerun confirmed the count'}
  - {command: 'cargo test --workspace --locked --quiet', result: PASS, notes: '591 passed; 0 failed'}
  - {command: "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v", result: PASS, notes: '629 tests; one skipped'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors at final source/document review'}
metrics: NOT_MEASURED (physical 4 GB performance, process RSS, installed size, ASR quality, and native latency)
doc_updates: README.md current preview commands and MVP-08 behavior; docs/ARCHITECTURE.md future bridge identity rule; docs/TASKS.md conservative milestone note and M3 evidence link.
risks: >-
  No native bridge or clipboard writer exists. The browser request token is not
  authorization; a future bridge must correlate it with the Rust coordinator's
  actual SessionId and CopyAttemptId. T-010 remains blocked. T-046, T-047,
  T-050, T-055, AC-09, AC-10, AC-11, and release gates remain open. The in-app
  browser surface did not expose a 320 CSS px viewport or 200% zoom control;
  those visual checks are NOT_RUN. A Playwright import through the available
  Node REPL failed because its module loader rejected a default export, and
  Vite preview could not bind another local port (EPERM). The visible
  desktop-width browser inspection did verify the long Unicode line wraps, the
  failure/retry/success flow, uncertain review, protected-target acknowledgment,
  and keyboard activation with Space/Return. Dismiss during a held pending
  request was covered by component tests with delayed acknowledgments. No
  screen-reader audit was performed.
review: PENDING
---

## Behavior covered

- Every result scenario gets a parent-owned numeric lifecycle ID. The counter checks `Number.isSafeInteger` before advancing; the component key and opaque request scope include that lifecycle ID, while each attempt uses a separate increasing suffix. Reopening the same scenario after Dismiss receives a new scope.
- Failure acknowledgment keeps the exact multiline Unicode text. A new explicit request receives a different attempt ID. Success clears only the current result; Dismiss and scenario replacement clear its state and acknowledgment.
- Uncertain insertion has no Copy Result panel until the manual review action. A protected-target result remains disabled until the user checks its acknowledgment; without an adapter the browser's ordinary Copy remains disabled.
- The development-only Copy failure scenario labels the panel action **Simulate Copy request**. The mock fails the first request, succeeds on explicit retry, and uses no timer, network, browser clipboard, microphone, or native API. Production-mode Home tests omit both the scenario selector and simulated Copy controls.

## Browser inspection

- The ChatGPT in-app browser showed the desktop-width Home view and Copy Result panel. The long, unbroken CJK line wrapped inside the selectable text area; the page showed no horizontal overflow at the inspected desktop width.
- Selecting Copy failure, activating the panel request, and retrying produced failure feedback with the text retained, then success feedback and the default unavailable state. The Copy request and retry were also activated with Space and Return from keyboard focus.
- Uncertain insertion was inspected before and after manual review. Protected target was inspected unchecked and checked; its Copy action stayed disabled because this browser preview has no clipboard adapter.
- A held pending state was not manually visible in the integrated mock because the deterministic acknowledgment is immediate. Tests hold an acknowledgment to verify pending status, dismissal, and rejection of stale success/failure callbacks.
- The selected browser did not expose viewport or browser-zoom controls. Exact 320 px and 200% zoom visual checks are **NOT_RUN**. No VoiceOver/Narrator/Orca screen-reader audit was performed, and no screenshot was committed.

## Remaining native blockers

This evidence establishes only React presentation and correlation behavior. It does not implement clipboard access, Tauri commands/events, Rust session binding, native focus validation, secure-field protection, target insertion, or OS permission flows. Simulated UI tests do not complete T-046, T-047, T-050, T-055, AC-09, AC-10, AC-11, or any release gate.
