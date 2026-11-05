---
task_id: T-036
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: Codex coordinator
base_commit: 37ca8ba44e4cd9673fb0d77af15c095e7182a217
result_commit: included in the commit containing this evidence; see git history and task handoff
platform: macOS 26.6.2 (25G83), arm64; repository host inventory identifies Apple M3 Pro and 18 GB installed RAM
implementation: Current-behavior policy proposal, runnable example, and public adapter integration tests; no production cleanup implementation changed
doc_updates: README.md; docs/TASKS.md partial T-036 note; docs/CLEANUP_MODE_POLICY.md
risks: Complete Verbatim semantics remain unapproved; physical 4 GB, OS integration, ASR, and release gates are not measured here
review: PENDING
---

# T-036 Clean/Verbatim Mode Contract — Partial Evidence

## Result

Added an executable characterization of the current `outflow-settings` adapter, nine public integration tests, user-facing run instructions, a non-authoritative policy decision document, and this evidence record. No production cleanup API, settings field, default, serialization format, or schema version changed. No dependency or lockfile changed.

The current adapter applies filler deletion only for `Clean && remove_fillers`. Whitespace normalization, independently enabled dictation commands, vocabulary correction, and protected punctuation spacing continue in both modes. This describes current code; it does not approve complete Verbatim semantics.

## Example output

Command:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example cleanup_mode_contract --locked
```

Observed output:

```text
Input: "  um, rust  new line  at ubc ."
Clean, remove_fillers=true: "Rust\nat UBC."
Clean, remove_fillers=false: "um, Rust\nat UBC."
Verbatim, remove_fillers=true: "um, Rust\nat UBC."
Verbatim, remove_fillers=false: "um, Rust\nat UBC."
Commands disabled (Verbatim): "um, Rust new line at UBC."
```

All five outputs are asserted. In the final line, literal `new line` remains because commands are disabled, while `rust`/`ubc` vocabulary correction and punctuation spacing still run.

## Verification commands and results

| Command | Result | Notes |
|---|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example cleanup_mode_contract --locked` | PASS | Built and ran; observed output is recorded above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS | 9 passed, 0 failed. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS | Initial check found formatting differences in the new test; applied `rustfmt` to that file and the final requested check passed. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS | Workspace check completed. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS | Workspace test run completed; a quiet summary rerun confirmed 191 core unit tests, 128 settings unit tests, 169 integration tests, and 11 doctests passed (499 total). |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | No warnings. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS | Workspace build completed. |
| `git diff --check` | PASS | Run on the final unstaged diff. |
| `git diff --cached --check` | PASS | Run after explicitly staging only the six task files. |

Environment observed: Rust/Cargo 1.99.1 stable, macOS 26.6.2 build 25G83, arm64. The repository README host inventory lists an Apple M3 Pro with 18 GB physical RAM; this is not the required 4 GB release reference. A direct `sysctl -n hw.memsize` query was denied by the sandbox. No cleanup performance, ASR quality, RSS, installed-size, platform integration, or release metric was measured.

## Focused regression coverage

The integration test asserts exact output for:

1. All four Clean/Verbatim and `remove_fillers` combinations.
2. Whitespace normalization in both modes.
3. Dictation commands independently enabled and disabled in both modes.
4. Vocabulary correction in both modes.
5. Protected punctuation spacing in both modes.
6. Filler-like vocabulary replacements across all four settings combinations.
7. Repeated fillers remaining unchanged.
8. Numbers, proper nouns, punctuation, decomposed Unicode, Japanese, and emoji.
9. CRLF, standalone carriage return, and paragraph/newline behavior.
10. Existing typed errors for invalid and duplicate vocabulary.
11. Input, preferences, and vocabulary entries remaining unchanged.
12. Repeated calls producing identical results.
13. The full existing schema-v1 JSON shape, cleanup keys, and round-trip remaining unchanged.

The new tests characterize the public cross-crate adapter. Existing adapter and core tests remain unchanged.

## Unresolved policy questions

[`docs/CLEANUP_MODE_POLICY.md`](../docs/CLEANUP_MODE_POLICY.md) is **PROPOSED — NOT APPROVED**. It compares:

- Literal transcript behavior, skipping commands, vocabulary correction, punctuation spacing, and filler deletion.
- Current pipeline behavior, retaining commands/vocabulary/punctuation in both modes and disabling only filler deletion in Verbatim.
- User-directed exceptions, retaining explicit commands and vocabulary corrections while skipping heuristic punctuation spacing and filler deletion in Verbatim.

Maintainer decision is still required on whether Verbatim skips additional formatting, whether vocabulary correction applies, and whether explicitly enabled dictation commands execute. Do not implement a new interpretation until those decisions are approved and PRD FR-07.1–FR-07.4, UF-05, C-06, and tests are reconciled together.

## Scope and task status

No production cleanup behavior changed. No UI, design system, dependency, model, microphone, Moonshine, Tauri, clipboard, or transcript-storage integration changed. **T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.** Reviewer decision: **PENDING**.
