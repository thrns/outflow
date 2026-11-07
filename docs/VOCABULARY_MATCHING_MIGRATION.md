# Vocabulary Matching Migration Compatibility

This document describes a read-only compatibility checker and migration-readiness assessment for vocabulary matching changes. They do not implement a settings migration, change saved configuration, or alter the production collection validator.

## Milestone 3 settings-service update (2026-11-06)

The production schema remains v1 and still has no trusted vocabulary-policy provenance field. `Settings::validate`, decoding, and persistence now use the core Unicode 18 canonical-caseless collection validator. `SettingsService` loads nonempty v1 collections as `SourcePolicyUnknown` and requires an explicit per-instance review acknowledgment before writes. If a schema-v1 collection conflicts under Unicode 18, the service uses a bounded, read-only review decoder to expose original entries and index-only readiness; edits or deletes must resolve the conflict before a candidate can be saved. No source policy is inferred from schema version, and the service does not add or migrate to schema v2.

Earlier implementation notes below describe the state at their respective task snapshots. Their statements that the settings decoder used lowercase-only uniqueness or could not inspect a conflicting file have been superseded by this update. The preflight remains read-only, and its source-policy readiness outcomes remain unchanged.

## Policies compared

The legacy policy is Rust `str::to_lowercase()`, identified by `rust-str-to-lowercase-v1` in checker results. It lowercases Unicode text but does not perform full Unicode case folding or canonical normalization. The identifier is Outflow's policy label; Rust's lowercase tables come from the compiler's standard library and do not name a separately pinned Unicode data release.

The current core policy is `Unicode18CanonicalCaselessV1`, with identifier `unicode-18.0.0-canonical-caseless-v1`. It uses Unicode 18.0.0 D145 through ICU4X 2.4.0 compiled data: `NFD(toCasefold(NFD(input)))`. It uses default, non-Turkic full case folding and canonical normalization. It does not trim whitespace, strip accents, or apply compatibility normalization.

Pairs that were distinct under lowercase matching can conflict under the current policy. The checker exercises real calls to both algorithms for examples including:

| Legacy lowercase input pair | Checker result |
|---|---|
| `straße` / `STRASSE` | New Unicode 18 conflict |
| composed `é` / `e` + combining acute accent | New Unicode 18 conflict |
| `σ` / `ς` | New Unicode 18 conflict |

Ordinary case variants such as `ÉCOLE` / `école` are already lowercase conflicts. Default dotted/dotless-I distinctions, cross-script confusables, and trailing whitespace remain distinct under these policies.

## Checker behavior

`outflow_core::vocabulary_migration::inspect_vocabulary_migration` accepts a borrowed `&[VocabularyEntryRef]` and returns either an ordered list of `VocabularyMigrationConflict` values or a typed validation error. It first enforces the existing 500-entry limit, then validates every spoken and written field with `validate_vocabulary_entry`, preserving the existing nonempty and 128-Unicode-scalar rules. It does not call the production collection validator as a gate, because that validator now rejects Unicode 18 duplicates before they can be classified.

The checker compares each spoken-phrase pair under both policies. An `ExistingLegacyConflict` has equal lowercase results. A `NewUnicode18Conflict` has different lowercase results and equal Unicode 18 canonical-caseless results. Each record reports the earlier and later input indices, classification, and source/target policy identifiers. It never includes vocabulary text in the report or errors. Pair records follow input order.

Original spoken and written values are borrowed and left unchanged. Derived keys exist only during the call. The API receives no settings path and performs no file, network, logging, or persistence operation. It does not delete entries, select a winner, rewrite replacements, or normalize original text. When a new conflict is found, manual resolution is required before adopting the new matching policy for that collection.

## Readiness assessment

`outflow_core::vocabulary_migration::assess_vocabulary_migration_readiness(entries, source_policy)` reuses the checker and returns a typed `VocabularyMigrationReadiness` result. `VocabularySourcePolicy::Unknown` means no trustworthy matching-policy provenance is available. Select either verified source-policy variant only when trusted evidence supports that claim; schema v1 by itself is insufficient.

