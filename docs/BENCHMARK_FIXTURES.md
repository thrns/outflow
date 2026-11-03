# Benchmark Audio Fixture Policy

## Purpose and status

This collection will provide a frozen, reproducible English reference set for the Phase 0 ASR evaluation described in [ARCHITECTURE.md §8.1](ARCHITECTURE.md#81-benchmark-protocol) and [TECH_STACK.md §3.2](TECH_STACK.md#32-mandatory-phase-0-asr-experiment). It supports comparison of the pinned quantized model against the same human references across supported hardware. The fixture set is evaluation input; it does not collect microphone audio from Outflow users.

[`benchmark-fixtures.json`](benchmark-fixtures.json) currently contains planned entries only. It contains no audio, checksums, or reference transcripts. A fixture is **not benchmark-ready** until its corresponding audio, verified reference file, provenance and redistribution permission evidence, and matching SHA-256 are present. No performance or WER result is implied by a planned row.

The future corpus target is at least **100 short speech phrases and 50 longer speech phrases**. These are targets, not existing counts. Freeze exact membership, versions, checksums, and normalization policy before using the set for a model decision.

### Public corpus leads checked 2026-11-03 (not selected or ready)

- Mozilla's [Common Voice Data Collective](https://commonvoice.mozilla.org/dav/datasets) lists English speech under CC0, but the current [Common Voice Legal Terms](https://commonvoice.mozilla.org/dav/terms) ask users not to post, distribute, or mirror any Common Voice dataset, in whole or part, outside Mozilla Data Collective. That conflicts with checking fixture audio into Outflow's repository. Do not copy a subset into this corpus without written clarification from Mozilla and the project's legal reviewer. Its current catalog scale also makes full-dataset acquisition unsuitable.
- [OpenSLR LibriSpeech SLR12](https://us.openslr.org/12/) lists read English speech under CC BY 4.0 and provides clean and more challenging sets. This may support a small selected subset if exact files, attribution, release metadata, and references are verified. It does not establish broad accent, conversational, or real-world noise coverage; the listed archives are hundreds of megabytes, so obtain a small subset only through a lawful reproducible method.

Neither source has been acquired into the repository or approved for this frozen benchmark. No file hashes, verified references, permission records, or coverage counts are inferred from catalog listings. There are still **0 ready fixtures**. Mozilla's no-mirroring request is a concrete blocker for Common Voice; LibriSpeech still needs exact-file provenance and human audio/reference review.

## Manifest fields

The JSON document uses schema version `1`. The Python validator rejects unsupported versions and invalid metadata.

| Field | Meaning |
|---|---|
| `schema_version` | Integer schema version for manifest parsing. |
| `language` | Corpus language; this manifest is English-only. |
| `collection_status` | Collection-level state: `planned`, `in_progress`, or `complete`. It does not establish benchmark readiness. |
| `reference_transcription_policy` | Human reference and evaluation policy applying to the collection. |
| `reference_transcription_policy_version` | Version of the manual verbatim transcription-review procedure. |
| `normalization_policy_version` | Frozen lexical WER tokenization policy used by the scorer. |
| `audio_integrity_requirements` | Hash algorithm, exact-byte scope, and missing-audio rule. |
| `fixtures` | List of individual planned or ready fixture records. |
| `id` | Stable, unique fixture identifier. Do not recycle an ID for changed audio. |
| `status` | `planned` or `ready`. Only validated `ready` entries with all required files count as ready. |
| `category` | One coverage label: `short_speech`, `long_speech`, `quiet_environment`, `noisy_environment`, `accented_english`, `proper_nouns_technical_vocabulary`, `numbers`, or `silence`. |
| `duration_class` | Backward-compatible optional field for planned rows; required for ready rows. Use `short` (measured duration ≤5 seconds), `long` (>5 seconds), or `silence_control`. This is independent of the descriptive `category`. |
| `coverage_attributes` | Backward-compatible optional list for planned rows; required for ready rows. Allowed independent tags: `quiet_environment`, `noisy_environment`, `accented_english`, `proper_nouns`, `technical_vocabulary`, `numbers`, and `abbreviations`. A fixture cannot be both quiet and noisy. |
| `speaker_variety` | Optional non-identifying, source-reported English variety label; required only when `accented_english` is tagged. Do not infer demographic identity or label speakers from voice alone. |
| `recording_condition` | Non-sensitive description of the planned or observed recording condition. For ready entries, record the actual condition and relevant source metadata. |
| `audio_file` | Repository-relative path to the exact audio file, or `null` while unavailable. |
| `audio_sha256` | SHA-256 hex digest of the exact audio-file bytes, or `null` while unavailable. |
| `reference_file` | Repository-relative path to the human reference file, or `null` while unavailable. Do not put transcript text in this manifest. |
| `reference_sha256` | SHA-256 of the separate human reference file. Required for ready entries. |
| `audio_metadata` | Measured PCM format, sample rate, channels, frame count, and duration. Required for ready entries. |
| `human_transcription_review_method` | Non-identifying method used to verify the full reference against its audio. |
| `provenance` | `source`, `source_release`, and `citation` identify the exact corpus release and source record. |
| `license_or_permission_status` | `not_verified` until redistribution permission is checked; ready entries require `verified`. |
| `license_or_permission_evidence` | License name/version and a stable citation or permission record sufficient to review redistribution rights. Do not include private contact details. |
| `human_transcription_status` | `not_verified` until a human has checked the complete reference against the audio; ready entries require `verified`. |

Audio and reference paths must be relative to the repository, remain within it (including after symlink resolution), and use forward slashes. Do not use absolute paths or `..` traversal. The validator checks audio and reference hashes for every ready fixture, requires a non-empty reference file, and cross-checks measured duration against sample rate and frame count.

## Provenance and redistribution permission

Every ready fixture must identify the original corpus or authorized recording source, source release/version or recording record, and relevant source citation. The project must retain evidence that the specific audio and reference may be redistributed in the repository and used for commercial ASR evaluation. Verify the license terms and any attribution, notice, privacy, or share-alike conditions for the actual files; do not infer a dataset's rights from its hosting site or a general repository license. For consent-based recordings, retain explicit permission covering recording, transcription, redistribution, and commercial evaluation outside the repository's public fixture files.

Do not add personal recordings, customer speech, confidential material, identifying speech, or sensitive transcripts to the repository. Prefer a suitable openly licensed corpus or purpose-recorded, non-sensitive speech with documented permission. If rights or provenance cannot be verified, keep the fixture planned and do not check its audio or reference into the repository.

## Human references and normalization

Each ready audio file needs a human-verified reference in its separate reference file. The reviewer must listen to the full corresponding recording and check every spoken word, number, proper noun, abbreviation, and intended punctuation. A second review or adjudication is required for uncertain words and names. Record the review method and reference policy version in the corpus's non-sensitive review evidence; the manifest records only the verification status. Never present an ASR output, automatic transcript, or guessed text as a human reference.

For primary lexical Word Error Rate (WER), the frozen `lexical-wer-unicode-boundary-v1` implementation normalizes both hypothesis and reference with Unicode NFC, lowercase, and collapsed whitespace; contiguous Unicode letters, numbers, and combining marks form tokens, while punctuation and other characters are boundaries. It preserves numeric forms and does not expand contractions, convert number spellings, remove fillers, or apply vocabulary corrections. Results record this version. The stable diagonal/deletion/insertion alignment tie order is covered by scorer tests.

Report lexical recognition errors separately from punctuation errors. Punctuation removed for lexical WER must still be compared against the punctuation-bearing reference using a separately described punctuation metric or annotated error count. Also keep any deterministic cleanup/vocabulary transformations separate from raw ASR output so recognition changes are not attributed to punctuation or cleanup.

## Coverage requirements

The frozen English set must meet the TECH_STACK Phase 0 minimum of at least 100 short phrases and 50 longer phrases. It must also include documented examples covering:

- Quiet and noisy recording conditions, with the condition described consistently enough to reproduce and compare.
- Different English accents and speaker varieties, without treating one variety as the only reference standard. Record only the non-sensitive accent/variety label needed to interpret results.
- Proper nouns, names, technical vocabulary, and abbreviations.
- Numbers in forms that expose recognition errors, including distinguishing the spoken reference from any later formatting transformation.
- Silence/no-speech controls, kept separate from speech WER denominators and reported as no-speech behavior.

Balance categories across short/long utterances and quiet/noisy conditions where the source permits. Document actual counts and known gaps when freezing the set; category labels alone do not prove representative coverage.

The manifest validator independently counts each ready fixture's `duration_class` and coverage attributes. Complete coverage requires at least 100 short and 50 long utterances; at least one quiet and one noisy speech fixture; accented English with at least two distinct source-reported variety labels; at least one proper-noun, technical-vocabulary, number, abbreviation, and silence-control fixture; and verified provenance plus human-reviewed references for every ready fixture. These are minimum inclusion criteria, not a claim of statistical representativeness. `collection_status: "complete"` is rejected unless every criterion passes. One hundred short and fifty long labels alone never establish coverage.

## Audio integrity and versioning

Store audio and reference SHA-256 values as lowercase or uppercase hexadecimal in the manifest; the validator accepts either. Hash the exact bytes at each repository-relative path. Any audio edit, re-encoding, trim, channel conversion, reference correction, or replacement changes the fixture and requires a new checksum and review. Do not claim a duration or format unless it has been measured from that file.

Keep a frozen manifest with each reported benchmark result. Do not silently update an existing fixture's audio or reference while retaining its ID. Add a new ID for a changed fixture, or explicitly retire the old entry and document the reason in the change review. Dataset updates require rechecking licensing, provenance, transcription, and every affected hash.

## Adding or replacing a fixture

1. Select a non-sensitive English source with auditable provenance and redistribution rights. Review the exact file-level license or permission before adding any asset.
2. Confirm the recording meets a documented coverage need and record its actual condition without personal identifiers.
3. Put permitted audio and its reference in repository-relative locations such as `fixtures/benchmark-audio/` and `fixtures/benchmark-references/`; do not add either until privacy and redistribution checks pass.
4. Have a human verify the complete reference against the audio. Resolve uncertain words, names, numbers, and punctuation, and record the review method outside the transcript itself.
5. Compute SHA-256 from the final audio bytes. Add a unique fixture ID, category, paths, checksum, provenance, permission evidence, and verified-reference status to the manifest.
6. Run the validator and tests below. Review the exact diff for accidental personal, customer, or sensitive content before proposing the corpus change.
7. Freeze the updated manifest and normalization version before reporting WER or comparing models. Keep prior benchmark results tied to their original manifest version/hash.

Run from the repository root:

```sh
python3 scripts/validate_fixture_manifest.py
python3 -m unittest discover -s scripts/tests -v
```

The validator does not download or create audio. A structurally valid manifest can pass while the corpus remains unready; its summary reports the actual planned count, verified ready audio count, and `ASR benchmark readiness: NOT READY` until at least one valid ready entry exists. The current manifest has no reference recordings, human-verified references, permission evidence, or audio checksums, so it is not benchmark-ready and cannot satisfy T-003 or AC-06.
