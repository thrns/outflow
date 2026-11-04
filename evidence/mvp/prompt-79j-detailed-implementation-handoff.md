# Prompt 79J — detailed implementation handoff

**Status: VERIFIED — SYNTHETIC MOUSE AND RETURN PATHS PASS; PROMPT 79I CAUSE REMAINS UNKNOWN.** Both explicit activation paths reported AppKit success and pasted byte-for-byte equivalent text into separate verified new plain-text TextEdit documents. Both were discarded without saving, and Dismiss cleared each result. This verifies the fixed synthetic sample on this host; it does not explain the earlier real-result mismatch. This is development-prototype evidence only; T-046, T-047, T-010, T-011, AC-10, and MVP completion remain open.

**Scope:** F-10 / UF-08 / C-07 / TS-07 / AC-10; development probe follow-up for T-047. The product's clipboard writer and production Tauri implementation are unchanged.

## A. Starting point and Prompt 79I observation

- Repository: `/Users/bixxie/Desktop/Projects/outflow` (`thrns/outflow`).
- Starting branch: `codex/prompt79i-copy-result-identity`.
- Verified starting commit: `0f256777d8a0f29239b9b7e017b3f4ca9b770da5`.
- Starting worktree was clean. Work is on `codex/prompt79j-synthetic-copy-diagnostic`.
- Prompt 79I had identified the correct development app, observed a nonempty final result, clicked Copy Result, and received AppKit success. TextEdit received nonempty text after ⌘V, but its in-memory value did not compare equal to the final result. The cause was undetermined. Return activation was not repeated.
- Prompt 79J does not infer that the writer, TextEdit, or UI automation caused the earlier mismatch.

## B. Changed files and execution flow

- `dev/macos-mic-probe/launch_policy.hpp` validates that synthetic mode has one valid verification ID and no timed-fixture or explicit microphone-capture argument. It also describes the three mutually exclusive input modes; absence of the new flag still selects microphone mode.
- `dev/macos-mic-probe/synthetic_copy_diagnostic.hpp` defines the only source constant for the fixed synthetic UTF-8 text and a narrowly named helper that advances an Idle session through Preparing, Listening, Finalizing, and ResultReady without audio or recognition.
- `dev/macos-mic-probe/app.mm` parses the new flag and rejects invalid combinations in `main` before AppKit starts. The synthetic window has an explicit label, omits the partial area, shows the fixed text in the existing selectable final-result area, and labels it as not recognized speech. It seeds the result under a nonzero session ID and blocks the Start, settings, worker, and audio-source paths. Dismiss uses the existing sensitive-result clearing and state-dismiss logic.
- `scripts/run_macos_mic_probe.sh` forwards the flag. Synthetic mode requires a verification ID but does not require or verify a runtime model directory; normal microphone and fixture launches retain existing model verification and arguments.
- `dev/macos-mic-probe/tests/core_test.cpp` covers launch validation, default/mutually exclusive modes, UTF-8 and sample shape, state ownership, one injected writer call, stale-session rejection, and cleared-result rejection.
- `scripts/test_macos_mic_probe.sh` adds four pre-AppKit rejection checks for invalid synthetic launches while retaining the isolated AppKit test.
- `scripts/tests/test_macos_mic_probe_privacy.py` adds source-contract checks for diagnostic isolation, startup pasteboard privacy, Dismiss clearing, the shared mouse/Return action, and continued use of a unique pasteboard in the integration test.
- `README.md` documents the mode, expected UI, controlled TextEdit procedure, privacy warning, classifications, test commands, and troubleshooting.
- `docs/TASKS.md` records the Prompt 79J evidence while leaving the T-046/T-047 checkboxes and release tasks open.

## C. Existing state policy and writer reuse

No edit was made to `copyResultPressed:`, `copy_result_policy.hpp`, or `pasteboard_writer.hpp`. The existing path remains:

`Copy Result button or Return → copyResultPressed: → current-session eligibility → UTF-8 conversion → general pasteboard → clearContents → writeObjects: → AppKit result`

The button is enabled through `copy_result_eligible` only for nonempty text owned by the current ResultReady session. The writer is called once by the eligible explicit action. It does not read, restore, or retry clipboard contents. The same existing replacement warning stays visible. Dismiss, startup, and shutdown do not write.

## D. Normal microphone and fixture behavior

Without `--synthetic-copy-diagnostic`, input-mode selection defaults to the existing microphone path or the timed-fixture path when `--timed-fixture` is supplied. Synthetic mode guards prevent a call into microphone capture, fixture production, a recognition worker, or model loading. The authorized real-fixture regression still produces partial and final results, and existing worker lifecycle tests pass. No normal capture or worker implementation was changed.