- `EmptyCollection`: there are no vocabulary entries to migrate, regardless of provenance. It says nothing about whether the complete settings file may be rewritten.
- `SourcePolicyUnknown { conflicts }`: entries exist but source provenance is unknown. Conflicts are retained for review, and even an empty conflict list is not migration eligibility.
- `ManualResolutionRequired { conflicts }`: a verified source collection has at least one legacy or Unicode 18 conflict. Reports preserve pair indices and classification without selecting a winner.
- `CompatibleForExplicitMigration`: verified legacy data has no conflicts under either policy. It is only compatibility for a future explicit migration, not persistence success or authorization to rewrite settings.
- `AlreadyCurrent`: verified Unicode 18 V1 data has no conflicts and needs no matching-policy migration.

Validation errors remain the checker's typed size or first-invalid-entry errors. Conflict and error reports use indices and typed reasons, never vocabulary text. No readiness outcome performs migration or settings-file I/O.

## Future settings integration contract

The readiness assessment and the phases below are architectural requirements, not completed settings functionality. The pure assessment does not load configuration, change schema, save a file, or make a collection valid.

### Phase A — Read-only assessment

- Decode a settings file using only an explicitly supported schema.
- Do not treat schema v1 as proof of the vocabulary matching-policy provenance; the schema contains no policy marker.
- Validate the vocabulary with `inspect_vocabulary_migration`, then use trusted source-policy evidence with `assess_vocabulary_migration_readiness`.
- Preserve the original on-disk file during assessment.

### Phase B — Conflict review

- If conflicts are detected, preserve every original entry and report conflicts by index for internal review.
- Require explicit user review before any persistent change.
- Never automatically discard either mapping or select a preferred entry.

### Phase C — Future versioned migration

Only after a separately approved settings-schema migration exists:

- Store an explicit matching-policy marker.
- Revalidate the entire collection using Unicode 18 canonical-caseless V1.
- Preserve original spoken and written strings.
- Write atomically, retain a recoverable last-known-good configuration, and verify rollback on write failure.
- Never silently overwrite a legacy file whose vocabulary is incompatible under the target interpretation.

The current settings schema is version 1 and has no vocabulary matching-policy field. Its collection validator uses Unicode 18 canonical-caseless matching, but this algorithm does not establish which policy previously validated or wrote an existing file. Schema versions describe serialized structure; a missing matching-policy field does not establish matching-policy provenance. Unknown or unprovable lineage must remain ambiguous rather than being assumed to be legacy lowercase data.

The settings crate accepts schema v1 only and rejects unsupported schema versions. `Settings::from_json` and `Deserialize` validate decoded values; `load_settings` uses `from_json`; `initialize_settings` validates either loaded settings or first-run defaults. `Settings::to_json` and the atomic save path validate before writing. There is no schema migration implementation. The settings service uses the existing readiness assessment with `Unknown` provenance for every nonempty collection and requires explicit review before durable changes.

## Settings DTO compatibility preflight (Task 48)

The settings crate now exposes `outflow_settings::preflight_vocabulary_compatibility(&settings, source_policy)` as a separate, opt-in read-only API. It accepts `&Settings` and explicit `VocabularySourcePolicy`, builds a temporary ordered `Vec<VocabularyEntryRef<'_>>` borrowing the DTO's owned spoken and written strings, then calls the existing core `assess_vocabulary_migration_readiness` API. The adapter returns the core `VocabularyMigrationReadiness` or `VocabularyMigrationError` and re-exports the related policy and conflict types. It ignores entry IDs for matching and preserves all DTO fields and original strings.

The adapter does not participate in `Settings::validate`, JSON serialization/deserialization, `load_settings`, `initialize_settings`, reset, or atomic save. It has no path, network, logging, or persistence operation. It reuses core's 500-entry and per-field validation and matching implementation instead of duplicating Unicode normalization or case folding.

