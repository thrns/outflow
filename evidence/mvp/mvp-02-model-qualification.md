# MVP 02 — Native qualification readiness

**Date:** 2026-11-06

**Branch:** `feature/mvp-model-qualification`

**Base:** `fe70c87cba37bd7c14e677469ba7f6ddac469127`

**Result commit:** see the Git commit that contains this handoff.

**Host:** macOS 26.6.2 (25G83), arm64, Mac15,6 / Apple M3 Pro, 18 GB physical RAM.
**Scope:** T-010 evidence preparation for F-06/UF-05/AC-06; Copy Result verification boundary F-10/UF-08/AC-10. No task acceptance or release approval is claimed.

## Qualification decision

**T-010: NO-GO / BLOCKED. Neither candidate is approved.** The pinned source and local Small artifacts are identifiable, and the Tiny source contract is ready for a separately approved evaluation. The prerequisites for an evidence-backed Small/Tiny decision are incomplete:

- The frozen benchmark corpus has zero verified, human-referenced ready fixtures. No real corpus WER is available.
- Tiny's source contract is known, but no Tiny weights, complete local SHA-256 manifest, or acquisition approval exists. No Tiny inference was run and no weights were downloaded.
- The available MacBook Pro (Mac15,6, Apple M3 Pro, arm64, macOS 26.6.2) has 18 GB physical RAM. No low-memory constrained run was performed. It is not a physical 4 GB reference system. For macOS, PRD §5 permits the nearest available low-memory Apple Silicon with memory-constrained validation as supplemental fallback evidence; that evidence is also absent. Physical 4 GB remains the certification standard for applicable supported targets.
- Full-app aggregate process-tree RSS, macOS physical footprint, and installed application size are not measured. The 950,000,000-byte RSS and installed-footprint release thresholds remain unchanged.
- Final model/runtime license and redistribution approval is absent. The current asset manifests retain `review-pending`/`undetermined` states.

The 12-second fixture runs below are native helper smoke observations on the 18 GB host. They do not establish WER, p95 release latency, complete-app memory, physical 4 GB eligibility, or model approval.

## Candidate identity and local artifacts

| Candidate | Pinned source contract | Local artifact verification | Evaluation state |
|---|---|---|---|
| Small | Moonshine `234f60faa0eb388b01cdf7e60aca232af37aefda`; C API `30000`; architecture `4`; eight source-listed files; source-contract SHA-256 `bebe38ff906238d5b7339f6852a3b68c733ac4f38d11c11b8c2b039fb3831917` | Eight existing files verified against the current inventory and pinned source catalog; `142,300,974` bytes | Development default retained. No candidate decision. |
| Tiny | Same pinned source and C API; architecture `2`; eight source-derived file entries; source-contract SHA-256 `8e35ed5a13d5f70f9b6341d73be0572cc9c94a7974f04fd7ca0a46d5aa43e3b9` | **BLOCKED:** the local Tiny artifact directory is empty; no exact local SHA-256 values or approval | No inference. Explicit `--candidate tiny` path only. |

The native runtime probe verified the local arm64 `libmoonshine.dylib` SHA-256 `42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5` and ONNX Runtime SHA-256 `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`. The eight Small model file hashes remain in [`evidence/mvp/prompt-79-model-assets.json`](prompt-79-model-assets.json). The dated catalog path and computed source contract are not represented as an immutable model artifact revision.

The source-derived streaming file list is `adapter.ort`, `cross_kv.ort`, `decoder_kv.ort`, `encoder.ort`, `frontend.model.ort`, `frontend.weights.ort`, `streaming_config.json`, and `tokenizer.bin`. The pinned Tiny source catalog reports a total expected size of `45,233,659` bytes with its own per-file sizes and CRC32C values. This describes source metadata; it is not evidence that Tiny files are present or verified.

`prepare_native_asr_candidate_manifest.py --candidate tiny` exited `2` with an incomplete-artifact block. It does not download. The Tiny manifest writer records SHA-256 only for local files matching the pinned per-file source metadata and leaves approval unrecorded. The real benchmark also requires explicit Tiny selection and approval evidence. The comparison tool is review-only and blocks unless both validated candidate measurements use the same complete approved corpus and comparable verified physical CPU hardware.

## Fresh native Small fixture observations

The pinned local Small model, runtime and licensed `two_cities_16k.wav` fixture were used for three separate 12-second CPU-only helper runs. Each produced nonempty partial and final recognition, 17 partial updates, and zero queue overflow.

| Run | Model load (ms) | First partial (ms) | Stop-to-final (ms) | Helper high-water RSS (bytes) | 100 ms sampled process-tree peak RSS (bytes) | Samples / gaps |
|---|---:|---:|---:|---:|---:|---:|
| 1 | 289.866 | 1,950.147 | 150.158 | 643,809,280 | 643,809,280 | 128 / 0 |
| 2 | 331.036 | 1,939.867 | 161.556 | 633,765,888 | 633,044,992 | 125 / 0 |
| 3 | 290.715 | 1,895.112 | 859.743 | 686,604,288 | 686,604,288 | 132 / 0 |

The sampler backend was `macos-libproc-proc_listchildpids+PROC_PIDTASKALLINFO`; each sample contained one process in these helper runs. RSS sums intentionally include shared pages and can overcount. Sampling can miss short-lived processes and high-water peaks. Helper `getrusage` high-water RSS, sampled process-tree RSS, macOS physical footprint, and the complete Outflow application are different measures. Physical footprint was `not_collected`; the raw temporary traces are not committed because they contain process identifiers.

