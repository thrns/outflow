#!/usr/bin/env python3
"""Generate the development probe's embedded model-hash table from evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Sequence


EXPECTED_ASSET_COUNT = 8
SAFE_NAME = re.compile(r"[A-Za-z0-9_.-]{1,128}\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")


def render_header(manifest_path: Path) -> str:
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError("model asset manifest could not be read") from error
    assets = manifest.get("assets") if isinstance(manifest, dict) else None
    if not isinstance(assets, list) or len(assets) != EXPECTED_ASSET_COUNT:
        raise ValueError("model asset manifest does not contain exactly eight assets")
    records: list[tuple[str, str]] = []
    seen: set[str] = set()
    for asset in assets:
        if not isinstance(asset, dict):
            raise ValueError("model asset record is malformed")
        name = asset.get("file_path")
        digest = asset.get("sha256")
        if (
            asset.get("status") != "file-verified"
            or not isinstance(name, str)
            or SAFE_NAME.fullmatch(name) is None
            or name in seen
            or not isinstance(digest, str)
            or SHA256.fullmatch(digest) is None
        ):
            raise ValueError("model asset record is invalid")
        seen.add(name)
        records.append((name, digest))
    records.sort()
    rows = ",\n".join(f'    ModelAsset{{"{name}", "{digest}"}}' for name, digest in records)
    return (
        "#pragma once\n\n"
        "#include <array>\n"
        "#include <string_view>\n\n"
        "namespace outflow::mic_probe {\n"
        "struct ModelAsset { std::string_view filename; std::string_view sha256; };\n"
        f"inline constexpr std::array<ModelAsset, {EXPECTED_ASSET_COUNT}> kVerifiedModelAssets{{{{\n"
        f"{rows}\n"
        "}};\n"
        "}  // namespace outflow::mic_probe\n"
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        header = render_header(args.manifest)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("x", encoding="utf-8") as output:
            output.write(header)
    except (OSError, ValueError) as error:
        print(f"model hash header generation failed: {error}", file=sys.stderr)
        return 1
    print("model hash header generated from the verified eight-file manifest")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