Source policy is always explicit. Schema v1 does not contain a policy marker; schema version, modification time, application version, operating system, or the absence of conflicts cannot establish trusted provenance. A nonempty collection with `Unknown` provenance remains `SourcePolicyUnknown`, even with an empty conflict list. `CompatibleForExplicitMigration` means only that verified legacy entries have no reported conflict. No outcome means a migration was applied, conflict resolution occurred, or a settings file was written.

The adapter can inspect only `Settings` values that are available in memory. The settings service now has a narrowly scoped, bounded review path for schema-v1 files rejected solely for a Unicode 18 duplicate-source conflict; malformed fields, unsupported versions, and other invalid content remain rejected. This path preserves original text and file bytes until an explicitly reviewed candidate is saved. It is not a general raw-file recovery API.

Run the focused settings coverage and printable synthetic summary from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_preflight integration_summary_uses_readiness_results_from_real_preflight_calls --locked -- --nocapture
```

The integration test uses real preflight calls to report unknown provenance, compatible verified legacy data, a legacy Unicode 18 collision requiring manual resolution, and verified current data. It also demonstrates that the DTO is not automatically loaded, saved, migrated, or modified. These tests do not complete T-017, T-018, T-038, T-039, or AC-08. Remaining work before any automatic settings integration includes trusted provenance, an approved versioned migration/schema decision, review and rollback behavior, and explicit wiring with persistence tests. See [Task 48 evidence](../evidence/rust-settings-vocabulary-preflight-task-48.md).

## Run locally

From the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary_migration --locked
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_migration_check --locked
```

The example uses only synthetic entries and prints indices and policy outcomes, never vocabulary content. It demonstrates an unchanged collection, a legacy conflict, one new Unicode 18 conflict, and several new conflicts. It does not access or modify user settings.

## Proposed versioned policy marker (Task 49)

The marker contract names the currently approved matching algorithm with the exact, case-sensitive identifier `unicode18-canonical-caseless-v1`. It represents Unicode data version 18.0.0, canonical-caseless matching, default full non-Turkic case folding, NFD normalization before and after folding, and matching-policy interface version 1. A marker contains no spoken or written phrases, comparison keys, locale-specific casing, device/user identifiers, or build timestamps. A Unicode data or algorithm change requires a new identifier; an existing identifier must never be reused with different semantics.

This is a **proposal**, not an approved or implemented persisted schema. The illustrative schema extension is:

```json
{
  "schema_version": 2,
  "vocabulary_matching_policy": "unicode18-canonical-caseless-v1"
}
```

The fragment is incomplete and omits the other required Settings fields. The production `Settings` DTO, schema version, JSON serialization, decoder, validator, load/save paths, and migration behavior remain unchanged. No production `SettingsV2` DTO is defined. A future schema version must explicitly define how a marker is written, validated, and interpreted before any migration is authorized.

A marker read from JSON is a **declaration**, not cryptographic proof that the source was originally validated under that policy. The isolated `outflow_settings::inspect_vocabulary_policy_marker` helper recognizes only the exact supported identifier and returns `Absent`, `Recognized`, or `Unsupported`. It does not create a `VocabularySourcePolicy`, infer provenance, or make a collection eligible for migration. Do not invent a signature, timestamp, or other trust mechanism for local settings files.

| Condition | Intended behavior |
|---|---|
| Schema v1, no marker | Preserve current behavior; matching-policy provenance remains unknown unless trustworthy evidence exists separately. |
| Schema v1, unexpected marker | Do not infer trusted provenance. Current schema-v1 unknown-field rejection remains in force. |
| Proposed schema v2, valid marker | Recognize the declared policy; validate it through a future approved integration. |
| Proposed schema v2, missing required marker | Reject or require recovery as defined by the future schema contract. |
| Proposed schema v2, unknown marker | Fail closed; do not silently downgrade to lowercase matching or another policy. |
| Future unsupported schema version | Preserve the existing unsupported-version rejection. |
| Vocabulary collision | Require manual resolution; do not select or delete a mapping automatically. |
| Corrupt settings file | Preserve recovery and last-known-good requirements. |

