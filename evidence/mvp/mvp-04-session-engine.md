---
task_id: MVP-04 (no canonical T-xxx assigned)
feature_ids: [F-03, F-04, F-05, F-06, F-07, F-09, F-10, F-12]
flows: [UF-03, UF-04, UF-05, UF-07, UF-08]
acceptance: [AC-03, AC-04, AC-05, AC-06, AC-07, AC-09, AC-10, AC-12]
owner: Codex primary agent; isolated clone on feature/mvp-session-engine
base_commit: 9ab47786fcad17eb82971878b9f7ef7d2769a9e0
result_commit: c377229aa9e61db19cbf59511b26e7fba6b09b42 (session-engine implementation and synchronized documentation; this evidence is recorded in the following documentation commit)
platform: macOS 26.6.2 (25G83), arm64; rustc/cargo 1.99.1; Node.js v22.23.3/npm 10.9.9; Python 3.13.13; physical RAM not measured
implementation: Pure Rust C-02 reducer in crates/core/src/session_engine.rs; 28 reducer integration tests; synthetic mock-effects example; duplicate settings vocabulary-ID validation; cleanup handoff integration test.
commands:
  - {command: 'npm ci', result: PASS, notes: 'Installed 204 locked packages.'}
  - {command: 'npm run typecheck', result: PASS, notes: 'No TypeScript errors.'}
  - {command: 'npm run lint', result: PASS, notes: 'ESLint completed.'}
  - {command: 'npm run test -- --run', result: PASS, notes: '11 files, 160 tests.'}
  - {command: 'npm run build', result: PASS, notes: 'Vite production build completed.'}
  - {command: 'cargo fmt --all -- --check', result: PASS, notes: 'Workspace formatting clean.'}
  - {command: 'cargo check --workspace --locked', result: PASS, notes: 'Locked workspace checked.'}
  - {command: 'cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'cargo test --workspace --locked', result: PASS, notes: '549 tests passed across unit, integration, and doc-test targets.'}
  - {command: 'cargo run -p outflow-settings --example session_engine --locked', result: PASS, notes: 'Synthetic example completed with the deterministic output recorded in README.md.'}
  - {command: "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v", result: PASS, notes: '629 tests passed; one existing test skipped.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors before commits.'}
  - {command: "! rg -n '^\\s*(use|extern crate)\\s+(tauri|cpal|moonshine|tokio|reqwest|ureq|std::fs|std::net)' crates/core/src/session_engine.rs", result: PASS, notes: 'No platform, filesystem, network, native inference, or async-runtime imports in the reducer.'}
metrics: Session correctness was exercised with synthetic events only. Model qualification, WER, latency, aggregate RSS, installed size, battery, physical 4 GB hardware, and native OS integration are NOT_MEASURED / NOT_RUN.
doc_updates: README.md (run instructions, expected example output, troubleshooting); docs/ARCHITECTURE.md (reducer, target, ID, and settings concurrency behavior); docs/TASKS.md (partial reusable-core status); this evidence record.
risks: T-023 remains open, so T-031 is not complete. Native shortcut registration, microphone capture, ASR, UI/Tauri wiring, target revalidation and insertion adapters, real clipboard behavior, and supported-platform checks are absent. T-034, T-042 and their mapped acceptance criteria remain incomplete; no acceptance criterion was marked complete. T-010 remains blocked and T-011 remains unimplemented. The existing source checkout's .git/FETCH_HEAD was not writable; the remote base SHA was verified read-only through GitHub, and implementation ran in a separate writable clone rooted at that exact SHA.
review: PENDING
---

## Outcome

The reducer now implements `Uninitialized`, `Idle`, `Loading`, `Listening`, `Finalizing`, `Committing`, `CopyReady`, `Uncertain`, `Cancelled`, and `Faulted`. Its typed events cover start/stop, model and microphone acknowledgments/failures, captured input, partial and final transcripts, cleanup success/failure, target verification, insertion outcomes, explicit Copy, dismissal, cancellation, device loss, and bounded-buffer overflow. Typed effects describe future work without performing it.

A checked process-wide atomic counter provides monotonic, nonzero session IDs. Each reducer accepts one strictly increasing event sequence. The engine rejects stale/duplicate callbacks, guards repeated push-to-talk key-down, ignores toggle key-up, and permits one active session per reducer. Cancellation and dismissal clear volatile session data. Final text crosses into the existing settings cleanup function once; Clean and Verbatim remain distinct. Matching editable ordinary targets produce one InsertOnce effect. Changed, protected, unsupported, or definitely failed insertions produce CopyReady; uncertain outcomes require manual review before CopyReady. Copy effects occur only in response to an explicit user action.

The duplicate-ID investigation confirmed that schema-v1 settings previously allowed two entries with the same stable ID. Strict decoding and the vocabulary-review decoder now reject this with an index-only `DuplicateVocabularyId` error. Tests verify that strict loading, service open, and rejected writes preserve the original file. Unicode 18 phrase validation and migration behavior were left unchanged.

## Mock example output

```text
Outflow Rust session engine example (synthetic)
Native shortcuts: NOT INSTALLED
Session 1: Idle; insertion_attempted=true
  effects: RequestModelPreparation, RequestMicrophoneStart, PublishPrivatePartial, PublishPrivatePartial, RequestMicrophoneStop, RequestAsrFinalization, PrepareFinalText, RequestTargetVerification, AttemptInsertionOnce, ClearPrivateResult
Session 2: CopyReady -> Idle; target changed; explicit Copy requested
  effects: RequestModelPreparation, RequestMicrophoneStart, RequestMicrophoneStop, RequestAsrFinalization, PrepareFinalText, RequestTargetVerification, ShowCopyResult, ExplicitCopyRequested, DismissAndClearResult
Session 3: Cancelled; late final rejected=true
  effects: RequestModelPreparation, RequestAsrCancellation, ClearPrivateResult
Native model, microphone, insertion, clipboard, persistence, network: NOT USED
```

## Settings-service concurrency note

Each `SettingsService` instance owns an independent snapshot. If simultaneous instances write stale snapshots, atomic replacement prevents a torn file but the last replacement wins and may overwrite the other update. Writer serialization remains caller-owned; this milestone adds no lock, watcher, or concurrent-write mechanism.

## Verification boundary

All unit and synthetic integration checks passed on the recorded macOS arm64 development host. They do not prove microphone, Moonshine, Tauri, OS insertion, clipboard, accessibility, Windows, Linux, Wayland, model-quality, or release-resource behavior. No task checkbox or acceptance criterion was changed to complete.
