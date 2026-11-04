# Prompt 79H — detailed implementation handoff

**Status: PARTIALLY VERIFIED.** The explicit replacement writer and isolated AppKit integration test pass. The corrected desktop button was not reachable through the available UI automation binding, so a successful general-pasteboard copy and manual TextEdit paste are **NOT RUN**. No production task or release gate is complete.

## A. Starting state

- Repository: `/Users/bixxie/Desktop/Projects/outflow` (`thrns/outflow`).
- Supplied starting branch: `codex/prompt79g-copy-result`.
- Verified starting commit: `b17c01ab9ee1f91db3696a9ff05e2349ee95c4ab`.
- Starting worktree was clean. A task branch, `codex/prompt79h-copy-result`, was created from that commit.
- The old `app.mm` writer converted UTF-8 to `NSString`, then called `writeObjects:` on `NSPasteboard.generalPasteboard` without first calling `clearContents`. It reported the `writeObjects:` boolean and had no retry. Its failure message tried to diagnose permission from `accessBehavior`.
- Before editing, the existing timed fixture was replayed in the prior probe. After a nonempty partial and final result, one explicit Copy Result click showed: “Copy failed. Select the text above, then press ⌘C to copy it manually.” The final result remained visible/selectable. No clipboard content was read or restored. The cause of that observed failure was not determined.
- Existing Prompt 79G handoff records a prior failed write and an unsuccessful manual fallback. That is historical baseline evidence; it does not prove the cause or the behavior of the corrected writer.

## B. Root-cause analysis

The starting implementation skipped the documented replacement preparation step. Apple's `clearContents` documentation says to clear the pasteboard before supplying replacement data; its return value is a change count, not a success boolean. `writeObjects:` returns whether the objects were written. The corrected path follows that order: convert, clear once, write once.

The sequence defect is confirmed by source inspection. It is a plausible explanation for the baseline failure, but the causal link between omission of `clearContents` and the observed `writeObjects:` failure remains **uncertain**: the old UI exposed only the copy-failed message, and the earlier event did not capture a lower-level diagnostic. No permission-denial cause was inferred.

`NSPasteboard.accessBehavior` was removed from the diagnostic path. It reports an access behavior policy and is not treated as proof of why one attempted write failed. Preparation failure is classified only if AppKit throws while `clearContents` is called; the API's change-count return is not misclassified as an error.

