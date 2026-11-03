from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from scripts.generate_mic_probe_asset_header import render_header


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
ASSET_MANIFEST = REPOSITORY_ROOT / "evidence" / "mvp" / "prompt-79-model-assets.json"


class GenerateMicProbeAssetHeaderTests(unittest.TestCase):
    def test_header_uses_exact_verified_manifest_names_and_hashes(self) -> None:
        manifest = json.loads(ASSET_MANIFEST.read_text(encoding="utf-8"))
        header = render_header(ASSET_MANIFEST)
        self.assertIn("std::array<ModelAsset, 8>", header)
        for asset in manifest["assets"]:
            self.assertIn(f'{{"{asset["file_path"]}", "{asset["sha256"]}"}}', header)

    def test_duplicate_names_and_unsafe_names_are_rejected(self) -> None:
        manifest = json.loads(ASSET_MANIFEST.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "assets.json"
            duplicated = json.loads(json.dumps(manifest))
            duplicated["assets"][1]["file_path"] = duplicated["assets"][0]["file_path"]
            path.write_text(json.dumps(duplicated), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "invalid"):
                render_header(path)

            unsafe = json.loads(json.dumps(manifest))
            unsafe["assets"][0]["file_path"] = "../adapter.ort"
            path.write_text(json.dumps(unsafe), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "invalid"):
                render_header(path)

    def test_invalid_hash_and_wrong_asset_count_are_rejected(self) -> None:
        manifest = json.loads(ASSET_MANIFEST.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "assets.json"
            manifest["assets"][0]["sha256"] = "not-a-sha256"
            path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "invalid"):
                render_header(path)

            manifest = json.loads(ASSET_MANIFEST.read_text(encoding="utf-8"))
            manifest["assets"].pop()
            path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "exactly eight"):
                render_header(path)


if __name__ == "__main__":
    unittest.main()
