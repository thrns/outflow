#!/usr/bin/env python3
"""Read-only audit of candidate Unicode crates in an Outflow Cargo workspace.

This tool parses Cargo TOML and lockfile data only. It does not execute
repository content, invoke Cargo, access the network, or modify any files.
Presence in Cargo.lock is not proof that a crate is a direct dependency of
outflow-core or that its implementation conforms to D145; consult
docs/VOCABULARY_UNICODE_RUST_OPTIONS.md for the source audit.
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from collections import deque
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence


@dataclass(frozen=True)
class Candidate:
    name: str
    investigated_version: str


@dataclass(frozen=True)
class CandidateFinding:
    candidate: Candidate
    direct_core: bool | None
    lock_present: bool | None
    locked_versions: tuple[str, ...]
    core_relationship: str
    explicitly_approved: bool
    approval_reference: str | None


@dataclass(frozen=True)
class AuditReport:
    workspace_status: str
    workspace_manifest_count: int
    core_status: str
    lock_status: str
    policy_status: str
    comparator_status: str
    findings: tuple[CandidateFinding, ...]
    diagnostics: tuple[str, ...]

    @property
    def complete(self) -> bool:
        return (
            self.workspace_status == "INSPECTED"
            and self.core_status == "INSPECTED"
            and self.lock_status == "INSPECTED"
        )


CANDIDATES = (
    Candidate("caseless", "0.2.2"),
    Candidate("unicode-normalization", "0.1.24"),
    Candidate("unicode-casefold", "0.2.0"),
)


def _read_toml(path: Path) -> tuple[dict[str, Any] | None, str]:
    try:
        content = path.read_bytes()
    except FileNotFoundError:
        return None, "MISSING"
    except OSError as error:
        return None, f"UNREADABLE ({error.__class__.__name__})"

    try:
        return tomllib.loads(content.decode("utf-8")), "INSPECTED"
    except (UnicodeDecodeError, tomllib.TOMLDecodeError):
        return None, "MALFORMED"


def _inside_repo(path: Path, repo: Path) -> bool:
    try:
        path.resolve().relative_to(repo.resolve())
    except ValueError:
        return False
    return True


def _workspace_manifest_paths(
    repo: Path,
    workspace_data: dict[str, Any] | None,
) -> tuple[Path, ...]:
    paths: set[Path] = {repo / "Cargo.toml"}
    if workspace_data is not None:
        workspace = workspace_data.get("workspace", {})
        members = workspace.get("members", []) if isinstance(workspace, dict) else []
        if isinstance(members, list):
            for member in members:
                if not isinstance(member, str):
                    continue
                try:
                    matches = (repo / member).glob("Cargo.toml")
                except (NotImplementedError, OSError):
                    continue
                for manifest in matches:
                    if _inside_repo(manifest, repo):
                        paths.add(manifest)

    # The core manifest is an explicit audit target even if a workspace
    # manifest omits or misstates its member list.
    core_manifest = repo / "crates" / "core" / "Cargo.toml"
    if _inside_repo(core_manifest, repo):
        paths.add(core_manifest)
    return tuple(sorted(paths))


def _dependency_package_name(
    dependency_key: str,
    specification: Any,
    workspace_dependencies: dict[str, Any],
) -> str:
    resolved_specification = specification
    if isinstance(specification, dict) and specification.get("workspace") is True:
        resolved_specification = workspace_dependencies.get(dependency_key, specification)

    if isinstance(resolved_specification, dict):
        package_name = resolved_specification.get("package")
        if isinstance(package_name, str):
            return package_name
    return dependency_key


def _core_direct_dependencies(
    core_data: dict[str, Any] | None,
    workspace_data: dict[str, Any] | None,
) -> set[str] | None:
    if core_data is None:
        return None

    workspace_table = workspace_data.get("workspace", {}) if workspace_data else {}
    if not isinstance(workspace_table, dict):
        workspace_table = {}
    workspace_dependencies = workspace_table.get("dependencies", {})
    if not isinstance(workspace_dependencies, dict):
        workspace_dependencies = {}

    direct: set[str] = set()

    def collect(dependency_table: Any) -> None:
        if not isinstance(dependency_table, dict):
            return
        for key, specification in dependency_table.items():
            if isinstance(key, str):
                direct.add(
                    _dependency_package_name(key, specification, workspace_dependencies)
                )

    for table_name in ("dependencies", "dev-dependencies", "build-dependencies"):
        collect(core_data.get(table_name))

    target_tables = core_data.get("target", {})
    if isinstance(target_tables, dict):
        for target_table in target_tables.values():
            if not isinstance(target_table, dict):
                continue
            for table_name in ("dependencies", "dev-dependencies", "build-dependencies"):
                collect(target_table.get(table_name))

    return direct


def _lock_package_name_and_version(reference: str) -> tuple[str, str | None]:
    """Parse Cargo.lock's `name [version] [(source)]` dependency spelling."""
    parts = reference.split()
    if not parts:
        return "", None
    version = parts[1] if len(parts) > 1 and re.fullmatch(r"\d+\.\d+\.\d+[^\s]*", parts[1]) else None
    return parts[0], version