The reusable `run_native_asr_benchmark.py --candidate small` path exited `2` before native inference because the frozen corpus has zero verified ready fixtures. It wrote no measured result. The full ready-corpus native integration test was consequently skipped once in the 624-test Python suite because its eight explicit local fixture/model environment inputs were not configured. This does not conflict with the separate real-fixture smoke run above: that fixture has no corresponding approved human reference in the frozen benchmark corpus, so it cannot produce a valid WER result.

## macOS AppKit launch and clipboard boundary

The native test script built the `.app`, verified `CFBundleExecutable=OutflowMicProbe`, checked the executable exists inside `Contents/MacOS` and is mode `0755`, verified the ad-hoc code signature, and passed the native runtime identity check. The same isolated AppKit LaunchServices invocation returned `kLSNoExecutableErr` with no test result from both a temporary build directory and a stable build directory under the worktree. This removes a missing plist executable, non-executable file, invalid signature, and temp-only path as likely script/bundle causes. The internal LaunchServices reason remains unresolved in this host context; the CUA connector explicitly disallows Terminal.app here.

Result classification is fail-closed: only a missing result with the recognized LaunchServices `kLSNoExecutableErr` is `BLOCKED`; an explicit test failure, a nonzero launch with a PASS result, or any other missing result is `FAIL`. The native test output therefore reports core/worker PASS, real fixture PASS, and AppKit LaunchServices BLOCKED separately, and exits `2` without counting the bundle launch as a pass.

The added standalone `scripts/run_macos_appkit_pasteboard_test.sh` compiles the existing isolated eight-value AppKit test. Running it from this restricted shell produced the explicit result `FAIL unique pasteboard unavailable; released=false`; that is not a desktop-context pass and does not locate an Outflow writer defect. CUA did not permit Terminal.app, and TextEdit did not provide a usable window in this task, so the controlled TextEdit comparison is **BLOCKED / NOT RUN**. No Copy Result action, general-pasteboard read, or TextEdit paste was performed in this milestone.

For the authorized desktop check, open **Terminal.app** in the logged-in macOS session and run:

```sh
scripts/run_macos_appkit_pasteboard_test.sh
```

Expected isolated test output is an explicit `PASS isolated AppKit pasteboard integration tests: 8 fixed synthetic UTF-8 forms...`. A nonzero exit, missing result, or explicit `FAIL` is not a pass. The separate `.app` LaunchServices result remains `BLOCKED` unless that launch actually produces a valid result.

For the controlled TextEdit comparison, use the README's “Controlled TextEdit comparison” procedure: in the freshly built probe, start the identified `--synthetic-copy-diagnostic` view, verify its fixed sample and clipboard warning, click Copy Result once, paste once into a new blank unsaved plain-text TextEdit document, and compare the complete value exactly. The fixed sample has two LF line breaks, no trailing newline, NFC `é`, punctuation, and emoji. Record only `MATCH`/`MISMATCH` and the boundary; discard without saving. If desktop access is unavailable, leave it BLOCKED. Do not retry or inspect the general clipboard.

## Executed validation

| Command or check | Result |
|---|---|
| `npm ci` | PASS; 204 locked packages installed; no lockfile change |
| `npm run typecheck`, `npm run lint`, `npm run test -- --run`, `npm run build` | PASS; 160 frontend tests across 11 files |
| `cargo fmt --all -- --check`, `cargo check --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` | PASS |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS; 625 tests, one expected native corpus integration skip |
| Shell syntax checks for build, app runner, native test, and standalone AppKit runner | PASS |
| Native batch helper compilation and unsupported `--model-arch 3` rejection | PASS; invalid architecture exits `2` with empty stdout |
| Three-run `scripts/test_macos_mic_probe.sh --real-fixture` | Native core/worker PASS; Small fixture PASS; AppKit launch BLOCKED; overall exit `2` |
| Tiny manifest preparation | BLOCKED; local artifact directory incomplete; exit `2` |
| Reusable Small native benchmark | BLOCKED; zero verified ready corpus fixtures; exit `2`; no result file |
| Standalone AppKit harness from current restricted shell | FAIL in this shell context: unique pasteboard unavailable; no desktop verification |
| `scripts/run_macos_appkit_pasteboard_test.sh` from Terminal.app | NOT RUN; Terminal.app access denied by the CUA host connector |
| Controlled synthetic Copy Result → TextEdit comparison | BLOCKED / NOT RUN; no usable desktop GUI session |
| Real corpus WER, 4 GB reference, full-app RSS/footprint, installed bytes, final licensing approval | NOT RUN / unavailable |

The root `docs/TASKS.md` checklist remains unchanged. T-010, T-011, AC-06, and AC-10 stay open.

## Next executable milestone

Complete T-010's qualification gates: obtain a lawful human-verified diverse reference corpus; secure explicit authorization before acquiring Tiny, then verify every local file against pinned source size/CRC and local SHA-256; run paired Small/Tiny CPU measurements on comparable reference hardware; obtain the required physical 4 GB evidence for applicable targets (plus the PRD's macOS low-memory constrained fallback if no 4 GB Apple Silicon is available); measure full-app 100 ms process-tree RSS, macOS footprint, installed bytes, quality, latency, and battery; and complete model/runtime redistribution review. Only after one candidate passes all unchanged gates should T-011's production Tauri tray shell proceed.
