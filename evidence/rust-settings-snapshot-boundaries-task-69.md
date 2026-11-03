# Task 69 — Settings snapshot ambiguity and concurrent-change boundaries

```yaml
task_id: "Task 69 request; repository T-069 status not changed"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-10, UF-11]
acceptance: [AC-13]
acceptance_scope: "Test-only classification of settings snapshots; no production concurrency or recovery behavior"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; isolated temporary directories on APFS local journaled data volume; Rust stable 1.99.1"
implementation: "Added five focused scenario tests and one assertion-backed summary test to the existing Task 68 integration target; updated README and the settings recovery matrix"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 69 snapshot boundaries", "evidence/rust-settings-temporary-write-recovery-task-56.md: Task 69 matrix row", "this report"]
risks: "No overlapping filesystem-operation safety, linearizability, locking, transaction isolation, lost-update prevention, crash durability, Windows or Linux behavior, or recovery guarantee is established"
review: PENDING
```

## Scope and inspected contracts

The repository is `/Users/bixxie/Desktop/Projects/outflow`, on branch `codex/task23-moonshine-build-scope`, based at `8c978bbd53617b7291fd0aeacf7c42cc260a0142`. The initial worktree already contained Task 68's untracked integration test and evidence, Task 67's untracked test/evidence, and other uncommitted settings and documentation work. That work was preserved. The root `AGENTS.md` applies; no nested `AGENTS.md` or `AGENTS.override.md` was found. Repository task completion markers were not changed.

Inspected `AGENTS.md`; PRD FR-13.4; ARCHITECTURE C-08; USER_FLOWS UF-10 and UF-11; TECH_STACK TS-08; TASKS T-017 and T-018; `crates/settings/src/storage.rs`; `crates/settings/src/lib.rs`; both existing settings integration targets; Task 68 evidence; README Task 68 text; and the Task 56 settings recovery matrix.

Confirmed before editing:

- The private `inspect_visible_settings` helper calls the existing public `load_settings` function once per inspection.
- Its result is a unit-only classification enum and contains no settings values or operation metadata.
- Equal original and attempted values both matching the loaded configuration produce `BothSnapshotsMatch`.
- The inspector calls no writer, initializer, reset, recovery, retry, backup, or migration API.
- Repository search found no production reference to `inspect_visible_settings` or `VisibleSettingsInspection`; the contract remains local to this integration-test target.
- Task 69 changed no production source, public API, dependency, Cargo manifest, lockfile, or schema. Existing uncommitted settings-source edits in the initial worktree were preserved.

## Added tests and actual assertions

All valid fixtures are distinct synthetic schema-v1 `Settings` values created with the existing `original_settings`, `attempted_settings`, `settings_variant`, and `write_valid_settings` helpers. Explicit writes in the tests use the public `save_settings_atomic` writer. Inspections use the existing Task 68 helper.

1. `earlier_observation_stays_stale_after_an_explicit_write`
   - Writes A; a loader-backed inspection returns `OriginalVisible`.
   - Explicitly writes distinct B and retains the original enum value in memory; it remains `OriginalVisible`.
   - A new inspection returns `AttemptedVisible`, and a fresh `load_settings` call returns B.
   - The fresh inspection leaves the post-write directory snapshot unchanged.

2. `equal_snapshots_are_ambiguous_across_distinct_write_histories`
   - History A writes a configuration once and inspects without another write; result is `BothSnapshotsMatch`.
   - History B explicitly writes the same configuration again before inspection; result is also `BothSnapshotsMatch`.
   - The test asserts equal results and checks each inspection leaves its fixture snapshot unchanged. It does not use bytes, timestamps, hashes, or file identity to infer history.

3. `a_b_a_inspections_do_not_reveal_the_intermediate_history`
   - Keeps A (`original`) and B (`attempted`) fixed throughout: write A → `OriginalVisible`; write B → `AttemptedVisible`; write A → `OriginalVisible`.
   - Asserts the first and final classifications are equal, the intermediate classification differs, and a fresh load returns A. The final classification therefore does not encode the observed intermediate B state.