def _core_lock_reachability(
    lock_data: dict[str, Any] | None,
    core_data: dict[str, Any] | None,
) -> tuple[set[str] | None, set[str]]:
    if lock_data is None:
        return None, set()

    packages = lock_data.get("package", [])
    if not isinstance(packages, list):
        return None, set()

    core_package = core_data.get("package", {}) if core_data else {}
    core_name = core_package.get("name") if isinstance(core_package, dict) else None
    core_version = core_package.get("version") if isinstance(core_package, dict) else None
    if not isinstance(core_name, str):
        return None, {item.get("name") for item in packages if isinstance(item, dict) and isinstance(item.get("name"), str)}

    nodes: list[dict[str, Any]] = [item for item in packages if isinstance(item, dict)]
    core_nodes = [
        index
        for index, item in enumerate(nodes)
        if item.get("name") == core_name
        and (not isinstance(core_version, str) or item.get("version") == core_version)
    ]
    if not core_nodes:
        return None, {
            item.get("name")
            for item in nodes
            if isinstance(item.get("name"), str)
        }

    adjacency: dict[int, list[int]] = {index: [] for index in range(len(nodes))}
    for index, package in enumerate(nodes):
        dependencies = package.get("dependencies", [])
        if not isinstance(dependencies, list):
            continue
        for reference in dependencies:
            if not isinstance(reference, str):
                continue
            dependency_name, dependency_version = _lock_package_name_and_version(reference)
            for target_index, candidate_package in enumerate(nodes):
                if candidate_package.get("name") != dependency_name:
                    continue
                if dependency_version is not None and candidate_package.get("version") != dependency_version:
                    continue
                adjacency[index].append(target_index)

    reachable_nodes: set[int] = set()
    queue: deque[int] = deque(core_nodes)
    while queue:
        index = queue.popleft()
        if index in reachable_nodes:
            continue
        reachable_nodes.add(index)
        queue.extend(adjacency[index])

    reachable_names = {
        nodes[index].get("name")
        for index in reachable_nodes
        if isinstance(nodes[index].get("name"), str)
    }
    locked_names = {
        item.get("name")
        for item in nodes
        if isinstance(item.get("name"), str)
    }
    return reachable_names, locked_names


def _policy_status(repo: Path) -> tuple[str, str]:
    policy_path = repo / "docs" / "VOCABULARY_UNICODE_POLICY.md"
    try:
        text = policy_path.read_text(encoding="utf-8")
    except (FileNotFoundError, OSError, UnicodeDecodeError):
        return "UNKNOWN (policy document unavailable)", "MISSING"

    for line in text.splitlines()[:20]:
        match = re.search(r"\*\*Status:\s*([^.*]+)", line, re.IGNORECASE)
        if match:
            status = match.group(1).strip()
            if status.upper().startswith("APPROVED") and "NOT APPROVED" not in status.upper():
                return status, "APPROVED"
            return status, "PENDING"
    return "UNKNOWN (no status marker found)", "PENDING"


