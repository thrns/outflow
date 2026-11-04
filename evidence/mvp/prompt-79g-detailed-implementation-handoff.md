# Prompt 79G — detailed implementation handoff

**Status: PARTIALLY VERIFIED.** This is evidence for the development-only AppKit prototype. It does not complete production T-046/T-047, T-010, T-011, AC-10, or the MVP.

## 1. Starting point and prior behavior

- Starting branch: `codex/prompt79f-mic-cancellation`
- Starting HEAD: `52336d7c6f3bde1dbf431835bb0cec2ca99d3f34`
- Task branch: `codex/prompt79g-copy-result`
- The Prompt 79F tree was clean before branch creation. Its existing probe captured microphone audio or replayed a local fixture, streamed it to the pinned Moonshine Small Streaming runtime, displayed temporary partial text, and displayed a final result after Stop. Cancel invalidated/terminated capture or fixture work, joined the worker, and cleared preview/final text; Dismiss returned to Idle. There was no clipboard or insertion path.
- The relevant architecture is the isolated macOS development probe under C-07. The production Tauri shell, target identity validation, protected-field handling, insertion, and cross-platform adapters are outside this change.

## 2. Changed files and responsibilities

The Prompt 79G branch changes these repository paths:

- `dev/macos-mic-probe/app.mm` — AppKit button, status and privacy warning; final-result ownership; NSPasteboard writer; lifecycle clearing; button enablement; Return key equivalent.
- `dev/macos-mic-probe/copy_result_policy.hpp` — pure eligibility predicate and injectable one-call writer policy.
- `dev/macos-mic-probe/tests/core_test.cpp` — fake-writer policy coverage for states, text, stale sessions, call counts, and failure retention.
- `scripts/tests/test_macos_mic_probe_privacy.py` — source contracts for the write boundary, no content read, lifecycle behavior, result eligibility, and keyboard/accessibility properties.
- `README.md` — Prompt 79G build/run, button behavior, failure recovery, privacy warning, and test instructions.
- `docs/TASKS.md` — truthful development-prototype evidence; T-046/T-047 and all production checkboxes remain open.
- `evidence/mvp/prompt-79g-detailed-implementation-handoff.md` — this handoff.

No dependency, lockfile, model, Moonshine source, C ABI, RSS limit, ASR selection, or GitHub workflow changed. Temporary silent fixture/app copies and build outputs were kept under `/private/tmp`, not in Git.

## 3. Final transcript ownership and lifetime

`handleWorkerEvent:` accepts a final event only after the existing current-session/state guards. For an accepted final event, the delegate moves `event.text` into `_copy_result_text`, associates it with `_copy_result_session_id`, and renders that exact string into the existing read-only, selectable final `NSTextView`. Partial events continue to update only the temporary preview; they never populate the copyable string.

An empty actual final string remains empty in `_copy_result_text`; the UI may show “No speech was recognized in this session.” as display-only text. That placeholder is not passed as transcript data. No cleanup, punctuation change, trimming, or reformatting is applied to actual final bytes. Conversion to `NSString` uses the byte length and UTF-8 encoding, preserving accepted Unicode, punctuation, whitespace, and newlines.

`clearFinalResult` scrubs `_copy_result_text` through the probe's sensitive-text helper, zeros the result session ID, and clears the final view and copy status. It runs at the start of a new session, on Cancel, Dismiss, Error/Cancelled worker events, app shutdown, and window closure. A successful result remains visible until Dismiss.

## 4. Eligibility and invocation policy

`copy_result_eligible` returns true only when all of these are true:

1. State is `ResultReady`.
2. The accepted session ID is nonzero.
3. The final-text owner ID equals the current accepted session ID.
4. The actual final transcript is nonempty.

`refreshControls` applies that predicate to the button's enabled state. `copyResultPressed:` calls the policy again at activation time, so disabled-state rendering is not the security boundary. A stale final event cannot make an older session eligible because the current event/session guards and the owner-ID equality must both pass. A new session clears the previous text before it can accept new events.

The policy receives an injected writer. It invokes it once for one eligible activation and never for rejected states. The automated fake writer verifies no invocation before activation, rejection in every other state, empty text, placeholder exclusion, session mismatch, exact UTF-8/emoji/multiline/punctuation pass-through, one request per activation, and retained eligibility/text after writer failure.

## 5. Explicit action to NSPasteboard

The only production source call to `write_clipboard_text` is inside `copyResultPressed:`. That method passes the guarded current state/session/result to `request_copy_result`. The injected AppKit writer creates an `NSString` from the exact UTF-8 byte range and calls `[[NSPasteboard generalPasteboard] writeObjects:@[value]]` once. It does not call `clearContents`, `declareTypes`, read a prior clipboard item, retry, synthesize input, or inspect another application.

