# Prompt 79E — detailed implementation handoff

**Status: PARTIALLY VERIFIED / BLOCKED for the full milestone.** The development-only prototype built and launched on the named Mac, received normal macOS microphone authorization, captured microphone audio, and produced a real nonempty partial and final transcript through the pinned local Moonshine Small Streaming CPU runtime in one spoken session. One other live session captured 47.8 seconds without recognition. Fixture, state, native lifecycle, process-tree, and repository tests are recorded below. The successful speech was not saved or assessed against a human reference. This is not completion of T-011, T-025–T-034, AC-06, T-010, or the MVP.

## A. What existed before Prompt 79E

Prompt 79D's baseline at `ace904569f6966ea0027a0e426a29df2b0de1a65` already included a native Moonshine streaming smoke executable, a batch recognizer, benchmark and RSS tools, a pinned upstream C API/header contract, a locally built arm64 Moonshine/ONNX Runtime pair, and a hash-verified local Small Streaming model inventory. The streaming smoke used local fixture audio and exercised the actual streaming API, but it was not a microphone application. There was no native macOS permission flow, capture callback, bounded live-audio handoff, app window, production Rust FFI, Tauri shell, text insertion, or Copy Result feature.

The existing feasibility blockers remain material: no ready human-reviewed WER corpus (0 ready fixtures), no approved physical 4 GB reference-class machine, no Tiny fallback run, no completed model/license release decision, and no production app size/RSS or insertion evidence. The 79D default-thread native-only measurements included a 1,010,614,272-byte peak; the experimental upstream single-thread configuration was not adopted. See `evidence/mvp/prompt-79d-detailed-implementation-handoff.md`, `evidence/mvp/prompt-79d-native-memory-measurements.json`, and `evidence/model/MODEL_DECISION.md`.

## B. What was implemented

Prompt 79E adds a standalone Objective-C++/C++ macOS development probe under `dev/macos-mic-probe/`. A small AppKit window drives explicit permission, Start, Stop, Cancel, Dismiss, and normal Microphone Settings recovery. AVAudioEngine opens the default input only after Start is user initiated and permission is granted. A callback copies device samples into a fixed-capacity queue. A dedicated worker converts/downmixes/resamples to the format expected by the pinned C API, calls actual CPU streaming inference, polls/copies partial results, then finalizes/copies one final result. UI previews remain in volatile memory.

The same worker can be fed by an explicitly selected local WAV fixture at real-time pace for repeatable no-microphone tests. A separate headless harness profiles actual native fixture inference. Metrics include capture and partial timing, queue occupancy/overflow, finalization, process CPU/RSS/thread observations. An opt-in, exclusive metrics file stores metadata only; it does not store text, buffers, command arguments, or environment.

The probe is development-only, requires a logged-in macOS desktop session and a local app build, and currently targets the host-tested macOS 26 SDK/deployment minimum. It is not connected to the Rust workspace, the browser preview, an installer, or a release. It does not implement no-speech policy, cleanup, focus/protected-field checks, keyboard shortcuts, insertion, clipboard copying, model packaging, Tauri lifecycle, or Windows/Linux support.

## C. File-by-file walkthrough

Line references below refer to this branch's implementation before the two delivery commits; file paths are repository-relative.