The proposed schema-v2 cases are design requirements only. The production loader does not recognize schema v2 or apply these rules. In particular, absent schema-v1 marker data never implies the legacy lowercase policy, and a recognized marker never upgrades untrusted provenance.

## Future migration and rollback requirements

Before any persistent schema migration is implemented, the reviewed design and tests must require the application to:

1. Read and validate the original settings file without modifying it.
2. Preserve every original vocabulary entry.
3. Determine source-policy provenance conservatively; unknown evidence stays unknown.
4. Run the existing read-only compatibility preflight.
5. Require explicit resolution of all reported vocabulary collisions.
6. Validate the complete target Settings object before writing.
7. Use an approved, versioned schema and explicit marker contract.
8. Write atomically with appropriate platform-specific permissions.
9. Preserve a recoverable last-known-good configuration.
10. Test rollback and interruption scenarios, including failures around replacement and synchronization.
11. Never silently downgrade an unsupported policy marker.
12. Never automatically delete or alter conflicting vocabulary mappings.

A future application downgrade must not reinterpret schema-v2 data silently as schema v1. The existing settings writer's cross-platform and rollback acceptance work is not complete: Windows owner-only ACL support remains outstanding, and the required end-to-end rollback/interruption acceptance has not been established. These requirements are not implemented by the marker or its tests.

Run the isolated marker contract tests and their actual-code summary with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings vocabulary_policy_marker --locked -- --nocapture
```

The schema-v2 fixture is test-only and does not register with the production loader. T-017, T-018, T-038, T-039, and AC-08 remain incomplete; this proposal does not update repository T-049.

## Isolated proposed schema-v2 contract fixture (Task 50)

[`crates/settings/tests/vocabulary_schema_v2_contract.rs`](../crates/settings/tests/vocabulary_schema_v2_contract.rs) defines a partial test-only envelope with `schema_version`, `vocabulary_matching_policy`, and a minimal vocabulary array of synthetic `spoken`/`written` pairs. It deliberately omits the rest of Settings and is not a production DTO, decoder, persistence repository, or migration handler.

The fixture shape is illustrated by:

```json
{
  "schema_version": 2,
  "vocabulary_matching_policy": "unicode18-canonical-caseless-v1",
  "vocabulary": [
    { "spoken": "North Parkade", "written": "North Parkade" }
  ]
}
```

The fixture accepts only schema version 2 with the exact marker `unicode18-canonical-caseless-v1`. It rejects missing, empty, wrong-case, unsupported-policy, unsupported-Unicode-version, and non-string marker values. Missing, wrongly typed, schema-v1, and future schema versions have separate contract failures; future versions are not reinterpreted as schema v1. Vocabulary shape errors are rejected by the fixture, while phrase uniqueness and the existing 500-entry/128-scalar checks are delegated to the existing core validator and readiness assessor.

The integration test calls the existing `VocabularyPolicyMarker` inspection API and the versioned Unicode 18 matcher. It exercises the new conflicts for `straße` / `STRASSE`, composed `é` / decomposed `e` plus U+0301, and `σ` / `ς`. The compatibility report retains stable indices and typed classifications, and both original mappings remain unchanged.

Marker recognition does not prove that the source was created or validated under the declared policy. Schema version, conflict-free data, and a recognized marker do not promote `VocabularySourcePolicy::Unknown`. Tests using either verified provenance variant label it as an explicit synthetic caller assumption; no trust system is created.

Production remains schema v1: its marker field is absent, its serializer and decoder are unchanged, and its decoder rejects schema version 2. The proposed-v2 parser exists only in this integration test. It performs in-memory parsing and read-only assessment; it does not touch a settings file or run a migration.

Run the isolated contract and its concise asserted summary with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test vocabulary_schema_v2_contract \
  --locked -- --nocapture
```

This fixture does not satisfy the outstanding requirements for trusted source-policy provenance, a reviewed complete schema and migration, explicit collision review, full target-settings validation, cross-platform atomic persistence, recoverable last-known-good state, and rollback/interruption testing. T-017, T-018, T-038, T-039, and AC-08 remain incomplete; repository T-050 is unchanged.

