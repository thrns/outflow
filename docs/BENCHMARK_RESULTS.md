# ASR benchmark results format

`scripts/validate_asr_results.py` accepts **synthetic numerical results only** under schema version `1`; it rejects measured records. Genuine native measurements use the separate schema version `2` and `result_type: "measured_native_asr"` implemented by `scripts/run_native_asr_benchmark.py` and validated by `scripts/validate_measured_asr_results.py`. Neither schema calls a result certified: a valid record is not by itself proof that AC-06 passed.

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

`scripts/run_native_asr_benchmark.py` writes real native inference measurements to schema version 2. The measured record identifies the pinned source, C API header, model catalog and per-file observed hashes, fixture manifest hash, native binary hashes, actual macOS dyld runtime identity, exact installed physical-memory byte query and source, physical/reference-machine status, CPU-only execution, coverage counts, per-fixture WER counts, latency, RTF, and CPU time. Runtime identity includes SHA-256 values for the helper, Moonshine library, and ONNX Runtime, plus assertions that the resolved image paths match the selected inputs and the `otool` dependency chain is verified. The helper queries dyld image paths and the image containing the Moonshine C API symbol; hashing caller-supplied paths alone is not accepted. It rejects an unresolved identity, ambiguous dependency, architecture mismatch, or unapproved non-system image. Apple shared-cache system images can be listed but not always independently hashed; the verifier allows only its documented system roots and does not claim third-party provenance for system components. The standalone `scripts/verify_native_runtime.py` can verify runtime linkage when the fixture set is empty.

The hardware record separates `physical_memory_bytes` and its exact source from `physical_hardware_status`, `reference_hardware_class`, `cpu_only_inference_exercised`, and `real_hardware_measurements_completed`. A value of 4,000,000,000 bytes or less is only a necessary memory bound: eligibility also requires independently verified physical hardware and a class in the explicitly approved allowlist. The current allowlist is empty. Unknown or simulated systems cannot establish physical eligibility; a machine above the cap is `INELIGIBLE`. Even an eligible reference class remains `NOT_ASSESSED` for the physical 4 GB release gate until the benchmark has actually run on that machine. CPU-provider selection, physical eligibility, and release performance are recorded as separate facts.

The measured runner requires at least one ready fixture with a local audio file, matching SHA-256, measured PCM metadata, file-level provenance/permission evidence, a human-verified reference file, and its SHA-256. It validates all ready fixtures before starting native inference. A successful validator check is a schema check; the file is measured evidence only when it was emitted by a reproducible native run tied to the exact manifest and artifacts.

Prompt 79D strengthened fixture coverage, physical-hardware classification, actual loaded-library identity, and pre-write validation. Corpus completeness is recalculated from independently recorded duration and coverage attributes, including short and long counts, quiet/noisy conditions, source-reported accent variety, names, technical terms, numbers, abbreviations, silence controls, verified provenance, and human-reviewed references. The repository manifest still has **zero ready fixtures**, so the native evaluator fails closed and writes no measured result. The paired native smoke memory experiment is recorded separately in `evidence/mvp/prompt-79d-native-memory-measurements.json`; it is not WER evidence. The single LibriVox smoke file has no human-verified reference and is not a benchmark fixture.

## Limits and future measured results

Arithmetic validation alone cannot verify that error counts came from genuine word alignments. It cannot establish that audio was processed, a model was used, the reference was correct, or a benchmark is reproducible. The synthetic scorer and this validator do not satisfy T-003, T-010, or AC-06.

Complete measured evaluation still requires the project-sized frozen corpus (at least 100 short phrases and 50 long phrases), required category coverage, human-checked references, frozen normalization/tokenization, artifact-specific license/provenance review, a physical 4 GB CPU-only machine, and reproducible evidence. It must also meet the separate quality, latency, installed-size, and process-tree resource gates in [`docs/ARCHITECTURE.md` §8.1](ARCHITECTURE.md) and [`docs/TECH_STACK.md` §3.2](TECH_STACK.md). The native runner does not certify Tauri integration, hardware compliance, accessibility, or redistribution.
