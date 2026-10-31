#!/usr/bin/env python3
"""Validate Outflow's planned Moonshine build scope against pinned source notes.

This standard-library-only checker reads local JSON files. It does not contact
the network, configure or build Moonshine, inspect binaries, download models, or
write files. CMake source evidence is kept separate from compiled-artifact proof.
"""

from __future__ import annotations

import json
from pathlib import Path
import re
import sys
from typing import Any
from urllib.parse import urlsplit


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
SCOPE_PATH = REPOSITORY_ROOT / "docs" / "moonshine-build-scope.json"
DEPENDENCY_INVENTORY_PATH = REPOSITORY_ROOT / "docs" / "moonshine-third-party.json"
NOTICE_MANIFEST_PATH = REPOSITORY_ROOT / "docs" / "third-party-notices.json"
PINNED_REVISION = "234f60faa0eb388b01cdf7e60aca232af37aefda"
UPSTREAM_REPOSITORY = "https://github.com/moonshine-ai/moonshine"
MAX_JSON_BYTES = 1_048_576

SCOPES = {"required", "excluded", "undetermined"}
EVIDENCE_STATUSES = {"confirmed", "conditional", "unverified"}
BUILD_CONTROL_STATUSES = {"required", "none-verified", "partial", "undetermined"}
SOURCE_PATHS = {
    "core/CMakeLists.txt",
    "core/ort-utils/CMakeLists.txt",
    "core/bin-tokenizer/CMakeLists.txt",
    "core/moonshine-tts/CMakeLists.txt",
}
SOURCE_LINE_LIMITS = {
    "core/CMakeLists.txt": 1191,
    "core/ort-utils/CMakeLists.txt": 105,
    "core/bin-tokenizer/CMakeLists.txt": 46,
    "core/moonshine-tts/CMakeLists.txt": 315,
}

ROOT_FIELDS = {
    "schema_version",
    "source",
    "application",
    "intended_build_target",
    "source_evidence",
    "build_options",
    "features",
    "third_party_dependencies",
    "unresolved_build_questions",
    "execution",
}
SOURCE_FIELDS = {"repository", "revision", "review_scope"}
APPLICATION_FIELDS = {
    "purpose",
    "language",
    "recognition_mode",
    "models_per_installer",
    "cpu_only_requirement",
    "cpu_only_verification_status",
    "target_operating_systems",
    "platform_verification_status",
    "model_asset_revision_status",
    "project_requirements",
}
PLATFORM_FIELDS = {"name", "architectures", "support_intent"}
TARGET_FIELDS = {"name", "kind", "definition_evidence_id", "configuration_status"}
EVIDENCE_FIELDS = {"evidence_id", "source_path", "url", "finding"}
OPTION_FIELDS = {
    "name",
    "kind",
    "declaration_status",
    "default",
    "conditional_expression",
    "effect",
    "evidence_ids",
}
FEATURE_FIELDS = {
    "feature_id",
    "name",
    "intended_scope",
    "source_evidence_status",
    "actual_compiled_status",
    "evidence_ids",
    "build_control",
    "unresolved_question_ids",
    "notes",
}
BUILD_CONTROL_FIELDS = {"status", "option_name", "requested_value", "assessment"}
DEPENDENCY_FIELDS = {"dependency_id", "build_use_classification", "scope_relationship", "notes"}
QUESTION_FIELDS = {
    "question_id",
    "status",
    "question",
    "affected_feature_ids",
    "resolution_evidence",
}
EXECUTION_FIELDS = {
    "cmake_configuration",
    "compilation",
    "linking",
    "runtime_tests",
    "resource_measurements",
    "model_download",
    "binary_contents",
    "minimal_runtime_feasibility",
}