## Read-only proposed migration planning report (Task 51)

[crates/settings/tests/vocabulary_schema_v2_migration_plan.rs](../crates/settings/tests/vocabulary_schema_v2_migration_plan.rs) adds a pure planner inside integration-test scope. It accepts a synthetic schema-v1-shaped Settings value, a synthetic proposed-v2 JSON contract, and an explicit VocabularySourcePolicy. Conflict-bearing DTOs are constructed in memory because the production decoder now rejects them; separate tests continue to prove strict decoding. The parser validates the proposed schema version, exact marker identifier, and partial vocabulary field shape by reusing inspect_vocabulary_policy_marker; the planner then delegates source readiness to preflight_vocabulary_compatibility. It does not define a production v2 DTO or change persisted settings.

The two policy inputs remain independent. The exact target declaration is unicode18-canonical-caseless-v1; it identifies the proposed target matching policy only. Source provenance is supplied separately as Unknown, VerifiedLegacyLowercase, or VerifiedUnicode18CanonicalCaselessV1. Verified values in this fixture are explicit synthetic assumptions. Neither schema v1, schema v2, a recognized target marker, nor conflict-free content proves how source settings were historically validated.

The report includes schema versions, the recognized target-policy identifier, the caller's provenance classification, the readiness status, and ordered index/classification conflict records. It contains no vocabulary strings or comparison keys. Statuses map directly from the core assessment:

| Core readiness | Planning status | Meaning |
|---|---|---|
| EmptyCollection | NoVocabularyMigrationNeeded | No vocabulary-key migration is needed; this does not authorize rewriting the complete settings document. |
| SourcePolicyUnknown | SourceProvenanceReviewRequired | Review provenance even when there are no conflicts; retain any detected conflict records. |
| ManualResolutionRequired | ManualConflictResolutionRequired | Review each conflict manually; the planner never chooses a surviving entry. |
| CompatibleForExplicitMigration | CompatibleForFutureMigrationReview | Verified legacy entries are compatible for a future explicit decision, not approved to write. |
| AlreadyCurrent | AlreadyCurrentForReview | Verified current-policy entries need no vocabulary-key migration; this does not approve a schema rewrite. |

The planner does not mutate source or target values, read or write settings files, or apply a migration. Invalid source phrases and 501-entry collections are rejected by the strict schema-v1 decoder; conflict-bearing planner inputs are now synthetic in-memory DTOs. The 500-entry accepted case and Unicode 18 collisions are assessed through the planner without bypassing the production service's review requirements.

Run the focused test and its content-free summary from the repository root:

~~~sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test vocabulary_schema_v2_migration_plan \
  --locked -- --nocapture
~~~

Production still supports schema v1 only and rejects schema v2. The service persists only Unicode 18-valid candidates after explicit review; this does not record policy provenance or migrate schema. Any future schema migration still needs trusted provenance, a reviewed complete target schema, explicit manual conflict decisions, complete target validation, cross-platform atomic-write behavior, recoverable last-known-good state, and tested interruption/rollback. Windows owner-only ACL support and end-to-end rollback verification remain outstanding. This milestone's service and 500/501 persistence tests provide partial T-038/AC-08 evidence only; T-017, T-018, T-038, T-039, and AC-08 remain incomplete, and repository T-051 was not changed.

## Manual conflict review grouping contract (Task 52)

The test-only review projection groups pairwise conflicts into connected components so every entry linked directly or transitively is presented as one review decision. For example, three equivalent entries produce one group with three indices; presenting their three pairwise reports as three independent decisions could let separate decisions leave another unresolved collision. Sorted group indices map directly to positions in the original ordered vocabulary collection, and each original pair and classification remains attached to its group.

Until a user explicitly decides, the original spoken and written values must remain unchanged. Migration stays blocked while any conflict is unresolved. The application must never choose a winner automatically; cancelling review must preserve every original mapping and the existing configuration. Editing or deleting a mapping requires separate validated CRUD functionality, explicit confirmation, and persistent settings support. A conflict report may contain indices, classifications, group counts, and policy identifiers only; it must not expose vocabulary text, comparison keys, settings JSON, transcripts, or audio through diagnostics, logs, migration metadata, or exported error reports.

