#!/bin/bash
set -euo pipefail

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
  printf 'mic probe launch failed: %s\n' "$1" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "the development app runs only on macOS"
: "${OUTFLOW_MIC_PROBE_APP:?set OUTFLOW_MIC_PROBE_APP to the built .app bundle}"
[[ -d "$OUTFLOW_MIC_PROBE_APP/Contents/MacOS" ]] || fail "the development app bundle is missing"
[[ -x "$OUTFLOW_MIC_PROBE_APP/Contents/MacOS/OutflowMicProbe" ]] || fail "the app executable is missing"

APP_ARGUMENTS=()
FORWARDED_ARGUMENTS=()
VERIFICATION_ID=""
VERIFICATION_ID_SET=0
SYNTHETIC_COPY_DIAGNOSTIC=0
TIMED_FIXTURE_SET=0
while (($#)); do
  case "$1" in
    --verification-id)
      (($# >= 2)) || fail "--verification-id requires a value"
      ((VERIFICATION_ID_SET == 0)) || fail "--verification-id may be supplied only once"
      [[ "$2" =~ ^[A-Za-z0-9-]{1,32}$ ]] || \
        fail "--verification-id must be 1-32 ASCII letters, digits, or hyphens"
      VERIFICATION_ID="$2"
      VERIFICATION_ID_SET=1
      FORWARDED_ARGUMENTS+=(--verification-id "$VERIFICATION_ID")
      shift 2
      ;;
    --timed-fixture)
      (($# >= 2)) || fail "--timed-fixture requires a PCM16 mono WAV path"
      TIMED_FIXTURE_SET=1
      FORWARDED_ARGUMENTS+=(--timed-fixture "$2")
      shift 2
      ;;
    --fixture-seconds|--metrics-file|--auto-cancel-after-ms)
      (($# >= 2)) || fail "$1 requires a value"
      FORWARDED_ARGUMENTS+=("$1" "$2")
      shift 2
      ;;
    --auto-exit-after-final|--auto-exit-after-cancel)
      FORWARDED_ARGUMENTS+=("$1")
      shift
      ;;
    --synthetic-copy-diagnostic)
      ((SYNTHETIC_COPY_DIAGNOSTIC == 0)) || fail "--synthetic-copy-diagnostic may be supplied only once"
      SYNTHETIC_COPY_DIAGNOSTIC=1
      FORWARDED_ARGUMENTS+=("$1")
      shift
      ;;
    *)
      fail "unsupported option: $1"
      ;;
  esac
done

if ((SYNTHETIC_COPY_DIAGNOSTIC)); then
  ((VERIFICATION_ID_SET)) || fail "--synthetic-copy-diagnostic requires --verification-id"
  ((TIMED_FIXTURE_SET == 0)) || fail "synthetic copy diagnostic cannot be combined with --timed-fixture"
else
  : "${OUTFLOW_MIC_PROBE_MODEL_DIR:?set OUTFLOW_MIC_PROBE_MODEL_DIR to the locally authorized Small Streaming model directory}"
  python3 - "$REPOSITORY_ROOT" "$OUTFLOW_MIC_PROBE_MODEL_DIR" <<'PY'
from pathlib import Path
import sys

from scripts.run_native_asr_benchmark import BenchmarkError, verify_model_assets

repository_root = Path(sys.argv[1])
model_directory = Path(sys.argv[2])
manifest = repository_root / "evidence" / "mvp" / "prompt-79-model-assets.json"
try:
    result = verify_model_assets(manifest, model_directory)
except BenchmarkError as error:
    print(f"mic probe launch failed: local model verification failed ({error})", file=sys.stderr)
    raise SystemExit(1)
print(f"Verified {len(result['asset_sha256'])} local model files ({result['total_bytes']} bytes).")
PY
  APP_ARGUMENTS+=(--model-dir "$OUTFLOW_MIC_PROBE_MODEL_DIR")
fi
APP_ARGUMENTS+=("${FORWARDED_ARGUMENTS[@]}")

printf 'Requesting LaunchServices launch for app bundle: %s\n' "$OUTFLOW_MIC_PROBE_APP"
if ((VERIFICATION_ID_SET)); then
  printf 'Development verification ID: %s\n' "$VERIFICATION_ID"
else
  printf 'Development verification ID: not supplied\n'
fi
printf 'Confirm the visible window title and PID before interacting; an open success status alone does not verify the target window.\n'

# Launch through LaunchServices so macOS resolves the app bundle and its
# NSMicrophoneUsageDescription for the normal TCC permission prompt.
# A fresh process is required because LaunchServices otherwise may reuse a
# running probe and ignore this invocation's fixture/metrics arguments.
exec /usr/bin/open -n -a "$OUTFLOW_MIC_PROBE_APP" --args "${APP_ARGUMENTS[@]}"