EXPECTED_OPTIONS = {
    "MOONSHINE_TTS_BUILD_ONNX": (
        "option", "verified", "ON", "if(MOONSHINE_TTS_BUILD_ONNX)"
    ),
    "MOONSHINE_RELIABILITY": (
        "option", "verified", "OFF", "if (MOONSHINE_RELIABILITY)"
    ),
    "MOONSHINE_SANITIZER": (
        "cache-string", "verified", "address,undefined", "if (MOONSHINE_RELIABILITY)"
    ),
    "MOONSHINE_BUILD_SHARED": (
        "referenced-variable",
        "referenced-default-unknown",
        None,
        "if (NOT MOONSHINE_BUILD_SHARED AND (IOS OR MOONSHINE_BUILD_SWIFT OR WIN32 OR EMSCRIPTEN))",
    ),
    "MOONSHINE_BUILD_SWIFT": (
        "referenced-variable",
        "referenced-default-unknown",
        None,
        "if (NOT MOONSHINE_BUILD_SHARED AND (IOS OR MOONSHINE_BUILD_SWIFT OR WIN32 OR EMSCRIPTEN))",
    ),
}


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Reject duplicate JSON keys so validation cannot depend on key order."""
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON object key")
        result[key] = value
    return result


def load_json(path: str | Path) -> Any:
    """Load one bounded UTF-8 JSON document without network or write access."""
    try:
        with Path(path).open("rb") as source_file:
            contents = source_file.read(MAX_JSON_BYTES + 1)
    except (OSError, TypeError, ValueError) as error:
        raise ValueError("a required local JSON document could not be read") from error
    if len(contents) > MAX_JSON_BYTES:
        raise ValueError("a local JSON document exceeds the supported size limit")
    try:
        return json.loads(contents.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeError, json.JSONDecodeError, ValueError, RecursionError) as error:
        raise ValueError("a local document is not valid UTF-8 JSON") from error


def _check_fields(value: dict[str, Any], expected: set[str], label: str, errors: list[str]) -> None:
    missing = expected - value.keys()
    extra = value.keys() - expected
    if missing:
        errors.append(f"{label} is missing required field(s): {', '.join(sorted(missing))}")
    if extra:
        errors.append(f"{label} contains unsupported field(s): {', '.join(sorted(extra))}")


def _valid_immutable_source_url(value: Any, source_path: Any) -> bool:
    """Require a pinned GitHub file URL and a syntactically valid line anchor."""
    if not _nonempty_string(value) or not _nonempty_string(source_path):
        return False
    if not isinstance(source_path, str) or source_path not in SOURCE_PATHS:
        return False
    if any(character.isspace() for character in value):
        return False
    try:
        parsed = urlsplit(value)
        if (
            parsed.scheme != "https"
            or parsed.netloc != "github.com"
            or parsed.query
            or parsed.username is not None
            or parsed.password is not None
        ):
            return False
        expected_path = (
            f"/moonshine-ai/moonshine/blob/{PINNED_REVISION}/{source_path}"
        )
        if parsed.path != expected_path:
            return False
        match = re.fullmatch(r"L([1-9][0-9]*)(?:-L([1-9][0-9]*))?", parsed.fragment)
        if not match:
            return False
        start = int(match.group(1))
        end = int(match.group(2) or start)
        return start <= end <= SOURCE_LINE_LIMITS[source_path]
    except (ValueError, TypeError):
        return False


def _string_list(value: Any, label: str, errors: list[str], *, nonempty: bool = True) -> list[str]:
    if not isinstance(value, list) or (nonempty and not value):
        errors.append(f"{label} must be a {'non-empty ' if nonempty else ''}list of strings")
        return []
    if any(not _nonempty_string(item) for item in value):
        errors.append(f"{label} must contain only non-empty strings")
        return []
    return value


def summarize_scope(data: Any) -> dict[str, Any]:
    """Derive feature and evidence counts from the supplied manifest."""
    features = data.get("features", []) if isinstance(data, dict) else []
    questions = data.get("unresolved_build_questions", []) if isinstance(data, dict) else []
    counts = {scope: 0 for scope in sorted(SCOPES)}
    evidence_counts = {status: 0 for status in sorted(EVIDENCE_STATUSES)}
    if isinstance(features, list):
        for feature in features:
            scope = feature.get("intended_scope") if isinstance(feature, dict) else None
            if isinstance(scope, str) and scope in counts:
                counts[feature["intended_scope"]] += 1
    if isinstance(features, list):
        for feature in features:
            status = feature.get("source_evidence_status") if isinstance(feature, dict) else None
            if isinstance(status, str) and status in evidence_counts:
                evidence_counts[status] += 1
    return {
        "feature_count": len(features) if isinstance(features, list) else 0,
        "scope_counts": counts,
        "evidence_counts": evidence_counts,
        "question_count": len(questions) if isinstance(questions, list) else 0,
    }


def validate_scope(
    data: Any,
    dependency_inventory: Any,
    notice_manifest: Any,
) -> tuple[list[str], dict[str, Any]]:
    """Return local structural/source-contract errors and derived counts."""
    errors: list[str] = []
    summary = summarize_scope(data)
    if not isinstance(data, dict):
        return ["build-scope root must be a JSON object"], summary
    _check_fields(data, ROOT_FIELDS, "build-scope", errors)

    version = data.get("schema_version")
    if type(version) is not int or version != 1:
        errors.append("schema_version must be 1")

    source = data.get("source")
    if not isinstance(source, dict):
        errors.append("source must be an object")
        source = {}
    else:
        _check_fields(source, SOURCE_FIELDS, "source", errors)
    if source.get("repository") != UPSTREAM_REPOSITORY:
        errors.append("source.repository must identify the reviewed Moonshine repository")
    if source.get("revision") != PINNED_REVISION:
        errors.append(f"source.revision must be {PINNED_REVISION}")
    if not _nonempty_string(source.get("review_scope")):
        errors.append("source.review_scope must state the evidence boundary")

    application = data.get("application")
    if not isinstance(application, dict):
        errors.append("application must be an object")
        application = {}
    else:
        _check_fields(application, APPLICATION_FIELDS, "application", errors)
    fixed_application = {
        "purpose": "English streaming speech-to-text",
        "language": "English",
        "recognition_mode": "streaming",
        "models_per_installer": 1,
        "cpu_only_requirement": "required",
        "cpu_only_verification_status": "not-tested",
        "platform_verification_status": "not-tested",
        "model_asset_revision_status": "undetermined",
    }
    for field, expected in fixed_application.items():
        if application.get(field) != expected:
            errors.append(f"application.{field} must remain {expected!r}")
    platforms = application.get("target_operating_systems")
    expected_platforms = {
        "macOS 13+": ("arm64", "x86_64", "target"),
        "Windows 10/11": ("x86_64", "target"),
        "Linux X11": ("x86_64", "target"),
        "Linux Wayland": ("x86_64", "best-effort"),
    }
    if not isinstance(platforms, list) or not platforms:
        errors.append("application.target_operating_systems must be a non-empty list")
    else:
        seen_platforms: set[str] = set()
        for index, platform in enumerate(platforms):
            label = f"application.target_operating_systems[{index}]"
            if not isinstance(platform, dict):
                errors.append(f"{label} must be an object")
                continue
            _check_fields(platform, PLATFORM_FIELDS, label, errors)
            name = platform.get("name")
            if not isinstance(name, str) or name not in expected_platforms:
                errors.append(f"{label}.name is not an intended Outflow platform")
                continue
            if name in seen_platforms:
                errors.append(f"{label}.name duplicates another target operating system")
            seen_platforms.add(name)
            architectures, support_intent = expected_platforms[name][:-1], expected_platforms[name][-1]
            actual_architectures = platform.get("architectures")
            if actual_architectures != list(architectures):
                errors.append(f"{label}.architectures does not match the documented target matrix")
            if platform.get("support_intent") != support_intent:
                errors.append(f"{label}.support_intent must be {support_intent!r}")
        if seen_platforms != set(expected_platforms):
            errors.append("application.target_operating_systems must list the documented OS targets")
    requirements = _string_list(application.get("project_requirements"), "application.project_requirements", errors)
    if not any("PRD.md" in item and "AC-06" in item for item in requirements):
        errors.append("application.project_requirements must identify the ASR product acceptance source")

    target = data.get("intended_build_target")
    if not isinstance(target, dict):
        errors.append("intended_build_target must be an object")
        target = {}
    else:
        _check_fields(target, TARGET_FIELDS, "intended_build_target", errors)
    if target.get("name") != "moonshine" or target.get("kind") != "native library":
        errors.append("intended_build_target must identify the native moonshine library")
    if target.get("configuration_status") != "not-configured":
        errors.append("intended_build_target.configuration_status must remain not-configured")

    evidence = data.get("source_evidence")
    evidence_ids: set[str] = set()
    if not isinstance(evidence, list) or not evidence:
        errors.append("source_evidence must be a non-empty list")
        evidence = []
    for index, item in enumerate(evidence):
        label = f"source_evidence[{index}]"
        if not isinstance(item, dict):
            errors.append(f"{label} must be an object")
            continue
        _check_fields(item, EVIDENCE_FIELDS, label, errors)
        identifier = item.get("evidence_id")
        if not _nonempty_string(identifier):
            errors.append(f"{label}.evidence_id must be a non-empty string")
        elif identifier in evidence_ids:
            errors.append(f"{label}.evidence_id duplicates another source-evidence identifier")
        else:
            evidence_ids.add(identifier)
        if not _nonempty_string(item.get("finding")):
            errors.append(f"{label}.finding must be a non-empty source-level statement")
        if not _valid_immutable_source_url(item.get("url"), item.get("source_path")):
            errors.append(f"{label}.url must be an immutable pinned source URL with a valid line anchor")

    options = data.get("build_options")
    option_names: set[str] = set()
    options_by_name: dict[str, dict[str, Any]] = {}
    if not isinstance(options, list):
        errors.append("build_options must be a list")
        options = []
    for index, option in enumerate(options):
        label = f"build_options[{index}]"
        if not isinstance(option, dict):
            errors.append(f"{label} must be an object")
            continue
        _check_fields(option, OPTION_FIELDS, label, errors)
        name = option.get("name")
        if not _nonempty_string(name):
            errors.append(f"{label}.name must be a non-empty string")
            continue
        if name in option_names:
            errors.append(f"{label}.name duplicates another build option")
        option_names.add(name)
        options_by_name[name] = option
        expected = EXPECTED_OPTIONS.get(name)
        if expected is None:
            errors.append(f"{label}.name is not a verified option or explicitly unknown CMake variable")
        else:
            kind, declaration_status, default, conditional_expression = expected
            if option.get("kind") != kind:
                errors.append(f"{label}.kind does not match the reviewed CMake declaration")
            if option.get("declaration_status") != declaration_status:
                errors.append(f"{label}.declaration_status does not match the reviewed CMake source")
            if option.get("default") != default:
                errors.append(f"{label}.default does not match the reviewed CMake value")
            if option.get("conditional_expression") != conditional_expression:
                errors.append(f"{label}.conditional_expression does not match the reviewed CMake expression")
        if not _nonempty_string(option.get("effect")):
            errors.append(f"{label}.effect must explain the source-level effect and limits")
        refs = _string_list(option.get("evidence_ids"), f"{label}.evidence_ids", errors)
        if any(reference not in evidence_ids for reference in refs):
            errors.append(f"{label}.evidence_ids references unknown source evidence")
    if option_names != set(EXPECTED_OPTIONS):
        errors.append("build_options must record verified options and explicitly unknown shared-build variables")

    features = data.get("features")
    feature_ids: set[str] = set()
    feature_by_id: dict[str, dict[str, Any]] = {}
    feature_question_refs: dict[str, list[str]] = {}
    if not isinstance(features, list) or not features:
        errors.append("features must be a non-empty list")
        features = []
    for index, feature in enumerate(features):
        label = f"features[{index}]"
        if not isinstance(feature, dict):
            errors.append(f"{label} must be an object")
            continue
        _check_fields(feature, FEATURE_FIELDS, label, errors)
        identifier = feature.get("feature_id")
        if not _nonempty_string(identifier):
            errors.append(f"{label}.feature_id must be a non-empty string")
        elif identifier in feature_ids:
            errors.append(f"{label}.feature_id duplicates another feature identifier")
        else:
            feature_ids.add(identifier)
            feature_by_id[identifier] = feature
        if not _nonempty_string(feature.get("name")):
            errors.append(f"{label}.name must be a non-empty string")
        scope = feature.get("intended_scope")
        if not isinstance(scope, str) or scope not in SCOPES:
            errors.append(f"{label}.intended_scope must be required, excluded, or undetermined")
        evidence_status = feature.get("source_evidence_status")
        if not isinstance(evidence_status, str) or evidence_status not in EVIDENCE_STATUSES:
            errors.append(f"{label}.source_evidence_status must be confirmed, conditional, or unverified")
        if feature.get("actual_compiled_status") != "not-tested":
            errors.append(f"{label}.actual_compiled_status must remain not-tested")
        refs = _string_list(feature.get("evidence_ids"), f"{label}.evidence_ids", errors)
        if any(reference not in evidence_ids for reference in refs):
            errors.append(f"{label}.evidence_ids references unknown source evidence")
        if evidence_status == "unverified" and not _string_list(
            feature.get("unresolved_question_ids"), f"{label}.unresolved_question_ids", errors
        ):
            errors.append(f"{label} needs an unresolved question when source evidence is unverified")

        control = feature.get("build_control")
        if not isinstance(control, dict):
            errors.append(f"{label}.build_control must be an object")
            control = {}
        else:
            _check_fields(control, BUILD_CONTROL_FIELDS, f"{label}.build_control", errors)
        control_status = control.get("status")
        if not isinstance(control_status, str) or control_status not in BUILD_CONTROL_STATUSES:
            errors.append(f"{label}.build_control.status is unsupported")
        option_name = control.get("option_name")
        requested_value = control.get("requested_value")
        if option_name is not None and (
            not isinstance(option_name, str) or option_name not in options_by_name
        ):
            errors.append(f"{label}.build_control.option_name is an unsupported or invented build option")
        elif isinstance(control_status, str) and control_status in {
            "none-verified", "undetermined", "required"
        } and option_name is not None:
            errors.append(f"{label}.build_control must not claim an option for status {control_status}")
        elif control_status == "partial":
            if not isinstance(option_name, str) or option_name not in options_by_name:
                errors.append(f"{label}.build_control.partial must reference a verified option")
            if not isinstance(identifier, str) or identifier not in {"tts-g2p", "optional-utilities"}:
                errors.append(f"{label}.build_control.partial is not supported for this feature")
            if option_name == "MOONSHINE_TTS_BUILD_ONNX" and requested_value != "OFF":
                errors.append(f"{label}.build_control must state that TTS ONNX was requested OFF for partial exclusion")
        elif requested_value is not None:
            errors.append(f"{label}.build_control.requested_value must be null without a partial control")
        if not _nonempty_string(control.get("assessment")):
            errors.append(f"{label}.build_control.assessment must state what is and is not gated")
        if scope == "excluded" and (
            not isinstance(control_status, str)
            or control_status not in {"none-verified", "partial", "undetermined"}
        ):
            errors.append(f"{label} cannot claim an excluded feature has a verified build exclusion")
        if scope == "required" and (
            not isinstance(control_status, str)
            or control_status not in {"required", "undetermined"}
        ):
            errors.append(f"{label} required scope must not be recorded as excluded")
        questions = _string_list(
            feature.get("unresolved_question_ids"), f"{label}.unresolved_question_ids", errors
        )
        if isinstance(identifier, str):
            feature_question_refs[identifier] = questions
        if scope in {"excluded", "undetermined"} and not questions:
            errors.append(f"{label} must retain unresolved build questions instead of treating scope as verified")
        if not _nonempty_string(feature.get("notes")):
            errors.append(f"{label}.notes must state the evidence boundary")

    dependencies = data.get("third_party_dependencies")
    scope_dependency_ids: set[str] = set()
    if not isinstance(dependencies, list) or not dependencies:
        errors.append("third_party_dependencies must be a non-empty list")
        dependencies = []
    for index, dependency in enumerate(dependencies):
        label = f"third_party_dependencies[{index}]"
        if not isinstance(dependency, dict):
            errors.append(f"{label} must be an object")
            continue
        _check_fields(dependency, DEPENDENCY_FIELDS, label, errors)
        identifier = dependency.get("dependency_id")
        if not _nonempty_string(identifier):
            errors.append(f"{label}.dependency_id must be a non-empty string")
        elif identifier in scope_dependency_ids:
            errors.append(f"{label}.dependency_id duplicates another dependency")
        else:
            scope_dependency_ids.add(identifier)
        classification = dependency.get("build_use_classification")
        if not isinstance(classification, str) or classification not in {
            "confirmed-build-input", "conditional", "test-only"
        }:
            errors.append(f"{label}.build_use_classification is unsupported")
        for field in ("scope_relationship", "notes"):
            if not _nonempty_string(dependency.get(field)):
                errors.append(f"{label}.{field} must be a non-empty explanation")

    _validate_dependency_consistency(
        dependency_inventory, notice_manifest, scope_dependency_ids, dependencies, errors
    )

    questions = data.get("unresolved_build_questions")
    question_ids: set[str] = set()
    if not isinstance(questions, list) or not questions:
        errors.append("unresolved_build_questions must be a non-empty list")
        questions = []
    question_feature_refs: dict[str, list[str]] = {}
    for index, question in enumerate(questions):
        label = f"unresolved_build_questions[{index}]"
        if not isinstance(question, dict):
            errors.append(f"{label} must be an object")
            continue
        _check_fields(question, QUESTION_FIELDS, label, errors)
        identifier = question.get("question_id")
        if not _nonempty_string(identifier):
            errors.append(f"{label}.question_id must be a non-empty string")
        elif identifier in question_ids:
            errors.append(f"{label}.question_id duplicates another question")
        else:
            question_ids.add(identifier)
        if question.get("status") != "unresolved":
            errors.append(f"{label}.status must remain unresolved")
        for field in ("question", "resolution_evidence"):
            if not _nonempty_string(question.get(field)):
                errors.append(f"{label}.{field} must be a non-empty statement")
        affected = _string_list(question.get("affected_feature_ids"), f"{label}.affected_feature_ids", errors)
        if isinstance(identifier, str):
            question_feature_refs[identifier] = affected
        if any(feature_id not in feature_ids for feature_id in affected):
            errors.append(f"{label}.affected_feature_ids references an unknown feature")

    for feature_id, references in feature_question_refs.items():
        for question_id in references:
            if question_id not in question_ids:
                errors.append(f"feature {feature_id} references an unknown unresolved question")
            elif feature_id not in question_feature_refs.get(question_id, []):
                errors.append(f"question {question_id} does not list affected feature {feature_id}")
    for question_id, affected_feature_ids in question_feature_refs.items():
        for feature_id in affected_feature_ids:
            if feature_id in feature_by_id and question_id not in feature_question_refs.get(feature_id, []):
                errors.append(f"feature {feature_id} does not reference its question {question_id}")

    execution = data.get("execution")
    if not isinstance(execution, dict):
        errors.append("execution must be an object")
        execution = {}
    else:
        _check_fields(execution, EXECUTION_FIELDS, "execution", errors)
    expected_execution = {
        "cmake_configuration": "NOT_RUN",
        "compilation": "NOT_RUN",
        "linking": "NOT_RUN",
        "runtime_tests": "NOT_RUN",
        "resource_measurements": "NOT_RUN",
        "model_download": "NOT_RUN",
        "binary_contents": "NOT_VERIFIED",
        "minimal_runtime_feasibility": "NOT_ASSESSED",
    }
    for field, expected in expected_execution.items():
        if execution.get(field) != expected:
            errors.append(f"execution.{field} must remain {expected}")

    return errors, summary


def _validate_dependency_consistency(
    inventory: Any,
    notices: Any,
    scope_ids: set[str],
    scope_entries: list[Any],
    errors: list[str],
) -> None:
    if not isinstance(inventory, dict) or not isinstance(notices, dict):
        errors.append("third-party dependency inventory and notice manifest must be JSON objects")
        return
    if inventory.get("reviewed_moonshine_commit") != PINNED_REVISION:
        errors.append("dependency inventory must use the pinned Moonshine source revision")
    if notices.get("upstream_commit") != PINNED_REVISION:
        errors.append("notice manifest must use the pinned Moonshine source revision")
    if inventory.get("upstream_repository") != "moonshine-ai/moonshine":
        errors.append("dependency inventory repository does not match the pinned source")
    if notices.get("upstream_repository") != "moonshine-ai/moonshine":
        errors.append("notice manifest repository does not match the pinned source")

    inventory_entries = inventory.get("dependencies")
    notice_entries = notices.get("dependencies")
    if not isinstance(inventory_entries, list) or not isinstance(notice_entries, list):
        errors.append("third-party inventory dependencies must be lists")
        return
    inventory_map = {
        item.get("dependency_id"): item
        for item in inventory_entries
        if isinstance(item, dict) and isinstance(item.get("dependency_id"), str)
    }
    notice_map = {
        item.get("dependency_id"): item
        for item in notice_entries
        if isinstance(item, dict) and isinstance(item.get("dependency_id"), str)
    }
    if len(inventory_map) != len(inventory_entries) or len(notice_map) != len(notice_entries):
        errors.append("third-party inventory and notice dependencies must have unique IDs")
    if set(inventory_map) != set(notice_map) or scope_ids != set(inventory_map):
        errors.append("build scope must account for the same dependency IDs as both audited inventories")
    scope_map = {
        item.get("dependency_id"): item
        for item in scope_entries
        if isinstance(item, dict) and isinstance(item.get("dependency_id"), str)
    }
    for dependency_id, inventory_entry in inventory_map.items():
        classification = inventory_entry.get("build_use_classification")
        notice_entry = notice_map.get(dependency_id, {})
        scope_entry = scope_map.get(dependency_id, {})
        if notice_entry.get("build_use_classification") != classification:
            errors.append(f"notice manifest classification differs for {dependency_id}")
        if scope_entry.get("build_use_classification") != classification:
            errors.append(f"build scope classification differs for {dependency_id}")


def render_summary(data: Any, summary: dict[str, Any]) -> str:
    """Render stable counts derived only from manifest contents."""
    source = data.get("source", {}) if isinstance(data, dict) else {}
    application = data.get("application", {}) if isinstance(data, dict) else {}
    counts = summary["scope_counts"]
    evidence = summary["evidence_counts"]
    return "\n".join(
        (
            "Outflow Moonshine build scope valid",
            f"Source revision: {source.get('revision', 'UNKNOWN')}",
            f"Target: {application.get('purpose', 'UNKNOWN')}, CPU-only",
            (
                "Feature scope: "
                f"{counts['required']} required, {counts['excluded']} excluded, "
                f"{counts['undetermined']} undetermined"
            ),
            (
                "Source evidence: "
                f"{evidence['confirmed']} confirmed, {evidence['conditional']} conditional, "
                f"{evidence['unverified']} unverified"
            ),
            f"Unresolved build questions: {summary['question_count']}",
            "Build plan: DOCUMENTED",
            "Compilation: NOT RUN",
            "Binary contents: NOT VERIFIED",
            "Minimal runtime feasibility: NOT ASSESSED",
        )
    )


def main() -> int:
    try:
        manifest = load_json(SCOPE_PATH)
        inventory = load_json(DEPENDENCY_INVENTORY_PATH)
        notices = load_json(NOTICE_MANIFEST_PATH)
    except ValueError as error:
        print(f"Moonshine build scope validation failed: {error}", file=sys.stderr)
        return 1

    errors, summary = validate_scope(manifest, inventory, notices)
    if errors:
        print("Outflow Moonshine build scope validation failed", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print(render_summary(manifest, summary))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
