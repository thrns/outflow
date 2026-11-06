#!/usr/bin/env python3
"""Sample a process and its currently visible descendants without reading content.

The sampler uses target-scoped Apple ``libproc`` calls on macOS and ``/proc``
on Linux. It records RSS in bytes, a timestamped per-process snapshot, process
start identities, and the sum-of-RSS peak. Unreadable target descendants block
a complete measurement. RSS intentionally double-counts shared pages. A 100 ms poll
can miss short-lived processes; this is a measured process tree, not complete
application certification or a substitute for platform-native footprint tools.

Use command mode to launch a target without a shell, or attach mode to sample a
PID for a fixed duration. Command arguments and environment variables are
never recorded. Process names are reduced to their executable basename.
"""

from __future__ import annotations

import argparse
import ctypes
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
LAUNCH_SNAPSHOT_RETRY_SECONDS = 1.0
DARWIN_SNAPSHOT_RETRY_BACKOFF_SECONDS = 0.01
PROC_PIDTASKALLINFO = 2


class MeasurementError(RuntimeError):
    """A process snapshot or measurement could not be trusted."""


class _ProcessDisappeared(MeasurementError):
    """A process exited between child enumeration and its identity/RSS query."""


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


class ProcessSnapshot(dict[int, ProcessInfo]):
    """One OS snapshot with its collector identity and explicit completeness."""

    def __init__(
        self,
        processes: Mapping[int, ProcessInfo],
        *,
        backend: str,
        gaps: Sequence[str] = (),
    ) -> None:
        super().__init__(processes)
        self.backend = backend
        self.gaps = tuple(gaps)


class _ProcBsdInfo(ctypes.Structure):
    _fields_ = [
        ("flags", ctypes.c_uint32), ("status", ctypes.c_uint32),
        ("xstatus", ctypes.c_uint32), ("pid", ctypes.c_uint32),
        ("ppid", ctypes.c_uint32), ("uid", ctypes.c_uint32),
        ("gid", ctypes.c_uint32), ("ruid", ctypes.c_uint32),
        ("rgid", ctypes.c_uint32), ("svuid", ctypes.c_uint32),
        ("svgid", ctypes.c_uint32), ("reserved", ctypes.c_uint32),
        ("comm", ctypes.c_char * 16), ("name", ctypes.c_char * 32),
        ("nfiles", ctypes.c_uint32), ("pgid", ctypes.c_uint32),
        ("pjobc", ctypes.c_uint32), ("tty_dev", ctypes.c_uint32),
        ("tty_pgid", ctypes.c_uint32), ("nice", ctypes.c_int32),
        ("start_sec", ctypes.c_uint64), ("start_usec", ctypes.c_uint64),
    ]


class _ProcTaskInfo(ctypes.Structure):
    _fields_ = [
        ("virtual_size", ctypes.c_uint64), ("resident_size", ctypes.c_uint64),
        ("total_user", ctypes.c_uint64), ("total_system", ctypes.c_uint64),
        ("threads_user", ctypes.c_uint64), ("threads_system", ctypes.c_uint64),
        ("policy", ctypes.c_int32), ("faults", ctypes.c_int32),
        ("pageins", ctypes.c_int32), ("cow_faults", ctypes.c_int32),
        ("messages_sent", ctypes.c_int32), ("messages_received", ctypes.c_int32),
        ("syscalls_mach", ctypes.c_int32), ("syscalls_unix", ctypes.c_int32),
        ("context_switches", ctypes.c_int32), ("thread_count", ctypes.c_int32),
        ("running_threads", ctypes.c_int32), ("priority", ctypes.c_int32),
    ]


class _ProcTaskAllInfo(ctypes.Structure):
    _fields_ = [("bsd", _ProcBsdInfo), ("task", _ProcTaskInfo)]


