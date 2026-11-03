# ASR benchmark results format

The current results validator accepts **synthetic numerical results only**. It supports schema version `1` and requires `result_type` to be exactly `synthetic`. It rejects measured or certified results; a valid file is never evidence that a model was run, that AC-06 passed, or that any real ASR benchmark was completed.

## Schema version 1

The JSON root has exactly four fields: `schema_version`, `result_type`, `samples`, and `totals`. Unknown and missing fields are errors. A sample contains only an identifier and numerical alignment counts; it must not contain audio, transcription, or personal data.

```json
{
  "schema_version": 1,
  "result_type": "synthetic",
  "samples": [
    {
      "id": "synthetic-001",
      "reference_words": 1,
      "substitutions": 1,
      "deletions": 0,
      "insertions": 0
    },
    {
      "id": "synthetic-002",
      "reference_words": 9,
      "substitutions": 0,
      "deletions": 0,
      "insertions": 0
    }
  ],
  "totals": {
    "utterances": 2,
    "reference_words": 10,
    "substitutions": 1,
    "deletions": 0,
    "insertions": 0,
    "wer": 0.1
  }
}
```

| Field | Meaning |
|---|---|
| `schema_version` | Integer schema version; only `1` is supported. |
| `result_type` | Must be `synthetic`; no real measurement type is accepted. |
| `samples` | Non-empty list with unique IDs matching `synthetic-` followed by digits. |
| `reference_words` | Positive integer reference-word count for a sample; totals are the sample sum. |
| `substitutions` | Nonnegative integer substitutions; totals are the sample sum. |
| `deletions` | Nonnegative integer deletions; totals are the sample sum. |
| `insertions` | Nonnegative integer insertions; totals are the sample sum. |
| `totals.utterances` | Number of samples. |
| `totals.wer` | Declared corpus WER as a nonnegative finite number. |

All count fields must be nonnegative JSON integers; booleans and floating-point counts are rejected. For each sample, substitutions plus deletions cannot exceed its reference-word count. The validator recalculates all aggregate counts and compares them with `totals`.

Corpus WER is recalculated as `(substitutions + deletions + insertions) / reference_words` using the aggregate sample counts. The declared WER is compared with a documented absolute tolerance of `1e-9` to accommodate floating-point representation. There is no 100% cap: insertions can make WER exceed 100%.

## Validate a file

From the repository root, run:

```sh
python3 scripts/validate_asr_results.py --file scripts/testdata/synthetic_asr_results.json
```

The command prints a concise count and WER summary, labels the result `SYNTHETIC`, and reports `Real ASR benchmark status: NOT ASSESSED`. It exits nonzero for unreadable or invalid JSON, duplicate JSON keys, unsupported schema/result types, missing or unknown fields, invalid counts, duplicate IDs, inconsistent totals, or incorrect WER. Errors identify the invalid field or sample index without printing input values. Validation reads the file and writes no result files.

## Measured native ASR results are a separate schema

`scripts/run_native_asr_benchmark.py` writes real native inference measurements to a separate schema validated by `scripts/validate_measured_asr_results.py`. The measured record identifies the pinned source, C API header, model catalog and per-file observed hashes, fixture manifest hash, local binaries, machine, reference/normalization policy versions, aggregate and per-fixture WER counts, latency, RTF, CPU time, and gate assessment. It rejects transcript-bearing extensions and never serializes hypotheses or references. Native stdout is captured into volatile memory for scoring; error messages omit captured output.

The measured runner requires at least one ready fixture with a local audio file, matching SHA-256, measured PCM metadata, file-level provenance/permission evidence, a human-verified reference file, and its SHA-256. It validates all ready fixtures before starting native inference. A successful validator check is a schema check; the file is measured evidence only when it was emitted by a reproducible native run tied to the exact manifest and artifacts.

Prompt 79C added the runner and validator but the repository manifest has **zero ready fixtures**. The runner therefore fails closed and writes no measured result. `evidence/model/MODEL_DECISION.md` records the actual native smoke memory/timing measurements separately and labels WER and corpus latency as not assessed. The single LibriVox smoke file is not a verified benchmark reference.

## Limits and future measured results

Arithmetic validation alone cannot verify that error counts came from genuine word alignments. It cannot establish that audio was processed, a model was used, the reference was correct, or a benchmark is reproducible. The synthetic scorer and this validator do not satisfy T-003, T-010, or AC-06.

Complete measured evaluation still requires the project-sized frozen corpus (at least 100 short phrases and 50 long phrases), required category coverage, human-checked references, frozen normalization/tokenization, artifact-specific license/provenance review, a physical 4 GB CPU-only machine, and reproducible evidence. It must also meet the separate quality, latency, installed-size, and process-tree resource gates in [`docs/ARCHITECTURE.md` §8.1](ARCHITECTURE.md) and [`docs/TECH_STACK.md` §3.2](TECH_STACK.md). The native runner does not certify Tauri integration, hardware compliance, accessibility, or redistribution.
