---
task_id: MVP-05 (scoped milestone; no T-xxx task assigned)
feature_ids: [F-03, F-04, F-06, F-07, F-09, F-10, F-12]
flows: [UF-03, UF-04, UF-05, UF-07, UF-08, UF-10]
acceptance: [AC-03, AC-04, AC-06, AC-07, AC-09, AC-10, AC-12]
owner: Codex
base_commit: 40a1a7547b15c930bbf1712a5f67e45ab8d6367f
result_commit: 13f8d58f54f0440fcc03ded370dcac5e90910bed
platform: macOS 26.6.2 arm64; host RAM NOT_MEASURED
implementation: crates/coordinator owns the reducer/event sequencer and ordered adapter routing; reducer Copy acknowledgments use checked session-scoped attempt IDs
metrics: NOT_MEASURED (no native app, model, hardware, RSS, latency, WER, or installer measurement)
doc_updates: [README.md, docs/ARCHITECTURE.md, docs/TASKS.md]
risks: Native resource closure, focus verification, insertion, clipboard, UI, model, and release behavior remain unimplemented or unverified; unresolved speech cleanup blocks a subsequent session
review: PENDING
---

# MVP-05 — Safe session effect coordination

This is synthetic orchestration evidence for a Rust coordinator boundary. The coordinator routes effects from the existing reducer through deterministic mock speech, target, preview, result-panel, and explicit-Copy adapters. It calls the existing cleanup pipeline once per accepted final with a cloned per-session cleanup and vocabulary snapshot. No third-party runtime dependency was added.

## Changes

- Added `outflow-coordinator`, which owns one `SessionReducer` and `EventSequencer`, dispatches ordered effects through `SpeechBackend` and `TargetService`, and exposes narrow private UI/Copy ports.
- Added a checked, one-based `CopyAttemptId` scoped to a session. Unsolicited, stale, duplicate, or mismatched completions are rejected; repeated pending clicks emit no second effect; failed attempts retain text and permit a later explicit retry; dismissal clears the session and invalidates acknowledgments.
- Queue failures are converted to operation-specific reducer events or conservative fallbacks. A failed stop prevents finalization. Accepted insertion with unknown side-effect status remains uncertain and requires manual review. Speech-resource closure remains `UNRESOLVED` until a matching closure callback arrives.
- Replaced the prior manually driven settings example with a mock-adapter coordinator example. It reports states, effect labels, and sanitized outcomes only.
- Updated README and architecture/task notes. T-010 remains blocked. T-023, T-031, T-034, T-042 and all mapped acceptance criteria remain open; this evidence does not change their status.

## Verification

Environment: macOS 26.6.2, arm64; Rust/Cargo 1.99.1, Node 22.23.3, npm 10.9.9, Python 3.13.13. Host RAM and native release measurements were not taken.

| Command | Result | Notes |
|---|---|---|
| `npm ci` | PASS | Installed 204 packages from the checked-in lockfile. |
| `npm run typecheck` | PASS | No diagnostics. |
| `npm run lint` | PASS | No diagnostics. |
| `npm run test -- --run` | PASS | 160 tests passed across 11 files. |
| `npm run build` | PASS | Typecheck and Vite production build completed. |
| `cargo fmt --all -- --check` | PASS | Final workspace format check. |
| `cargo check --workspace --locked` | PASS | All workspace crates checked. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | Strict warning-free workspace check. |
| `cargo test --workspace --locked` | PASS | 566 unit/integration tests and 11 doctests passed. Coordinator integration suite: 26 passed; core reducer integration suite: 30 passed. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS | 629 tests run: 628 passed, 1 skipped. |
| `cargo run -p outflow-coordinator --example session_coordinator --locked` | PASS | Output below; mock-only and content-free. |
| `git diff --check` | PASS | Repeated after the evidence file was added; no whitespace errors. |

The example printed:

```text
Outflow session coordinator example (synthetic)
Push-to-talk: Idle; outcome=guarded_insertion_succeeded
  effects: RequestModelPreparation, RequestMicrophoneStart, PublishPrivatePartial, PublishPrivatePartial, RequestMicrophoneStop, RequestAsrFinalization, PrepareFinalText, RequestTargetVerification, AttemptInsertionOnce, ClearPrivateResult
Changed target: CopyReady -> Idle; outcome=explicit_copy_succeeded
  effects: RequestModelPreparation, RequestMicrophoneStart, PublishPrivatePartial, PublishPrivatePartial, RequestMicrophoneStop, RequestAsrFinalization, PrepareFinalText, RequestTargetVerification, ShowCopyResult, ExplicitCopyRequested, DismissAndClearResult
Copy retry: CopyReady -> CopyReady -> Idle; outcome=first_failed_text_retained_then_retry_succeeded
  effects: RequestModelPreparation, RequestMicrophoneStart, PublishPrivatePartial, PublishPrivatePartial, RequestMicrophoneStop, RequestAsrFinalization, PrepareFinalText, RequestTargetVerification, ShowCopyResult, ExplicitCopyRequested, CopyFailed, ExplicitCopyRequested, DismissAndClearResult
Pending cancellation: Cancelled; outcome=late_ack_rejected:true; cleanup=RESOLVED
  effects: RequestModelPreparation, RequestAsrCancellation, ClearPrivateResult
Native microphone, model, insertion, clipboard, UI shell, network, persistence: NOT USED
```

One Python test was skipped and is not counted as a pass. A first strict Clippy run found two style diagnostics in the new coordinator; the adapter constructor was grouped into `CoordinatorAdapters`, the conditional was simplified, and the exact strict Clippy command passed after the fixes.

## Unresolved behavior and limits

- Mocks acknowledge speech-resource closure. If a real adapter cannot confirm Stop/Cancel cleanup, coordinator status remains `UNRESOLVED` and blocks another session; native teardown, thread joining, or microphone release is not proven here.
- The target adapter contract requires immediate focus/protection revalidation and correct uncertainty classification, but no OS adapter or target matrix was exercised. The successful insertion is a mock outcome only.
- Preview and result-panel adapters are in-memory interfaces. Copy is explicit and correlated, but no real clipboard implementation exists.
- No microphone, Moonshine inference/model acquisition, Tauri shell, native shortcuts, OS insertion, network, transcript persistence, or release-size/RSS/latency/quality measurement was used.
- This synthetic result does not complete T-023, T-031, T-034, T-042, AC-03/04/06/07/09/10/12, or any release gate. T-010 remains blocked.

## Git

Implementation/documentation commit: `13f8d58f54f0440fcc03ded370dcac5e90910bed`, parent `40a1a7547b15c930bbf1712a5f67e45ab8d6367f`. Evidence is recorded separately in the follow-up commit.
