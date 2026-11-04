# Prompt 79I — detailed implementation handoff

**Status: PARTIALLY VERIFIED — COPY/PASTE MATCH NOT VERIFIED.** The development app is identifiable by a validated ID and PID. Native policy, launcher, malformed-ID, and isolated AppKit tests pass. The correctly identified app reported AppKit copy success, but a nonempty TextEdit paste did not compare equal to the final result. The cause is undetermined. Return-key activation was not repeated. This is development-prototype evidence only; no production task or release gate is complete.

**Scope:** F-10 / UF-08 / C-07 / TS-07 / AC-10 development probe follow-up, related to T-047. The T-047 checkbox remains unchecked. T-046, T-047, T-010, T-011, AC-10, and MVP completion remain open.

## A. Starting evidence

- Repository: `/Users/bixxie/Desktop/Projects/outflow` (`thrns/outflow`).
- Supplied starting branch: `codex/prompt79h-copy-result`.
- Verified starting commit: `0a3748fd2f7120809c06fa50779fdd0fe32202e5`.
- The starting worktree was clean. Work began on `codex/prompt79i-copy-result-identity` from that commit.
- Prompt 79H's AppKit writer converts the accepted UTF-8 value, calls `clearContents` once, then `writeObjects:` once. It does not read the pasteboard or retry. The isolated AppKit test passed for four fixed synthetic values; that does not establish general-pasteboard delivery.
- Prompt 79H's desktop blocker was an automation binding to an older no-arguments probe window. That window had Copy Result disabled, so its behavior was not evidence about the new fixture app. Prompt 79H records the details in `evidence/mvp/prompt-79h-detailed-implementation-handoff.md`.

## B. Launch identity implementation

| File | Change |
|---|---|
| `dev/macos-mic-probe/verification_identity.hpp` | Adds `is_valid_verification_id`, `verification_id_arguments_valid`, and `verification_diagnostic`. IDs must contain 1–32 ASCII letters, digits, or hyphens. The helper rejects control characters and returns only the validated ID plus process ID for display. |
| `dev/macos-mic-probe/app.mm` | Parses the optional `--verification-id <identifier>` argument, rejects missing, repeated, empty, oversized, and malformed values, and checks arguments in `main` before AppKit initialization. The delegate repeats validation at launch. A supplied ID appears in the window title, `NSWindow.accessibilityTitle`, and a noneditable `NSTextField` label with `getpid()`. With no ID, the original title and normal microphone UI remain unchanged. |
| `scripts/run_macos_mic_probe.sh` | Validates the same alphabet/length, rejects duplicates, appends `--verification-id` exactly once, preserves the existing model, fixture, metrics, and auto-cancel/exit arguments, and continues to launch via `open -n -a`. It prints the target bundle, the ID (or “not supplied”), and a reminder to verify the actual window/PID. It does not treat `open` success as proof of window identity. |
| `dev/macos-mic-probe/tests/core_test.cpp` | Covers valid and omitted IDs, missing/empty/oversized/invalid/control-character/repeated arguments, and safe diagnostic formatting. |
| `scripts/test_macos_mic_probe.sh` | Runs five malformed-ID launch cases against the built native executable and requires rejection status 2 before running the isolated AppKit test. |
| `scripts/tests/test_macos_mic_probe_privacy.py` | Adds source-contract checks for the visible read-only diagnostic, default title, single launcher forwarding, preserved launch options, content-free diagnostics, and absence of settings persistence. |

The ID is process-local development metadata. It is not put into settings, metrics, or persistent files. The bundle identifier in `Info.plist` and TCC configuration were not changed.

## C. Clipboard behavior

No pasteboard API or copy-policy change was necessary. The existing `copyResultPressed:` remains the sole general-pasteboard call site. It delegates eligibility to `request_copy_result`, which requires `ResultReady`, the current nonzero session ID, and nonempty final text. Only an eligible button action invokes the injected writer. The writer still converts before pasteboard access, clears once, writes one plain-text object once, and reports AppKit's result.

The existing UI warning remains visible before the action: “Copy replaces the clipboard. If writing fails after clearing it, prior contents may be lost. Clipboard managers or sync may retain copies.” Start, partial/final recognition, Stop, Cancel, Dismiss, launch, and shutdown do not write. Cancel and Dismiss still clear volatile result memory. No previous clipboard data was read or restored.

## D. Automated testing

Platform: macOS 26.6.2 (25G83), arm64. The repository hardware inventory identifies the host as an Apple M3 Pro with 18 GB RAM; the physical-memory query was unavailable in the restricted shell. No 4 GB release gate was run.