| File | Responsibilities, ownership, errors, and tests |
|---|---|
| `dev/macos-mic-probe/audio_pipeline.hpp:18-120` | Defines PCM encodings and device format views, bounded `AudioBlock`, conversion error values, the fixed SPSC queue, `MonoResampler`, and monotonic audio-time helpers. The queue owns its preallocated block storage; the capture callback is its sole producer and the worker its sole consumer. |
| `dev/macos-mic-probe/audio_pipeline.cpp:11-174` | Reads/converts float32, signed int16/int32, and unsigned 8-bit samples; validates finite/range/stride/channel dimensions; downmixes channels; splits large callbacks into fixed blocks using `copy_pcm_frames_to_queue` (`:114`); records overflow instead of growing. Queue publication/wakeup and clear-after-producer-stop are at `:177-270`; worker-side mono/rate conversion is at `:272-342`. Covered by `tests/core_test.cpp`, including downmix, formats, queue bounds/overflow, wakeup, oversized callback split, and resampling. |
| `dev/macos-mic-probe/session_state.hpp:7-38`, `session_state.cpp:5-82` | `SessionStateMachine` accepts a monotonically distinct session ID and gates `begin`, `listening`, `finalize`, `ready`, `cancel`, `fail`, and `dismiss`. Stale events are rejected by `accepts`. Covered by 100 transition-cycle assertions and stale-ID/idempotence cases in `tests/core_test.cpp`. |
| `dev/macos-mic-probe/permission_policy.hpp:7-24`, `permission_policy.cpp:5-30` | Maps macOS permission outcomes to display names and allowed action (`request`, `open settings`, or start). It is a pure policy mapper. Unit tests simulate each state; they are not proof of OS permission behavior. Actual TCC permission was separately observed in the live app. |
| `dev/macos-mic-probe/macos_capture.hpp:1-36`, `macos_capture.mm:15-212` | `MacMicrophoneCapture` owns an AVAudioEngine and tap. It discovers sample rate, channel count and native PCM representation; starts/stops the input and reports device/format failures. The tap copies into preallocated queue blocks at `:95` and never performs ASR or UI work. Stop removes the tap and waits for callbacks in flight before detaching queue state (`:112-137`, `:196-210`). An observed >4096-frame tap callback led to bounded splitting into at most 16 blocks and a corresponding regression test. `tests/core_test.cpp` exercises the generic callback splitter; only live macOS use exercises AVAudioEngine itself. |
| `dev/macos-mic-probe/model_verifier.hpp:1-11`, `model_verifier.cpp:16-89` | Hashes the eight expected local assets and rejects missing, symlinked, non-regular, or mismatched files before Moonshine load. Uses bounded file reads and reports a sanitized failure string. Called by `SessionWorker` before the native API. Model mismatch/load cases are exercised using an injected verifier in `tests/worker_test.cpp`; the build/run scripts also validate real asset hashes. |
| `dev/macos-mic-probe/session_worker.hpp:16-108`, `session_worker.cpp:28-490` | Owns one worker thread, queue consumption, resampler, cancellation/finalization flags, metrics, and the Moonshine transcriber/stream handles. `start`, `request_finalize`, `request_cancel`, `request_failure`, `join` are at `:132-193`; inference lifecycle is `run` at `:195-478`. The API table is injected in tests but production uses pinned C functions and explicit CPU provider option (`:211-228`). Transcript pointer bounds and immediate string copying are in `copy_transcript` (`:55-74`); partial/final API calls are around `:326-343` and `:406-421`. Worker cleanup frees stream before transcriber on all paths. Tests: `tests/worker_test.cpp:172-314`. |
| `dev/macos-mic-probe/wav_fixture.hpp:1-18`, `wav_fixture.cpp:13-139` | Loads only bounded PCM16 mono WAV fixtures, validates RIFF chunks, 16 kHz requirements, file/recording duration limits, and sample counts. The fixture is explicitly local/test-only and its data remains memory-resident. Malformed and boundary WAV inputs are covered by `tests/core_test.cpp`. |
| `dev/macos-mic-probe/fixture_producer.hpp:1-44`, `fixture_producer.cpp:19-113` | Owns a cancellable producer thread that supplies 1,600 samples per 100 ms using `steady_clock`, separate from inference. Start copies/owns the fixture for repeat sessions; `stop` joins. Tested for deterministic two-session reuse and by real-time fixture harness runs. |
| `dev/macos-mic-probe/app.mm:103-107, 275-792` | AppKit app/controller, temporary status/result labels and controls. Window setup is `applicationDidFinishLaunching` (`:306`); permission is checked/requested from Start (`:447-485`); controls call Start/Stop/Cancel/Dismiss/Settings (`:447-538`); `beginSession` creates one ID, queue, worker and event dispatch (`:539-574`); source opens only after worker Ready (`:575-657`); stale events and partial/final/error update are gated in `handleWorkerEvent` (`:658-750`); stop/cancel/exit cleanup is `stopSourcesForShutdown` and `cancelActiveSessionForDismissal` (`:752-777`). `main` supports `--model-dir`, `--timed-fixture`, and optional `--metrics-file` (`:779-792`). UI behavior was exercised with two consecutive visible timed fixture sessions, Cancel clearing the preview, and the real microphone sessions. Static privacy tests assert no pasteboard/injection/persistence calls. |
| `dev/macos-mic-probe/Info.plist` | Minimal app bundle metadata with a development bundle identifier, `NSMicrophoneUsageDescription`, and host SDK deployment floor. The build produces only an external development `.app`; it is not release packaging. |
| `dev/macos-mic-probe/tests/core_test.cpp:26-316` | Eleven grouped native-core tests for state cycles, permissions, PCM formats, channel downmix/resample, bounded queue/overflow, oversized callback splitting, wakeup/shutdown, and bounded WAV parsing/timed fixture behavior. |
| `dev/macos-mic-probe/tests/worker_test.cpp:30-315` | Fake C API only for lifecycle/error injection: model load error, one stream lifecycle and one final, copied partial/final event order, cancel, overflow, device error, handle release, and 100 sequential start/stop/cancel cycles. These tests prove worker policy/lifecycle under the fake API; native real inference is separately exercised by the fixture harness and live app. |
| `dev/macos-mic-probe/tests/real_fixture_harness.cpp:20-176` | Headless integration executable that loads/verifies the real model and calls the real Moonshine API using `TimedFixtureProducer`; outputs metadata only (never transcript text). Supplies timed worker timing and per-process rusage/thread metrics. |
| `scripts/build_macos_mic_probe.sh:1-83` | Validates explicit local source/header/library/model inputs, exact expected API constants and Moonshine/ORT dylib hashes/arm64, generates model hash constants, compiles Objective-C++/C++ and creates an externally located app bundle with microphone plist and local ad-hoc signature. Fails rather than downloads. |
| `scripts/run_macos_mic_probe.sh:1-59` | Validates existing built app/model directory and launches through LaunchServices, preserving explicit fixture/metrics CLI arguments. |
| `scripts/test_macos_mic_probe.sh:1-112` | Compiles core and worker tests, and optionally the real fixture harness (`--real-fixture`) with repeat count, timed duration, and the existing process-tree sampler. All paths are explicit environment inputs. |
| `scripts/generate_mic_probe_asset_header.py:19-78`, `scripts/tests/test_generate_mic_probe_asset_header.py:1-56` | Generates C++ expected asset hashes from the existing repository model manifest without bundling model files. Tests malformed manifests, path/hash validation, and deterministic generation. |
| `scripts/tests/test_macos_mic_probe_privacy.py:1-78` | Static source checks for absence of transcript/audio persistence or logging, clipboard/injection APIs, and proper user-initiated capture entry points. Static checks do not replace OS or live behavior. |
| `scripts/run_native_asr_benchmark.py:60-70,663-681`, `scripts/tests/test_native_asr_benchmark.py:79-98`, `scripts/tests/test_validate_measured_asr_results.py:246-274` | The conservative RAM-size screen now allows values through 4,294,967,296 bytes for classification, with explicit physical-status and approved-reference-class checks kept separate; boundary tests cover 4,000,000,000, 4,294,967,296, and 4,294,967,297 bytes. The release RSS cap is unchanged and no hardware gate was passed. Product wording remains unresolved; see §H. |
| `evidence/mvp/prompt-79e-timed-fixture-measurements.json` | Sanitized observed measurement record. Includes 10 timed native fixture runs, one post-callback-fix native run, two UI fixture sessions, one app RSS trace, and two mic-session summaries. It contains no transcript text or audio. |
| `README.md:1207-1301`, `docs/TASKS.md:34,97,217-219` | README provides exact local build/run/test/RSS instructions and clearly labels the probe. TASKS records evidence and limits without changing task checkboxes or product acceptance. |

