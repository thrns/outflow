# M1 feasibility baseline handoff

**Result: IN PROGRESS.** The advanced source lineage is preserved. The exact-byte synthetic pasteboard check passes, and the pinned native probe builds and recognizes its local smoke fixture. T-010 remains blocked: prior process-tree measurements miss the RSS cap or latency target, current tree sampling is denied by the host sandbox, and physical 4 GB and licensed WER evidence are unavailable. The real TextEdit mismatch remains unexplained. No production task or acceptance criterion is complete.

## Repository and result identity

- Repository: thrns/outflow.
- Audited main base: efb3ba8a7b48f608a0eb5aaf29a3f656f57f893f.
- Verified advanced source: codex/ac07-verified-bundle-scoring at f5ceff01a2ac0feab7ccefe630e90bae56afbbef; 88 commits ahead and 0 behind main at audit.
- Integration branch: feature/mvp-feasibility-baseline, created in a separate worktree from the advanced source.
- Final implementation SHA before this handoff (code, tests, README): 8db05c34e28e1bb91233e52fb4fbcba3a9f0dc52. This is the feature-branch tip immediately before the evidence-only commit.
- Prior task commit: 75e813409a0ef9ad9b9eaec0284045e822e0da94.
- Scope IDs: T-010, T-011, T-046, T-047; F-06/F-10; UF-05/UF-08; AC-06/AC-07/AC-10. All remain open.

## Changes

- dev/macos-mic-probe/tests/pasteboard_integration_test.mm — expanded the isolated AppKit check to eight fixed synthetic forms: ASCII, tabs and spaces, Unicode, emoji, composed and decomposed accents, LF/CRLF/CR, and multiline text. Each explicit write uses the production UTF-8 conversion and writer, then compares the returned string type after UTF-8 encoding byte-for-byte with the fixture. The test reads only its unique pasteboard.
- scripts/tests/test_macos_mic_probe_privacy.py — verifies fixture coverage, exact byte comparison, and separation from generalPasteboard.
- scripts/run_native_asr_benchmark.py — corrected header parsing for the actual pinned parenthesized C macros for header version and model architecture.
- scripts/tests/test_native_asr_benchmark.py — added a regression test that fails under the former parser and accepts the pinned macro form.
- src/app/VocabularyPreviewList.test.tsx — raised the existing 500-entry test timeout from 30 to 60 seconds after a full-suite run exceeded 30 seconds under load. No UI behavior changed.
- README.md — clarified the current preview, native probe prerequisites and limitations, current LaunchServices issue, and known TextEdit boundary.
- evidence/mvp/mvp-01-feasibility-baseline.md — this evidence handoff.
- docs/TASKS.md is unchanged because no acceptance criterion has its required evidence.

No production Tauri integration, speech/audio/vocabulary/cleanup implementation, dependency, lockfile, model, native runtime, recording, transcript, credential, or machine-specific path was added to the repository.

## Environment and native identities

- macOS 26.6.2, arm64, Mac15,6, Apple M3 Pro, 18 GB physical RAM. This is not the required physical 4 GB test system.
- Python 3.13.13; Node 22.23.3/npm 10.9.9; Rust/Cargo 1.99.1.
- The clean local Moonshine source verified at 234f60faa0eb388b01cdf7e60aca232af37aefda. Header version 30000 and Small Streaming architecture 4 were freshly verified.
- The existing native build script verified the arm64 Moonshine runtime (SHA-256 42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5), the arm64 ONNX Runtime 1.23.2 binary (SHA-256 480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4), and the declared dependency chain.
- All eight locally present Small Streaming model files passed the repository manifest hash checks; total model bytes: 142,300,974. Redistribution remains undetermined and license review remains pending.
- The source, runtime and model remained outside the repository. No runtime/model download or upstream source edit occurred.

## Validation commands and results

| Command or operation | Status | Result |
|---|---|---|
| git fetch origin main codex/ac07-verified-bundle-scoring; ancestry check | PASS | Advanced tip matched the audited SHA; 88 ahead, 0 behind main. |
| npm ci | PASS | 204 locked packages installed; no lockfile change. |
| npm run typecheck | PASS | TypeScript check passed. |
| npm run lint | PASS | ESLint passed. |
| npm run test -- --run | PASS | 11 files; 160 tests passed. |
| npm run build | PASS | Vite production build passed; 29 modules. |
| cargo fmt --all -- --check | PASS | No formatting changes. |
| cargo check --workspace --locked | PASS | Workspace check passed. |
| cargo clippy --workspace --all-targets --locked -- -D warnings | PASS | No warnings. |
| cargo test --workspace --locked | PASS | Unit, integration, and documentation tests passed. |
| PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v | PASS | 612 tests passed; one licensed human-reference native ASR test skipped because no ready corpus is configured. The new header parser regression is included. |
| bash -n scripts/build_macos_mic_probe.sh scripts/run_macos_mic_probe.sh scripts/test_macos_mic_probe.sh | PASS | All three scripts parse. |
| scripts/build_macos_mic_probe.sh with the verified local source/runtime | PASS | App bundle compiled and signed locally; runtime identity check passed. |
| scripts/test_macos_mic_probe.sh | FAIL at final desktop launch | Core tests passed (16 groups, including 100 state cycles); worker tests passed (9 lifecycle groups, including 100 injected cycles); malformed launch checks passed (5 verification-ID and 4 synthetic-launch cases). Its final open -n -W launch returned kLSNoExecutableErr before the embedded AppKit test could report. |
| scripts/test_macos_mic_probe.sh --real-fixture, three fresh helper runs, no process-tree sampler | FAIL at final desktop launch; fixture subtests PASS | Local model was verified and each of the three 12-second paced fixture runs produced nonempty partial/final output, 17 partial updates, and zero queue overflows. The same later LaunchServices step returned kLSNoExecutableErr. |
| Existing isolated AppKit harness in a temporary desktop-launched runner | PASS | Eight fixed synthetic forms, eight explicit writes, exact UTF-8 round-trip bytes, unique pasteboard released. No general clipboard access. The shell-based open path cannot launch temporary app bundles on this host; the test result was written by the desktop-launched runner. |
| Existing process-tree RSS sampler at 100 ms | BLOCKED | macOS process listing was denied with PermissionError. No new process-tree RSS measurement is claimed. |
| git diff --check | PASS | No whitespace errors after final edits. |

