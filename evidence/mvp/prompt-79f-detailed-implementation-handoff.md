# Prompt 79F — detailed implementation handoff

**Status: VERIFIED for the requested development prototype.** The cancellation lifecycle is implemented and automated, the final development app rebuilt, visible fixture Cancel/Dismiss/restart was verified, and live microphone Cancel, restart/Stop, and early Cancel were physically observed on the macOS development machine. The code remains a development-only AppKit/AVFoundation probe. This work does not complete production dictation tasks or release gates. No raw audio, recognized words, screenshots, or model files are stored in this handoff.

## 1. Starting implementation and defect addressed

Prompt 79F started from `be88ca6ae78bc5da4e68764e905093081e1645e0` on `codex/prompt79e-mic-prototype`, with a clean worktree. Work was moved to the task branch `codex/prompt79f-mic-cancellation` before edits. The starting app had a permission completion that only tested `granted` and `Idle` before calling `beginSession`. A delayed grant therefore had no ownership token tying it to the request, session, current authorization, or open window. The user could not cancel while this request remained outside the state machine.

The earlier Cancel handler marked the state cancelled and stopped the capture source, but it only requested worker cancellation and returned to the UI without synchronously joining the worker. In-flight partial/final events could still reach the main queue, and Dismiss did not consistently wait for terminal native cleanup before a restart. The change repairs these paths in the existing session state machine and app delegate; it does not add another lifecycle framework.

## 2. Changed files and main functions

- `dev/macos-mic-probe/app.mm`: `startPressed:`, `prepareSession`, `startWorkerForSession:`, `invalidatePermissionRequest`, `cancelPressed:`, `dismissPressed:`, `handleWorkerEvent:`, `stopSourcesForShutdown`, `cancelActiveSessionForDismissal`, and app/window shutdown handlers.
- `dev/macos-mic-probe/permission_policy.hpp` and `.cpp`: pure current-request and current-authorization guards.
- `dev/macos-mic-probe/session_worker.hpp` and `.cpp`: cancellation checks around model/stream operations, queue consumption, partial polling, and final drain; injected partial poll interval; synchronous join/queue scrub; volatile text/audio clearing; native stream/transcriber cleanup.
- `dev/macos-mic-probe/audio_pipeline.cpp`: non-elidable zeroing of audio queue slots and prior resampler output before reuse.
- `dev/macos-mic-probe/tests/core_test.cpp`: permission-generation, cancellation-event rejection, restart, queue, and state-cycle coverage.
- `dev/macos-mic-probe/tests/worker_test.cpp`: deterministic worker lifecycle cases and 100 injected cycles.
- `scripts/test_macos_mic_probe.sh`: links the session state into the worker test binary.
- `scripts/tests/test_macos_mic_probe_privacy.py`: permission, callback ordering, no-content persistence, stale callback, and dismissal contracts.
- `scripts/run_macos_mic_probe.sh`: launches a fresh bundle process with `open -n -a`, preserving command-line fixture/metrics arguments.
- `README.md`: physical cancellation, fixture verification, test, expected state, and recovery instructions.
- `docs/TASKS.md`: Prompt 79F evidence note only; no production task checkbox or release gate changed.
- This handoff.

## 3. Permission callback and session-generation ownership

When Start requires a normal macOS permission request, `prepareSession` first reserves a session ID in `Preparing`. The app records a monotonically increasing permission-request generation and a pending flag. The completion handler captures both that generation and the reserved session ID, captures the delegate weakly, and dispatches to the main queue.

Before doing anything, the callback requires: the delegate still exists; the request is still pending and its generation is current; the window/app is not closing; the state still accepts the captured session ID; and the state is still `Preparing`. It then re-reads AVFoundation authorization and starts the worker only if the callback says granted and the current authorization is still `Authorized`. Otherwise it fails the still-current session without opening capture. Cancel, Dismiss, window close, and app termination invalidate the pending generation. A new Start gets a new session ID and request generation.

## 4. Cancel data flow and native ownership

`cancelPressed:` first transitions the session to `Cancelled`, which gates later events. It timestamps the request, snapshots whether AVAudioEngine was active, invalidates any permission request, and calls `stopSourcesForShutdown`. That stops and joins the timed fixture producer or calls `MacMicrophoneCapture::stop()` before worker cancellation.

If a worker exists, Cancel records queue depth/high-water/overflow metadata, sets the worker's cancel flag, wakes the queue wait, joins the worker, and resets it. Worker cancellation checks prevent new queued blocks from being fed and suppress partial/final publication after the native call returns. The worker stops/frees the Moonshine stream and frees the transcriber before join returns; `join()` then clears all queue slots after the producer has stopped. RAII scrubbers clear resampled audio and temporary transcript strings. The app clears both visible text panes and reports `Cancelled` only after cleanup is complete. The Cancel handler does not call the final metrics writer, clipboard APIs, or insertion APIs.

Dismiss stops any remaining source, joins/resets any terminal worker, clears volatile result fields, and only then transitions the state machine from the terminal state to `Idle`. This makes a second Start safe after Cancel. Repeated Cancel is harmless because a terminal Cancelled state rejects another transition.

