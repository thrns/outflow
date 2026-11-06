#!/bin/bash
set -euo pipefail

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fail() {
  printf 'FAIL actual AppKit integration test failure: %s\n' "$1" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "the isolated AppKit test runs only on macOS"
SDK_PATH="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null)" || fail "macOS SDK is unavailable"
TEST_DIR="$(mktemp -d "${TMPDIR:-/private/tmp}/outflow-appkit-pasteboard.XXXXXX")"
trap 'rm -rf "$TEST_DIR"' EXIT
RUNNER="$TEST_DIR/pasteboard_integration_test"
RESULT="$TEST_DIR/result.txt"
LAUNCH_LOG="$TEST_DIR/launch.log"

clang++ \
  -std=c++20 -O2 -fobjc-arc -Wall -Wextra -Wpedantic -Werror -pthread \
  -isysroot "$SDK_PATH" -mmacosx-version-min=26.0 \
  -DOUTFLOW_PASTEBOARD_INTEGRATION_STANDALONE \
  -I"$REPOSITORY_ROOT/dev/macos-mic-probe" \
  -I"$REPOSITORY_ROOT/dev/macos-mic-probe/tests" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/tests/pasteboard_integration_test.mm" \
  -framework AppKit -framework Foundation \
  -o "$RUNNER" || fail "standalone AppKit test compilation failed"
chmod 0755 "$RUNNER" || fail "standalone AppKit test executable could not be marked executable"
codesign --force --sign - "$RUNNER" >/dev/null || fail "standalone AppKit test signing failed"
codesign --verify --strict "$RUNNER" >/dev/null || fail "standalone AppKit test signature verification failed"

RUN_STATUS=0
"$RUNNER" "$RESULT" >"$LAUNCH_LOG" 2>&1 || RUN_STATUS=$?
if python3 "$REPOSITORY_ROOT/scripts/classify_macos_appkit_test.py" classify \
  --launch-status "$RUN_STATUS" \
  --result "$RESULT" \
  --launch-log "$LAUNCH_LOG"; then
  cat "$RESULT"
else
  CLASSIFICATION_STATUS=$?
  if [[ -f "$RESULT" ]]; then
    cat "$RESULT"
  fi
  exit "$CLASSIFICATION_STATUS"
fi
