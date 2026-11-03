from __future__ import annotations

import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts.measure_process_tree_rss import (
    MeasurementError,
    ProcessInfo,
    _linux_process_snapshot,
    _sample_until,
    build_measurement,
    main,
    make_sample,
    parse_macos_ps_snapshot,
    select_process_tree,
)


def process(pid: int, ppid: int, rss: int, identity: str | None = None) -> ProcessInfo:
    return ProcessInfo(
        pid=pid,
        ppid=ppid,
        identity=identity or f"start:{pid}",
        name=f"process-{pid}",
        rss_bytes=rss,
    )


class ProcessTreeRssTests(unittest.TestCase):
    def test_macos_ps_parser_uses_bytes_and_start_identity(self) -> None:
        snapshot = parse_macos_ps_snapshot(
            "101 1 10 Mon Nov  3 11:30:00 2026 /usr/bin/python3\n"
        )

        self.assertEqual(snapshot[101].rss_bytes, 10 * 1024)
        self.assertEqual(snapshot[101].identity, "darwin-lstart:Mon Nov 3 11:30:00 2026")
        self.assertEqual(snapshot[101].name, "python3")

    def test_macos_parser_rejects_duplicate_or_malformed_pid_rows(self) -> None:
        row = "101 1 10 Mon Nov 3 11:30:00 2026 /usr/bin/python3"
        with self.assertRaises(MeasurementError):
            parse_macos_ps_snapshot(row + "\n" + row)
        with self.assertRaises(MeasurementError):
            parse_macos_ps_snapshot("not a process record")

    def test_linux_parser_reads_start_ticks_and_resident_pages(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            proc_root = Path(temporary)
            process_dir = proc_root / "42"
            process_dir.mkdir()
            stat_fields = ["S", "7"] + ["0"] * 18
            stat_fields[19] = "1234"
            (process_dir / "stat").write_text(
                "42 (worker (native)) " + " ".join(stat_fields), encoding="ascii"
            )
            (process_dir / "statm").write_text("200 50 0 0 0 0 0", encoding="ascii")
            with mock.patch("scripts.measure_process_tree_rss.os.sysconf", return_value=4096):
                snapshot = _linux_process_snapshot(proc_root)

        self.assertEqual(snapshot[42].ppid, 7)
        self.assertEqual(snapshot[42].identity, "linux-start-ticks:1234")
        self.assertEqual(snapshot[42].rss_bytes, 50 * 4096)
        self.assertEqual(snapshot[42].name, "worker (native)")

    def test_parent_child_and_multiple_descendant_levels_sum_once(self) -> None:
        snapshot = {
            10: process(10, 1, 1000),
            11: process(11, 10, 2000),
            12: process(12, 11, 3000),
            13: process(13, 10, 4000),
            99: process(99, 1, 8000),
        }

        selected = select_process_tree(snapshot, 10)
        sample = make_sample(
            snapshot,
            10,
            timestamp_utc="2026-11-03T20:00:00.000Z",
            elapsed_ms=0,
        )

        self.assertEqual([item.pid for item in selected], [10, 11, 12, 13])
        self.assertEqual(sample.tree_rss_bytes, 10_000)

    def test_process_startup_and_termination_are_reflected_between_snapshots(self) -> None:
        before = {1: process(1, 0, 100)}
        during = {1: process(1, 0, 100), 2: process(2, 1, 250)}
        after = {1: process(1, 0, 100)}

        self.assertEqual([p.pid for p in select_process_tree(before, 1)], [1])
        self.assertEqual([p.pid for p in select_process_tree(during, 1)], [1, 2])
        self.assertEqual([p.pid for p in select_process_tree(after, 1)], [1])

    def test_pid_reuse_for_root_fails_measurement(self) -> None:
        snapshots = iter(
            [
                {1: process(1, 0, 100, "start:old")},
                {1: process(1, 0, 100, "start:new")},
            ]
        )
        with mock.patch("scripts.measure_process_tree_rss.time.sleep", return_value=None):
            with self.assertRaisesRegex(MeasurementError, "identity changed"):
                _sample_until(
                    1,
                    interval_ms=20,
                    duration_seconds=1,
                    launched_process=None,
                    snapshot_provider=lambda: next(snapshots),
                )

    def test_missing_permission_or_failed_snapshot_propagates(self) -> None:
        def unavailable() -> dict[int, ProcessInfo]:
            raise MeasurementError("process table cannot be read")

        with self.assertRaisesRegex(MeasurementError, "cannot be read"):
            _sample_until(
                1,
                interval_ms=20,
                duration_seconds=1,
                launched_process=None,
                snapshot_provider=unavailable,
            )

    def test_disappeared_attached_target_fails_instead_of_claiming_a_peak(self) -> None:
        snapshots = iter([{1: process(1, 0, 100)}, {}])
        with mock.patch("scripts.measure_process_tree_rss.time.sleep", return_value=None):
            with self.assertRaisesRegex(MeasurementError, "disappeared"):
                _sample_until(
                    1,
                    interval_ms=20,
                    duration_seconds=1,
                    launched_process=None,
                    snapshot_provider=lambda: next(snapshots),
                )

    def test_short_lived_process_limit_is_written_to_evidence(self) -> None:
        samples = [
            make_sample(
                {1: process(1, 0, 100)},
                1,
                timestamp_utc="2026-11-03T20:00:00.000Z",
                elapsed_ms=0,
            ),
            make_sample(
                {1: process(1, 0, 100)},
                1,
                timestamp_utc="2026-11-03T20:00:00.100Z",
                elapsed_ms=100,
            ),
        ]
        result = build_measurement(
            samples,
            root_pid=1,
            root_identity="start:1",
            interval_ms=100,
            target_exit_code=0,
            max_observed_gap_ms=100,
            platform_name="linux",
        )

        self.assertTrue(any("shorter-lived" in item for item in result["limitations"]))

    def test_peak_detection_preserves_per_process_observations_and_bytes(self) -> None:
        samples = [
            make_sample(
                {1: process(1, 0, 1000), 2: process(2, 1, 500)},
                1,
                timestamp_utc="2026-11-03T20:00:00.000Z",
                elapsed_ms=0,
            ),
            make_sample(
                {1: process(1, 0, 2000), 2: process(2, 1, 1500)},
                1,
                timestamp_utc="2026-11-03T20:00:00.100Z",
                elapsed_ms=100,
            ),
        ]
        result = build_measurement(
            samples,
            root_pid=1,
            root_identity="start:1",
            interval_ms=100,
            target_exit_code=0,
            max_observed_gap_ms=100,
            platform_name="linux",
        )

        self.assertEqual(result["tree_peak_rss_bytes"], 3500)
        self.assertEqual(result["sample_count"], 2)
        self.assertEqual(
            {entry["pid"]: entry["peak_rss_bytes"] for entry in result["process_peak_rss"]},
            {1: 2000, 2: 1500},
        )

    def test_reused_child_pid_has_separate_per_process_identity(self) -> None:
        samples = [
            make_sample(
                {1: process(1, 0, 100), 2: process(2, 1, 200, "child:old")},
                1,
                timestamp_utc="2026-11-03T20:00:00.000Z",
                elapsed_ms=0,
            ),
            make_sample(
                {1: process(1, 0, 100), 2: process(2, 1, 300, "child:new")},
                1,
                timestamp_utc="2026-11-03T20:00:00.100Z",
                elapsed_ms=100,
            ),
        ]
        result = build_measurement(
            samples,
            root_pid=1,
            root_identity="start:1",
            interval_ms=100,
            target_exit_code=0,
            max_observed_gap_ms=100,
            platform_name="linux",
        )

        child_records = [entry for entry in result["process_peak_rss"] if entry["pid"] == 2]
        self.assertEqual({entry["identity"] for entry in child_records}, {"child:old", "child:new"})

    def test_output_does_not_record_environment_or_command_arguments(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            main(["--help"])
        self.assertEqual(raised.exception.code, 0)

    def test_short_command_that_cannot_be_sampled_twice_is_not_passed(self) -> None:
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            result = main(["--interval-ms", "20", "--", "/usr/bin/true"])

        self.assertEqual(result, 2)
        self.assertIn("measurement failed", stderr.getvalue())

    def test_command_mode_passes_sampling_gap_into_sanitized_record(self) -> None:
        samples = [
            make_sample(
                {7: process(7, 1, 100)},
                7,
                timestamp_utc="2026-11-03T20:00:00.000Z",
                elapsed_ms=0,
            ),
            make_sample(
                {7: process(7, 1, 200)},
                7,
                timestamp_utc="2026-11-03T20:00:00.100Z",
                elapsed_ms=100,
            ),
        ]
        launched = mock.Mock(pid=7, returncode=0)
        stdout = io.StringIO()
        with mock.patch("scripts.measure_process_tree_rss.subprocess.Popen", return_value=launched), mock.patch(
            "scripts.measure_process_tree_rss._sample_until",
            return_value=(samples, "start:7", 0, 100),
        ) as sample_until, contextlib.redirect_stdout(stdout):
            result = main(["--interval-ms", "100", "--", "/usr/bin/example"])

        self.assertEqual(result, 0)
        self.assertEqual(sample_until.call_count, 1)
        record = json.loads(stdout.getvalue())
        self.assertEqual(record["max_observed_gap_ms"], 100)
        self.assertEqual(record["tree_peak_rss_bytes"], 200)


if __name__ == "__main__":
    unittest.main()
