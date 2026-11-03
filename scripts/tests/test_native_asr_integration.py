from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from scripts.validate_measured_asr_results import validate_measured_results


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
REQUIRED_ENVIRONMENT = {
    "OUTFLOW_BENCHMARK_NATIVE_RUNNER": "native batch runner",
    "OUTFLOW_BENCHMARK_MODEL_DIR": "locally authorized Small model directory",
    "OUTFLOW_BENCHMARK_SOURCE_DIR": "clean pinned Moonshine source checkout",
    "OUTFLOW_BENCHMARK_MOONSHINE_LIBRARY": "built libmoonshine binary",
    "OUTFLOW_BENCHMARK_ONNXRUNTIME_LIBRARY": "ONNX Runtime binary",
    "OUTFLOW_BENCHMARK_ASSET_ROOT": "directory of hash-verified model files",
    "OUTFLOW_BENCHMARK_FIXTURE_MANIFEST": "benchmark fixture manifest",
    "OUTFLOW_BENCHMARK_FIXTURE_ROOT": "fixture manifest repository root",
}


class NativeAsrIntegrationTests(unittest.TestCase):
    def test_verified_fixture_runs_through_real_pinned_native_asr(self) -> None:
        missing = [name for name in REQUIRED_ENVIRONMENT if not os.environ.get(name)]
        manifest_path = Path(os.environ.get(
            "OUTFLOW_BENCHMARK_FIXTURE_MANIFEST",
            REPOSITORY_ROOT / "docs" / "benchmark-fixtures.json",
        ))
        if not missing:
            pass
        else:
            self.skipTest(
                "real native ASR integration requires a verified, licensed fixture corpus and "
                f"configured local artifacts; missing: {', '.join(missing)}"
            )
        if not manifest_path.is_file():
            self.skipTest("configured real fixture manifest is unavailable")
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError):
            self.fail("configured real fixture manifest is unreadable")
        if not any(item.get("status") == "ready" for item in manifest.get("fixtures", [])):
            self.skipTest("no integrity-verified, human-reviewed real fixture is ready")

        with tempfile.TemporaryDirectory() as temporary:
            output_path = Path(temporary) / "measured-native-asr.json"
            command = [
                sys.executable,
                str(REPOSITORY_ROOT / "scripts" / "run_native_asr_benchmark.py"),
                "--native-runner", os.environ["OUTFLOW_BENCHMARK_NATIVE_RUNNER"],
                "--model-dir", os.environ["OUTFLOW_BENCHMARK_MODEL_DIR"],
                "--source-dir", os.environ["OUTFLOW_BENCHMARK_SOURCE_DIR"],
                "--moonshine-library", os.environ["OUTFLOW_BENCHMARK_MOONSHINE_LIBRARY"],
                "--onnxruntime-library", os.environ["OUTFLOW_BENCHMARK_ONNXRUNTIME_LIBRARY"],
                "--asset-root", os.environ["OUTFLOW_BENCHMARK_ASSET_ROOT"],
                "--fixture-manifest", str(manifest_path),
                "--repository-root", os.environ["OUTFLOW_BENCHMARK_FIXTURE_ROOT"],
                "--output", str(output_path),
            ]
            completed = subprocess.run(command, capture_output=True, text=True, timeout=3600, check=False)
            self.assertEqual(completed.returncode, 0, "native benchmark failed without exposing content")
            self.assertTrue(output_path.is_file())
            result = json.loads(output_path.read_text(encoding="utf-8"))
            summary = validate_measured_results(result)

        self.assertGreater(summary["fixtures"], 0)
        self.assertEqual(result["result_type"], "measured_native_asr")


if __name__ == "__main__":
    unittest.main()
