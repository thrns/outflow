#!/usr/bin/env python3
"""Score validated AC-07 bundles without making an acceptance decision.

This development-only adapter binds the four AC-07 artifacts through the
existing validator, assembles one shared scoring record per manifest case ID,
and passes those records to the established coordinator. It does not run
production cleanup, authenticate a declared source, or approve AC-07.
"""

from __future__ import annotations

import argparse
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence, cast

try:  # Support direct script execution and package-based imports.
    from . import validate_cleanup_ac07_actual_outputs as actual_output_binding
    from .evaluate_cleanup_ac07 import (
        CleanupAc07Evaluation,
        evaluate_cleanup_ac07_cases,
    )
    from .score_cleanup_ac07 import CleanupScoreValidationError
    from .score_cleanup_protected_ac07 import ProtectedCleanupValidationError
    from .validate_cleanup_ac07_actual_outputs import (
        CleanupActualOutputBindingSummary,
    )
except ImportError:  # pragma: no cover - exercised by direct CLI invocation.
    import validate_cleanup_ac07_actual_outputs as actual_output_binding
    from evaluate_cleanup_ac07 import CleanupAc07Evaluation, evaluate_cleanup_ac07_cases
    from score_cleanup_ac07 import CleanupScoreValidationError
    from score_cleanup_protected_ac07 import ProtectedCleanupValidationError
    from validate_cleanup_ac07_actual_outputs import (
        CleanupActualOutputBindingSummary,
    )


@dataclass(frozen=True)
class CleanupAc07BundleScore:
    """Separate binding and scoring results for a complete verified bundle set.

    The nested result objects are also frozen dataclasses. A successful value
    means validation and scoring completed; it does not mean any gate passed
    or that the corpus qualifies for formal acceptance.
    """

    actual_output_binding_summary: CleanupActualOutputBindingSummary
    evaluation_result: CleanupAc07Evaluation
    assembled_case_count: int


def score_verified_cleanup_bundles(
    manifest: object,
    reference_bundle: object,
    annotation_bundle: object,
    actual_bundle: object,
) -> CleanupAc07BundleScore:
    """Validate and score all cases associated by their exact case IDs.

    Input and expected strings come from the hash-verified reference bundle,
    actual strings come only from the separately supplied output bundle, and
    protected annotations come only from the validated annotation bundle.
    Every record is assembled in manifest order without changing any string.

    Raises:
        CleanupActualOutputBindingError: If manifest, reference, annotation,
            or actual-output validation fails.
        ProtectedCleanupValidationError: If the shared coordinator rejects
            the assembled protected-token records.
        CleanupScoreValidationError: If the shared coordinator rejects the
            projected exact-output records.
    """
    binding_summary = actual_output_binding.validate_cleanup_actual_outputs(
        manifest, reference_bundle, annotation_bundle, actual_bundle
    )

    manifest_object = cast(Mapping[str, object], manifest)
    reference_object = cast(Mapping[str, object], reference_bundle)
    annotation_object = cast(Mapping[str, object], annotation_bundle)
    actual_object = cast(Mapping[str, object], actual_bundle)
    manifest_cases = cast(list[Mapping[str, object]], manifest_object["cases"])
    reference_cases = cast(list[Mapping[str, object]], reference_object["cases"])
    annotation_cases = cast(
        list[Mapping[str, object]], annotation_object["cases"]
    )
    actual_cases = cast(list[Mapping[str, object]], actual_object["cases"])

    reference_by_id = {cast(str, case["id"]): case for case in reference_cases}
    annotation_by_id = {cast(str, case["id"]): case for case in annotation_cases}
    actual_by_id = {cast(str, case["id"]): case for case in actual_cases}

    records: list[dict[str, object]] = []
    for manifest_case in manifest_cases:
        case_id = cast(str, manifest_case["id"])
        reference_case = reference_by_id[case_id]
        annotation_case = annotation_by_id[case_id]
        actual_case = actual_by_id[case_id]
        records.append(
            {
                "id": case_id,
                "input": reference_case["input"],
                "expected": reference_case["expected"],
                "actual": actual_case["actual"],
                "protected_tokens": annotation_case["protected_tokens"],
            }
        )

    evaluation_result = evaluate_cleanup_ac07_cases(records)
    return CleanupAc07BundleScore(
        actual_output_binding_summary=binding_summary,
        evaluation_result=evaluation_result,
        assembled_case_count=len(records),
    )


