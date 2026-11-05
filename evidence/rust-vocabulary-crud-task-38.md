---
task_id: T-038
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: Codex coordinator
base_commit: f160643a47a39a44b45f4c7039cff2d2288b02dd
result_commit: 450b268ef7807202023c694b8bdacf63dfccbd1a
platform: macOS 26.6.2 arm64; repository inventory identifies an M3 Pro with 18 GB RAM, not a 4 GB reference machine
implementation: In-memory stable-ID vocabulary CRUD in crates/settings/src/vocabulary_crud.rs; synthetic example in crates/settings/examples/vocabulary_crud.rs
metrics: NOT_MEASURED; no application resource, recognition, or release measurements were in scope
doc_updates: README.md native CRUD section; docs/TASKS.md partial T-038 status; this evidence
risks:
  - CRUD is in-memory only and is not connected to production settings persistence or the browser preview.
  - Schema-v1 validation, lowercase matching, JSON shape, and save/load behavior remain unchanged.
  - Boundary-aware correction and complete T-038/AC-08 acceptance remain outstanding.
  - T-017, T-018, T-038, T-039, and all release gates remain open.
review: PENDING
---

# Native in-memory vocabulary CRUD — Task 38 partial evidence

## Outcome

Added public `add_vocabulary_entry`, `edit_vocabulary_entry`, and `delete_vocabulary_entry` functions to `outflow-settings`. Each applies its operation to a clone and replaces the caller's vector only after validation succeeds. Phrase and collection checks call the existing `outflow_core::vocabulary::validate_vocabulary_collection`, which provides the 500-entry cap, 128 Unicode scalar value field limit, and Unicode 18 canonical-caseless duplicate detection. Stable IDs are also checked for uniqueness. Errors expose only variants, field reasons, and collection indices; they never include vocabulary phrases or IDs.

Edit changes the spoken and written fields while preserving the stable ID and vector position. Delete removes an unambiguous stable ID and preserves remaining order. The collection validator and settings DTO are kept separate: schema-v1 validation and matching continue to use their existing lowercase-only behavior.

The example uses synthetic entries for Add → Edit → Delete. Between Edit and Delete it creates a uniquely named temporary directory, saves/reloads schema-v1 settings through the existing atomic writer/loader, verifies the values in memory, and cleans the directory. It prints only operation status and entry counts. This example does not make CRUD persistent.

## Automated verification

All commands below were run from the repository root on macOS 26.6.2 arm64.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 367 tests passed across core, settings, integration tests, and doctests |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings vocabulary_crud --locked` | PASS; 14 focused tests passed |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo run -q -p outflow-settings --example vocabulary_crud --locked` | PASS; output below |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 11 files and 160 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite built 29 modules |
| `git diff --check` | PASS |

The focused tests cover valid Add; Edit value, ID, and position behavior; Delete order; missing and duplicate IDs; duplicate spoken phrases for `straße`/`STRASSE` and composed/decomposed Unicode; valid Unicode/emoji; the exact 128/129 scalar boundary; the 500/501 entry boundary; unchanged collections after rejected changes; and errors without user values. Synthetic fixtures only.

## Example output

```text
Add: success (entries: 1)
Edit: success (entries: 1)
Temporary save/reload: success (entries: 1)
Delete: success (entries: 0)
```

## Scope and remaining work

No React source, package manifest, Cargo dependency, lockfile, schema, migration, or production persistence behavior changed. The frontend checks confirm its existing code still passes; the UI remains a browser-only preview. The temporary-file demonstration uses the existing writer on this Unix host. The writer's existing unsupported-platform behavior remains in effect on targets without its owner-only atomic replacement implementation.

This is a first native capability toward T-038, not completion of T-038 or AC-08. Settings persistence integration, boundary-aware replacement, and complete save/load/delete acceptance remain outstanding. T-017, T-018, T-038, T-039, and all release gates remain open. No physical 4 GB or release performance claim is made. A live `sysctl -n hw.memsize` query was denied by the host environment; the 18 GB value above comes from the repository's existing development-machine inventory.
