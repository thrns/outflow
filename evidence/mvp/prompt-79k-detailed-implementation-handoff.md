# Prompt 79K — detailed implementation handoff

**Status: PARTIALLY VERIFIED — REAL RESULT MISMATCH.** The identified Moonshine fixture produced a nonempty final result after an observed Stop action. The session-owned source and actual AppKit final-result view reported exact parity. One explicitly approved Copy Result action returned AppKit success, but the nonempty TextEdit paste did not compare exactly with the final result. The first observed mismatch is at the TextEdit document-value comparison boundary. Its cause is not established. The earlier Prompt 79I mismatch reproduced at the same observable boundary; this run does not explain it. The separate Cancel/Dismiss regression passed. This is development-prototype evidence only; no production task or release gate is complete.

**Scope:** F-10 / UF-08 / C-07 / TS-07 / AC-10 development probe evidence for T-047. T-046, T-047, T-010, T-011, AC-10, and MVP completion remain open.

## A. Starting point and unresolved Prompt 79I result

- Repository: `thrns/outflow`, `/Users/bixxie/Desktop/Projects/outflow`.
- Starting branch: `codex/prompt79j-synthetic-copy-diagnostic`.
- Verified starting SHA: `76edf646ce8c8f5b8ab552dec093be27caa2d8d3`.
- New task branch: `codex/prompt79k-real-asr-copy-integrity`.
- The Prompt 79I handoff records that the correctly identified fixture app reported AppKit copy success and TextEdit received nonempty pasted content, but the in-memory comparison with the Outflow result did not match. Prompt 79I did not observe the Stop-button action because the 12-second timer finalized first. It did not establish whether the mismatch came from the writer, pasteboard delivery, TextEdit, or comparison interface. Prompt 79J later passed the fixed synthetic sample but explicitly left the real-result mismatch unresolved.

## B. Final-result source, display, and copy flow

1. In `SessionWorker::run()`, the final Moonshine call is `api_.transcribe_stream(..., 0, &final_transcript)`. `copy_transcript` reads the native final transcript's bounded UTF-8 lines, joins them with LF separators into `final_text`, then the worker publishes a `WorkerEventKind::Final` containing that string.
2. `MicProbeAppDelegate::handleWorkerEvent:` rejects events not accepted for the current session, transitions the accepted session to ResultReady, moves the event text into `_copy_result_text`, records `_copy_result_session_id`, and assigns `_final_text.string` from that owned source. The current session state machine plus the session ID binds the result to its owner; `_copy_result_text` is the finalized UTF-8 value passed to the copy policy.
3. For eligible identified timed-fixture runs, `updateFinalDisplayParity` reads the actual AppKit `NSTextView.string` property, converts it to UTF-8 with lossless conversion required, and compares it with `_copy_result_text` using `compare_final_result_parity`.
4. The explicit `copyResultPressed:` action calls `request_copy_result`. Once eligible, it converts the exact owned UTF-8 value before entering `write_plain_text_to_pasteboard`. The existing `pasteboard_writer.hpp` implementation remains the only general-pasteboard write path: one `clearContents`, one `writeObjects:` attempt, and AppKit-result-based feedback.

## C. Parity-check implementation and privacy

`final_result_parity.hpp` defines MATCH, MISMATCH, and UNAVAILABLE plus the pure exact comparison. It first verifies that both inputs are well-formed UTF-8, then compares the complete byte sequences as-is. It does not trim, normalize, case-fold, rewrite newlines, hash, log, store, export, or access the clipboard.

The diagnostic is created only when timed-fixture mode and a valid `--verification-id` are both present, and synthetic diagnostic mode is absent. The comparison runs only for a current ResultReady session with a nonzero accepted ID, matching result-session ID, and nonempty owned text. The display side is obtained from the real AppKit text view and losslessly encoded; it is not a second comparison copy of the C++ source string. Its UI reports only the status. MISMATCH and UNAVAILABLE disable Copy and are rejected again at the action boundary, while leaving the result selectable. Ordinary microphone and synthetic diagnostic UI/copy paths do not show or require parity.

