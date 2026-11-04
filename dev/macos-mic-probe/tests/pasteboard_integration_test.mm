#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>

#include "pasteboard_integration_test.hpp"
#include "pasteboard_writer.hpp"

#include <array>
#include <fstream>
#include <string>
#include <string_view>

using namespace outflow::mic_probe;

namespace {

struct PasteboardReleaseGuard {
  NSPasteboard* __strong pasteboard;
  bool* release_called;

  ~PasteboardReleaseGuard() {
    [pasteboard releaseGlobally];
    *release_called = true;
  }
};

bool run_isolated_pasteboard_test(bool& release_called, std::string& failure) {
  @autoreleasepool {
    NSPasteboard* pasteboard = [NSPasteboard pasteboardWithUniqueName];
    if (pasteboard == nil) {
      failure = "unique pasteboard unavailable";
      return false;
    }
    PasteboardReleaseGuard release_guard{pasteboard, &release_called};

    // The unique board stays untouched until an explicit writer call below.
    // The test reads only this synthetic pasteboard, never generalPasteboard.
    if (pasteboard.pasteboardItems.count != 0 ||
        [pasteboard stringForType:NSPasteboardTypeString] != nil) {
      failure = "unique pasteboard was not initially empty";
      return false;
    }

    const std::array<std::string_view, 4> synthetic_values{
        "  ASCII text with exact spaces  ",
        "Unicode: Café — naïve façade.",
        "Emoji: 😀📝",
        "Line one.\nLine two!\n",
    };

    for (const auto text : synthetic_values) {
      // Each call is an explicit test action. Later writes must replace the
      // previous synthetic item rather than appending stale pasteboard items.
      NSString* expected = [[NSString alloc] initWithBytes:text.data()
                                                    length:text.size()
                                                  encoding:NSUTF8StringEncoding];
      const auto result = write_plain_text_to_pasteboard(pasteboard, expected);
      NSArray<NSPasteboardItem*>* items = pasteboard.pasteboardItems;
      NSString* actual = [pasteboard stringForType:NSPasteboardTypeString];
      if (result != PasteboardWriteResult::Succeeded || expected == nil ||
          items.count != 1 ||
          ![items[0].types containsObject:NSPasteboardTypeString] ||
          actual == nil || ![actual isEqualToString:expected]) {
        failure = "isolated plain-text replacement check failed";
        return false;
      }
    }
    return true;
  }
}

}  // namespace

namespace outflow::mic_probe {

int run_pasteboard_integration_test(const char* result_path) {
  if (result_path == nullptr || *result_path == '\0') return 2;

  bool release_called = false;
  std::string failure;
  const bool passed = run_isolated_pasteboard_test(release_called, failure);
  if (!release_called && failure.empty()) {
    failure = "unique pasteboard was not released";
  }

  std::ofstream result_file(result_path, std::ios::out | std::ios::trunc);
  if (!result_file) return 2;
  if (!passed || !release_called) {
    result_file << "FAIL " << failure << "; released="
                << (release_called ? "true" : "false") << '\n';
    return 1;
  }

  result_file << "PASS isolated AppKit pasteboard integration tests: 4 fixed "
                 "synthetic text forms; 4 explicit replacements; no implicit "
                 "write; unique pasteboard released\n";
  return result_file.good() ? 0 : 2;
}

}  // namespace outflow::mic_probe
