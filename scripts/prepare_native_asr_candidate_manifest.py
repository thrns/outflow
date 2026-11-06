#!/usr/bin/env python3
"""Hash a caller-provided local candidate bundle against pinned source metadata.

This tool has no network or model-download path. It records local SHA-256 values
only after the pinned source size and CRC32C metadata match, and always leaves
acquisition approval unrecorded for a maintainer to decide separately.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import sys
from typing import Sequence

if __package__:
    from .run_native_asr_benchmark import (
        BenchmarkError,
        MODEL_CANDIDATES,
        prepare_candidate_asset_manifest,
        verify_source,
    )
else:
    from run_native_asr_benchmark import (
        BenchmarkError,
        MODEL_CANDIDATES,
        prepare_candidate_asset_manifest,
        verify_source,
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Prepare a local model manifest from pinned Moonshine source; does not download or infer."
    )
    parser.add_argument("--candidate", choices=tuple(MODEL_CANDIDATES), required=True)
    parser.add_argument("--source-dir", type=Path, required=True)
    parser.add_argument("--asset-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args(argv)
    try:
        verify_source(arguments.source_dir, arguments.candidate)
        result = prepare_candidate_asset_manifest(
            arguments.source_dir,
            arguments.candidate,
            arguments.asset_root,
            arguments.output,
        )
    except (BenchmarkError, OSError, ValueError, TypeError) as error:
        print(f"candidate manifest preparation failed: {error}", file=sys.stderr)
        return 2
    print(
        "LOCAL_CANDIDATE_ASSET_MANIFEST: READY "
        f"({result['candidate']} architecture {result['model_architecture']}, "
        f"{len(result['assets'])} source-verified files)"
    )
    print("Acquisition approval: NOT_RECORDED; real Tiny inference remains blocked until approved evidence is supplied.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
