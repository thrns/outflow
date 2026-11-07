task_id: T-038 (partial evidence; task remains unchecked)
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: implementation engineer / coordinator
base_commit: 361b038c60ae2583a560040520bb18f809450762
result_commit: af33c602ffc60e9523ac9f021517bfbd0b35f037
branch: feature/mvp-settings-persistence
platform: macOS 26.6.2, arm64; Rust 1.99.1; Cargo 1.99.1; Node.js 22.23.3; npm 10.9.9
implementation: >
  Added an explicit-path SettingsService with candidate validation, atomic
  persistence before live-state replacement, stable-ID vocabulary CRUD,
  per-instance review for nonempty schema-v1 vocabulary, and explicitly
  confirmed reset. The strict settings validator now delegates uniqueness and
  phrase limits to the core Unicode 18 validator. A bounded review-only decoder
  exposes otherwise-valid conflict-bearing schema-v1 entries without changing
  their order or text. Added a temporary-directory lifecycle example and
  filesystem tests. Separately added human-reviewed ASR hardware evidence
  sidecars bound to exact measured-result bytes and machine/session identity;
  removed the duplicate architecture validator and made README commands
  portable.
commands:
  - {command: "npm ci", result: PASS, notes: "Installed locked frontend dependencies; no lockfile change."}
  - {command: "npm run typecheck", result: PASS, notes: "TypeScript check completed."}
  - {command: "npm run lint", result: PASS, notes: "ESLint completed."}
  - {command: "npm run test -- --run", result: PASS, notes: "11 files, 160 tests passed."}
  - {command: "npm run build", result: PASS, notes: "Vite production build completed; browser preview remains development-only."}
  - {command: "cargo fmt --all -- --check", result: PASS, notes: "Workspace formatting check completed."}
  - {command: "cargo check --workspace --locked", result: PASS, notes: "Workspace checked with the lockfile."}
  - {command: "cargo clippy --workspace --all-targets --locked -- -D warnings", result: PASS, notes: "No warnings."}
  - {command: "cargo test --workspace --locked", result: PASS, notes: "514 tests passed across workspace targets and doctests; 0 failed."}
  - {command: "cargo test -p outflow-settings --test settings_service --locked", result: PASS, notes: "12 filesystem integration tests passed."}
  - {command: "cargo test -p outflow-settings service::tests::injected_storage_write_failure_preserves_live_snapshot_and_prior_file --locked", result: PASS, notes: "Injected storage failure preserved the prior file and live snapshot."}
  - {command: "cargo run -p outflow-settings --example settings_lifecycle --locked", result: PASS, notes: "Ran first-run, preference, vocabulary CRUD, save, and reload lifecycle in a new temporary directory."}
  - {command: "cargo run -p outflow-settings --example settings_lifecycle --locked -- --reset", result: PASS, notes: "Explicit option reset only the newly created temporary example file."}
  - {command: "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_measured_asr_results -v", result: PASS, notes: "18 focused measured-result and hardware-evidence tests passed."}
  - {command: "PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v", result: PASS, notes: "629 tests discovered: 628 passed, 1 optional real native ASR integration test skipped because verified fixture/local artifact configuration is unavailable."}
  - {command: "python3 scripts/compare_native_asr_candidates.py --help", result: PASS, notes: "CLI exposes the required separate Small and Tiny hardware-evidence paths."}
  - {command: "git diff --check", result: PASS, notes: "No whitespace errors."}
  - {command: "rg -n '/Users/|/private/tmp|/private/var' README.md", result: PASS, notes: "No developer-specific absolute paths remain in README."}
metrics: >
  NOT_MEASURED: installed application size, process-tree peak RSS, ASR WER,
  latency, CPU real-time factor, and physical 4 GB hardware eligibility.
  No hardware approval sidecar was created for the current machine.
doc_updates: >
  README lifecycle instructions and portability; architecture service contract;
  partial T-038 evidence note; current vocabulary-policy and benchmark-review
  status. T-038, T-039, T-010, T-011, AC-08, and Windows owner-only ACL
  acceptance remain unchecked.
risks: >
  Persistence was exercised on macOS only. Linux and Windows persistence were
  not tested; Windows owner-only ACL replacement remains unimplemented. Schema
  v1 records no matching-policy provenance, so nonempty loaded vocabulary
  requires an explicit per-instance review and must pass Unicode 18 validation
  before save. No schema-v2 migration, React/Tauri connection, microphone/model
  integration, or release gate was implemented. Hardware sidecars require a
  human reviewer to verify cited approval materials; structural validation
  alone does not authenticate the reviewer or certify physical 4 GB status.
review: PENDING
