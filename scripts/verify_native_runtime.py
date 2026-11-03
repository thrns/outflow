#!/usr/bin/env python3
"""Verify the actual macOS dyld images used by a native benchmark helper."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Sequence

if __package__:
    from .run_native_asr_benchmark import BenchmarkError, verify_native_runtime
else:
    from run_native_asr_benchmark import BenchmarkError, verify_native_runtime


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Verify loaded Moonshine and ONNX Runtime binary identity on macOS."
    )
    parser.add_argument("--native-runner", type=Path, required=True)
    parser.add_argument("--moonshine-library", type=Path, required=True)
    parser.add_argument("--onnxruntime-library", type=Path, required=True)
    arguments = parser.parse_args(argv)
    try:
        result = verify_native_runtime(
            arguments.native_runner,
            arguments.moonshine_library,
            arguments.onnxruntime_library,
        )
    except BenchmarkError as error:
        print(f"native runtime identity failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps({"status": "VERIFIED", **result}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