## D. Live execution flow

1. **Permission:** the app displays current AVFoundation authorization. Only the user pressing Start triggers the normal TCC request. Denial leaves capture closed and exposes normal System Settings recovery. No Accessibility or Automation prompt is requested.
2. **Preparation:** an accepted Idle Start allocates a new session ID, a fixed 16-slot queue, a worker and one event callback. The worker verifies all eight model assets, loads the pinned Small Streaming architecture, explicitly selects CPU execution, creates one transcriber and one stream, then starts that stream. UI status remains Preparing until the worker sends Ready.
3. **Capture:** the UI thread opens the default AVAudioEngine input only for microphone source. The tap records the actual device format. It views the callback's interleaved or planar buffers, converts supported PCM sample representations into fixed `AudioBlock`s and publishes them. Large AVAudioEngine tap callbacks are split into at most 16 preallocated 4,096-frame blocks. If the callback cannot be represented or the 16-slot queue fills, an atomic producer error/overflow is set; the callback returns and the worker causes fail-closed capture shutdown.
4. **Worker:** the worker dequeues blocks, downmixes to mono and deterministically resamples to 16,000 Hz, then passes the resulting `float` samples to the pinned API. The worker polls partial state and immediately copies C API transcript strings into `std::string` before another native call can invalidate them. It posts only copied strings and metadata to the main queue. The capture callback does not run ASR, wait on inference, write files, or update UI.
5. **Final:** Stop timestamps the request, stops and detaches the audio source (so mic release precedes final model draining), then requests exactly one finalization on the worker. The worker drains queued audio, calls stop/final once, copies the final result, frees stream then transcriber and reports ResultReady. Repeated Stop is inert once Finalizing. Cancel sets cancellation, stops fixture/microphone production and joins/cleans worker handles, clears partial/final preview, and enters Cancelled. Device, format, queue, model, or ASR errors also stop capture before presenting Error.
6. **Preview/cleanup:** partial/final text lives only in controller memory and labels until Cancel, Dismiss, new session initialization, error cleanup, window close, or app exit. The optional metrics output path is created exclusively with restrictive permissions and contains numeric/timing metadata only. No user audio or transcript is serialized, copied automatically, pasted, injected, or sent over network.

