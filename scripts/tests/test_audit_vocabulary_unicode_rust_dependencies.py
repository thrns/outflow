"""Tests for the read-only Cargo dependency audit."""

from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path

from scripts.audit_vocabulary_unicode_rust_dependencies import (
    audit_repository,
    main,
    render_report,
)


ROOT_MANIFEST = '''
[workspace]
members = ["crates/core", "crates/settings"]
resolver = "3"
'''

CORE_MANIFEST = '''
[package]
name = "outflow-core"
version = "0.1.0"
edition = "2024"
'''

EMPTY_LOCK = '''
version = 4

[[package]]
name = "outflow-core"
version = "0.1.0"
'''


def write_file(root: Path, relative_path: str, contents: str) -> Path:
    path = root / relative_path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")
    return path


def create_repo(
    root: Path,
    *,
    root_manifest: str = ROOT_MANIFEST,
    core_manifest: str | None = CORE_MANIFEST,
    lockfile: str | None = EMPTY_LOCK,
    policy: str = "# Policy\n\n**Status: PROPOSED — NOT APPROVED OR INTEGRATED.**\n",
    source: str = 'let comparison_key = entry.spoken.to_lowercase();\n',
) -> None:
    write_file(root, "Cargo.toml", root_manifest)
    if core_manifest is not None:
        write_file(root, "crates/core/Cargo.toml", core_manifest)
    write_file(root, "crates/settings/Cargo.toml", '[package]\nname = "outflow-settings"\nversion = "0.1.0"\n')
    if lockfile is not None:
        write_file(root, "Cargo.lock", lockfile)
    write_file(root, "docs/VOCABULARY_UNICODE_POLICY.md", policy)
    write_file(root, "crates/core/src/vocabulary.rs", source)