def _explicit_candidate_approval(repo: Path, candidate_name: str) -> tuple[bool, str | None]:
    docs_path = repo / "docs"
    if not docs_path.is_dir():
        return False, None

    # Require an explicit relationship between the crate name and approval.
    # A nearby mention of an "approved Unicode version" is not approval of
    # the crate itself.
    quoted_name = rf"`?{re.escape(candidate_name)}`?"
    approval_claims = (
        re.compile(
            rf"^\s*(?:[-*]\s*)?{quoted_name}(?:\s+v?\d+(?:\.\d+)*)?\s*[:—-]\s*(?:explicitly\s+)?(?:approved|accepted)\b",
            re.IGNORECASE,
        ),
        re.compile(
            rf"^\s*(?:[-*]\s*)?(?:explicitly\s+)?(?:approved|accepted)\s*[:—-]\s*{quoted_name}\b",
            re.IGNORECASE,
        ),
        re.compile(
            rf"\b{quoted_name}(?:\s+v?\d+(?:\.\d+)*)?\s+(?:is|was|has been)\s+(?:explicitly\s+)?(?:approved|accepted)\b",
            re.IGNORECASE,
        ),
    )
    for path in sorted(docs_path.rglob("*.md")):
        if not _inside_repo(path, repo):
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError):
            continue
        for line in lines:
            if any(pattern.search(line) for pattern in approval_claims):
                return True, path.relative_to(repo).as_posix()
    return False, None


def _comparator_status(repo: Path) -> str:
    source_path = repo / "crates" / "core" / "src" / "vocabulary.rs"
    try:
        source = source_path.read_text(encoding="utf-8")
    except (FileNotFoundError, OSError, UnicodeDecodeError):
        return "UNKNOWN (validator source unavailable)"

    if "spoken.to_lowercase()" in source:
        return "LOWERCASE-ONLY"
    return "NOT THE LOWERCASE-ONLY IMPLEMENTATION (manual review required)"


def audit_repository(repo: Path) -> AuditReport:
    repo = repo.resolve()
    root_manifest = repo / "Cargo.toml"
    workspace_data, workspace_status = _read_toml(root_manifest)

    manifest_paths = _workspace_manifest_paths(repo, workspace_data)
    parsed_manifests: dict[Path, dict[str, Any]] = {}
    manifest_diagnostics: list[str] = []
    manifests_complete = workspace_data is not None
    if workspace_status != "INSPECTED":
        manifest_diagnostics.append(f"Manifest Cargo.toml: {workspace_status}")
    for path in manifest_paths:
        if path == root_manifest and workspace_data is not None:
            parsed_manifests[path] = workspace_data
            continue
        data, status = _read_toml(path)
        if data is not None:
            parsed_manifests[path] = data
        else:
            manifests_complete = False
            manifest_diagnostics.append(f"Manifest {path.relative_to(repo)}: {status}")

    if workspace_status != "INSPECTED":
        manifests_complete = False

    core_manifest = repo / "crates" / "core" / "Cargo.toml"
    core_data = parsed_manifests.get(core_manifest)
    core_status = "INSPECTED" if core_data is not None else "MISSING"
    if core_manifest in manifest_paths and core_data is None:
        _, core_status = _read_toml(core_manifest)

    if not manifests_complete:
        workspace_status = workspace_status if workspace_data is None else "INCOMPLETE"
    else:
        workspace_status = "INSPECTED"

    lock_data, lock_status = _read_toml(repo / "Cargo.lock")
    if lock_data is not None and not isinstance(lock_data.get("package"), list):
        lock_status = "MALFORMED"
        lock_data = None

    direct_dependencies = _core_direct_dependencies(core_data, workspace_data)
    reachable_names, locked_names = _core_lock_reachability(lock_data, core_data)
    lock_packages = lock_data.get("package", []) if lock_data else []
    versions_by_name: dict[str, tuple[str, ...]] = {}
    for package in lock_packages:
        if not isinstance(package, dict):
            continue
        name = package.get("name")
        version = package.get("version")
        if isinstance(name, str) and isinstance(version, str):
            versions_by_name.setdefault(name, tuple())
            versions_by_name[name] = tuple(sorted(set(versions_by_name[name]) | {version}))

    policy_status, _ = _policy_status(repo)
    findings: list[CandidateFinding] = []
    for candidate in CANDIDATES:
        direct = None if direct_dependencies is None else candidate.name in direct_dependencies
        lock_present = None if lock_data is None else candidate.name in locked_names
        versions = versions_by_name.get(candidate.name, ())

        if direct:
            relationship = "DIRECT"
        elif reachable_names is None:
            relationship = "UNKNOWN (core lock graph unavailable)"
        elif candidate.name in reachable_names:
            relationship = "TRANSITIVE"
        elif lock_present:
            relationship = "LOCKFILE ONLY (not reachable from outflow-core)"
        else:
            relationship = "NOT PRESENT"

        explicitly_approved, approval_reference = _explicit_candidate_approval(
            repo, candidate.name
        )
        findings.append(
            CandidateFinding(
                candidate=candidate,
                direct_core=direct,
                lock_present=lock_present,
                locked_versions=versions,
                core_relationship=relationship,
                explicitly_approved=explicitly_approved,
                approval_reference=approval_reference,
            )
        )

    return AuditReport(
        workspace_status=workspace_status,
        workspace_manifest_count=len(parsed_manifests),
        core_status=core_status,
        lock_status=lock_status,
        policy_status=policy_status,
        comparator_status=_comparator_status(repo),
        findings=tuple(findings),
        diagnostics=tuple(manifest_diagnostics),
    )