## E. Session state machine

The central enum is `Idle`, `Preparing`, `Listening`, `Finalizing`, `ResultReady`, `Cancelled`, and `Error`.

| Current state | Event | Next state / behavior |
|---|---|---|
| Idle, ResultReady, Cancelled, Error | accepted Start, permission granted | Preparing with a fresh session ID; prior preview is cleared |
| Preparing | worker Ready, source successfully opens | Listening |
| Preparing or Listening | user Stop | Finalizing once; stop source before drain |
| Finalizing | one copied final event | ResultReady |
| Preparing, Listening, Finalizing | Cancel, close, or exit | Cancelled during interactive use; cleanup on exit |
| Preparing or Listening | worker/model/device/queue/ASR failure | Error after capture stop and native cleanup |
| ResultReady | Dismiss | Idle, preview cleared |
| Any state | duplicate Start while active | Rejected; no second input or stream opens |
| Finalizing/terminal | duplicate Stop or final | Rejected/idempotent; no second final result |

The state machine accepts events only for the active session ID. Delayed Ready/Partial/Final/Error messages from an old session cannot mutate a newer state or display stale text. Tests cover transitions, stale IDs, rejected actions, repeated events, two fixture starts, worker shutdown, and 100 injected worker cycles. The actual AVAudioEngine OS callbacks and TCC transitions were not exhaustively fault-injected.

## F. Tests and verification

Environment: macOS 26.6.2 (25G83), arm64 Apple M3 Pro, 19,327,352,832 installed physical RAM bytes (18 GB marketed/available host class). Physical hardware is not the approved 4 GB reference device.

**Native development checks**

- `scripts/build_macos_mic_probe.sh` — PASS; built the external development-only `OutflowMicProbe.app`, then launched it through LaunchServices. Build verified the explicit source/header contract, model asset hashes, arm64 Moonshine dylib and ONNX Runtime dylib. The external source snapshot did not retain `.git` metadata, so its revision could not be independently proven with `git rev-parse`; the explicit expected revision, header constants and exact binary hashes were checked.
- `scripts/test_macos_mic_probe.sh` — PASS; core tests reported 11 groups, including 100 state cycles and two fixture starts; worker injection tests covered model-load error, native lifecycle, final-once, cancellation, overflow, device failure, and 100 cycles.
- `scripts/test_macos_mic_probe.sh --real-fixture` — PASS; real local model and C API, 12-second paced fixture, nonempty partial/final, no overflow. A post-callback-splitting confirmation run measured 17 partial updates, 1/16 maximum queue depth, 1,950.944 ms first partial, 596.731 ms mean cadence, 132.115 ms stop-to-final, 809,189,376-byte worker high-water RSS, 12,561.5 ms CPU, 22 threads. The 10-run distribution below predates that small capture slicing fix; the separate post-fix run is retained separately in the JSON.
- `python3 -m unittest discover -s scripts/tests -v` — PASS after correcting a test selector; final recorded result: 377 tests run, 1 skipped (the benchmark integration remains skipped because there is no ready corpus), no failures. This includes the new privacy and asset-header tests plus the 4 GB boundary tests.
- Rust: `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check`; `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked`; `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked`; `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings`; `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` — all PASS. No Rust source changed in this prototype.
- Frontend: `npm run typecheck`; `npm run test -- --run` (136 tests); `npm run lint`; `npm run build` — all PASS. The existing browser preview remains separate and does not capture audio.
- `git diff --check` — PASS on the final staged implementation diff and the delivery-metadata update.