class VocabularyUnicodeRustDependencyAuditTests(unittest.TestCase):
    def test_workspace_dependency_inheritance_is_parsed_as_core_direct_dependency(self) -> None:
        root_manifest = ROOT_MANIFEST + '\n[workspace.dependencies]\nunicode-normalization = { version = "=0.1.24" }\n'
        core_manifest = CORE_MANIFEST + '\n[dependencies]\nunicode-normalization = { workspace = true }\n'
        lockfile = '''
version = 4

[[package]]
name = "outflow-core"
version = "0.1.0"
dependencies = ["unicode-normalization"]

[[package]]
name = "unicode-normalization"
version = "0.1.24"
'''
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root, root_manifest=root_manifest, core_manifest=core_manifest, lockfile=lockfile)

            report = audit_repository(root)

        finding = next(item for item in report.findings if item.candidate.name == "unicode-normalization")
        self.assertTrue(finding.direct_core)
        self.assertTrue(finding.lock_present)
        self.assertEqual(finding.locked_versions, ("0.1.24",))
        self.assertEqual(finding.core_relationship, "DIRECT")

    def test_core_runtime_dependencies_are_detected(self) -> None:
        core_manifest = CORE_MANIFEST + '\n[dependencies]\ncaseless = "=0.2.2"\n'
        lockfile = '''
version = 4

[[package]]
name = "outflow-core"
version = "0.1.0"
dependencies = ["caseless"]

[[package]]
name = "caseless"
version = "0.2.2"
'''
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root, core_manifest=core_manifest, lockfile=lockfile)
            report = audit_repository(root)

        finding = next(item for item in report.findings if item.candidate.name == "caseless")
        self.assertTrue(finding.direct_core)
        self.assertEqual(finding.core_relationship, "DIRECT")

    def test_target_specific_dependency_is_detected(self) -> None:
        core_manifest = CORE_MANIFEST + '''
[target.'cfg(unix)'.dependencies]
unicode-normalization = "=0.1.24"
'''
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root, core_manifest=core_manifest)
            report = audit_repository(root)

        finding = next(item for item in report.findings if item.candidate.name == "unicode-normalization")
        self.assertTrue(finding.direct_core)

    def test_lockfile_presence_and_locked_version_are_reported(self) -> None:
        lockfile = '''
version = 4

[[package]]
name = "outflow-core"
version = "0.1.0"
dependencies = ["unicode-casefold 0.2.0"]

[[package]]
name = "unicode-casefold"
version = "0.2.0"
'''
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root, lockfile=lockfile)
            report = audit_repository(root)

        finding = next(item for item in report.findings if item.candidate.name == "unicode-casefold")
        self.assertFalse(finding.direct_core)
        self.assertTrue(finding.lock_present)
        self.assertEqual(finding.locked_versions, ("0.2.0",))

    def test_transitive_dependency_is_not_misreported_as_direct(self) -> None:
        core_manifest = CORE_MANIFEST + '\n[dependencies]\nserde = "1"\n'
        lockfile = '''
version = 4

[[package]]
name = "outflow-core"
version = "0.1.0"
dependencies = ["serde"]

[[package]]
name = "serde"
version = "1.0.0"
dependencies = ["unicode-casefold 0.2.0"]

[[package]]
name = "unicode-casefold"
version = "0.2.0"
'''
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root, core_manifest=core_manifest, lockfile=lockfile)
            report = audit_repository(root)

        finding = next(item for item in report.findings if item.candidate.name == "unicode-casefold")
        self.assertFalse(finding.direct_core)
        self.assertTrue(finding.lock_present)
        self.assertEqual(finding.core_relationship, "TRANSITIVE")

    def test_candidate_missing_from_direct_dependencies_and_lockfile(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root)
            report = audit_repository(root)

        finding = next(item for item in report.findings if item.candidate.name == "caseless")
        self.assertFalse(finding.direct_core)
        self.assertFalse(finding.lock_present)
        self.assertEqual(finding.locked_versions, ())
        self.assertEqual(finding.core_relationship, "NOT PRESENT")

    def test_missing_core_manifest_is_reported_without_crashing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root, core_manifest=None)
            report = audit_repository(root)
            output = render_report(report)

        self.assertEqual(report.core_status, "MISSING")
        self.assertFalse(report.complete)
        self.assertIn("outflow-core manifest: MISSING", output)
        self.assertIn("Manifest crates/core/Cargo.toml: MISSING", output)

    def test_missing_workspace_manifest_is_reported_without_crashing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root)
            (root / "Cargo.toml").unlink()
            report = audit_repository(root)
            output = render_report(report)

        self.assertEqual(report.workspace_status, "MISSING")
        self.assertIn("Manifest Cargo.toml: MISSING", output)

    def test_missing_lockfile_is_reported_without_crashing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root, lockfile=None)
            report = audit_repository(root)
            output = render_report(report)

        self.assertEqual(report.lock_status, "MISSING")
        self.assertFalse(report.complete)
        self.assertIn("Cargo lockfile: MISSING", output)

    def test_malformed_workspace_core_and_lock_toml_are_reported(self) -> None:
        malformed_cases = (
            ("Cargo.toml", "[workspace\nmembers = []\n", "Manifest Cargo.toml: MALFORMED"),
            ("crates/core/Cargo.toml", "[package\nname = 'outflow-core'\n", "Manifest crates/core/Cargo.toml: MALFORMED"),
            ("Cargo.lock", "version = [\n", "Cargo lockfile: MALFORMED"),
        )
        for relative_path, malformed_text, expected in malformed_cases:
            with self.subTest(relative_path=relative_path):
                with tempfile.TemporaryDirectory() as temporary_directory:
                    root = Path(temporary_directory)
                    create_repo(root)
                    write_file(root, relative_path, malformed_text)
                    report = audit_repository(root)
                    output = render_report(report)
                self.assertIn(expected, output)
                self.assertFalse(report.complete)

    def test_unrelated_dependencies_do_not_become_candidate_findings(self) -> None:
        core_manifest = CORE_MANIFEST + '\n[dependencies]\nserde = "1"\nunicode-ident = "1"\n'
        lockfile = '''
version = 4

[[package]]
name = "outflow-core"
version = "0.1.0"
dependencies = ["serde", "unicode-ident"]

[[package]]
name = "serde"
version = "1.0.0"

[[package]]
name = "unicode-ident"
version = "1.0.0"
'''
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root, core_manifest=core_manifest, lockfile=lockfile)
            report = audit_repository(root)

        self.assertEqual(len(report.findings), 3)
        for finding in report.findings:
            self.assertFalse(finding.direct_core)
            self.assertFalse(finding.lock_present)
            self.assertEqual(finding.core_relationship, "NOT PRESENT")

    def test_audit_output_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root)
            first = render_report(audit_repository(root))
            second = render_report(audit_repository(root))

        self.assertEqual(first, second)

    def test_audit_is_read_only(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root)
            tracked = (
                root / "Cargo.toml",
                root / "crates/core/Cargo.toml",
                root / "Cargo.lock",
                root / "crates/core/src/vocabulary.rs",
            )
            before = {path: (path.read_bytes(), path.stat().st_mtime_ns) for path in tracked}

            audit_repository(root)

            after = {path: (path.read_bytes(), path.stat().st_mtime_ns) for path in tracked}

        self.assertEqual(before, after)

    def test_report_does_not_claim_d145_conformance_or_candidate_approval(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root)
            output = render_report(audit_repository(root))

        self.assertIn("D145 conformance: NOT ESTABLISHED BY THIS AUDIT", output)
        self.assertIn("Candidate approval: PENDING", output)
        self.assertIn("explicitly approved in project docs: NO", output)
        self.assertIn("Production matching policy: LOWERCASE-ONLY", output)

    def test_mention_of_approved_unicode_version_does_not_approve_crate(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root)
            write_file(
                root,
                "docs/OTHER.md",
                "Verify caseless and normalization tables use the approved Unicode version.\n",
            )
            finding = next(
                item
                for item in audit_repository(root).findings
                if item.candidate.name == "caseless"
            )

        self.assertFalse(finding.explicitly_approved)

    def test_explicit_candidate_approval_statement_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root)
            write_file(root, "docs/APPROVAL.md", "caseless 0.2.2: APPROVED\n")
            finding = next(
                item
                for item in audit_repository(root).findings
                if item.candidate.name == "caseless"
            )

        self.assertTrue(finding.explicitly_approved)
        self.assertEqual(finding.approval_reference, "docs/APPROVAL.md")

    def test_cli_reports_actual_repository_state_without_modifying_it(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            create_repo(root)
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                exit_code = main(["--repo", str(root)])

        self.assertEqual(exit_code, 0)
        self.assertIn("Workspace manifests: INSPECTED (3 parsed)", output.getvalue())
        self.assertIn("Cargo dependencies modified: NO", output.getvalue())


if __name__ == "__main__":
    unittest.main()
