---
task_id: MVP-06 (scoped milestone; no T-xxx task assigned)
feature_ids: [F-03, F-04, F-06, F-09, F-10, F-12]
flows: [UF-03, UF-04, UF-05, UF-07, UF-08, UF-10]
acceptance: [AC-03, AC-04, AC-06, AC-09, AC-10, AC-12]
owner: Codex
base_commit: aa52de78c1fe73d40c88649232d1263613e1717e
result_commit: 2944695363aec12364c199bc259ad080b1666068 (implementation and documentation commit; evidence follows separately)
platform: macOS 26.6.2 (25G83), arm64; CPU model and RAM not verified in this run
implementation: Checked stop-operation IDs and typed microphone-stop acknowledgments gate finalization; capture state is separate from full speech-resource closure; target submission errors distinguish guaranteed pre-dispatch rejection from unknown possible side effects; existing coordinator mocks and example exercise asynchronous outcomes
commands:
  - {command: 'npm ci', result: PASS, notes: 'Installed 204 packages from the lockfile.'}
  - {command: 'npm run typecheck', result: PASS, notes: 'No diagnostics.'}
  - {command: 'npm run lint', result: PASS, notes: 'No diagnostics.'}
  - {command: 'npm run test -- --run', result: PASS, notes: '160 tests passed across 11 files.'}
  - {command: 'npm run build', result: PASS, notes: 'Typecheck and Vite production build completed; 29 modules transformed.'}
  - {command: 'cargo fmt --all -- --check', result: PASS, notes: 'Workspace format check.'}
  - {command: 'cargo check --workspace --locked', result: PASS, notes: 'All workspace crates checked.'}
  - {command: 'cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'Strict warning-free check after correcting one redundant Option pattern.'}
  - {command: 'cargo test --workspace --locked', result: PASS, notes: '591 passed across 22 test harnesses, including doctests; 0 failed. Coordinator integration: 40 passed.'}
  - {command: "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v", result: PASS, notes: '629 tests run: 628 passed, 1 skipped.'}
  - {command: 'cargo run -p outflow-coordinator --example session_coordinator --locked', result: PASS, notes: 'Synthetic output below; no native adapters or transcript content.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors before the implementation commit.'}
metrics: NOT_MEASURED (no native microphone, model, process RSS, installed size, latency, WER, hardware qualification, or release measurement)
doc_updates: [README.md, docs/ARCHITECTURE.md, docs/TASKS.md, docs/USER_FLOWS.md]
risks: Missing stop acknowledgment remains safely pending without a timer; missing full closure keeps cleanup unresolved and blocks another session. No native adapter, Tauri shell, microphone, Moonshine, clipboard, OS insertion, or release qualification is implemented or proven. T-010 remains blocked; T-011, T-023, T-031, T-034, T-042, T-048 and mapped acceptance criteria remain open.
review: PENDING
---

# MVP-06 — Safe asynchronous operation boundaries

This is evidence for reusable Rust coordination contracts exercised through synthetic adapters. The verified base was `aa52de78c1fe73d40c88649232d1263613e1717e` on `feature/mvp-session-coordination`; work was performed on `feature/mvp-async-safety`. The development-only AppKit probe was inspected: its stop handler synchronously stops capture before requesting finalization. That Objective-C++ probe was not changed and is not a production adapter.

## Changes

- Added checked `StopOperationId` values and `MicrophoneStopCompleted { session_id, operation_id, outcome }` callbacks. Queue acceptance only records a pending stop. The coordinator submits the reducer's deferred finalization effect only for the matching `Stopped` acknowledgment.
- Added content-free `CaptureStatus` separate from `ResourceCleanupStatus`. Stop completion means capture callbacks ceased and the microphone stream was released; only the separate `SpeechResourcesClosed` acknowledgment confirms all session-owned speech resources are released.
- Cancellation and device-loss/overflow faults clear pending finalization. Stop submission rejection or a failed stop acknowledgment never finalizes. A finalization submission error dispatches failure/cancellation cleanup. Stale and duplicate stop callbacks are rejected or ignored without a second finalization.
- Changed `TargetService::submit` to return `TargetSubmissionError::{DefinitelyNotDispatched, OutcomeUnknown}`. The first maps to one CopyReady fallback; the second maps to Uncertain and manual review, with no retry or Copy. A successful submit still waits for an ordinary insertion completion. The adapter contract says to classify uncertainty conservatively because a failed native API call alone does not prove no insertion occurred.
- Extended the existing mock adapters with independently controlled submission failures, delayed/suppressed/failed/duplicated callbacks and closure acknowledgments. Tests cover complete effect order, both recording modes, Copy attempt correlation, content-free diagnostics, and 100 completed sessions with no pending callbacks, open capture, or unresolved cleanup.
- Updated the coordinator example to show a separate stop acknowledgment before finalization, cancellation while that acknowledgment is pending, unknown insertion outcome and review gating, and a separate successful CopyReady flow with a matching Copy attempt acknowledgment.

## Final example output

```text
Outflow session coordinator example (synthetic)
Push-to-talk before stop ack: Finalizing; finalization_submitted=false; outcome=waiting_for_capture_release
  submitted: RequestModelPreparation, RequestMicrophoneStart, PublishPrivatePartial, PublishPrivatePartial, RequestMicrophoneStop
Push-to-talk after stop ack: Idle; finalization_submitted=true; outcome=guarded_insertion_succeeded
  after acknowledgment: RequestAsrFinalization, PrepareFinalText, RequestTargetVerification, AttemptInsertionOnce, ClearPrivateResult
Cancel before stop ack: Cancelled; finalization_submitted=false; late_stop_ack_rejected=true; outcome=cancelled_without_finalization
  operations: RequestAsrCancellation, ClearPrivateResult
Unknown insertion submission: Uncertain -> CopyReady; copy_before_review_blocked=true; outcome=manual_review_required
  operations: ClearPrivateResult, RequestModelPreparation, RequestMicrophoneStart, PublishPrivatePartial, PublishPrivatePartial, RequestMicrophoneStop, RequestAsrFinalization, PrepareFinalText, RequestTargetVerification, AttemptInsertionOnce, RequireManualReview, ShowCopyResult
Explicit Copy from CopyReady: Idle; copy_ack=matching_attempt:true; outcome=success
  operations: DismissAndClearResult, RequestModelPreparation, RequestMicrophoneStart, PublishPrivatePartial, PublishPrivatePartial, RequestMicrophoneStop, RequestAsrFinalization, PrepareFinalText, RequestTargetVerification, ShowCopyResult, ExplicitCopyRequested, DismissAndClearResult
Native microphone, model, insertion, clipboard, UI shell, network, persistence: NOT USED
```

The `Unknown insertion submission` line reaches CopyReady only after `UncertaintyReviewed`; its `copy_before_review_blocked=true` reports that the earlier Copy request was rejected. The final Copy acknowledgment matches the attempt ID issued for that explicit action.

## Verification and limitations

The 40 coordinator integration tests pass, including delayed/failed/duplicate/stale stop acknowledgments, session and operation mismatch, cancellation and fault races, failed stop/finalization submissions, distinct capture and full-closure status, definite and ambiguous insertion submission, explicit definite-failure and uncertain completion, delayed/duplicate insertion callbacks, Copy gating and correlation, and complete PTT/toggle lifecycles. The full workspace suite passes 591 tests across 22 harnesses, including doctests. Frontend checks pass with 160 tests across 11 files. Python discovery ran 629 tests: 628 passed and one was skipped.

When stop completion is missing, state may remain `Finalizing` indefinitely; no timer promotes an unconfirmed stop to success. When full speech-resource closure is missing, cleanup remains unresolved and a later recording is blocked. These cases are intentionally fail-safe and need a real adapter acknowledgment to recover.

No microphone, Moonshine binding/inference, clipboard writer, native insertion, Tauri shell, network, transcript persistence, or app packaging was added or used. This synthetic evidence does not establish F/AC acceptance or complete T-010, T-011, T-023, T-031, T-034, T-042, or T-048. The 4 GB RAM, <950 MB RSS/installed-size, latency, WER, accessibility, OS safety, offline installer, signature, and commercial-license gates remain unmeasured or blocked.

## Git

Implementation and documentation commit: `2944695363aec12364c199bc259ad080b1666068`. This evidence is a separate follow-up commit so it can name the implementation commit without a circular SHA reference.
