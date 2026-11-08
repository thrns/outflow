task_id: MVP-07
feature_ids: [F-09, F-10, F-11, F-12]
flows: [UF-03, UF-04, UF-07, UF-08, UF-09]
acceptance: [AC-09, AC-10, AC-11]
owner: Codex / feature/mvp-home-session-ui
base_commit: 3052bccc75d4522c63b572d3ce5df6ee013bc4b8
result_commit: 00bfd80329ccb4e81da6b11c0c9ea4fe58a40626
platform: macOS 26.6.2 / arm64; Node v22.23.3; npm 10.9.9; Rust 1.99.1
implementation: Content-free typed SessionStatus, attempt-correlated CopyResultPanel, explicit uncertain-insertion review, and one-at-a-time Vite DEV synthetic scenario selector. README and task ledger updated. No Rust, package manifest, or lockfile changes.
commands:
  - {command: 'npm ci', result: PASS, notes: '204 packages installed; package manifest and lockfiles unchanged'}
  - {command: 'npm run typecheck', result: PASS, notes: 'TypeScript native compiler; final run'}
  - {command: 'npm run lint', result: PASS, notes: 'ESLint; final run'}
  - {command: 'npm run test -- --run', result: PASS, notes: '15 files; 201 tests passed'}
  - {command: 'npm run build', result: PASS, notes: 'Typecheck and Vite production build; 32 modules transformed'}
  - {command: "if rg -qF 'Inspect synthetic scenarios' dist/assets/*.js; then echo FAIL; exit 1; else echo PASS; fi; if rg -qF 'Simulate Copy action' dist/assets/*.js; then echo FAIL; exit 1; else echo PASS; fi", result: PASS, notes: 'Both quiet checks returned no matches; development controls are absent from production bundle'}
  - {command: 'cargo fmt --all -- --check', result: PASS, notes: 'Rust workspace formatting'}
  - {command: 'cargo check --workspace --locked', result: PASS, notes: 'Rust workspace'}
  - {command: 'cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'Rust workspace; warnings denied'}
  - {command: 'cargo test --workspace --locked', result: PASS, notes: '591 tests passed, including doctests; coordinator source unchanged'}
  - {command: "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v", result: PASS, notes: '629 tests run; 628 passed and one skipped'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors'}
  - {command: 'git ls-remote origin refs/heads/feature/mvp-async-safety refs/heads/feature/mvp-home-session-ui', result: PASS, notes: 'Remote base matched requested SHA; feature branch did not exist before push'}
visual_verification:
  browser: 'Brave, local Vite dev server at 127.0.0.1:5173; browser automation/accessibility tree and screenshots used'
  cases: 'Home default, CopyReady changed target, uncertain review then CopyReady, simulated Copy failure and result retention, Select All, Dismiss, Settings navigation; desktop and 320 px viewport; 200% browser zoom inspected on Home and the CopyReady reason'
  responsive: 'At 320 CSS px, document and body scroll widths matched viewport; synthetic result text wrapped without horizontal overflow.'
  keyboard: 'Scenario selection, Select All, uncertain review, simulated Copy action, and Dismiss were activated with keyboard input.'
  limitations: 'No VoiceOver audit. At 200% zoom the Copy Result card continued below the visible viewport; inspection covered the Home status, scenario selector and visible CopyReady reason. No screenshots were saved as repository artifacts.'
metrics: NOT_MEASURED (physical 4 GB performance, process RSS, installed bytes, recognition quality and native latency)
doc_updates: README.md MVP-07 instructions; docs/TASKS.md M3 progress note and evidence link. T-046, T-047, T-050, T-055 remain unchecked.
risks: 'No Tauri shell, microphone/ASR, clipboard adapter, native target insertion, or native accessibility integration. AC-09, AC-10, AC-11 and release gates remain open. Frontend mocks do not establish those acceptance criteria.'
review: PENDING

## Behavior covered

- `SessionStatus` displays Ready, Preparing, Recording, Finalizing, Inserted, No speech, CopyReady, Uncertain, Error and Unavailable presentations with accessible text and polite status announcements. Synthetic recording explicitly says no microphone is active.
- `CopyResultPanel` keeps one selectable finalized result in React memory; includes the fallback reason, explicit Copy, Select All, Dismiss and clipboard-manager warning. Without an adapter callback Copy is disabled. A matching failure acknowledgment retains the text; success and Dismiss clear the owner state.
- Uncertain insertion explains possible partial insertion and blocks Copy until one explicit manual-review acknowledgment. The preview does not claim that the target contents were checked.
- Fixed development-only scenarios replace the current state and discard prior result text. The separate simulated Copy control changes preview feedback only and does not call browser or OS clipboard APIs.

## Native and release status

The remote base SHA was verified. Frontend and workspace checks establish only code-level behavior in the tested browser and host environment. Native microphone/model operation, OS-specific focus protection and insertion, real clipboard behavior, screen-reader audits, 4 GB performance, packaging, and release acceptance were not established by this milestone. No AC or release gate is marked complete.