The ordinary and real-fixture script exit codes are reported as failures because the script includes the unsuccessful LaunchServices step. Their earlier native core/worker and fixture results are reported separately above. The manual desktop test uses the existing AppKit test function; it does not turn the script's LaunchServices result into a pass.

## Copy Result boundaries

| Boundary | Status | Evidence |
|---|---|---|
| A. Session-owned final UTF-8 text to displayed AppKit result | PRIOR PASS; not repeated in this run | Prompt 79K records an exact MATCH for the verified timed fixture. The current work did not inspect or retain any transcript text. |
| B. Explicit Copy to pasteboard string bytes | PASS for synthetic cases | The existing writer and production UTF-8 conversion passed exact byte comparison for eight fixed fixtures on an isolated unique pasteboard. This confirms the synthetic string-type round trip, not a general clipboard read or the TextEdit destination. |
| C. Pasteboard content to TextEdit document value | KNOWN MISMATCH; unresolved | Prompt 79K observed AppKit write SUCCESS and a nonempty TextEdit paste, but the complete document value compared unequal to the final result. Cause and visual equality remain unknown. No new TextEdit document or general clipboard content was inspected in this run. |

No Outflow clipboard encoding defect is established. Explicit Copy, current-session identity checks, and no automatic retry/overwrite remain intact. AC-10 stays open. The required next destination check is one fixed synthetic sample copied once to a new blank, unsaved plain-text TextEdit document; compare the entire value without normalization, record only equality/status, discard without saving, and stop on mismatch.

## Native feasibility and next decision

The fresh helper build and paced fixture runs are smoke evidence only. Values below are single-process getrusage high-water measurements, not sampled process-tree RSS and not the complete Tauri application. Each run used a fresh helper process, 12 seconds of CPU-only 16 kHz fixture audio, and produced 17 partial updates with zero queue overflows. Total session time includes the paced capture. These three-run samples do not establish a release p95.

Default threading:

| Run | Model load ms | First partial ms | Stop-to-final ms | Total session ms | Process CPU ms | CPU RTF | Helper high-water bytes | Threads |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 276.058 | 1949.106 | 97.015 | 12296.110 | 10931.101 | 0.911 | 602292224 | 22 |
| 2 | 227.829 | 1919.348 | 108.941 | 12259.564 | 11168.984 | 0.931 | 648740864 | 22 |
| 3 | 226.810 | 1960.356 | 111.876 | 12265.314 | 12015.952 | 1.001 | 517783552 | 22 |

With the upstream MOONSHINE_ORT_SINGLE_THREAD=1 reliability switch:

| Run | Model load ms | First partial ms | Stop-to-final ms | Total session ms | Process CPU ms | CPU RTF | Helper high-water bytes | Threads |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 288.178 | 2040.585 | 290.925 | 12501.152 | 5212.736 | 0.434 | 652787712 | 2 |
| 2 | 316.827 | 2075.346 | 323.197 | 12576.889 | 5275.146 | 0.440 | 653082624 | 2 |
| 3 | 257.448 | 2022.424 | 276.797 | 12453.088 | 5096.590 | 0.425 | 637960192 | 2 |

The setting lowered helper CPU time in these paced runs but did not lower this in-process high-water measurement; it increased first-partial and stop-to-final times. It remains an experimental upstream reliability setting, not a production recommendation. A 100 ms process-tree sampler was attempted separately but the sandbox denied process listing.

Existing Prompt 79D measured evidence was inspected, not rerun as part of this task: ten repeated native-helper runs per configuration, 100 ms process-tree sampling, same M3 Pro host and pinned build. Default threading reached 1,010,614,272 bytes (2/10 exceeded the 950,000,000-byte threshold), with p95 session latency 1,214.0 ms. MOONSHINE_ORT_SINGLE_THREAD=1 reached 907,902,976 bytes, but p95 session latency rose to 3,269.5 ms. The single-thread switch is an upstream reliability setting, not an approved production profile. Both are native helper results; neither certifies the full app or physical 4 GB hardware.

**Decision:** Small requires an approved optimization; Tiny evaluation is required before a model go/no-go. T-010 remains BLOCKED. No licensed human-reference WER run is available; Tiny assets/performance/quality and redistribution approval are absent; physical 4 GB performance and full-app process-tree memory are unmeasured. Keep AC-06, AC-07, AC-10, T-010 and T-011 open. Do not start the production Tauri application while the T-010 gate is blocked.

## Recommended next MVP milestone

Complete T-010 qualification before T-011: obtain a physical 4 GB CPU-only reference system and a lawful, human-verified WER fixture set; secure separate authorization before acquiring the pinned Tiny artifacts; then use the existing benchmark and sampler on repeated Small/Tiny runs to measure WER, model load, partial and final latency, CPU, process-tree RSS, and installed bytes. Record the candidate decision without changing the hard limits. Only proceed to the production Tauri shell if one candidate passes every required gate.