The app reports “Copied. Paste manually where you want the text.” only when `writeObjects:` returns success. It checks the OS return value and does not claim a failed write is rolled back atomically. On failure, the transcript stays visible/selectable and the app suggests manual selection and ⌘C. `accessBehavior` is read only to tailor the error message; clipboard content is never read by the app.

The warning displayed before the button action is exactly: “Copied text may be retained by clipboard managers or synced to other devices.” Clipboard managers and OS synchronization may retain the copied content.

## 6. Failure behavior and clipboard verification limit

On the tested macOS host, both a mouse click and Return activation reached the guarded writer, but `NSPasteboard.writeObjects:` returned failure. The UI showed the manual-copy recovery message and retained the final preview. No successful **Copy Result** write was observed.

A later manual-selection/⌘C fallback attempt did not result in the fixture transcript appearing in the blank TextEdit test document; it exposed clipboard content that was not the fixture result. I stopped that check, did not record or save the unexpected content, discarded the unsaved test document, and did not clear or restore the pasteboard. This means successful copy and manual paste are unverified. An earlier disposable-sentinel check appeared unchanged after a no-click fixture run and failed button attempt, but the final mismatch prevents treating the end-to-end clipboard state as conclusively verified.

Source and fake-writer tests prove that non-copy operations do not call the writer. Those tests cannot establish the macOS pasteboard state or successful desktop copy behavior. No claim is made that the previous clipboard is guaranteed to survive a failed NSPasteboard write; the implementation does not intentionally clear it before the single write call.

## 7. Keyboard access and UI feedback

The button is visibly named **Copy Result**, has an accessibility help string, and uses Return as its key equivalent to invoke the same guarded action as a mouse click. This Return activation was exercised on a ready fixture result and reached the same error path. The final preview remains noneditable but selectable. The concise warning and a labeled status area are placed next to/below the final result. The app keeps the preview visible after copy failure and only clears it through the existing lifecycle actions.

## 8. Cancel, Dismiss, and stale-session safeguards

- **Cancel:** the existing cancellation sequence stops the source and worker before clearing the preview and copy-owned final string. A post-cancel late worker result is rejected by session/state handling. The UI exercise showed Cancelled, cleared partial/final panes, and disabled Copy Result.
- **Dismiss:** joins terminal worker cleanup, clears final text/session ownership, and returns to Idle. The UI exercise showed empty panes and disabled Copy Result after Dismiss.
- **New session:** clears the previous final result before preparing another session and assigns a distinct session ID.
- **Error/Cancelled event:** clears the copy-owned final string and view.
- **Shutdown/window close:** clears volatile result ownership after stopping capture and joining the worker.
- No clipboard write is present in Start, Stop, Cancel, Dismiss, worker finalization, shutdown, or window-close handlers. The only writer call site is the explicit button action.

## 9. Automated tests and results

All commands below were run on macOS 26.6.2 (build 25G83), arm64. The full test-suite skip is explicit rather than counted as a pass.

| Command | Result |
|---|---|
| `scripts/build_macos_mic_probe.sh` with the pinned existing source/libraries and fresh `OUTFLOW_MIC_PROBE_BUILD_DIR=/private/tmp/outflow-prompt79g-finalbuild` | PASS; bundle built and local runtime identity verification passed |
| `scripts/test_macos_mic_probe.sh` with required local Moonshine environment variables | PASS; 13 core groups, 100 state cycles; 9 worker lifecycle groups, 100 injected start/stop/cancel/dismiss cycles |
| `scripts/test_macos_mic_probe.sh --real-fixture` with the existing local model/WAV | PASS; same core/worker suites and 12-second real ASR fixture, nonempty partial/final, zero overflow |
| `python3 scripts/tests/test_macos_mic_probe_privacy.py` | PASS; 12 tests |
| `python3 -m unittest discover -s scripts/tests -v` | PASS; 385 tests, 1 skipped. The skip requires the separate approved/licensed corpus and its native-runner/model/fixture environment variables, which were not configured for this suite run. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 353 passed, 0 failed, 0 ignored; 0 doctest failures |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 10 files, 136 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; typecheck and Vite production build (28 modules) |
| `git diff --check` | PASS after final source/docs changes |

The focused native commands initially exited before compilation when required environment variables were unset. They were rerun with the existing pinned local source, library, runtime, model, and WAV paths listed in §12; those configured runs passed. No assets were downloaded.

## 10. Fixture and visual verification