def render_report(report: AuditReport) -> str:
    policy_approved = report.policy_status.upper().startswith("APPROVED") and not report.policy_status.upper().startswith(
        "NOT APPROVED"
    )
    lines = [
        "Outflow Unicode dependency audit",
        f"Workspace manifests: {report.workspace_status} ({report.workspace_manifest_count} parsed)",
        f"outflow-core manifest: {report.core_status}",
        f"Cargo lockfile: {report.lock_status}",
        f"Production matching policy: {report.comparator_status}",
        "Candidate policy: D145 CANONICAL CASELESS",
        f"Policy document status: {report.policy_status}",
        "Candidate approval: SEE PER-CRATE FINDINGS" if policy_approved else "Candidate approval: PENDING",
        "D145 conformance: NOT ESTABLISHED BY THIS AUDIT",
        "Cargo dependencies modified: NO",
        "Production Rust validator modified by audit: NO",
        "",
        "Candidate findings (lockfile presence is not direct API availability):",
    ]

    for finding in report.findings:
        direct = "UNKNOWN" if finding.direct_core is None else "YES" if finding.direct_core else "NO"
        locked = "UNKNOWN" if finding.lock_present is None else "YES" if finding.lock_present else "NO"
        versions = ", ".join(finding.locked_versions) if finding.locked_versions else "NONE"
        approval = "YES" if finding.explicitly_approved else "NO"
        if finding.approval_reference:
            approval += f" ({finding.approval_reference})"
        lines.extend(
            (
                f"- {finding.candidate.name} (investigated {finding.candidate.investigated_version}):",
                f"  outflow-core direct dependency: {direct}",
                f"  Cargo.lock package present: {locked}; locked version(s): {versions}",
                f"  outflow-core lock relationship: {finding.core_relationship}",
                f"  explicitly approved in project docs: {approval}",
            )
        )

    if report.diagnostics:
        lines.extend(("", "Diagnostics:", *(f"- {diagnostic}" for diagnostic in report.diagnostics)))
    return "\n".join(lines)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True, help="Outflow repository root")
    arguments = parser.parse_args(argv)
    report = audit_repository(arguments.repo)
    print(render_report(report))
    return 0 if report.complete else 2


if __name__ == "__main__":
    sys.exit(main())