The app continues to require current-session eligibility and explicit user activation for every general-pasteboard write. There is no automatic copy, retry, general-pasteboard read, or clipboard restoration. Cancel and Dismiss clear the owned result, session binding, parity status, and final view. No recognized text, clipboard content, transcript hash, screenshot, recording, model, or binary was committed.

## D. Match, mismatch, and fail-closed tests

`dev/macos-mic-probe/tests/core_test.cpp` injects values directly into the pure comparison and policy code. It covers identical ASCII, Unicode/emoji, and multiline strings; changed characters; whitespace differences; LF/CRLF differences; distinct Unicode normalization forms; malformed UTF-8; missing display values; valid-ID/timed-fixture eligibility; stale sessions; parity mismatch without a writer call; exactly one writer call for a matching eligible result; and cancelled/dismissed state rejection. The comparison has no clipboard dependency.

`scripts/tests/test_macos_mic_probe_privacy.py` checks that the AppKit code reads `_final_text.string`, requires lossless UTF-8 conversion, is gated to the verified fixture mode, prevents copy when parity is not MATCH, clears state on Cancel/Dismiss, and adds no pasteboard, logging, hashing, or persistence path to the comparison. A temporary AppKit text view was not required because the pure comparator is exercised directly and source-contract tests verify that the UI supplies the actual property.

## E. Automated tests and exact results

Host: macOS 26.6.2 (25G83), arm64, Mac15,6, approximately 18 GiB RAM. The host is not physical 4 GB release hardware. The licensed human-reference WER fixture is unavailable and remains **SKIPPED**, not passed.

| Command | Result |
|---|---|
| Fresh `scripts/build_macos_mic_probe.sh` | PASS. Initial real-result bundle: `/private/tmp/outflow-prompt79k.rrt0gV/OutflowMicProbe.app`. A second fresh final-source bundle for Cancel and synthetic runs: `/private/tmp/outflow-prompt79k-final.CfaQfG/OutflowMicProbe.app`. |
| `scripts/test_macos_mic_probe.sh` | PASS: 16 core test groups; 9 worker-lifecycle groups including 100 injected cycles; 5 malformed verification-ID launch rejections; 4 invalid synthetic-launch rejections; isolated AppKit integration passed for 4 fixed values and 4 explicit writes on its unique test pasteboard. No general clipboard access. |
| `scripts/test_macos_mic_probe.sh --real-fixture` | PASS. Pinned 12-second WAV fixture: nonempty partial and final, 17 updates, zero queue overflows, max queue depth 1, 130.130 ms stop-to-final, native harness peak RSS 613,498,880 bytes. This is harness-only evidence, not the release process-tree memory gate or licensed WER. |
| `python3 -m unittest scripts.tests.test_macos_mic_probe_privacy -v` | PASS: 22 tests. |
| `python3 -m unittest discover -s scripts/tests -v` | PASS: 395 passed, 1 skipped, 0 failed. The skipped test is `test_verified_fixture_runs_through_real_pinned_native_asr`; its licensed human-reference corpus environment was not configured. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS: 353 passed, 0 failed, 0 ignored across the reported suites/doctests. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS. |
| `npm run typecheck` | PASS. |
| `npm run test -- --run` | PASS: 10 files, 136 tests. |
| `npm run lint` | PASS. |
| `npm run build` | PASS. |
| `bash -n scripts/build_macos_mic_probe.sh scripts/run_macos_mic_probe.sh scripts/test_macos_mic_probe.sh` | PASS. |
| `git diff --check` | PASS after final documentation and evidence edits. |

The real-fixture harness metrics above are not a human-reference quality result. The separate licensed WER test remains skipped. No release hardware, installer, platform, or production acceptance gate is marked complete.