## 5. Rejection of late permission and worker events

The pure permission policy rejects stale generations, mismatched sessions, non-pending requests, closing windows, and non-Preparing states. Authorization is checked again at callback time rather than trusting the captured grant result alone. A Cancel that wins during model loading causes the worker to release a loaded transcriber without creating or starting a stream and without emitting Ready.

The app accepts Partial only while the matching session is Listening. Final is committed only if `SessionState::ready(session_id)` accepts the Finalizing session. If Cancel won, the final is scrubbed and discarded. Cancelled/Error events are accepted only for active or already-cancelled matching states. Worker event text is cleared when stale, handled, rejected, or when the queued main-thread block outlives its delegate. Window/app closing invalidates permission and joins worker cleanup before native objects disappear.

## 6. Automated tests

`scripts/test_macos_mic_probe.sh` passed on the pinned local macOS arm64 runtime:

- Core binary: **12 groups**, including 100 state cycles, permission generation and restart guards, queue/wakeup/overflow behavior, callback slicing, and two fixture starts.
- Worker binary: **9 lifecycle groups**, including cancel during model load; immediately after Ready; with six queued blocks; after a nonempty partial; during final drain; repeated Cancel/Stop; model/stream release counts; and queue-slot zeroing. The suite also ran **100 synchronized start/stop/cancel/dismiss worker cycles** and verified cleanup after each.
- Python privacy/source contracts: **8 tests** for content persistence, metrics schema, lack of clipboard/injection, capture gating, permission generation/current authorization, invalidate→source-stop→cancel→join order, late callbacks, and join-before-Dismiss.

Synchronization uses injected condition-variable gates for model load, audio feed, and final transcription; the partial poll interval is injectable so tests do not depend on arbitrary sleeps. Tests assert event state, queue sample contents, API call counts, and native resource ownership rather than only return values.

## 7. Physical and visible outcomes

Host: macOS **26.6.2 (25G83)**, Apple **M3 Pro arm64**, **18 GB installed RAM**. The normal macOS microphone authorization path reached Granted; no TCC database was read or edited and no permission reset was performed. The host is not a physical 4 GB release reference machine.

**Live Test A — speech partial then Cancel (real microphone): PASS.** The app displayed a nonempty temporary partial before Cancel. Cancel stopped capture, cleared the preview and final pane, displayed Cancelled, and no later final appeared. Observed UI timings were **45.7 ms** request-to-microphone-release and **53.1 ms** request-to-worker-cleanup. No cancellation metrics file was written.

**Live Test B — Dismiss, restart, then Stop (real microphone): PASS.** Dismiss returned to Idle. A new live session displayed fresh partial and final content, reached Result ready, and released the microphone without an app restart. Sanitized UI metrics: model load **337 ms**, first frame **143 ms**, first partial **2,607 ms** after first frame, Stop-to-final **51 ms**, microphone release **48.9 ms**, queue maximum **3/16**, and displayed native high-water RSS **703 MB** across **34 threads**. No recognized wording is included here.

Tests A and B used the Prompt 79F candidate build before the final queued-event scrub and expanded Cancel queue metadata label were added. Those last changes do not alter the capture/worker cancellation ordering. The final rebuilt app then passed the visible fixture Cancel/restart checks and live Test C below.

**Live Test C — early Cancel (real microphone, final rebuilt app): PASS.** Start initially showed Preparing while the normal permission request resolved. Cancel then stopped an active microphone source; the UI showed Cancelled with empty temporary/final panes and no result. Sanitized values: microphone release **18.2 ms**, worker cleanup **28.9 ms**, queue-at-cancel **0/16**, maximum queue **7/16**, overflow **0**. The UI stated that no cancellation metrics file was written. The request raced into an active capture before the Cancel click; this test does not claim a physically observed permission grant callback arriving only after the invalidation.

**Visible fixture Cancel/Dismiss/restart: PASS on the final rebuilt app.** The app showed Listening and a nonempty partial. Cancel displayed Cancelled, both panes were empty, no final was present, and the sanitized label reported queue-at-cancel **0/16**, maximum **2/16**, overflow **0**. A separate fresh `--auto-cancel-after-ms 3500` fixture run observed a partial, then Cancelled with empty panes and no final; its unique metrics path was confirmed absent. Dismiss returned to Idle. A subsequent fixture session started, showed a partial, completed by Stop with a nonempty final, and Dismiss returned to Idle. The second completed fixture could not overwrite its already-used explicit metrics path; the app correctly reported an exclusive-create metrics-save failure and retained the UI result. It did not overwrite prior metadata.

The 12-second headless real-model fixture worker passed with `partial_nonempty=true`, `final_nonempty=true`, **12,000 ms** paced audio, **101.724 ms** Stop-to-final, **596.195 ms** mean partial cadence, **1/16** maximum queue, **zero overflow**, **637,550,592-byte** native process peak RSS, and **22 threads**. These values are local fixture smoke metadata, not WER or release certification.

