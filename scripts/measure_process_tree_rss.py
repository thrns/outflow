#!/usr/bin/env python3
"""Sample a process and its currently visible descendants without reading content.

The sampler uses ``ps`` on macOS and ``/proc`` on Linux. It records RSS in
bytes, a timestamped per-process snapshot, process start identities, and the
sum-of-RSS peak. RSS intentionally double-counts shared pages. A 100 ms poll
can miss short-lived processes; this is a measured process tree, not complete
application certification or a substitute for platform-native footprint tools.

Use command mode to launch a target without a shell, or attach mode to sample a
PID for a fixed duration. Command arguments and environment variables are
never recorded. Process names are reduced to their executable basename.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
from dataclasses import asdict, dataclass
from datetime import UTC, datetime
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from typing import Callable, Mapping, Sequence


DEFAULT_INTERVAL_MS = 100
SCHEMA_VERSION = 1
MAX_TRUSTED_GAP_MULTIPLIER = 10


class MeasurementError(RuntimeError):
    """A process snapshot or measurement could not be trusted."""


@dataclass(frozen=True)
class ProcessInfo:
    """Identity and resident-set size for one process at a snapshot instant."""

    pid: int
    ppid: int
    identity: str
    name: str
    rss_bytes: int


@dataclass(frozen=True)
class Sample:
    """One complete process-tree observation."""

    timestamp_utc: str
    elapsed_ms: float
    tree_rss_bytes: int
    processes: tuple[ProcessInfo, ...]


def parse_macos_ps_snapshot(output: str) -> dict[int, ProcessInfo]:
    """Parse ``ps -axo pid=,ppid=,rss=,lstart=,comm=`` output.

    macOS reports ``rss`` in KiB. The five ``lstart`` fields provide a
    best-effort process start identity with one-second precision.
    """

    processes: dict[int, ProcessInfo] = {}
    for line in output.splitlines():
        if not line.strip():
            continue
        parts = line.split(maxsplit=8)
        if len(parts) != 9:
            raise MeasurementError("macOS process snapshot has an unsupported row format")
        try:
            pid, ppid, rss_kib = (int(parts[index]) for index in range(3))
        except ValueError as error:
            raise MeasurementError("macOS process snapshot contains invalid numeric fields") from error
        if pid <= 0 or ppid < 0 or rss_kib < 0 or pid in processes:
            raise MeasurementError("macOS process snapshot contains invalid or duplicate identities")
        start_identity = "darwin-lstart:" + " ".join(parts[3:8])
        executable = parts[8].strip()
        name = Path(executable.rstrip("/")).name or "unknown"
        processes[pid] = ProcessInfo(
            pid=pid,
            ppid=ppid,
            identity=start_identity,
            name=name,
            rss_bytes=rss_kib * 1024,
        )
    if not processes:
        raise MeasurementError("macOS process snapshot is empty")
    return processes


def _linux_process_snapshot(proc_root: Path = Path("/proc")) -> dict[int, ProcessInfo]:
    """Read Linux process identities and resident pages from ``/proc``."""

    try:
        page_size = os.sysconf("SC_PAGE_SIZE")
    except (OSError, ValueError) as error:
        raise MeasurementError("Linux page size is unavailable") from error
    if not isinstance(page_size, int) or page_size <= 0:
        raise MeasurementError("Linux page size is invalid")

    processes: dict[int, ProcessInfo] = {}
    try:
        entries = tuple(proc_root.iterdir())
    except OSError as error:
        raise MeasurementError("Linux process table cannot be enumerated") from error

    for entry in entries:
        if not entry.name.isdecimal():
            continue
        try:
            stat_text = (entry / "stat").read_text(encoding="ascii")
            statm_fields = (entry / "statm").read_text(encoding="ascii").split()
        except (OSError, UnicodeError):
            # Exiting processes and protected unrelated processes can vanish
            # between directory enumeration and the two reads.
            continue
        open_paren = stat_text.find("(")
        close_paren = stat_text.rfind(")")
        if open_paren < 0 or close_paren <= open_paren:
            raise MeasurementError("Linux process snapshot has an unsupported stat row")
        name = stat_text[open_paren + 1 : close_paren]
        fields_after_comm = stat_text[close_paren + 1 :].split()
        if len(fields_after_comm) <= 19 or len(statm_fields) < 2:
            raise MeasurementError("Linux process snapshot is missing identity or RSS fields")
        try:
            pid = int(entry.name)
            ppid = int(fields_after_comm[1])
            start_ticks = int(fields_after_comm[19])
            resident_pages = int(statm_fields[1])
        except ValueError as error:
            raise MeasurementError("Linux process snapshot contains invalid numeric fields") from error
        if pid <= 0 or ppid < 0 or start_ticks < 0 or resident_pages < 0:
            raise MeasurementError("Linux process snapshot contains an invalid process record")
        processes[pid] = ProcessInfo(
            pid=pid,
            ppid=ppid,
            identity=f"linux-start-ticks:{start_ticks}",
            name=name,
            rss_bytes=resident_pages * page_size,
        )
    if not processes:
        raise MeasurementError("Linux process snapshot is empty")
    return processes


def get_process_snapshot() -> dict[int, ProcessInfo]:
    """Return an OS-native process table with RSS represented in bytes."""

    if sys.platform == "darwin":
        try:
            completed = subprocess.run(
                ["/bin/ps", "-axo", "pid=,ppid=,rss=,lstart=,comm="],
                check=True,
                capture_output=True,
                text=True,
                timeout=5,
            )
        except (OSError, subprocess.SubprocessError) as error:
            raise MeasurementError("macOS process table could not be sampled") from error
        return parse_macos_ps_snapshot(completed.stdout)
    if sys.platform.startswith("linux"):
        return _linux_process_snapshot()
    raise MeasurementError("process-tree RSS sampling supports macOS and Linux only")


def select_process_tree(
    processes: Mapping[int, ProcessInfo], root_pid: int
) -> tuple[ProcessInfo, ...]:
    """Return the root and all descendants present in one process snapshot.

    PID is unique within a single OS process table. Sorting makes both evidence
    and tests deterministic. A process that starts and exits between samples
    cannot be discovered by polling and is reported as a measurement limit.
    """

    if root_pid not in processes:
        raise MeasurementError("target process is absent from the process snapshot")
    children: dict[int, list[int]] = defaultdict(list)
    for process in processes.values():
        if process.pid != process.ppid:
            children[process.ppid].append(process.pid)

    selected: list[ProcessInfo] = []
    pending = [root_pid]
    seen: set[int] = set()
    while pending:
        pid = pending.pop()
        if pid in seen:
            continue
        seen.add(pid)
        process = processes.get(pid)
        if process is None:
            continue
        selected.append(process)
        pending.extend(reversed(sorted(children.get(pid, ()))))
    return tuple(sorted(selected, key=lambda process: process.pid))


def make_sample(
    processes: Mapping[int, ProcessInfo],
    root_pid: int,
    *,
    timestamp_utc: str,
    elapsed_ms: float,
) -> Sample:
    """Aggregate RSS once per PID and preserve each process observation."""

    tree = select_process_tree(processes, root_pid)
    if len({process.pid for process in tree}) != len(tree):
        raise MeasurementError("process snapshot would double-count a PID")
    return Sample(
        timestamp_utc=timestamp_utc,
        elapsed_ms=elapsed_ms,
        tree_rss_bytes=sum(process.rss_bytes for process in tree),
        processes=tree,
    )


def _utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def _sample_until(
    root_pid: int,
    *,
    interval_ms: int,
    duration_seconds: float | None,
    launched_process: subprocess.Popen[bytes] | None,
    snapshot_provider: Callable[[], dict[int, ProcessInfo]] = get_process_snapshot,
) -> tuple[list[Sample], str, int | None, int]:
    """Collect samples until the launched command exits or the duration elapses."""

    start = time.monotonic()
    samples: list[Sample] = []
    root_identity: str | None = None
    max_gap_ms = 0.0
    next_deadline = start
    while True:
        now = time.monotonic()
        if samples and now - next_deadline > (interval_ms / 1000) * MAX_TRUSTED_GAP_MULTIPLIER:
            raise MeasurementError("sampling gap exceeded the trusted interval limit")

        process_table = snapshot_provider()
        root = process_table.get(root_pid)
        if root is None:
            if launched_process is not None and launched_process.poll() is not None and samples:
                break
            raise MeasurementError("target process disappeared during measurement")
        if root_identity is None:
            root_identity = root.identity
        elif root.identity != root_identity:
            raise MeasurementError("target PID identity changed during measurement")

        timestamp = _utc_now()
        sample = make_sample(
            process_table,
            root_pid,
            timestamp_utc=timestamp,
            elapsed_ms=(now - start) * 1000,
        )
        if samples:
            gap_ms = sample.elapsed_ms - samples[-1].elapsed_ms
            max_gap_ms = max(max_gap_ms, gap_ms)
        samples.append(sample)

        if launched_process is not None:
            if launched_process.poll() is not None:
                break
        elif duration_seconds is not None and now - start >= duration_seconds:
            break

        next_deadline += interval_ms / 1000
        delay = next_deadline - time.monotonic()
        if delay > 0:
            time.sleep(delay)
        else:
            # Do not take a burst of catch-up samples after scheduler delays.
            next_deadline = time.monotonic()

    if len(samples) < 2:
        raise MeasurementError("fewer than two process-tree samples were collected")
    if root_identity is None:
        raise MeasurementError("target process identity was not established")
    return samples, root_identity, launched_process.returncode if launched_process else None, round(max_gap_ms)


def build_measurement(
    samples: Sequence[Sample],
    *,
    root_pid: int,
    root_identity: str,
    interval_ms: int,
    target_exit_code: int | None,
    max_observed_gap_ms: int,
    platform_name: str | None = None,
) -> dict[str, object]:
    """Create a sanitized schema-v1 measurement document."""

    if not samples:
        raise MeasurementError("no process-tree samples were collected")
    peak = max(samples, key=lambda sample: sample.tree_rss_bytes)
    root = next((p for p in samples[0].processes if p.pid == root_pid), None)
    if root is None or root.identity != root_identity:
        raise MeasurementError("target identity is inconsistent across snapshots")
    per_process: dict[tuple[int, str], int] = {}
    for sample in samples:
        for process in sample.processes:
            key = process.pid, process.identity
            per_process[key] = max(per_process.get(key, 0), process.rss_bytes)
    ordered_process_peaks = [
        {"pid": pid, "identity": identity, "peak_rss_bytes": peak_bytes}
        for (pid, identity), peak_bytes in sorted(per_process.items())
    ]
    elapsed_ms = samples[-1].elapsed_ms - samples[0].elapsed_ms
    gaps = [
        round(samples[index].elapsed_ms - samples[index - 1].elapsed_ms)
        for index in range(1, len(samples))
    ]
    return {
        "schema_version": SCHEMA_VERSION,
        "measurement_type": "process_tree_rss",
        "platform": platform_name or sys.platform,
        "root": {"pid": root_pid, "identity": root_identity, "name": root.name},
        "sampling_interval_requested_ms": interval_ms,
        "sample_count": len(samples),
        "duration_ms": round(elapsed_ms, 3),
        "max_observed_gap_ms": max(gaps, default=max_observed_gap_ms),
        "tree_peak_rss_bytes": peak.tree_rss_bytes,
        "tree_peak_timestamp_utc": peak.timestamp_utc,
        "process_peak_rss": ordered_process_peaks,
        "samples": [
            {
                "timestamp_utc": sample.timestamp_utc,
                "elapsed_ms": sample.elapsed_ms,
                "tree_rss_bytes": sample.tree_rss_bytes,
                "processes": [asdict(process) for process in sample.processes],
            }
            for sample in samples
        ],
        "target_exit_code": target_exit_code,
        "additional_macos_memory_metric": {
            "status": "not_collected",
            "reason": "No low-overhead physical-footprint source was verified at the 100 ms sampling cadence.",
        }
        if (platform_name or sys.platform) == "darwin"
        else None,
        "limitations": [
            "RSS is summed across processes and may double-count shared pages.",
            "Processes shorter-lived than the polling interval may be missed.",
            "Only descendants visible in each sampled process table are included.",
            "This measurement covers the launched process tree, not a complete Outflow application.",
        ],
    }


def _write_measurement(document: dict[str, object], output: Path | None) -> None:
    """Write one JSON record, refusing to replace an existing evidence file."""

    serialized = json.dumps(document, indent=2, sort_keys=True) + "\n"
    if output is None:
        sys.stdout.write(serialized)
        return
    try:
        with output.open("x", encoding="utf-8") as destination:
            destination.write(serialized)
    except FileExistsError as error:
        raise MeasurementError("output file already exists; choose a new evidence path") from error
    except OSError as error:
        raise MeasurementError("measurement output could not be written") from error


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Sample process-tree RSS on macOS or Linux without recording command arguments."
    )
    parser.add_argument("--pid", type=int, help="attach to an existing process")
    parser.add_argument("--duration-seconds", type=float, help="required with --pid")
    parser.add_argument("--interval-ms", type=int, default=DEFAULT_INTERVAL_MS)
    parser.add_argument("--output", type=Path, help="new JSON evidence path; defaults to stdout")
    parser.add_argument("command", nargs=argparse.REMAINDER, help="command and arguments after --")
    arguments = parser.parse_args(argv)

    command = arguments.command
    if command and command[0] == "--":
        command = command[1:]
    command_mode = bool(command)
    if (arguments.pid is None) == (not command_mode):
        parser.error("choose either --pid with --duration-seconds or a command after --")
    if arguments.interval_ms < 20 or arguments.interval_ms > 5000:
        parser.error("--interval-ms must be between 20 and 5000")
    if arguments.pid is not None and arguments.pid <= 0:
        parser.error("--pid must be positive")
    if arguments.pid is not None:
        if arguments.duration_seconds is None or arguments.duration_seconds <= 0:
            parser.error("--duration-seconds must be positive with --pid")
    elif arguments.duration_seconds is not None:
        parser.error("--duration-seconds is only valid with --pid")
    if not command_mode and command:
        parser.error("command mode requires a command after --")

    launched: subprocess.Popen[bytes] | None = None
    try:
        if command_mode:
            launched = subprocess.Popen(command, close_fds=True)
            root_pid = launched.pid
        else:
            root_pid = arguments.pid
        samples, root_identity, exit_code, max_gap_ms = _sample_until(
            root_pid,
            interval_ms=arguments.interval_ms,
            duration_seconds=arguments.duration_seconds,
            launched_process=launched,
        )
        document = build_measurement(
            samples,
            root_pid=root_pid,
            root_identity=root_identity,
            interval_ms=arguments.interval_ms,
            target_exit_code=exit_code,
            max_observed_gap_ms=max_gap_ms,
        )
        _write_measurement(document, arguments.output)
        if exit_code not in {None, 0}:
            return exit_code if 1 <= exit_code <= 125 else 1
        return 0
    except (MeasurementError, OSError, ValueError) as error:
        if launched is not None and launched.poll() is None:
            launched.send_signal(signal.SIGTERM)
            try:
                launched.wait(timeout=2)
            except subprocess.TimeoutExpired:
                launched.kill()
                launched.wait()
        print(f"process-tree measurement failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