def _load_synthetic_bundles() -> tuple[object, object, object, object]:
    """Read only the four checked-in artifacts using the strict AC-07 loader."""
    testdata_dir = Path(__file__).resolve().parent / "testdata"
    return (
        actual_output_binding._load_json(
            testdata_dir / "synthetic_ac07_manifest.json", "manifest"
        ),
        actual_output_binding._load_json(
            testdata_dir / "synthetic_ac07_reference_bundle.json",
            "reference bundle",
        ),
        actual_output_binding._load_json(
            testdata_dir / "synthetic_ac07_protected_annotations.json",
            "annotation bundle",
        ),
        actual_output_binding._load_json(
            testdata_dir / "synthetic_ac07_actual_outputs.json",
            "actual-output bundle",
        ),
    )


def _print_demo(
    manifest: object, result: CleanupAc07BundleScore
) -> None:
    """Print scorer results and non-acceptance metadata, never case content."""
    manifest_object = cast(Mapping[str, object], manifest)
    exact = result.evaluation_result.exact_output_score
    protected = result.evaluation_result.protected_token_score
    binding = result.actual_output_binding_summary

    print("AC-07 verified-bundle scoring: SYNTHETIC DEMO")
    print(f"Corpus ID: {binding.corpus_id}")
    print(f"Revision: {binding.revision}")
    print(f"Run ID: {binding.run_id}")
    print(
        "Verified case associations: "
        f"{binding.matched_case_id_count}/{binding.reference_case_count}"
    )
    print(f"Exact matches: {exact.exact_matches}")
    print(f"Mismatches: {exact.mismatches}")
    print(f"Exact-output rate: {exact.exact_output_percentage:.2f}%")
    print(
        "Minimum corpus size: "
        f"{'PASS' if exact.minimum_corpus_size_met else 'FAIL'}"
    )
    print(
        "Exact-output threshold: "
        f"{'PASS' if exact.exact_output_threshold_met else 'FAIL'}"
    )
    print(
        "Protected numeric tokens: "
        f"{protected.protected_number_tokens} "
        f"({protected.altered_number_tokens} altered)"
    )
    print(
        "Protected proper nouns: "
        f"{protected.protected_proper_noun_tokens} "
        f"({protected.altered_proper_noun_tokens} altered)"
    )
    print(
        "Protected-token gate: "
        f"{'PASS' if protected.protected_token_gate_passes else 'FAIL'}"
    )
    source_label = binding.declared_source_kind.replace("_", " ").upper()
    print(f"Output source: {source_label}")
    execution_status = (
        "VERIFIED"
        if binding.production_execution_independently_verified
        else "NOT VERIFIED"
    )
    print(f"Production execution: {execution_status}")
    review_status = cast(str, manifest_object["review_status"]).upper()
    print(f"Reference review: {review_status}")
    print("Corpus approval: NOT ESTABLISHED")
    print("AC-07 acceptance: NOT EVALUATED")


def main(argv: Sequence[str] | None = None) -> int:
    """Run the content-free demonstration over the checked-in synthetic data."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--demo",
        action="store_true",
        help="validate and score only the four checked-in synthetic artifacts",
    )
    arguments = parser.parse_args(argv)
    if not arguments.demo:
        parser.error("--demo is required; real acceptance data is not loaded")

    try:
        manifest, references, annotations, actuals = _load_synthetic_bundles()
        result = score_verified_cleanup_bundles(
            manifest, references, annotations, actuals
        )
    except (
        actual_output_binding.CleanupActualOutputBindingError,
        ProtectedCleanupValidationError,
        CleanupScoreValidationError,
    ) as error:
        parser.exit(1, f"error: {error}\n")

    _print_demo(manifest, result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
