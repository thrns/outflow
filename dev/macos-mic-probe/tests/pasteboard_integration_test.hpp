#pragma once

namespace outflow::mic_probe {

/// Runs fixed synthetic AppKit pasteboard checks against a unique test board.
/// Returns 0 on PASS, 1 on test failure, or 2 when the sanitized result cannot
/// be written. The result file contains no dictated or clipboard content.
int run_pasteboard_integration_test(const char* result_path);

}  // namespace outflow::mic_probe