| Command / layer | Result |
|---|---|
| Fresh `scripts/build_macos_mic_probe.sh` | **PASS**. Final build: `/private/tmp/outflow-prompt79i-final.RE54uD/OutflowMicProbe.app`; pinned runtime identity check passed. |
| `scripts/test_macos_mic_probe.sh` | **PASS** after rerun in the permitted desktop-launch context: 14 core groups, 9 worker lifecycle groups, 5 malformed-ID native launches rejected without crashing, and isolated AppKit pasteboard test passed. Core/worker coverage includes 100 state cycles and 100 injected worker cycles. |
| `scripts/test_macos_mic_probe.sh --real-fixture` | **PASS**. The local 12-second fixture produced nonempty partial and final results, 17 partial updates, zero queue overflows, queue depth 1, 92.671 ms stop-to-final, and 601,948,160-byte native harness peak RSS in the final run. This is harness evidence, not the complete app process-tree or release-memory gate. |
| Isolated AppKit pasteboard integration | **PASS**: four fixed ASCII/Unicode/emoji/multiline values, four explicit replacements, exact isolated-board readback, and unique-board release. No general clipboard was used by this test. |
| `python3 -m unittest scripts.tests.test_macos_mic_probe_privacy -v` | **PASS**, 16 tests. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS**, 389 passed, 1 skipped. The separate skip is `test_verified_fixture_runs_through_real_pinned_native_asr`; the licensed human-reference corpus and its `OUTFLOW_BENCHMARK_*` environment are not configured. This skip is not counted as a pass. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS**, 353 tests passed, 0 failed (including doctests). |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS**, 10 files / 136 tests. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS**. |
| `bash -n scripts/run_macos_mic_probe.sh scripts/build_macos_mic_probe.sh scripts/test_macos_mic_probe.sh` | **PASS**. |
| `git diff --check` | **PASS** after final source, README, task ledger, and evidence edits. |

The first non-escalated AppKit test invocation returned `kLSNoExecutableErr` from LaunchServices in the restricted shell. The same suite passed when rerun in the permitted desktop-launch context. The error was a launch-context failure, not a pasteboard result.

Exact regression command block:

```sh
scripts/build_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh --real-fixture
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

## E. Visual launch verification

- The Copy test used the new window titled `Outflow Mic Probe — Dev — 79I-COPY-TEST-01 — PID 64151`. The same ID and PID appeared in the visible read-only diagnostic label and accessibility tree. This was not the older no-arguments error window.
- The fixture window showed “Microphone: not used,” reached Listening with a nonempty partial, and then reached Result ready with a nonempty final. The 12-second fixture timer invoked the app's stop/finalize path before the automation could click the Stop button; a manual Stop-button click is therefore **NOT VERIFIED**.
- A separate Cancel run used `79I-CANCEL-TEST-02`, PID `65829`, with a 120-second fixture limit. It reached a nonempty partial while Listening. Cancel displayed the cancelled state, cleared both preview areas, and left Copy Result disabled. Dismiss then returned it to Idle and the window was closed.
- After the final source change, a fresh normal-mode app was launched as `79I-NORMAL-VERIFY-03`, PID `68526`. The visible title and label matched. `ps` showed the executable path under `/private/tmp/outflow-prompt79i-final.RE54uD/OutflowMicProbe.app/Contents/MacOS/OutflowMicProbe`, and an `lsof` executable-path check matched the fresh bundle. The microphone was not started. This confirms the final build's window/process identity independently of the earlier fixture window.
- No screenshot was saved. A UI connector call during the second app lookup automatically emitted raw accessibility text once; that value is omitted from this record and was not copied into repository files, metrics, or subsequent UI output. Subsequent AX inspection was filtered in memory to status booleans.

## F. Real clipboard verification

- **Copy Result clicked:** YES, once.
- **Action callback invoked:** YES; the status changed after the guarded button action.
- **AppKit success observed:** YES — “Copied. Paste manually where you want the text.”
- **TextEdit paste attempted:** YES — a new blank, unsaved TextEdit document was set to plain-text mode, then ⌘V was pressed.
- **Paste succeeded:** NO for the acceptance test. TextEdit contained nonempty text, but the value did not compare equal to the app's final-result value.
- **Text matched:** NO in the in-memory accessibility-value comparison; **visual equality remains UNVERIFIED**. No transcript was included here.
- **Keyboard activation verified:** NO. The Return-key clipboard replacement was not repeated after the first copy/paste comparison failed.
- **Cancel and Dismiss verified:** YES. Dismiss cleared the first result and disabled Copy Result. A separate long fixture session verified Cancel clearing and disabled Copy Result; source and privacy tests confirm Cancel does not invoke the writer.

The unsaved TextEdit document was discarded without saving. No clipboard value was read before Copy, restored, hashed, saved, or committed. The manual ⌘V was the requested paste action after the explicit app copy.

## G. Privacy and security

The verification ID reveals only a short operator-chosen development token and the local process ID to a person viewing the app. It is bounded and ASCII-only to prevent control-character/title injection. It is not a product setting, transcript, audio field, model path, clipboard value, or environment-variable dump. Launch output includes only the app bundle path and optional verification ID; launch diagnostics include no recognized content.

The identity option creates no clipboard access. The app's only general-pasteboard writer remains behind the explicit, eligible Copy Result action. Isolated unit/integration tests do not depend on the system general clipboard. No TCC or security-database changes were made. The TextEdit document was not saved, and no transcript, audio, screenshot, app binary, model, secret, or clipboard value was added to the repository.

## H. Reproduction

Use macOS with the pinned local Moonshine source/runtime/model/WAV already authorized and available. The successful run used:

```sh
export MOONSHINE_SOURCE_DIR=/private/tmp/outflow-feasibility/moonshine-234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_BUILD_DIR="$(mktemp -d /private/tmp/outflow-prompt79i.XXXXXX)"
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"
scripts/run_macos_mic_probe.sh \
  --verification-id "79I-COPY-TEST-01" \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 12
