#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>

#include "pasteboard_integration_test.hpp"
#include "pasteboard_writer.hpp"

#include <array>
#include <cstring>
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

    const std::array<std::string_view, 8> synthetic_values{
        "Plain ASCII text.",
        "  Whitespace with a tab\t and trailing spaces  ",
        "Unicode: Café — naïve façade.",
        "Emoji: 😀📝",
        "Composed: \xC3\xA9",
        "Decomposed: e\xCC\x81",
        "LF\nCRLF\r\nCR\r",
        "Line one.\n\n  Indented line \nLast line.\n",
    };

    std::size_t value_index = 0;
    for (const auto text : synthetic_values) {
      // Each call is an explicit test action. Later writes must replace the
      // previous synthetic item rather than appending stale pasteboard items.
      // Compare the returned UTF-8 bytes with the source bytes directly:
      // NSString equality alone could miss a canonical Unicode change.
      NSString* expected = string_from_utf8(text);
      const auto result = write_plain_text_to_pasteboard(pasteboard, expected);
      NSArray<NSPasteboardItem*>* items = pasteboard.pasteboardItems;
      NSString* actual = [pasteboard stringForType:NSPasteboardTypeString];
      NSData* actual_utf8 = [actual
          dataUsingEncoding:NSUTF8StringEncoding
          allowLossyConversion:NO];
      const bool exact_utf8_bytes =
          actual_utf8 != nil && actual_utf8.length == text.size() &&
          std::memcmp(actual_utf8.bytes, text.data(), text.size()) == 0;
      if (result != PasteboardWriteResult::Succeeded || expected == nil ||
          items.count != 1 ||
          ![items[0].types containsObject:NSPasteboardTypeString] ||
          actual == nil || !exact_utf8_bytes) {
        failure = "isolated plain-text replacement check failed at synthetic form " +
                  std::to_string(value_index + 1);
        return false;
      }
      ++value_index;
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

  result_file << "PASS isolated AppKit pasteboard integration tests: 8 fixed "
                 "synthetic UTF-8 forms; 8 explicit replacements; exact bytes; no implicit "
                 "write; unique pasteboard released\n";
  return result_file.good() ? 0 : 2;
}

}  // namespace outflow::mic_probe

#if defined(OUTFLOW_PASTEBOARD_INTEGRATION_STANDALONE)
int main(int argc, char** argv) {
  NSString* result_path = nil;
  if (argc == 2 && argv[1] != nullptr && argv[1][0] != '\0') {
    result_path = [NSString stringWithUTF8String:argv[1]];
  } else if (argc == 1) {
    // The isolated test can also run as a short-lived desktop app when
    // LaunchServices is required to provide an AppKit pasteboard context.
    result_path = [[NSBundle mainBundle]
        objectForInfoDictionaryKey:@"OutflowPasteboardTestResultPath"];
  }
  if (result_path == nil) return 2;
  (void)[NSApplication sharedApplication];
  return outflow::mic_probe::run_pasteboard_integration_test(
      result_path.fileSystemRepresentation);
}
#endif