The final AppKit bundle was launched through `scripts/run_macos_mic_probe.sh` with the already-present 12-second `two_cities_16k.wav` fixture. Start showed nonempty temporary partial text. Stop produced a nonempty final result, released the fixture worker, and enabled Copy Result only in ResultReady. The copy warning remained visible. Mouse and Return actions both reached the same guarded error status because macOS reported the write as failed. The copied text could not be confirmed in a destination field.

A four-second synthetic silent PCM16 mono WAV was created only in `/private/tmp` and run using a temporary app copy with a unique bundle ID. It reached ResultReady, displayed “No speech was recognized in this session.” and kept Copy Result disabled. The app was then visually exercised through Dismiss and a second fixture Start→Cancel; Dismiss cleared both panes and disabled Copy, while Cancel cleared the temporary preview and left Copy disabled. The UI used fixture replay, not microphone audio.

The last real-fixture harness run processed 12.000 seconds of 16 kHz audio with 17 partial updates, 0 queue overflows, queue maximum depth 1, 144.988 ms stop-to-final time, and a measured native-process peak RSS of 593,362,944 bytes. This is one fixture run on this host, not a release-gate or quality result.

The normal LaunchServices run from the restricted command sandbox first returned `kLSNoExecutableErr`; the same authorized local app launch succeeded when run with the permitted escalated desktop-launch context. The in-app microphone Start→Speak→Stop→Copy→manual Paste sequence is **NOT RUN**: this environment provided no operator speech input. No microphone recording is claimed from this Prompt 79G work.

## 11. Privacy boundaries and limitations

- The app does not read, save, log, or restore prior clipboard content. It has exactly one guarded transcript write call site.
- Clipboard managers or OS synchronization may retain text after a successful explicit copy.
- The OS reported a failed write on this host. A failed return does not establish atomic rollback, and the app makes no such promise.
- The manual copy fallback did not reproduce the expected fixture text in the test field. The unexpected contents were not saved or recorded; the test document was discarded. The system clipboard was not cleared or restored.
- Fixture recordings and transcripts were not added to the repository or handoff. Temporary test fixtures and app copies remained under `/private/tmp`.
- No Accessibility permission, cross-app inspection, keyboard insertion, auto-paste, or silent clipboard overwrite was added.

## 12. Reproduction commands

These commands use the already-authorized local assets and match the paths used for verification. They do not download assets:

```sh
export MOONSHINE_SOURCE_DIR=/private/tmp/outflow-feasibility/moonshine-234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_BUILD_DIR=/private/tmp/outflow-prompt79g-repro-build
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"
scripts/run_macos_mic_probe.sh \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 12
scripts/test_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh --real-fixture
python3 scripts/tests/test_macos_mic_probe_privacy.py
python3 -m unittest discover -s scripts/tests -v
```

Run `scripts/run_macos_mic_probe.sh` without fixture options only when a person is ready to speak into the authorized microphone. A fixture-only visual run is not evidence for the physical microphone path.

## 13. Branch, commits, files, and worktree

- Branch: `codex/prompt79g-copy-result`, created from the supplied Prompt 79F HEAD.
- Starting commit: `52336d7c6f3bde1dbf431835bb0cec2ca99d3f34`.
- Implementation/evidence commit: `c42e1a1531e4e39324682b92f1bf24411c872e8e` (`feat(native): add explicit copy-result action to macOS probe`). This commit contains all seven changed paths listed in §2, including this handoff.
- The final completion response records the pushed branch's exact remote SHA. Since this handoff is itself part of the implementation commit, its own containing SHA is identified by the branch HEAD rather than duplicated as self-referential content.
- Changed repository paths are listed in §2.
- The worktree was clean after the implementation commit. The final completion response records the post-push clean-worktree check. No model files, audio, transcript, screenshot, native binary, clipboard value, or secret is intended for Git.

## 14. Remaining production blockers and recommended next step

This branch only demonstrates an AppKit prototype action. Production Copy Result still requires the approved Tauri shell and task prerequisites, typed command/session boundaries, production target and protected-field policy, robust macOS clipboard integration on a supported target, full AC-10 keyboard/accessibility and failure matrix, and platform-specific behavior. T-046 depends on T-011 and T-042; do not bypass those dependencies. The host's failed NSPasteboard return and unverified successful manual paste also need a supported macOS integration run before treating copy delivery as verified.

Recommended next step: have an operator resolve the host-specific clipboard denial/failure without changing the no-read/no-auto-copy policy, then repeat an explicit synthetic-result Copy Result→manual paste test in a blank field. Keep this prototype evidence separate while completing T-011/T-042 prerequisites for the production Tauri path. Do not mark T-046/T-047, T-010/T-011, AC-10, or the MVP complete from these results.