## E. Test construction and privacy safeguards

`kSyntheticCopyDiagnosticSample` is defined once:

```text
OUTFLOW-SYNTHETIC-COPY-START
Café — plain-text clipboard check! 😀
OUTFLOW-SYNTHETIC-COPY-END
```

Automated C++ checks validate UTF-8, exactly two LF newlines and no CR, both ASCII markers, the accented text, punctuation, and emoji. A fake injected writer records zero calls before explicit action and exactly one after the policy call. Tests reject a stale session and a cleared result. Source-contract tests verify that the startup branch has no pasteboard access and that Dismiss clears the result and refreshes the disabled button state.

The automated AppKit integration continues to use `pasteboardWithUniqueName` with four fixed synthetic forms. It never reads or writes `generalPasteboard`. The controlled desktop test used only the expected synthetic value in a newly verified empty TextEdit document; no unrelated clipboard value was inspected. The TextEdit document was discarded without saving. No transcript, recording, clipboard value, screenshot, app binary, model, or secret was added to the repository.

## F. Automated test results

Host: macOS 26.6.2, arm64, model identifier `Mac15,6`, 18 GiB RAM. This is not the required 4 GiB release machine. The real-fixture number below is a single native harness process measurement, not aggregate release process-tree certification.

The final fresh probe bundle built at `/private/tmp/outflow-prompt79j.Au7pAr/OutflowMicProbe.app` using the already-authorized local pinned runtime. The configured model files and WAV fixture remained outside the repository.

| Command | Result |
|---|---|
| `scripts/build_macos_mic_probe.sh` | PASS; fresh app bundle built and native runtime identity verified |
| `scripts/test_macos_mic_probe.sh` | PASS; 15 core groups, 9 worker lifecycle groups, 5 malformed verification-ID rejections, 4 invalid synthetic-launch rejections, isolated AppKit integration PASS (4 synthetic values, 4 explicit replacements, unique pasteboard released) |
| `scripts/test_macos_mic_probe.sh --real-fixture` | PASS; 12-second authorized fixture had nonempty partial and final output, 17 partial updates, 0 queue overflows, maximum queue depth 1; reported native-process peak RSS 816,824,320 bytes. This is not WER and not a release-gate measurement. |
| `python3 -m unittest discover -s scripts/tests -v` | PASS; 393 passed, 0 failed, 1 skipped. The skipped licensed-corpus native ASR integration lacked its configured runner, model, source, libraries, fixture manifest, and fixture root; it is NOT a WER pass. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 353 passed, 0 failed, 0 ignored across all reported test groups and doctests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 136 tests in 10 files |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite built 28 modules |
| `bash -n scripts/build_macos_mic_probe.sh scripts/run_macos_mic_probe.sh scripts/test_macos_mic_probe.sh` | PASS |
| `git diff --check` | PASS before final commit (rerun after evidence updates) |

Synthetic launch and isolated AppKit integration ran as separate app launches. The shell sandbox alone could not reach LaunchServices/AppKit and returned `kLSNoExecutableErr`; rerunning the requested native suite in the desktop user context succeeded. No workaround changed the clipboard implementation.

## G. Controlled desktop verification

- Fresh app: `/private/tmp/outflow-prompt79j.Au7pAr/OutflowMicProbe.app`.
- Visible identity: `Outflow Mic Probe — Dev — 79J-SYNTH-01 — PID 78657`.
- Read-only process check: `ps -p 78657 -o pid=,comm=` returned PID `78657` and `/private/tmp/outflow-prompt79j.Au7pAr/OutflowMicProbe.app/Contents/MacOS/OutflowMicProbe`.
- Before the click, the synthetic label, fixed sample, enabled Copy Result button, and clipboard-replacement warning were visible. Source and privacy checks established no startup pasteboard call; no system general-pasteboard read was made.
- Mouse action: clicked Copy Result exactly once. UI status was `Copied. Paste manually where you want the text.` (AppKit success).
- Destination setup: created a new Untitled TextEdit document, converted it to plain text, visually verified it was empty, and confirmed the text view had keyboard focus.
- Paste action: one normal ⌘V. TextEdit received the complete sample. Its in-memory accessibility value matched the constant exactly: start marker, two LF line breaks, accented text, punctuation, emoji, and end marker. An independent screenshot showed the same three visible lines. No normalization or other transformation was observed.
- Cleanup: discarded the unsaved TextEdit document without saving. Dismiss cleared the synthetic sample and status and disabled Copy Result.
- Fresh Return-test app: same built bundle, visible identity `Outflow Mic Probe — Dev — 79J-RETURN-02 — PID 80201`; `ps -p 80201 -o pid=,comm=` confirmed `/private/tmp/outflow-prompt79j.Au7pAr/OutflowMicProbe.app/Contents/MacOS/OutflowMicProbe`. The previously completed `79J-SYNTH-01` window was initially selected by the accessibility controller; it was confirmed Dismissed/Idle and closed before selecting the new ID/PID. No action was taken in the older window.
- Keyboard action: after the operator approved one more clipboard replacement, the first Return from the focused result text view did not invoke Copy Result. Tab moved first responder to the standard window; one subsequent Return invoked the existing button key equivalent. The UI reported `Copied. Paste manually where you want the text.` Only this successful invocation wrote the clipboard.
- Return destination setup: created a separate new Untitled TextEdit document, converted it to plain text, and verified it was empty with the insertion caret in the text view before pasting.
- Return paste: one normal ⌘V. The complete in-memory document matched the fixed constant exactly, including both LF line breaks, punctuation, accent, and emoji. An independent screenshot matched all three lines. No accessibility/visual disagreement or transformation was observed.
- Cleanup: discarded the Return-test document without saving. Dismiss cleared the second synthetic result and status and disabled Copy Result. No additional clipboard write occurred.