## F. Real Moonshine fixture UI verification

- Fresh built app: `/private/tmp/outflow-prompt79k.rrt0gV/OutflowMicProbe.app`.
- Verified visible window: `Outflow Mic Probe — Dev — 79K-REAL-COPY-01 — PID 87508`.
- Read-only `lsof -a -p 87508 -d txt` confirmed the executable at `/private/tmp/outflow-prompt79k.rrt0gV/OutflowMicProbe.app/Contents/MacOS/OutflowMicProbe` before interaction.
- Start timed fixture: **YES**. Nonempty partial while Listening: **YES**. Stop button clicked while Listening: **YES**. ResultReady reached with nonempty actual final: **YES**.
- Source/display parity: **MATCH**. Copy Result enabled: **YES**. Clipboard replacement/privacy warning reviewed: **YES**.
- Copy Result clicked: **YES, once after explicit operator approval**. AppKit write status: **SUCCESS**.
- TextEdit target: **YES**, a new blank, unsaved Untitled document, switched to plain-text mode, with its text insertion view focused. One normal ⌘V was performed and nonempty pasted text was observed.
- Exact final/paste equality: **MISMATCH** in the in-memory full-string comparison. The first observed failure is at the destination-document comparison boundary after AppKit success. No retry or direct pasteboard read was performed.
- Visual comparison: **UNVERIFIED**. No screenshot or transcript-bearing visual artifact was saved or included.
- TextEdit document: discarded without saving. Outflow Dismiss cleared the final result and parity and disabled Copy.

Only sanitized outcomes were retained. The recognized words, full TextEdit value, and clipboard contents are absent from this handoff.

## G. Copy Result, AppKit, and TextEdit outcome

| Required metadata | Observation |
|---|---|
| Correct app/window | YES — exact verification ID, PID, and executable path matched the fresh fixture build. |
| Stop clicked | YES — while Listening, before timed auto-finalization. |
| Nonempty final | YES. |
| Outflow source/display parity | MATCH. |
| Copy Result clicked | YES — once. |
| AppKit write | SUCCESS. |
| TextEdit paste | OBSERVED — nonempty paste into the verified blank plain-text document. |
| Exact final/paste equality | MISMATCH. |
| Visual comparison | UNVERIFIED. |

## H. Prompt 79I mismatch classification

The Prompt 79I mismatch **reproduced** at the same observable TextEdit equality comparison. The source-to-final-view boundary passed exact byte parity, Copy Result returned AppKit success, and TextEdit exposed a nonempty pasted document value that did not compare equal to the Outflow final view. This is the first observed differing boundary.

This evidence does **not** identify whether the content changed during pasteboard transfer, in TextEdit, or through the accessibility/in-memory comparison interface. The system clipboard was never read. Since visual equality is unverified, do not call this a clipboard-writer defect or claim the cause is explained.

## I. Cancel and privacy regressions

- A fresh fixture was launched with ID `79K-CANCEL-01` and a 120-second limit from the final build. The app reached Listening with a nonempty partial. Cancel produced the cancelled/discarded state, stopped fixture production, cleared partial/final results and parity status, and kept Copy Result disabled. No final was committed and no copy action occurred.
- Dismiss returned the cancelled session to Idle with final result and parity cleared and Copy disabled. No clipboard write occurred.
- The separate synthetic diagnostic was rerun from the fresh final build as `79K-SYNTH-01`. The fixed sample appeared in its existing result area; Copy Result was enabled and the existing warning was visible. No parity control appeared. It was dismissed without clicking Copy, so no second general-pasteboard replacement occurred.
- Automated comparator and privacy tests do not read or modify the system general clipboard. The isolated AppKit integration uses only its unique test pasteboard. The one manual real-result copy was the single operator-approved clipboard replacement; there was no speculative replacement retry, direct clipboard read, or restoration.