**Real OS/hardware checks**

- First explicit Start requested normal microphone access; user granted it. The real device reported 48,000 Hz, mono. A 47.8-second capture closed cleanly but produced no nonempty partial/final.
- A subsequent user-spoken sentence produced a visible nonempty partial and final through Moonshine and returned to `Result ready — microphone released`. The final recognized wording was imperfect; no reference text or WER was recorded.
- Start/Stop/Cancel, preview clearing, repeat sessions, microphone close on Stop/Cancel, and window display were exercised. Two visible app fixture sessions produced partial/final. Cancel in fixture mode cleared the preview and returned Cancelled.
- Two physical mic sessions were completed. A third live session was requested during delivery to meet the “at least three where available” criterion, but no third result was observed before delivery; that subcriterion remains unverified.
- Not individually hardware-tested: explicit denial then recovery, previously granted launch without prompt, absent/busy/disconnected physical mic, permission revocation mid-session, macOS sleep, or every device format. Pure policy/error injection tests cover software mapping but are not represented as OS tests.
- The 100 worker Start/Stop/Cancel cycles use an injected fake C API and fixtures; they are not 100 physical microphone sessions. There were two actual microphone sessions, one with recognized speech.

## G. Real-time performance and memory measurements

**Identity:** expected Moonshine source revision `234f60faa0eb388b01cdf7e60aca232af37aefda`; C header version `30000`; model architecture `MOONSHINE_MODEL_ARCH_SMALL_STREAMING = 4`; CPU provider. Moonshine dylib SHA-256 `42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5`. ONNX Runtime 1.23.2 arm64 dylib SHA-256 `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`. Eight local model files total `142,300,974` bytes; manifest SHA-256 `b676045b9cad1844320c0549c285719d6e68e76bbe331762a77a0eab2030480a`; they remain external to Git. The local fixture is pinned-checkout `two_cities_16k.wav`, PCM16 mono 16 kHz, SHA-256 `14a5a09479cab470db7de4fd16c0e18a8a88c03cc6d92a09a658e857ee1f73a5`; it is not a human-reviewed WER corpus.