Apple API references: [`clearContents`](https://developer.apple.com/documentation/appkit/nspasteboard/clearcontents%28%29), [`writeObjects:`](https://developer.apple.com/documentation/appkit/nspasteboard/writeobjects%28_%3A%29?language=objc), and [`accessBehavior`](https://developer.apple.com/documentation/appkit/nspasteboard/accessbehavior-86972).

## C. File-by-file walkthrough

| File | Change, ownership, and verification |
|---|---|
| `dev/macos-mic-probe/pasteboard_writer.hpp` | Adds `string_from_utf8(string_view)` and `write_plain_text_to_pasteboard(NSPasteboard*, NSString*)`, owned by the development AppKit probe. The writer returns `Succeeded`, `InvalidUtf8`, `PasteboardUnavailable`, `PreparationFailed`, or `WriteFailed`. The caller converts before acquiring the pasteboard; the writer then calls `clearContents` once and `writeObjects:` once. It reads no pasteboard data and does not retry. Pure privacy source tests and the real isolated AppKit test cover this path. |
| `dev/macos-mic-probe/app.mm` | Keeps the sole production-prototype call inside `copyResultPressed:`. It rechecks session eligibility, converts the accepted UTF-8 result, then acquires `generalPasteboard`, maps writer results to user-facing status, and shows the replacement-loss warning. Final text remains selectable after failure. Adds a development-only `--isolated-pasteboard-test <result-path>` launch option to run the isolated test in the normal AppKit app context. No lifecycle or worker event writes were added. |
| `dev/macos-mic-probe/copy_result_policy.hpp` | Renames the rejection enum to `RejectedBySessionPolicy`; the gate still requires current `ResultReady`, matching nonzero session identity, and nonempty actual final text. Its injected writer runs at most once for an eligible action. |
| `dev/macos-mic-probe/tests/core_test.cpp` | Extends the explicit-copy policy test with exact UTF-8/emoji/multiline bytes and failed-writer retention. It covers ineligible states, stale identity, empty/no-speech content, and one writer invocation. |
| `dev/macos-mic-probe/tests/pasteboard_integration_test.hpp` | Declares the small integration entry point and sanitized result contract. |
| `dev/macos-mic-probe/tests/pasteboard_integration_test.mm` | Uses `pasteboardWithUniqueName`, fixed synthetic strings only, plain-text readback from that isolated board, one item after each replacement, and `releaseGlobally` cleanup guarded by RAII. It never accesses `generalPasteboard`. |
| `scripts/build_macos_mic_probe.sh` | Compiles the integration test source into the development AppKit app so the test executes in a normal app process. |
| `scripts/test_macos_mic_probe.sh` | After existing core/worker and optional fixture tests, builds the app and runs its isolated test through LaunchServices; accepts only a sanitized PASS result file. |
| `scripts/tests/test_macos_mic_probe_privacy.py` | Adds source-level checks for write ordering, one guarded general-pasteboard call site, no content reads/accessBehavior diagnosis, lifecycle non-writes, and accurate warning/recovery text. |
| `docs/USER_FLOWS.md` | Corrects UF-08 failure recovery to disclose that replacement clearing may lose old clipboard contents; no read/restore rollback. |
| `README.md` | Marks Prompt 79G copy details as superseded, documents the current replacement path, warning, build/run/test steps, isolated test, manual TextEdit procedure, and troubleshooting/host limitations. |
| `docs/TASKS.md` | Leaves T-047 unchecked and adds a brief Prompt 79H evidence note; no milestone or acceptance criterion is marked complete. |

The requested inspection files `Info.plist`, `session_state.cpp`, `scripts/run_macos_mic_probe.sh`, and the earlier Prompt 79G evidence were read; no edits were needed in them.

## D. New clipboard execution flow

`Current final session → explicit Copy Result action → eligibility check → UTF-8 conversion → acquire general pasteboard → clearContents once → write one plain-text object → inspect AppKit result → UI feedback`

A rejected session never enters the writer. Invalid UTF-8 is reported before the pasteboard is touched. A nil pasteboard reports unavailable. An exception during clearing reports preparation failure; a false return or exception during `writeObjects:` reports write failure. Only a true `writeObjects:` result displays “Copied. Paste manually where you want the text.”

## E. Clipboard privacy

Start, Stop, Cancel, Final events, Dismiss, Error events, shutdown, and window close have no writer call. Mouse and Return activate the same button action and policy. No partial transcript or display-only no-speech placeholder is eligible. A failed write leaves the final transcript in the selectable in-app result view.

The displayed warning is: **“Copy replaces the clipboard. If writing fails after clearing it, prior contents may be lost. Clipboard managers or sync may retain copies.”** The app does not read, retain, hash, log, or restore prior clipboard content. It does not retry. Clearing and writing are separate operations, so an explicit replacement cannot promise rollback if the later write fails.

## F. Automated verification

Environment: macOS 26.6.2 (25G83), Darwin 25.6.0, arm64. Existing pinned Moonshine source/runtime/model/WAV fixture only; no model or other artifact was downloaded.

| Layer | Result |
|---|---|
| Pure copy eligibility/fake-writer policy | **PASS** — one dedicated copy-policy group within 13 core groups. Checks result-ready/current-session/nonempty eligibility, stale-session and no-speech rejection, exact UTF-8 pass-through, a single eligible write call, no call for other states, and retained final text after fake-writer failure. |
| Native session/worker regressions | **PASS** — 13 core groups, 100 state cycles; 9 worker lifecycle groups, 100 injected start/stop/cancel/dismiss cycles. Existing microphone/worker cancellation coverage remains. |
| Real isolated AppKit pasteboard | **PASS** — four fixed synthetic forms (ASCII with spaces, Unicode, emoji, multiline), four explicit writes/replacements, initial empty check before the test action, exact plain-text readback, one item after replacement, and unique pasteboard release. No general clipboard access. |
| Timed-fixture ASR harness | **PASS** — 12.000 seconds at 16 kHz; nonempty partial and final; 17 partial updates; 0 queue overflows; max queue depth 1; stop-to-final 128.933 ms; native harness peak RSS 815,562,752 bytes. This is a single harness measurement, not the full app process-tree release gate. |
| Desktop general-pasteboard Copy Result and manual TextEdit paste | **NOT RUN** after the fix. The UI automation binding resolved to an older no-arguments app window; see §G. |
| Focused source privacy tests | **PASS** — `python3 -m unittest scripts.tests.test_macos_mic_probe_privacy -v`: 13 tests. |
| Full Python suite | **PASS with one skip** — 386 passed, 1 skipped. Skip requires separate licensed real-ASR integration corpus/environment variables. |
| Rust | **PASS** — format check, locked check, tests (353 test cases, 0 failures), Clippy with warnings denied, and locked workspace build. |
| Frontend | **PASS** — typecheck; 10 test files / 136 tests; lint; production build. |
| Native app build | **PASS** — fresh development app build and pinned runtime identity check. |
| `git diff --check` | **PASS** after final edits (rerun before commit). |

Exact commands:

```sh
scripts/build_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh --real-fixture
python3 -m unittest scripts.tests.test_macos_mic_probe_privacy -v
python3 -m unittest discover -s scripts/tests -v
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
git diff --check
```

The native commands used the already-present values from §H. The native test script's LaunchServices phase initially returned `kLSNoExecutableErr` in the restricted shell context. Rerunning in the permitted interactive desktop-launch context passed the isolated test. That LaunchServices status was not a pasteboard failure.

## G. Desktop verification

- **Pre-fix reproduction:** one explicit Copy Result click on the previous app produced the old “Copy failed…” status. This is failure evidence for the old implementation only.
- **Corrected app's explicit Copy Result returned success:** **NOT RUN**.
- **Manual paste into unsaved blank TextEdit:** **NOT RUN**.
- **Visual equality including punctuation/whitespace:** **NOT RUN**; no dictated or fixture words were included in repository evidence.
- **Keyboard Return activation on corrected ready result:** **NOT RUN**.
- **Live Cancel and Dismiss after correction:** **NOT RUN**. Automated policy/source checks passed; they are not a visual desktop substitute.
- **Blocker observed:** LaunchServices launched a uniquely identified temporary app entry, but `cua.getApp` rejected that new app ID as invalid. Looking up the bundle path bound to an older `Outflow Mic Probe` instance showing “Error — launch with --model-dir <local-model-directory>” with Copy Result disabled. Thus the automation could not operate the fixture instance or report its button status. A separate restricted `open` attempt returned `kLSNoExecutableErr`; the isolated test later passed when launched in the permitted interactive context. Neither status establishes a clipboard permission denial.
- No post-fix system general-pasteboard write was attempted. No user clipboard was read, saved, or restored. The only general-pasteboard action during this task was the explicitly authorized pre-fix reproduction click described above.

## H. Reproduction

Prerequisites: macOS 26+, Xcode Command Line Tools/macOS SDK, the existing authorized local Moonshine source/runtime/model/WAV fixture, and an interactive graphical user session. Set:

```sh
export MOONSHINE_SOURCE_DIR=/private/tmp/outflow-feasibility/moonshine-234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_BUILD_DIR=/private/tmp/outflow-prompt79h-repro
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"
scripts/run_macos_mic_probe.sh --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" --fixture-seconds 12
```

The runner uses LaunchServices. In the UI, wait for actual partial text, press Stop, then wait for the final result. Before Copy Result the application must not access/write the system clipboard. An explicit Copy Result click may replace it. Confirm success, switch to an unsaved blank TextEdit document, press ⌘V, compare visually to the final preview, discard without saving, then Dismiss. Repeat with a new fixture using Return activation. Separately test Cancel and confirm the final view clears and Copy is disabled. Do not record recognized words or clipboard contents.

For the isolated-only test, run `scripts/test_macos_mic_probe.sh` in a session that permits LaunchServices to start the test app. Its result file contains only PASS/FAIL metadata. If LaunchServices reports `kLSNoExecutableErr`, treat that as a launch/host blocker; do not infer a clipboard permission failure. If `writeObjects:` itself fails in the app, record the displayed status and retain the selectable result; previous clipboard contents may already be lost after clearing. Do not reset TCC or weaken system security.

## I. Remaining blockers

**Prototype clipboard completeness:** the replacement writer and isolated AppKit integration are verified, but the corrected general-pasteboard button path and human manual paste are not. Repeat the final desktop sequence in a host where the automation/operator can address the newly launched fixture instance.

**Production/release:** no production Tauri integration; T-046/T-047 remain unchecked; AC-10, T-010, T-011, and all MVP milestones remain incomplete. No target identity/protected-field insertion behavior was added. No physical 4 GB CPU-only system test, full app process-tree RSS/installed-size gate, broad device/OS matrix, model WER/quality gate, signed/notarized package, license/legal review, or full accessibility sign-off was performed. ASR model choice and release limits are unchanged.

## J. GitHub delivery

- Branch: `codex/prompt79h-copy-result`.
- Base/source commit: `b17c01ab9ee1f91db3696a9ff05e2349ee95c4ab`.
- Implementation commit: **pending at handoff draft**.
- Changed repository files: `README.md`; `dev/macos-mic-probe/app.mm`; `dev/macos-mic-probe/copy_result_policy.hpp`; `dev/macos-mic-probe/pasteboard_writer.hpp`; `dev/macos-mic-probe/tests/core_test.cpp`; `dev/macos-mic-probe/tests/pasteboard_integration_test.hpp`; `dev/macos-mic-probe/tests/pasteboard_integration_test.mm`; `docs/TASKS.md`; `docs/USER_FLOWS.md`; `scripts/build_macos_mic_probe.sh`; `scripts/test_macos_mic_probe.sh`; `scripts/tests/test_macos_mic_probe_privacy.py`.
- Evidence file: `evidence/mvp/prompt-79h-detailed-implementation-handoff.md`.
- Model assets, audio, transcript text, clipboard values, TextEdit documents, app binaries, and secrets are excluded from the repository change.
- Final commit, push result, remote SHA verification, and clean-worktree status will be recorded after commit/push. No production task checkbox is changed.

## K. Next recommendation

Complete one controlled desktop run on a macOS session where the newly launched fixture app instance is addressable: explicit Copy Result, manual ⌘V into blank TextEdit, visual comparison without saving the document, Return activation, Cancel, and Dismiss. Keep Prompt 80 and production Tauri work out of this task until this remaining verification layer is resolved.