## J. Reproduction commands and troubleshooting

The run used already-authorized local assets from Moonshine revision `234f60faa0eb388b01cdf7e60aca232af37aefda`; fixture and model stayed outside the repository.

```sh
export MOONSHINE_SOURCE_DIR=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_BUILD_DIR="$(mktemp -d /private/tmp/outflow-prompt79k.XXXXXX)"
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"

scripts/run_macos_mic_probe.sh \
  --verification-id "79K-REAL-COPY-01" \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 120
```

Before UI interaction, verify the title's ID/PID and the executable path with `lsof -a -p "$PID_FROM_WINDOW" -d txt`. Start, wait for a partial, click Stop, inspect parity, review the warning, and do not copy unless the operator explicitly approves the single replacement. For TextEdit, create a new blank unsaved plain-text document, focus its insertion view, paste once, compare the complete value without normalization, and discard without saving. Do not inspect the general pasteboard or retry after mismatch.

For the separate Cancel check, use a fresh ID and the same real-fixture arguments with `--fixture-seconds 120`; press Start, wait for a nonempty partial, Cancel, verify the result/parity clear and Copy stays disabled, then Dismiss. For the synthetic regression, use a fresh build/app and `scripts/run_macos_mic_probe.sh --verification-id "79K-SYNTH-01" --synthetic-copy-diagnostic`; verify the sample, warning, and enabled Copy button. Do not click Copy without separate authorization.

If the UI connector binds another probe window, stop and check the exact ID/PID and executable path before any interaction. If LaunchServices returns `kLSNoExecutableErr` from a restricted shell, rerun through the authorized desktop launch context; do not interpret shell launch failure as a pasteboard result. If the TextEdit value mismatch recurs, preserve the selectable result and report the boundary; do not inspect clipboard data or retry. If the desktop app or comparison value is inaccessible, classify the desktop test as blocked or equality as unverified.

## K. Remaining production blockers

This work changes only the development AppKit probe, pure native comparison/policy tests, privacy source-contract tests, README evidence, and task evidence. No production Tauri integration, OS-wide insertion, new model, dependency, release gate, or GitHub Actions file was added. T-046/T-047, T-010/T-011, AC-10, physical 4 GB validation, aggregate release RSS and installed-size limits, licensed WER, latency/quality corpus acceptance, supported platform/focus/protected-field matrix, packaging/signing/notarization, and legal/accessibility reviews remain unchanged and open. The 12-second harness RSS observation does not meet or substitute for those gates.

## L. Git delivery record

- Branch: `codex/prompt79k-real-asr-copy-integrity`.
- Starting SHA: `76edf646ce8c8f5b8ab552dec093be27caa2d8d3`.
- Implementation commit: `c97cd1e6ff7ad21e6642fd1939f15c749b1a0f7f` (`test(native): verify finalized ASR display and copy parity`).
- GitHub repository/branch: `thrns/outflow`, `origin/codex/prompt79k-real-asr-copy-integrity`.
- The first push was verified with `git ls-remote`; the remote head matched implementation commit `c97cd1e6ff7ad21e6642fd1939f15c749b1a0f7f`. A documentation-only update to record this verification is being delivered as a follow-up commit. Since a commit cannot contain its own object ID, the final evidence-update SHA and final remote-head verification are reported in the delivery message.
- Changed files: `README.md`; `docs/TASKS.md`; `dev/macos-mic-probe/app.mm`; `dev/macos-mic-probe/copy_result_policy.hpp`; `dev/macos-mic-probe/final_result_parity.hpp`; `dev/macos-mic-probe/tests/core_test.cpp`; `scripts/tests/test_macos_mic_probe_privacy.py`; this handoff.
- No dependency manifest or lockfile changed. No speech, clipboard data, screenshot, recording, model asset, or binary is tracked.
- The delivered worktree is clean and the task branch tracks its pushed remote.