**Ten 12-second real-time-paced worker runs** (actual Moonshine CPU inference; 100 ms producer chunks; each 12 seconds of fixture samples paced against `steady_clock`; separate from Prompt 79D's accelerated smoke):

| Metric | Observed range | Median / other detail |
|---|---:|---:|
| Supplied/captured audio duration | 11,900.079–11,905.092 ms | 12.0 seconds expected |
| First nonempty partial after first frame | 1,871.122–1,940.902 ms | 1,899.057 ms |
| Mean partial cadence | 595.542–601.683 ms | 17 updates per run |
| Stop request to final ready | 79.404–207.722 ms | 90.261 ms |
| Native process high-water RSS | 599,457,792–721,518,592 bytes | median 650,682,368 bytes; 22 threads |
| Sampled process-tree RSS | 599,457,792–721,518,592 bytes | ten 100 ms profiles; reported maximum sampling gap up to 223 ms across runs |
| Process CPU time | 8,732.034–13,160.016 ms | median 10,106.792 ms |
| Queue maximum depth | 1–2 of 16 blocks | zero overflow in all runs |

**After callback slicing fix:** one additional 12-second actual worker run: first partial 1,950.944 ms; cadence 596.731 ms; final 132.115 ms after Stop; RSS high-water 809,189,376 bytes; CPU 12,561.5 ms; 22 threads; 1/16 queue maximum, zero overflow. Do not fold this single run into the prior ten-run distribution.

**Visible AppKit app:** one 30,000.184 ms process-tree recording with 301 samples and maximum sampling gap 105 ms across two consecutive fixture sessions. Tree peak was 919,814,144 bytes for one `OutflowMicProbe` process; no helper/child was observed. The app-level native high-water readings during the two fixture sessions were 909,803,520 and 919,879,680 bytes. Both UI sessions showed nonempty partial/final. This is the only full development-app trace and does not cover cold launch through all error/cancel phases or a Tauri/webview tree.

**Live microphone:** real input was 48,000 Hz mono. No-speech session: first frame 149.746 ms after capture start; captured 47.8 seconds / 47,855.991 ms; zero nonempty partials; Stop-to-final 59.232 ms; microphone release 50.108 ms; queue maximum 2/16; zero overflow; 31 threads; native high-water RSS 589,856,768 bytes. Successful spoken session metrics displayed in the UI (rounded): model load 313 ms, first frame 147 ms, first partial 9,806 ms after first frame, Stop-to-final 48 ms, mic release 48.0 ms, queue max 5/16, no overflow reported, 901 MB high-water RSS, 32 threads. The exact successful session metrics file was not saved because the chosen exclusive file path already existed from the earlier session. It was not overwritten. No transcript text is included in evidence.

The 950,000,000-byte app-process RSS release cap remains unchanged. The visible app's observed 919,814,144-byte sampled peak is close to the cap and was measured on an 18 GB host, with only two fixture sessions, so it does not pass a release resource gate or certify a physical 4 GB machine. The 79D native-only peak of 1,010,614,272 bytes is a separate historical measurement. No `MOONSHINE_ORT_SINGLE_THREAD=1` production setting is adopted. No battery, macOS physical-footprint, installed app size, or aggregate future Tauri memory measurement is claimed.

## H. Remaining blockers

- **T-010 / model feasibility and release gate:** remains blocked. There is no ready, human-reviewed speech corpus (100 short plus 50 long recordings and independent coverage/provenance checks), no WER/quality result, no eligible approved physical 4 GB reference machine, no tested Tiny fallback, no production app RSS/installed-size evidence, and no completed licensing/third-party legal decision. CPU-only small-model live feasibility is not enough.
- **4 GB definition:** PRD says “4 GB” without specifying decimal versus binary reported bytes. The benchmark classification helper now treats up to 4,294,967,296 bytes as a screening ceiling, and tests both 4,000,000,000 and 4,294,967,296 plus the next byte. This is not an approved product decision: owner clarification and the existing approved physical reference-class allowlist (currently empty) are still required. Do not change the 950,000,000-byte RSS cap.
- **T-011 and T-025–T-034 / AC-06:** remain incomplete. The prototype has no production Rust/CPAL/Tauri integration, no cross-platform audio service, no production FFI, no product no-speech policy, and no accepted corpus quality measurement.
- **Platform/app acceptance:** no Windows/Linux paths, shipping app lifecycle, bundled model/installer, accessibility checks, text cleanup, protected-field/focus validation, insertion, explicit Copy Result, or release signature/notarization work was done. Prototype permission simulation is not coverage of all TCC/hardware states.
- The 4 GB M3 Pro host is not reference hardware. No release gate is marked passed by this work; no task checkbox was changed.

## I. Reproduction commands

The following variables point to local artifacts used on the test Mac; the model and native dependencies are outside the repository and scripts will not download them:

```sh
export MOONSHINE_SOURCE_DIR=/private/tmp/outflow-feasibility/moonshine-234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
export OUTFLOW_MIC_PROBE_BUILD_DIR=/private/tmp/outflow-prompt79e-build-7
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
```

Build and launch the app from a logged-in macOS desktop session:

```sh
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"
scripts/run_macos_mic_probe.sh
```

Click Start recording and allow the normal macOS microphone prompt. Speak a short sentence, press Stop, and observe a temporary final result; Cancel clears it. If permission was previously denied, use the app's Microphone Settings control and normal System Settings UI.

Run native automated tests:

```sh
scripts/test_macos_mic_probe.sh
```

Run one real 12-second timed fixture integration test (do not treat it as live-mic or WER evidence):

```sh
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_FIXTURE_SECONDS=12
export OUTFLOW_MIC_PROBE_FIXTURE_RUNS=1
export OUTFLOW_MIC_PROBE_PROFILE_DIR=/private/tmp/outflow-mic-probe-rss-repro
scripts/test_macos_mic_probe.sh --real-fixture
```

Open the UI in timed fixture mode:

```sh
scripts/run_macos_mic_probe.sh \
  --timed-fixture "$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav" \
  --metrics-file /private/tmp/outflow-mic-probe-ui-run-01.json
```

For process-tree RSS, launch one app fixture session, then sample the app PID (the profiler writes numeric process metadata only):

```sh
APP_PID=$(/usr/bin/pgrep -n -x OutflowMicProbe)
python3 scripts/measure_process_tree_rss.py \
  --pid "$APP_PID" --duration-seconds 30 --interval-ms 100 \
  --output /private/tmp/outflow-mic-probe-app-rss-01.json
```

The app's metrics file uses exclusive creation; choose a new path per run. For the full repository, use the exact Python, Rust, and npm commands listed in §F. For known problems, see README's Prompt 79E recovery list: validate eight model hashes, use normal System Settings for denial, reconnect/release unavailable or busy devices, choose a fresh metrics filename, and rebuild/relaunch from the desktop session if LaunchServices/TCC cannot associate the app bundle.

## J. Architecture handoff

Reusable concepts for a later production implementation: state transitions and stale-session gates; bounded SPSC queue limits and fail-closed overflow; supported PCM format validation; worker-only resampling/inference; immediate copy of native-owned transcript strings; ordering of capture stop before model finalization; error paths that release mic and native handles; metadata-only performance instrumentation; and real-time paced fixture methodology. These are design/test references, not Rust integration already completed.

Keep AVAudioEngine, AppKit, Objective-C++ TCC UI, bundle plist, ad-hoc signing, UI controller, and `TimedFixtureProducer` as this probe's development harness. Production audio should use the approved Rust/CPAL/platform abstraction after its contract and dependency approval; production ASR should isolate unsafe FFI in the approved Rust module and reuse the pinned ABI/model identity through audited bindings. Do not copy Objective-C++ code into Rust or claim production CPAL/Tauri behavior from this app. The fake C API belongs only in tests. The licensed fixture belongs only in local development test paths and is not a user recording or quality corpus.

## K. GitHub delivery

- Starting point: verified Prompt 79D commit `ace904569f6966ea0027a0e426a29df2b0de1a65`.
- Development branch: `codex/prompt79e-mic-prototype`; no main branch changes.
- Intended changes are limited to the standalone `dev/macos-mic-probe/` app/tests, three development scripts and their tests, the boundary classifier tests, README/TASKS notes, and the two Prompt 79E evidence files.
- External model weights, fixture audio, generated app binaries, native libraries, per-run timing/RSS files, and transcripts are excluded from Git. No `.github/workflows` change is included.
- Implementation commit: `e78b4aab39a272600235c7dea13701b72ca2988f` (`feat(native): add local macOS microphone streaming feasibility probe`), [GitHub commit](https://github.com/thrns/outflow/commit/e78b4aab39a272600235c7dea13701b72ca2988f).
- Remote target is `origin` (`https://github.com/thrns/outflow.git`). `git ls-remote origin refs/heads/codex/prompt79e-mic-prototype` returned `e78b4aab39a272600235c7dea13701b72ca2988f`; it matched local HEAD at initial push verification.
- This delivery record is a follow-up documentation commit on the same branch; its branch-head SHA is verified after push and is reported in the final response.
- Final staged-diff checks passed. Generated `__pycache__` folders were removed. Only the intended 34 source, script, documentation, and measurement files are in the implementation commit. Model weights, fixture audio, transcript content, native binaries, secrets, and workflow files are absent. No force-push, PR, merge, release, or main update was performed.
- Final worktree status after the handoff update and push: clean, tracking `origin/codex/prompt79e-mic-prototype`.

## L. Recommended next implementation milestone

Continue the existing T-010 evidence gate. First resolve the owner decision on what a physical “4 GB” reference machine means in reported bytes and obtain/approve that reference hardware class; separately finish fixture licensing/provenance and human-reviewed coverage before running quality and resource gates. On that hardware, repeat full application process-tree memory/latency measurements and assess Small versus the preapproved Tiny fallback under the unchanged RSS cap. Only after T-010's original evidence and approval prerequisites pass should the production T-011/M1 and approved Rust/CPAL/Tauri integration proceed. Prompt 79E does not authorize or start Prompt 80.