4. `two_actors_observe_a_deterministically_ordered_write_between_inspections`
   - Uses two `std::sync::mpsc` channels and two Rust threads. The inspector sends its completed A classification. The writer waits for that message, explicitly writes B through `save_settings_atomic`, and sends its observed first classification and write result. The inspector waits for that completion message before making a fresh inspection.
   - Assertions verify the writer received `OriginalVisible`, its explicit write succeeded, the fresh inspection returned `AttemptedVisible`, the settings still load as B, and both completed inspections left their respective directory snapshots unchanged. Both threads are joined.
   - There are no sleeps, polling loops, random delays, unsynchronized races, external processes, thread pools, or async runtime. This proves only the chosen ordering of completed operations; it does not exercise overlapping filesystem calls or prove linearizability or lock-free correctness.

5. `removing_a_file_after_inspection_does_not_rewrite_the_prior_observation`
   - Writes A and inspects `OriginalVisible`; test code then explicitly removes the synthetic file.
   - The retained classification stays `OriginalVisible`; a new inspection returns `Missing`, leaves the directory unchanged, and does not recreate the file. The unrelated synthetic sentinel remains.

6. `snapshot_boundary_summary_is_assertion_backed_and_content_free`
   - Re-runs the five focused scenario tests synchronously and prints the requested fixed summary only after their assertions pass.
   - The output contains status labels only. Assertions ensure it contains no synthetic fixture values, schema JSON marker, or JSON object content.

The test target now has 17 tests: Task 68's original 11 remain passing, and Task 69 adds five scenario tests plus the summary test. The existing Task 68 malformed-document, unsupported-schema, and metadata-preservation matrices were not duplicated.

## Privacy and non-mutation

The only paths used are newly created synthetic files under `std::env::temp_dir()`. Settings and vocabulary entries contain synthetic values. The inspector returns classifications only; printed summaries contain no vocabulary values, replacements, JSON, transcript, or audio content. Existing Task 68 checks continue to verify classification debug output is redacted and repeated inspections preserve fixture contents and metadata. New tests compare directory snapshots only around inspection calls to verify non-mutation; no timestamp, hash, or inode is used as evidence of operation history.

Only explicit test setup/writer calls modify the synthetic files. Test code explicitly removes the fixture in the missing-after-inspection case. The inspector performs no settings writes, default creation, reset, retry, backup, restore, deletion, schema change, or migration. No real user settings were opened.

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; Rust stable 1.99.1; Node 22.23.3; Python 3.13.13. The isolated temporary directories are on the APFS local journaled data volume. Windows and Linux were not tested.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_post_error_inspection --locked -- --nocapture` | **PASS** — 17 tests, including the 11 existing Task 68 tests and six Task 69 tests; requested summary printed after assertions. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_persistence_guidance_contract --locked` | **PASS** — Task 67's 14 existing tests. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 245 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 338 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS**. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** — final working-tree diff checked after documentation updates. |

The initial workspace-test log capture ran the tests successfully but its surrounding zsh command then attempted to assign the read-only `status` variable. The direct `cargo test --workspace --locked` rerun passed with exit code 0 and the same 338 unit/integration plus 1 doc-test result. No Moonshine CMake configuration was attempted.

## Changed files and remaining limits

- `crates/settings/tests/settings_post_error_inspection.rs` — five scenario tests and one assertion-backed summary test, reusing the existing helper and fixtures.
- `README.md` — snapshot validity/staleness, ambiguity, A → B → A, ordered actor schedule, limitations, and focused command.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — one Task 69 recovery-matrix row.
- `evidence/rust-settings-snapshot-boundaries-task-69.md` — this evidence report.

The two-actor test does not overlap filesystem operations and proves no concurrent read/write safety, linearizability, file locking, lock-free behavior, transaction isolation, or lost-update prevention. No crash or power-loss test was performed; crash durability is unverified. No automatic retry, reset, rollback, production change-notification, file watcher, concurrency control, or schema migration was added. Windows and Linux behavior remain unverified. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete; repository T-069 and all other task completion markers remain unchanged.
