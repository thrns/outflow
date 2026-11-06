#!/bin/bash
set -euo pipefail

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXPECTED_SOURCE_REVISION="234f60faa0eb388b01cdf7e60aca232af37aefda"
EXPECTED_MOONSHINE_SHA256="42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5"
EXPECTED_ORT_SHA256="480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4"

fail() {
  printf 'FAIL actual native test or implementation failure: %s\n' "$1" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "native mic probe tests run only on macOS"
[[ $# == 0 || ( $# == 1 && "$1" == "--real-fixture" ) ]] || fail "supported option: --real-fixture"
RUN_REAL_FIXTURE="${1:-}"
: "${MOONSHINE_SOURCE_DIR:?set MOONSHINE_SOURCE_DIR to the pinned Moonshine source tree}"
: "${MOONSHINE_SOURCE_REVISION:?set MOONSHINE_SOURCE_REVISION to the verified source revision}"
: "${MOONSHINE_LIBRARY:?set MOONSHINE_LIBRARY to the verified libmoonshine.dylib}"
: "${MOONSHINE_ORT_LIBRARY:?set MOONSHINE_ORT_LIBRARY to the verified ONNX Runtime dylib}"

[[ "$MOONSHINE_SOURCE_REVISION" == "$EXPECTED_SOURCE_REVISION" ]] || fail "source revision differs from the verified development build"
[[ -f "$MOONSHINE_SOURCE_DIR/core/moonshine-c-api.h" ]] || fail "Moonshine C API header is missing"
[[ -f "$MOONSHINE_LIBRARY" && -f "$MOONSHINE_ORT_LIBRARY" ]] || fail "a verified native runtime library is missing"
[[ "$(shasum -a 256 "$MOONSHINE_LIBRARY" | awk '{print $1}')" == "$EXPECTED_MOONSHINE_SHA256" ]] || fail "Moonshine library identity mismatch"
[[ "$(shasum -a 256 "$MOONSHINE_ORT_LIBRARY" | awk '{print $1}')" == "$EXPECTED_ORT_SHA256" ]] || fail "ONNX Runtime identity mismatch"

SDK_PATH="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null)" || fail "macOS SDK is unavailable"
if [[ -n "${OUTFLOW_MIC_PROBE_TEST_BUILD_DIR:-}" ]]; then
  [[ ! -e "$OUTFLOW_MIC_PROBE_TEST_BUILD_DIR" ]] || fail "the explicit test build directory already exists"
  mkdir -p "$OUTFLOW_MIC_PROBE_TEST_BUILD_DIR"
  TEST_DIR="$OUTFLOW_MIC_PROBE_TEST_BUILD_DIR"
else
  TEST_DIR="$(mktemp -d "${TMPDIR:-/private/tmp}/outflow-mic-probe-tests.XXXXXX")"
  trap 'rm -rf "$TEST_DIR"' EXIT
fi
GENERATED_DIR="$TEST_DIR/generated"
mkdir -p "$GENERATED_DIR"
python3 "$REPOSITORY_ROOT/scripts/generate_mic_probe_asset_header.py" \
  --manifest "$REPOSITORY_ROOT/evidence/mvp/prompt-79-model-assets.json" \
  --output "$GENERATED_DIR/verified_model_assets.hpp" >/dev/null || fail "asset table generation failed"

COMMON_FLAGS=(-std=c++20 -O2 -Wall -Wextra -Wpedantic -Werror -pthread -isysroot "$SDK_PATH" -mmacosx-version-min=26.0)
INCLUDES=(-I"$REPOSITORY_ROOT/dev/macos-mic-probe" -I"$GENERATED_DIR" -I"$MOONSHINE_SOURCE_DIR/core")
LIBRARIES=("$MOONSHINE_LIBRARY" -Wl,-rpath,"$(dirname "$MOONSHINE_LIBRARY")" -Wl,-rpath,"$(dirname "$MOONSHINE_ORT_LIBRARY")")

clang++ "${COMMON_FLAGS[@]}" "${INCLUDES[@]}" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/tests/core_test.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/audio_pipeline.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/session_state.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/permission_policy.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/wav_fixture.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/fixture_producer.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/session_worker.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/model_verifier.cpp" \
  "${LIBRARIES[@]}" -o "$TEST_DIR/core_test" || fail "core test compilation failed"
"$TEST_DIR/core_test" || fail "native core test executable returned failure"

clang++ "${COMMON_FLAGS[@]}" "${INCLUDES[@]}" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/tests/worker_test.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/audio_pipeline.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/session_state.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/session_worker.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/model_verifier.cpp" \
  "${LIBRARIES[@]}" -o "$TEST_DIR/worker_test" || fail "worker test compilation failed"
"$TEST_DIR/worker_test" || fail "native worker test executable returned failure"
printf 'PASS native unit and worker tests\n'

if [[ "$RUN_REAL_FIXTURE" == "--real-fixture" ]]; then
  : "${OUTFLOW_MIC_PROBE_MODEL_DIR:?set OUTFLOW_MIC_PROBE_MODEL_DIR to the locally authorized model directory}"
  : "${OUTFLOW_MIC_PROBE_FIXTURE_WAV:?set OUTFLOW_MIC_PROBE_FIXTURE_WAV to the local licensed PCM16 mono WAV fixture}"
  python3 - "$REPOSITORY_ROOT" "$OUTFLOW_MIC_PROBE_MODEL_DIR" <<'PY'
from pathlib import Path
import sys

from scripts.run_native_asr_benchmark import BenchmarkError, verify_model_assets

try:
    result = verify_model_assets(
        Path(sys.argv[1]) / "evidence" / "mvp" / "prompt-79-model-assets.json",
        Path(sys.argv[2]),
    )
except BenchmarkError as error:
    print(f"mic probe tests failed: local model verification failed ({error})", file=sys.stderr)
    raise SystemExit(1)
print(f"Verified {len(result['asset_sha256'])} local model files ({result['total_bytes']} bytes).")
PY
  clang++ "${COMMON_FLAGS[@]}" "${INCLUDES[@]}" \
    "$REPOSITORY_ROOT/dev/macos-mic-probe/tests/real_fixture_harness.cpp" \
    "$REPOSITORY_ROOT/dev/macos-mic-probe/audio_pipeline.cpp" \
    "$REPOSITORY_ROOT/dev/macos-mic-probe/fixture_producer.cpp" \
    "$REPOSITORY_ROOT/dev/macos-mic-probe/wav_fixture.cpp" \
    "$REPOSITORY_ROOT/dev/macos-mic-probe/session_worker.cpp" \
    "$REPOSITORY_ROOT/dev/macos-mic-probe/model_verifier.cpp" \
    "${LIBRARIES[@]}" -o "$TEST_DIR/real_fixture_harness" || fail "real fixture harness compilation failed"
  FIXTURE_RUNS="${OUTFLOW_MIC_PROBE_FIXTURE_RUNS:-1}"
  [[ "$FIXTURE_RUNS" =~ ^[0-9]+$ ]] && (( FIXTURE_RUNS >= 1 && FIXTURE_RUNS <= 20 )) || fail "OUTFLOW_MIC_PROBE_FIXTURE_RUNS must be 1–20"
  if [[ -n "${OUTFLOW_MIC_PROBE_PROFILE_DIR:-}" ]]; then
    [[ ! -e "$OUTFLOW_MIC_PROBE_PROFILE_DIR" ]] || fail "the explicit profile directory already exists"
    mkdir -p "$OUTFLOW_MIC_PROBE_PROFILE_DIR"
  fi
  for ((run = 1; run <= FIXTURE_RUNS; ++run)); do
    if [[ -n "${OUTFLOW_MIC_PROBE_PROFILE_DIR:-}" ]]; then
      python3 "$REPOSITORY_ROOT/scripts/measure_process_tree_rss.py" \
        --interval-ms 100 \
        --output "$OUTFLOW_MIC_PROBE_PROFILE_DIR/run-${run}.json" \
        -- "$TEST_DIR/real_fixture_harness" "$OUTFLOW_MIC_PROBE_MODEL_DIR" \
        "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" "${OUTFLOW_MIC_PROBE_FIXTURE_SECONDS:-12}"
    else
      "$TEST_DIR/real_fixture_harness" "$OUTFLOW_MIC_PROBE_MODEL_DIR" \
        "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" "${OUTFLOW_MIC_PROBE_FIXTURE_SECONDS:-12}" || \
        fail "real ASR fixture harness returned failure"
    fi
  done
  printf 'PASS real ASR fixture tests\n'
else
  printf 'NOT RUN real ASR fixture tests (pass --real-fixture to execute)\n'
fi

PASTEBOARD_APP_BUILD_DIR="$TEST_DIR/pasteboard-probe-build"
OUTFLOW_MIC_PROBE_BUILD_DIR="$PASTEBOARD_APP_BUILD_DIR" \
  "$REPOSITORY_ROOT/scripts/build_macos_mic_probe.sh" >/dev/null || \
  fail "development probe build for the AppKit test failed"
PASTEBOARD_APP="$PASTEBOARD_APP_BUILD_DIR/OutflowMicProbe.app"

reject_invalid_verification_id() {
  local description="$1"
  shift
  local exit_status=0
  "$PASTEBOARD_APP/Contents/MacOS/OutflowMicProbe" "$@" >/dev/null 2>&1 || \
    exit_status=$?
  [[ "$exit_status" == 2 ]] || \
    fail "native app did not reject ${description} verification ID (exit ${exit_status})"
}

reject_invalid_verification_id "missing-value" --verification-id
reject_invalid_verification_id "empty" --verification-id ""
reject_invalid_verification_id "overlong" --verification-id "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
reject_invalid_verification_id "control-character" --verification-id $'line\nbreak'
reject_invalid_verification_id "repeated" \
  --verification-id first --verification-id second
printf 'PASS native app rejected 5 malformed verification ID launches without crashing.\n'

reject_invalid_launch() {
  local description="$1"
  shift
  local exit_status=0
  "$PASTEBOARD_APP/Contents/MacOS/OutflowMicProbe" "$@" >/dev/null 2>&1 || \
    exit_status=$?
  [[ "$exit_status" == 2 ]] || \
    fail "native app did not reject ${description} synthetic diagnostic launch (exit ${exit_status})"
}

reject_invalid_launch "missing verification ID" --synthetic-copy-diagnostic
reject_invalid_launch "invalid verification ID" \
  --verification-id "bad id" --synthetic-copy-diagnostic
reject_invalid_launch "timed fixture combination" \
  --verification-id 79J-INVALID --synthetic-copy-diagnostic \
  --timed-fixture /private/tmp/unused.wav
reject_invalid_launch "microphone capture combination" \
  --verification-id 79J-INVALID --synthetic-copy-diagnostic --microphone-capture
printf 'PASS native app rejected 4 invalid synthetic diagnostic launches without opening AppKit.\n'

python3 "$REPOSITORY_ROOT/scripts/classify_macos_appkit_test.py" \
  validate-bundle --bundle "$PASTEBOARD_APP" || fail "AppKit test bundle failed structural or code-signature validation"
PASTEBOARD_RESULT="$TEST_DIR/pasteboard-integration-result.txt"
PASTEBOARD_LAUNCH_LOG="$TEST_DIR/pasteboard-launch.log"
PASTEBOARD_OPEN_STATUS=0
if /usr/bin/open -n -W -a "$PASTEBOARD_APP" --args \
  --isolated-pasteboard-test "$PASTEBOARD_RESULT" >"$PASTEBOARD_LAUNCH_LOG" 2>&1; then
  PASTEBOARD_OPEN_STATUS=0
else
  PASTEBOARD_OPEN_STATUS=$?
fi
if python3 "$REPOSITORY_ROOT/scripts/classify_macos_appkit_test.py" classify \
  --launch-status "$PASTEBOARD_OPEN_STATUS" \
  --result "$PASTEBOARD_RESULT" \
  --launch-log "$PASTEBOARD_LAUNCH_LOG"; then
  cat "$PASTEBOARD_RESULT"
else
  APPKIT_CLASSIFICATION_STATUS=$?
  if [[ "$APPKIT_CLASSIFICATION_STATUS" == 2 ]]; then
    printf 'AppKit test was not counted as a pass. Run scripts/run_macos_appkit_pasteboard_test.sh from Terminal.app in the logged-in desktop session.\n' >&2
    exit 2
  fi
  exit 1
fi