The future intended flow is:

1. Detect conflicts with the existing checker.
2. Present affected entries locally for review.
3. Require an explicit user decision for each connected group.
4. Revalidate the complete proposed vocabulary collection.
5. Save only through an approved atomic migration operation.
6. Preserve a recoverable original configuration.

These are future requirements, not functionality implemented in Task 52. The projection remains inside an integration-test file: there is no review UI, conflict resolution, CRUD operation, production schema-v2 decoder, migration command, or settings write path added here. Trusted provenance, approved migration/schema behavior, validated CRUD, atomic persistence, recovery, and cancellation acceptance remain prerequisites.

## Hypothetical resolution proposal validator (Task 53)

[`crates/settings/tests/vocabulary_resolution_proposal.rs`](../crates/settings/tests/vocabulary_resolution_proposal.rs) defines a test-only validator for explicit `Retain` or `OmitFromProposedCollection` decisions, each keyed by an original entry index. It requires exactly one decision for every original entry; omissions are only part of a hypothetical collection and do not execute deletion. The result lists retained and omitted original indices in source order.

The helper uses the core migration inspector to validate the original collection's 500-entry limit and individual fields while allowing its conflict report, then builds a borrowed retained-entry view and calls `validate_vocabulary_collection` for the proposed collection. This reuses the existing 128-scalar, nonempty-field, and Unicode 18 canonical-caseless rules without normalizing or rewriting strings. Candidate positions map back to original indices, so typed errors identify the source entries even after omissions; errors contain no vocabulary text.

A valid result means only that this explicit hypothetical retained collection passes current validation. It does not infer trusted provenance, select a winner, approve migration, mutate or persist settings, or establish that a future schema can be written safely. The helper has no settings or filesystem interface, introduces no CRUD or production schema-v2 support, and leaves the existing settings duplicate validator unchanged.

Run the focused demonstration with:

~~~sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test vocabulary_resolution_proposal \
  --locked -- --nocapture
~~~

Future resolution still requires manual review and explicit user decisions, trusted source-policy provenance, a separately approved complete settings migration, full target validation, and tested atomic persistence, recovery, cancellation, and rollback. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete.

## Pre-commit review cancellation and retry (Task 54)

The Task 53 integration-test target now also models an active or cancelled review session. It borrows the original vocabulary immutably and stores partial, editable decisions separately. Changing a decision revises only that draft entry; it neither chooses a winner nor changes order or text. Validation calls the existing proposal validator. Incomplete decisions and Unicode 18 conflicts return its typed errors and leave the active draft available for correction. A corrected draft is validated again by the same validator.

Cancellation is available before validation and after either a valid or invalid proposal. It clears pending decisions, preserves every original spoken/written value and index, and is idempotent. A cancelled session cannot be edited or validated; a fresh session starts with an empty draft over the same original. A valid hypothetical assessment still does not authorize migration or a settings write.

**Task 54 verifies pre-commit cancellation and in-memory recovery invariants only.** It does not verify actual filesystem rollback. The test helper has no settings-file or filesystem interface. Existing settings-writer tests, including injected failure tests, are not evidence that a future schema-v2 migration can roll back safely.

Run the cancellation and retry checks with:

~~~sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test vocabulary_resolution_proposal \
  --locked -- --nocapture
~~~

Persistent migration still needs separate verification for temporary-file write failure, real disk-full behavior, permission denial, failure before replacement, platform-specific replacement failure, crash/interruption recovery, preservation of a valid last-known configuration, file-permission preservation, Windows replacement and ACL behavior, Linux filesystem behavior, and unsupported-schema downgrade handling. Windows owner-only ACL support and end-to-end migration rollback remain outstanding. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete; repository T-054 remains the unrelated onboarding demo-flow task. See [Task 54 evidence](../evidence/rust-vocabulary-cancellation-task-54.md).