```

Before interaction, compare the visible title and label with the requested ID and PID. Verify the PID against the executable in the new `.app` bundle; do not act on an older title without the ID. Wait for partial/final result, confirm Copy Result is enabled, and read the clipboard-overwrite warning before clicking. For manual paste, create a new blank unsaved TextEdit document in plain-text mode and press ⌘V. Compare without saving, logging, or screenshotting the transcript; discard the document. Dismiss and verify Copy Result disables.

For a Cancel check with adequate time before automatic finalization, use a separate process and a longer fixture limit:

```sh
scripts/run_macos_mic_probe.sh \
  --verification-id "79I-CANCEL-TEST-02" \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 120
```

Start the fixture, wait for a nonempty partial while Listening, press Cancel, verify both previews clear and Copy Result is disabled, then Dismiss. Do not click Copy in this Cancel run.

The focused and full regression commands are listed in §D and in the Prompt 79I section of `README.md`. If a test launched from a restricted shell reports `kLSNoExecutableErr`, rerun only in the permitted graphical LaunchServices context; it is not a pasteboard status. An automation connector may resolve an older window or fail to attach; use the title and PID to select the exact visible instance. If no correct window can be reached, stop the desktop attempt rather than changing clipboard APIs or system protections.

## I. Remaining blockers

- **Application behavior:** One correctly identified Copy action returned AppKit success, but one nonempty manual TextEdit paste did not compare equal to the final result. This is a real mismatch observation with cause **undetermined**; it does not establish permission denial or a particular writer defect. Do not change the pasteboard API until a new reproducible failure is established.
- **Desktop test gaps:** The fixture auto-finalized before a manual Stop click. Return activation and a visually reviewed character-by-character comparison were not run. The second clipboard replacement was skipped after the first comparison failed.
- **Host/automation:** LaunchServices failed once in the restricted command context and passed when rerun in the permitted desktop context. The UI connector initially timed out while opening TextEdit, then attached. Ghostty and Terminal UI attachment were denied by the connector; the task proceeded using LaunchServices and the exact app/window ID. The final process lookup required elevated read-only `ps`/`lsof` access after sandbox denial.
- **Production:** No production Tauri work was started. T-046/T-047, T-010/T-011, AC-10, physical 4 GB/resource/quality gates, installer/signing, full platform matrix, legal/license/accessibility reviews, and MVP completion remain unresolved.

## J. GitHub delivery

- Branch: `codex/prompt79i-copy-result-identity`, based on `0a3748fd2f7120809c06fa50779fdd0fe32202e5`.
- Changed files: `README.md`; `dev/macos-mic-probe/app.mm`; `dev/macos-mic-probe/verification_identity.hpp`; `dev/macos-mic-probe/tests/core_test.cpp`; `scripts/run_macos_mic_probe.sh`; `scripts/test_macos_mic_probe.sh`; `scripts/tests/test_macos_mic_probe_privacy.py`; `docs/TASKS.md`; and this handoff.
- Delivery commit, clean worktree, push result, and verified remote branch head are completed and recorded in the final implementation report. This file avoids self-referencing the SHA of its own containing commit.
- No model, fixture recording, transcript, clipboard value, screenshot, application binary, private log, or secret is committed.

## K. Next recommendation

Before another production task, run one focused, authorized desktop reproduction of the success-status/paste mismatch with a fixed synthetic value and an operator-confirmed TextEdit paste. Keep actual speech out of diagnostics. Preserve the current writer unless that reproduction establishes a specific defect. Then, if the manual paste path is understood, proceed to the next small production task only under its existing dependencies and approvals; do not mark T-046/T-047, AC-10, or the MVP complete.
