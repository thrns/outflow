#pragma once

#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>

#include <string_view>

namespace outflow::mic_probe {

/// Result of one explicit plain-text pasteboard replacement attempt.
///
/// `clearContents` returns a change count rather than a success flag. The
/// preparation failure is therefore reported only if AppKit raises while
/// preparing the pasteboard; a zero or nonzero change count is not interpreted
/// as success or failure. See Apple's NSPasteboard writing contract.
enum class PasteboardWriteResult {
  Succeeded,
  InvalidUtf8,
  PasteboardUnavailable,
  PreparationFailed,
  WriteFailed,
};

/// Converts exact UTF-8 bytes to an NSString without touching any pasteboard.
inline NSString* string_from_utf8(std::string_view text) {
  return [[NSString alloc] initWithBytes:text.data()
                                  length:text.size()
                                encoding:NSUTF8StringEncoding];
}

/// Replaces a pasteboard's contents with one exact UTF-8 plain-text value.
///
/// Callers must convert text with `string_from_utf8` and enforce session
/// eligibility before invoking this function. Passing nil text reports an
/// invalid UTF-8 conversion; passing nil pasteboard reports unavailability.
/// This performs no reads and no retries. Clearing and writing are separate
/// AppKit operations, so a failed write may leave the previous contents lost.
inline PasteboardWriteResult write_plain_text_to_pasteboard(
    NSPasteboard* pasteboard, NSString* value) {
  if (value == nil) return PasteboardWriteResult::InvalidUtf8;
  if (pasteboard == nil) return PasteboardWriteResult::PasteboardUnavailable;

  @try {
    // Apple documents clearContents as the first step when providing new data.
    // Its change-count return value does not indicate a preparation failure.
    (void)[pasteboard clearContents];
  } @catch (NSException* exception) {
    (void)exception;
    return PasteboardWriteResult::PreparationFailed;
  }

  @try {
    return [pasteboard writeObjects:@[value]]
               ? PasteboardWriteResult::Succeeded
               : PasteboardWriteResult::WriteFailed;
  } @catch (NSException* exception) {
    (void)exception;
    return PasteboardWriteResult::WriteFailed;
  }
}

}  // namespace outflow::mic_probe
