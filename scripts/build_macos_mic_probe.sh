#!/bin/bash
set -euo pipefail

REPOSITORY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXPECTED_SOURCE_REVISION="234f60faa0eb388b01cdf7e60aca232af37aefda"
EXPECTED_MOONSHINE_SHA256="42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5"
EXPECTED_ORT_SHA256="480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4"

fail() {
  printf 'mic probe build failed: %s\n' "$1" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "this development prototype builds only on macOS"
: "${MOONSHINE_SOURCE_DIR:?set MOONSHINE_SOURCE_DIR to the pinned Moonshine source tree}"
: "${MOONSHINE_SOURCE_REVISION:?set MOONSHINE_SOURCE_REVISION to the verified commit}"
: "${MOONSHINE_LIBRARY:?set MOONSHINE_LIBRARY to the verified libmoonshine.dylib}"
: "${MOONSHINE_ORT_LIBRARY:?set MOONSHINE_ORT_LIBRARY to the verified ONNX Runtime dylib}"

[[ "$MOONSHINE_SOURCE_REVISION" == "$EXPECTED_SOURCE_REVISION" ]] || fail "Moonshine source revision does not match the approved development build"
[[ -f "$MOONSHINE_SOURCE_DIR/core/moonshine-c-api.h" ]] || fail "pinned Moonshine C API header is missing"
[[ -f "$MOONSHINE_LIBRARY" && -f "$MOONSHINE_ORT_LIBRARY" ]] || fail "a required verified native library is missing"
rg -q '#define[[:space:]]+MOONSHINE_HEADER_VERSION[[:space:]]+\(30000\)' "$MOONSHINE_SOURCE_DIR/core/moonshine-c-api.h" || fail "Moonshine C API header version is not 30000"
rg -q '#define[[:space:]]+MOONSHINE_MODEL_ARCH_SMALL_STREAMING[[:space:]]+\(4\)' "$MOONSHINE_SOURCE_DIR/core/moonshine-c-api.h" || fail "Small Streaming architecture constant is not 4"

MOONSHINE_HASH="$(shasum -a 256 "$MOONSHINE_LIBRARY" | awk '{print $1}')"
ORT_HASH="$(shasum -a 256 "$MOONSHINE_ORT_LIBRARY" | awk '{print $1}')"
[[ "$MOONSHINE_HASH" == "$EXPECTED_MOONSHINE_SHA256" ]] || fail "Moonshine library SHA-256 differs from Prompt 79D verified runtime"
[[ "$ORT_HASH" == "$EXPECTED_ORT_SHA256" ]] || fail "ONNX Runtime SHA-256 differs from Prompt 79D verified runtime"
[[ " $(lipo -archs "$MOONSHINE_LIBRARY") " == *" arm64 "* ]] || fail "Moonshine library has no arm64 slice"
[[ " $(lipo -archs "$MOONSHINE_ORT_LIBRARY") " == *" arm64 "* ]] || fail "ONNX Runtime library has no arm64 slice"
otool -L "$MOONSHINE_LIBRARY" | rg -q 'libonnxruntime\.1\.23\.2\.dylib' || fail "Moonshine library does not declare the verified ONNX Runtime dependency"

SDK_PATH="$(xcrun --sdk macosx --show-sdk-path 2>/dev/null)" || fail "macOS SDK is unavailable; install Xcode Command Line Tools"
BUILD_DIR="${OUTFLOW_MIC_PROBE_BUILD_DIR:-}"
if [[ -z "$BUILD_DIR" ]]; then
  BUILD_DIR="$(mktemp -d "${TMPDIR:-/private/tmp}/outflow-mic-probe-build.XXXXXX")"
else
  [[ ! -e "$BUILD_DIR/OutflowMicProbe.app" ]] || fail "the selected build directory already contains an app; choose a fresh directory"
  mkdir -p "$BUILD_DIR"
fi
GENERATED_DIR="$BUILD_DIR/generated"
mkdir -p "$GENERATED_DIR"
python3 "$REPOSITORY_ROOT/scripts/generate_mic_probe_asset_header.py" \
  --manifest "$REPOSITORY_ROOT/evidence/mvp/prompt-79-model-assets.json" \
  --output "$GENERATED_DIR/verified_model_assets.hpp" >/dev/null || fail "model hash header generation failed"

APP_BUNDLE="$BUILD_DIR/OutflowMicProbe.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
mkdir -p "$APP_CONTENTS/MacOS" "$APP_CONTENTS/Resources"
cp "$REPOSITORY_ROOT/dev/macos-mic-probe/Info.plist" "$APP_CONTENTS/Info.plist"
printf 'APPL????' > "$APP_CONTENTS/PkgInfo"

clang++ \
  -std=c++20 -O2 -fobjc-arc -isysroot "$SDK_PATH" -mmacosx-version-min=26.0 \
  -Wall -Wextra -Wpedantic -Werror -pthread \
  -I"$REPOSITORY_ROOT/dev/macos-mic-probe" \
  -I"$GENERATED_DIR" \
  -I"$MOONSHINE_SOURCE_DIR/core" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/app.mm" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/macos_capture.mm" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/audio_pipeline.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/session_state.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/permission_policy.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/wav_fixture.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/fixture_producer.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/model_verifier.cpp" \
  "$REPOSITORY_ROOT/dev/macos-mic-probe/session_worker.cpp" \
  "$MOONSHINE_LIBRARY" \
  -Wl,-rpath,"$(dirname "$MOONSHINE_LIBRARY")" \
  -Wl,-rpath,"$(dirname "$MOONSHINE_ORT_LIBRARY")" \
  -framework AppKit -framework AVFoundation -framework AudioToolbox \
  -framework CoreAudio -framework Foundation \
  -o "$APP_CONTENTS/MacOS/OutflowMicProbe" || fail "native prototype compilation failed"

codesign --force --sign - "$APP_BUNDLE" >/dev/null || fail "local development app signing failed"
python3 "$REPOSITORY_ROOT/scripts/verify_native_runtime.py" \
  --native-runner "$APP_CONTENTS/MacOS/OutflowMicProbe" \
  --moonshine-library "$MOONSHINE_LIBRARY" \
  --onnxruntime-library "$MOONSHINE_ORT_LIBRARY" >/dev/null || fail "loaded native runtime identity did not match the pinned binaries"

printf 'Development app built: %s\n' "$APP_BUNDLE"
printf 'Set OUTFLOW_MIC_PROBE_APP=%s\n' "$APP_BUNDLE"