def _darwin_libproc_snapshot_once(root_pid: int) -> ProcessSnapshot:
    """Read only a target and its descendants with Apple's public libproc API.

    PROC_PIDTASKALLINFO returns parent PID, start identity, and current RSS in
    one per-process record. proc_listchildpids walks only the authorized target
    tree, avoiding a global process listing. An unreadable or inconsistent
    descendant blocks the sample instead of disappearing from the sum.
    """

    try:
        libproc = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
    except OSError as error:
        raise MeasurementError("macOS libproc is unavailable; process-tree RSS is blocked") from error
    list_children = libproc.proc_listchildpids
    list_children.argtypes = [ctypes.c_int32, ctypes.c_void_p, ctypes.c_int]
    list_children.restype = ctypes.c_int
    pid_info = libproc.proc_pidinfo
    pid_info.argtypes = [ctypes.c_int32, ctypes.c_int, ctypes.c_uint64, ctypes.c_void_p, ctypes.c_int]
    pid_info.restype = ctypes.c_int

    backend = "macos-libproc-proc_listchildpids+PROC_PIDTASKALLINFO"
    process_table: dict[int, ProcessInfo] = {}
    pending: list[tuple[int, int | None]] = [(root_pid, None)]

    while pending:
        pid, expected_parent = pending.pop()
        if pid in process_table:
            raise MeasurementError("macOS libproc returned a duplicate descendant PID")
        record = _ProcTaskAllInfo()
        ctypes.set_errno(0)
        copied = pid_info(
            pid, PROC_PIDTASKALLINFO, 0, ctypes.byref(record), ctypes.sizeof(record)
        )
        if copied != ctypes.sizeof(record):
            error_number = ctypes.get_errno()
            detail = "permission denied" if error_number == 1 else "process disappeared or became inaccessible"
            subject = "target root" if expected_parent is None else "target descendant"
            message = (
                f"macOS libproc {subject} info unavailable ({copied}/{ctypes.sizeof(record)} bytes, errno {error_number}); {detail}; complete process-tree attribution is blocked"
            )
            if error_number == 3:  # ESRCH: the enumerated process exited during this snapshot.
                raise _ProcessDisappeared(message)
            raise MeasurementError(message)
        bsd = record.bsd
        if bsd.pid != pid or (expected_parent is not None and bsd.ppid != expected_parent):
            raise MeasurementError("macOS libproc descendant parent identity changed during sampling")
        name_bytes = bytes(bsd.name).split(b"\0", 1)[0] or bytes(bsd.comm).split(b"\0", 1)[0]
        name = name_bytes.decode("utf-8", errors="replace") or "unknown"
        identity = f"darwin-start:{bsd.start_sec}.{bsd.start_usec:06d}"
        process_table[pid] = ProcessInfo(
            pid=pid,
            ppid=bsd.ppid,
            identity=identity,
            name=Path(name.rstrip("/")).name or "unknown",
            rss_bytes=record.task.resident_size,
        )

        capacity = 128
        while True:
            child_buffer = (ctypes.c_int32 * capacity)()
            ctypes.set_errno(0)
            # Apple libproc returns a PID count here, unlike proc_listpidspath,
            # whose result is a byte count. Equality with capacity is ambiguous
            # (exact fit or truncation), so grow and retry to prove completeness.
            child_count = list_children(pid, ctypes.byref(child_buffer), ctypes.sizeof(child_buffer))
            error_number = ctypes.get_errno()
            if child_count < 0 or (child_count == 0 and error_number != 0):
                detail = "permission denied" if error_number == 1 else "children could not be enumerated"
                message = f"macOS libproc {detail}; complete process-tree attribution is blocked"
                if error_number == 3:  # ESRCH: the parent exited during enumeration.
                    raise _ProcessDisappeared(message)
                raise MeasurementError(message)
            if child_count < capacity:
                break
            capacity *= 2
            if capacity > 1_048_576:
                raise MeasurementError("macOS libproc descendant list exceeds the supported bound")
        for index in range(child_count):
            child_pid = int(child_buffer[index])
            if child_pid <= 0:
                raise MeasurementError("macOS libproc returned an invalid descendant PID")
            pending.append((child_pid, pid))

    return ProcessSnapshot(process_table, backend=backend)


def _darwin_libproc_snapshot(root_pid: int) -> ProcessSnapshot:
    """Retry transient PID disappearance while keeping permission failures closed."""

    for attempt in range(3):
        try:
            return _darwin_libproc_snapshot_once(root_pid)
        except _ProcessDisappeared:
            if attempt == 2:
                raise MeasurementError(
                    "macOS process tree changed during three consecutive snapshots; complete attribution is blocked"
                ) from None
            time.sleep(DARWIN_SNAPSHOT_RETRY_BACKOFF_SECONDS)
    raise MeasurementError("macOS process tree snapshot retry limit was exceeded")


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