An optional process-tree RSS wrapper attempt failed with `process-tree measurement failed: macOS process table could not be sampled`. The same native real-fixture suite passed without optional profiling. A window-close action was also initiated while a live session was Listening; the UI automation then lost continuity to a no-argument inspection launch, so close-during-capture cleanup timing is **INCONCLUSIVE**, not a pass.

## 8. Timing and memory measurements

The observed Cancel-to-device-release / worker-cleanup pairs were **45.7/53.1 ms** for live Test A and **18.2/28.9 ms** for live Test C. The live Stop path released in **48.9 ms** and reached final UI in **51 ms** on Test B. Queue overflow was zero in the available physical/fixture observations. The final rebuilt fixture auto-cancel showed maximum queue depth **2/16**; the final rebuilt live early-cancel showed **7/16**.

RSS measurements collected for this task are the rounded live UI value **703 MB**, the visible fixture UI value **755 MB** (28 threads), and the one-run native fixture peak **637,550,592 bytes**. Earlier Prompt 79E's distinct 30-second app-tree trace was **919,814,144 bytes** and remains separate. No complete process-tree or macOS physical-footprint study was produced for cancellation. These measurements are on an 18 GB host and do not pass the 4 GB or 950,000,000-byte release gates.

## 9. Remaining untested cases and blockers

- A normal grant reached Granted, but every TCC permission state, explicit revocation while recording, denial recovery, and a grant callback provably delivered after invalidation were not all exercised on hardware. Deterministic generation/authorization unit and source-contract tests cover stale responses without changing TCC state.
- Device removal, busy-device contention, all unsupported input formats, and queue overflow through physical AVAudioEngine hardware remain untested.
- Window close during active capture was attempted but its cleanup observation was inconclusive as described above.
- No human-verified WER corpus integration ran. The repository's Python suite reports the licensed-corpus integration as **SKIPPED** because the verified fixture corpus and its environment are absent; it is not a passing benchmark.
- No 4 GB physical machine, Tiny model comparison, installed-size/resource gate, production Rust/CPAL/Tauri integration, clipboard/insertion path, Windows/Linux path, legal approval, or release evidence was added.
- T-011, T-025–T-034, AC-06, T-010, and all production/release gates remain incomplete. `docs/TASKS.md` checkboxes were not changed.

## 10. Reproduction commands

Use the existing local source, libraries, model, and fixture; none are fetched by these commands. Build the app outside the repository:

```sh
export MOONSHINE_SOURCE_DIR=/private/tmp/outflow-feasibility/moonshine-234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
export OUTFLOW_MIC_PROBE_BUILD_DIR=/private/tmp/outflow-prompt79f-build-final-01
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_FIXTURE_SECONDS=12
export OUTFLOW_MIC_PROBE_FIXTURE_RUNS=1
scripts/test_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh --real-fixture
python3 -m unittest discover -s scripts/tests -v
```

To verify visible fixture cancellation and subsequent restart, follow the Prompt 79F README section. For a reproducible timer pass, launch with `--timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" --fixture-seconds 12 --auto-cancel-after-ms 3500 --metrics-file <new-path>`, click Start, and verify Cancelled/empty panes/no final plus an absent `<new-path>`. For physical tests, launch `scripts/run_macos_mic_probe.sh` in the logged-in desktop; perform Tests A, B, and C in the README. Use the normal permission prompt/System Settings only; never reset TCC. Do not save test speech, transcripts, audio, or screenshots.

The requested repository regression commands also passed: stable `cargo fmt --all -- --check`; `cargo check --workspace --locked`; `cargo test --workspace --locked` (**353 passed** across unit, integration and doc tests); `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo build --workspace --locked`; `npm run typecheck`; `npm run test -- --run` (**136 passed in 10 files**); `npm run lint`; `npm run build`; and `git diff --check`. The full Python suite passed **381 tests with 1 licensed-corpus integration skip**.

## 11. GitHub branch, commits, and changed paths

- Repository: `thrns/outflow`
- Development branch: `codex/prompt79f-mic-cancellation`
- Starting verified commit: `be88ca6ae78bc5da4e68764e905093081e1645e0` (`codex/prompt79e-mic-prototype`)
- Result implementation commit: `a94401a639799c23038f004a9c0bb7d396a2d101` (`fix(native): harden macOS microphone cancellation lifecycle`). This handoff SHA update is a documentation-only follow-up commit on the same branch.
- Remote: `origin` (`https://github.com/thrns/outflow.git`)
- Changed paths are listed in §2. No binaries, audio, transcripts, model files, screenshots, or personal speech samples are included.

## 12. Recommended next implementation step

Continue with an isolated, approved T-025/T-026/T-031 interface design for the production Rust microphone permission, capture, and session services only after the M1 dependencies and architecture contracts are accepted. Keep this AppKit probe as feasibility evidence; do not mark production microphone tasks complete based on it. The next release decision still requires an independently reviewed human WER corpus, approved physical 4 GB hardware, resource and license evidence, and the existing model gates.