## H. Evidence-based classification and remaining cause

**Classification:** Basic desktop clipboard path verified for this fixed synthetic sample on this host through both mouse and Return activation. Each path invoked the existing AppKit writer once; TextEdit received the pasted value; and the complete accessibility value and visual rendering matched exactly.

This does not explain the earlier Prompt 79I real-result mismatch. It narrows the remaining investigation to that specific real-result/session or its earlier target/comparison setup. The prior cause remains undetermined. No writer change or clipboard workaround is justified by the Prompt 79J evidence.

## I. Reproduction and host limitations

The run used the already-present pinned Moonshine source/runtime, model directory, and licensed development WAV fixture described in the preceding Prompt 79I README section. The synthetic launch itself did not receive `OUTFLOW_MIC_PROBE_MODEL_DIR` and did not read model assets.

```sh
export MOONSHINE_SOURCE_DIR=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_BUILD_DIR="$(mktemp -d /private/tmp/outflow-prompt79j.XXXXXX)"
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"

scripts/run_macos_mic_probe.sh \
  --verification-id "79J-SYNTH-01" \
  --synthetic-copy-diagnostic
```

The shell launcher needed the desktop user context for LaunchServices in this environment. The correct fresh process was confirmed using its visible ID/PID and `ps` executable path. The 18 GiB test host does not satisfy the 4 GiB hardware gate, and the one fixture RSS observation does not certify memory limits. The licensed WER corpus remains unconfigured.

## J. GitHub delivery

- Task branch: `codex/prompt79j-synthetic-copy-diagnostic`.
- Base branch and SHA: `codex/prompt79i-copy-result-identity` at `0f256777d8a0f29239b9b7e017b3f4ca9b770da5`.
- Changed files: `README.md`; `dev/macos-mic-probe/app.mm`; `dev/macos-mic-probe/launch_policy.hpp`; `dev/macos-mic-probe/synthetic_copy_diagnostic.hpp`; `dev/macos-mic-probe/tests/core_test.cpp`; `scripts/run_macos_mic_probe.sh`; `scripts/test_macos_mic_probe.sh`; `scripts/tests/test_macos_mic_probe_privacy.py`; `docs/TASKS.md`; and this handoff.
- Implementation commit: `97a7963583bccc18e767c1e41d45362cc8641f0d` (`test(native): isolate desktop copy paste with synthetic result`).
- `git push -u origin codex/prompt79j-synthetic-copy-diagnostic` succeeded. `git ls-remote origin refs/heads/codex/prompt79j-synthetic-copy-diagnostic` returned `97a7963583bccc18e767c1e41d45362cc8641f0d`; local `HEAD` matched. The worktree was clean and tracking `origin/codex/prompt79j-synthetic-copy-diagnostic` after this push.
- This delivery note records the first pushed implementation head. A metadata-only evidence update follows this commit so the Return-key outcome and delivery record are durable. A commit cannot embed its own hash; the exact final branch head after that update is reported in the final delivery message.
- No GitHub Actions workflow, runtime dependency, lockfile, production UI, model artifact, real transcript, recording, screenshot, or clipboard content was added.

## K. Recommended next small step

After the Return-key decision, retain the writer unchanged. If the previous real-transcript mismatch still needs explanation, run one later controlled comparison with a real result only after identifying a safe, nonpersonal reference fixture and a complete, verified destination comparison. Do not treat this synthetic pass as proof that the Prompt 79I content mismatch is explained or as completion of T-046/T-047/AC-10.