def _linux_process_snapshot(proc_root: Path = Path("/proc")) -> ProcessSnapshot:
    """Read Linux process identities and resident pages from ``/proc``."""

    try:
        page_size = os.sysconf("SC_PAGE_SIZE")
    except (OSError, ValueError) as error:
        raise MeasurementError("Linux page size is unavailable") from error
    if not isinstance(page_size, int) or page_size <= 0:
        raise MeasurementError("Linux page size is invalid")

    processes: dict[int, ProcessInfo] = {}
    snapshot_gaps: list[str] = []
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
        except FileNotFoundError:
            # A process exited between enumeration and sampling.
            continue
        except PermissionError:
            snapshot_gaps.append("an enumerated process was inaccessible")
            continue
        except (OSError, UnicodeError):
            snapshot_gaps.append("an enumerated process snapshot could not be read")
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
    return ProcessSnapshot(
        processes, backend="linux-procfs-stat+statm", gaps=snapshot_gaps
    )


def get_process_snapshot(root_pid: int | None = None) -> ProcessSnapshot:
    """Return an OS-native target-tree snapshot with RSS represented in bytes."""

    if sys.platform == "darwin":
        if root_pid is None:
            raise MeasurementError("macOS libproc sampling requires the target root PID")
        return _darwin_libproc_snapshot(root_pid)
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
    snapshot_provider: Callable[[], Mapping[int, ProcessInfo]] | None = None,
) -> tuple[list[Sample], str, int | None, int]:
    """Collect samples until the launched command exits or the duration elapses."""

    start = time.monotonic()
    samples: list[Sample] = []
    root_identity: str | None = None
    max_gap_ms = 0.0
    previous_identities: dict[int, str] = {}
    next_deadline = start
    provider = snapshot_provider or (lambda: get_process_snapshot(root_pid))
    while True:
        now = time.monotonic()
        if samples and now - next_deadline > (interval_ms / 1000) * MAX_TRUSTED_GAP_MULTIPLIER:
            raise MeasurementError("sampling gap exceeded the trusted interval limit")

        try:
            process_table = provider()
        except MeasurementError:
            # A target that exits between polls can no longer be queried. The
            # prior samples are complete; stop at the launched root's lifetime
            # boundary. Any inaccessible process while it is still running fails.
            if samples and launched_process is not None and launched_process.poll() is not None:
                break
            # Popen returns before exec/process-info is always visible to libproc.
            # Retry only the initial launched target briefly; attached targets and
            # any later snapshot failure still fail closed immediately.
            if (
                samples
                or launched_process is None
                or time.monotonic() - start >= LAUNCH_SNAPSHOT_RETRY_SECONDS
            ):
                raise
            time.sleep(0.01)
            continue
        gaps = getattr(process_table, "gaps", ())
        if gaps:
            raise MeasurementError("process-tree snapshot reported an uncollected process; measurement is incomplete")
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
        for process in sample.processes:
            previous_identity = previous_identities.get(process.pid)
            if previous_identity is not None and previous_identity != process.identity:
                raise MeasurementError("descendant PID identity changed during measurement")
            previous_identities[process.pid] = process.identity
        if samples:
            gap_ms = sample.elapsed_ms - samples[-1].elapsed_ms
            max_gap_ms = max(max_gap_ms, gap_ms)
        if not samples:
            next_deadline = now
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
    measurement_backend: str | None = None,
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
    measurement_gaps = [
        {"after_sample": index, "observed_interval_ms": gap_ms}
        for index, gap_ms in enumerate(gaps, start=1)
        if gap_ms > interval_ms * 1.5
    ]
    return {
        "schema_version": SCHEMA_VERSION,
        "measurement_type": "process_tree_rss",
        "platform": platform_name or sys.platform,
        "measurement_backend": measurement_backend or (
            "macos-libproc-proc_listchildpids+PROC_PIDTASKALLINFO"
            if (platform_name or sys.platform) == "darwin"
            else "linux-procfs-stat+statm"
        ),
        "snapshot_completeness": "complete",
        "measurement_gaps": measurement_gaps,
        "root": {"pid": root_pid, "identity": root_identity, "name": root.name},
        "sampling_interval_requested_ms": interval_ms,
        "sample_count": len(samples),
        "duration_ms": round(elapsed_ms, 3),
        "max_observed_gap_ms": max(gaps, default=max_observed_gap_ms),
        "sampling_gap_count": len(measurement_gaps),
        "sampling_gaps_ms": gaps,
        "process_tree_attribution": "root-plus-recursively-enumerated-direct-descendants",
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
            "A macOS libproc or Linux procfs query failure blocks a complete result; inaccessible target descendants are not silently omitted.",
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
        status = "BLOCKED" if sys.platform == "darwin" else "FAILED"
        print(f"{status} process-tree RSS measurement: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
