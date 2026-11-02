# Frontend Task 39 — About & Diagnostics overview

> This frontend follow-on task is separate from repository T-039, which concerns vocabulary hints and post-correction. No task-ledger status was changed.

```yaml
task_id: frontend-task-39
related_features: [F-11, F-13]
related_flows: [UF-11, UF-12]
acceptance: [AC-11, AC-13]
owner: Codex / codex/task23-moonshine-build-scope
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: uncommitted
platform: macOS 26.6.2, arm64; physical RAM not measured
implementation: Static AboutDiagnosticsOverview integrated into the existing Settings panel; planned recovery, diagnostics and update behavior is distinguished from the current preview.
metrics: NOT_MEASURED
doc_updates: README.md Task 39 section; task ledger unchanged
review: PENDING
```

## Files

- Created `src/app/AboutDiagnosticsOverview.tsx` and `src/app/AboutDiagnosticsOverview.test.tsx`.
- Modified `src/app/App.tsx` to render the component in the existing About & Diagnostics tab.
- Modified `src/app/App.test.tsx` with integration coverage and an updated assertion for the replaced placeholder.
- Updated `README.md` with usage, implementation status, verification steps, and troubleshooting.
- Created this evidence file.
- No CSS, package manifest, lockfile, Rust source, task ledger, or native integration was changed for this task.

## Requirements represented

- The About section displays Outflow, the supplied offline-first development description, and a visible development-preview notice. It omits a version: `package.json`'s `0.1.0` is frontend package metadata, not verified native release metadata.
- Troubleshooting lists denied microphone permission, disconnected input, model load/integrity failure, resource or memory limits, recording-buffer overflow, and uncertain or unsupported insertion. The text says these are planned recovery capabilities, not detected problems.
- Diagnostics describes the intended future user-initiated export fields and excludes recorded audio, dictated transcripts, custom vocabulary values, and clipboard content. `SettingsStatusMessage` says collection/export is unavailable and no report can be generated.
- Application Updates describes an explicit user-initiated check that may contact the vendor release host, verified signed full-application packages including the bundled speech model, offline dictation independence, and safe failure/recovery. The page says update checking is not implemented.
- The component has no state, network or native calls, diagnostic collection, file export, persistence, telemetry, timer, or functional export/update control. No device diagnostics were collected, saved, or transmitted.
- Existing six-section Settings navigation, page heading focus, Launch at Login, Clean/Verbatim, vocabulary form/draft/preview/clear behavior, shared header, and Privacy content remain in place.

## Automated verification

Environment: Node.js `v22.23.3`, npm `10.9.9`, Rust `1.99.1`, Python `3.13.13`.

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 10 files, 136 tests; 122 existing tests retained and 14 added |
| `npm run lint` | PASS |
| `npm run build` | PASS — Vite 8.3.5 production build |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests (2 core + 70 settings) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |

The Rust workspace contains only `crates/core` and `crates/settings`. The Python suite's CMake-runner tests use mocks. No real Moonshine CMake configuration was attempted.

## Manual visual and interaction checks

- Started `npm run dev`; Vite 8.3.5 served `http://localhost:5173/`.
- Inspected the live About & Diagnostics screen at approximately 900 CSS px (the in-app browser screenshot was 897 px wide). Existing neutral Settings surfaces, spacing, wrapping, section dividers, and status styling were used. No horizontal overflow was visible at this width.
- Confirmed the identity and preview notice, all troubleshooting topics, planned diagnostic fields/exclusions, unavailable export message, signed full-app update description, and unavailable update-check message are visible. No version, runtime status, health statistic, export action, or update action appears.
- The accessibility tree showed the six existing Settings tabs. Arrow-key navigation moved between Privacy and About & Diagnostics. Keyboard navigation through Home and Settings focused each destination page heading.
- Manually toggled Launch at Login, chose Verbatim, entered the synthetic draft `north parkade` / `North Parkade`, visited About & Diagnostics, and confirmed those values remained when revisiting General and Text & Vocabulary. Restored Launch at Login to Off, Clean mode, and an empty vocabulary draft afterward.
- Existing App tests continue to cover all six sections and unchanged Privacy content. No browser console log capture was available in the connected in-app browser controls.

## Not run / remaining work

- **NOT RUN:** 320px viewport inspection. The connected in-app browser did not expose viewport resizing/emulation controls.
- **NOT RUN:** literal 200% browser zoom; screen-reader testing; browser console error capture.
- Diagnostics collection/export, native error codes and recovery, signed update checking/verification/install/rollback, and native release metadata remain unimplemented. T-019, T-064, T-072, T-077, AC-11, and AC-13 remain incomplete. Repository T-039 was not changed.

`git status --short --branch` and `git diff --check` were run after implementation. The original worktree was already dirty; its pre-existing changes remain unstaged and untouched except for the scoped edits to `README.md`, `src/app/App.tsx`, and `src/app/App.test.tsx` described above. No files were staged or committed.
