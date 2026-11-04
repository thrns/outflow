from __future__ import annotations

from pathlib import Path
import unittest


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PROBE_ROOT = REPOSITORY_ROOT / "dev" / "macos-mic-probe"


class MacMicProbePrivacyTests(unittest.TestCase):
    @staticmethod
    def app_method(app: str, signature: str, next_signature: str) -> str:
        implementation = app.split("@implementation MicProbeAppDelegate", 1)[1]
        return implementation.split(signature, 1)[1].split(next_signature, 1)[0]

    def test_audio_capture_and_asr_worker_do_not_persist_or_log_content(self) -> None:
        capture = (PROBE_ROOT / "macos_capture.mm").read_text(encoding="utf-8")
        worker = (PROBE_ROOT / "session_worker.cpp").read_text(encoding="utf-8")
        producer = (PROBE_ROOT / "fixture_producer.cpp").read_text(encoding="utf-8")

        for source in (capture, worker, producer):
            for forbidden in (
                "std::ofstream",
                "fopen(",
                "NSURLSession",
                "CFNetwork",
                "socket(",
                "std::cout",
                "std::cerr",
                "NSLog(",
            ):
                with self.subTest(forbidden=forbidden):
                    self.assertNotIn(forbidden, source)

    def test_explicit_metrics_writer_serializes_only_metadata_and_never_overwrites(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        start = app.index("std::string metrics_json(")
        end = app.index("bool write_metrics_exclusive(", start)
        serializer = app[start:end]
        writer_end = app.index("NSButton* make_button(", end)
        writer_region = app[end:writer_end]

        self.assertIn("const SessionMetrics& metrics", writer_region)
        self.assertIn("O_EXCL", writer_region)
        self.assertIn("metrics.", serializer)
        self.assertNotIn("WorkerEvent", serializer)
        self.assertNotIn("event.text", serializer)
        self.assertNotIn("transcript", serializer.lower())
        self.assertNotIn("_partial_text", writer_region)
        self.assertNotIn("_final_text", writer_region)

    def test_probe_has_no_cross_application_injection_api(self) -> None:
        sources = "\n".join(
            path.read_text(encoding="utf-8")
            for path in PROBE_ROOT.rglob("*")
            if path.is_file() and path.suffix in {".mm", ".cpp", ".hpp"}
        )
        for forbidden in (
            "CGEventPost",
            "AXUIElement",
            "CGPreflightPostEventAccess",
        ):
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(forbidden, sources)

    def test_clipboard_write_is_bound_to_one_explicit_button_action(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        writer = (PROBE_ROOT / "pasteboard_writer.hpp").read_text(encoding="utf-8")
        action = self.app_method(
            app, "- (void)copyResultPressed:(id)sender {", "- (void)settingsPressed:(id)sender {"
        )

        self.assertIn('make_button(@"Copy Result", self,', app)
        self.assertIn("@selector(copyResultPressed:)", app)
        self.assertEqual(action.count("request_copy_result("), 1)
        self.assertIn("_copy_result_text, [&](std::string_view text)", action)
        self.assertIn("string_from_utf8(text)", action)
        self.assertIn("write_plain_text_to_pasteboard(", action)
        self.assertIn("[NSPasteboard generalPasteboard]", action)
        self.assertLess(action.index("string_from_utf8(text)"),
                        action.index("[NSPasteboard generalPasteboard]"))
        self.assertLess(action.index("[NSPasteboard generalPasteboard]"),
                        action.index("write_plain_text_to_pasteboard("))
        self.assertEqual(app.count("write_plain_text_to_pasteboard("), 1)
        self.assertEqual(app.count("[NSPasteboard generalPasteboard]"), 1)
        self.assertNotIn("accessBehavior", app + writer)
        self.assertEqual(writer.count("writeObjects:@[value]"), 1)
        self.assertLess(writer.index("initWithBytes:"),
                        writer.index("[pasteboard clearContents]"))
        self.assertLess(writer.index("[pasteboard clearContents]"),
                        writer.index("writeObjects:@[value]"))
        self.assertNotIn("declareTypes", writer)
        self.assertIn("PasteboardWriteResult::InvalidUtf8", writer)
        self.assertIn("PasteboardWriteResult::PasteboardUnavailable", writer)
        self.assertIn("PasteboardWriteResult::PreparationFailed", writer)
        self.assertIn("PasteboardWriteResult::WriteFailed", writer)
        self.assertIn("RejectedBySessionPolicy", app)
        for clipboard_read in ("stringForType:", "dataForType:", "readObjectsForClasses:",
                               "URLFromPasteboard"):
            with self.subTest(clipboard_read=clipboard_read):
                self.assertNotIn(clipboard_read, app + writer)

        lifecycle_handlers = (
            ("- (void)startPressed:(id)sender {", "- (void)stopPressed:(id)sender {"),
            ("- (void)stopPressed:(id)sender {", "- (void)cancelPressed:(id)sender {"),
            ("- (void)cancelPressed:(id)sender {", "- (void)dismissPressed:(id)sender {"),
            ("- (void)dismissPressed:(id)sender {", "- (void)copyResultPressed:(id)sender {"),
            ("- (std::uint64_t)prepareSession {", "- (void)startWorkerForSession:"),
            ("- (void)handleWorkerEvent:(WorkerEvent)event {", "- (void)refreshControls {"),
            ("- (void)applicationWillTerminate:(NSNotification*)notification {",
             "- (BOOL)windowShouldClose:"),
        )
        for start, end in lifecycle_handlers:
            body = self.app_method(app, start, end)
            with self.subTest(handler=start):
                self.assertNotIn("write_plain_text_to_pasteboard", body)
                self.assertNotIn("request_copy_result", body)

        worker_events = self.app_method(
            app, "- (void)handleWorkerEvent:(WorkerEvent)event {", "- (void)refreshControls {"
        )
        self.assertNotIn("write_plain_text_to_pasteboard", worker_events)
        self.assertNotIn("request_copy_result", worker_events)

    def test_clipboard_warning_and_failure_copy_recovery_are_accurate(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        self.assertIn(
            "Copy replaces the clipboard. If writing fails after clearing it, prior "
            "contents may be lost. Clipboard managers or sync may retain copies.",
            app,
        )
        self.assertIn("Its previous contents may have been cleared; the result remains "
                      "selectable", app)
        self.assertIn("select it and press ⌘C", app)

    def test_copy_policy_requires_ready_nonempty_current_session_text(self) -> None:
        policy = (PROBE_ROOT / "copy_result_policy.hpp").read_text(encoding="utf-8")
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        refresh = self.app_method(
            app, "- (void)refreshControls {", "- (void)stopSourcesForShutdown {"
        )
        final = self.app_method(
            app, "- (void)handleWorkerEvent:(WorkerEvent)event {", "- (void)refreshControls {"
        )

        self.assertIn("state == SessionState::ResultReady", policy)
        self.assertIn("accepted_session_id != 0", policy)
        self.assertIn("result_session_id == accepted_session_id", policy)
        self.assertIn("!final_text.empty()", policy)
        self.assertIn("state, _state.session_id(), _copy_result_session_id, _copy_result_text",
                      refresh)
        self.assertIn("_copy_result_text = std::move(event.text);", final)
        self.assertIn('"No speech was recognized in this session."', final)
        self.assertIn("_copy_result_text.empty()", final)
        self.assertNotIn("_partial_text.string", final.split("case WorkerEventKind::Final:", 1)[1]
                         .split("case WorkerEventKind::Cancelled:", 1)[0])

    def test_copy_result_memory_clears_on_cancel_dismiss_and_shutdown(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        clear = self.app_method(app, "- (void)clearFinalResult {",
                                "- (void)cancelActiveSessionForDismissal {")
        cancel = self.app_method(app, "- (void)cancelPressed:(id)sender {",
                                 "- (void)dismissPressed:(id)sender {")
        dismiss = self.app_method(app, "- (void)dismissPressed:(id)sender {",
                                  "- (void)copyResultPressed:(id)sender {")
        shutdown = self.app_method(
            app, "- (void)applicationWillTerminate:(NSNotification*)notification {",
            "- (BOOL)windowShouldClose:")
        prepare = self.app_method(app, "- (std::uint64_t)prepareSession {",
                                  "- (void)startWorkerForSession:")
        handler = self.app_method(
            app, "- (void)handleWorkerEvent:(WorkerEvent)event {", "- (void)refreshControls {")

        self.assertIn("clear_sensitive_text(_copy_result_text)", clear)
        self.assertIn("_copy_result_session_id = 0", clear)
        self.assertIn("[self clearFinalResult]", cancel)
        self.assertIn("[self clearFinalResult]", dismiss)
        self.assertIn("[self clearFinalResult]", shutdown)
        self.assertIn("[self clearFinalResult]", prepare)
        self.assertGreaterEqual(handler.count("[self clearFinalResult]"), 2)

    def test_copy_button_and_result_preview_are_keyboard_accessible(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        self.assertIn("button.accessibilityLabel = title", app)
        self.assertIn("_copy_result_button.accessibilityHelp", app)
        self.assertIn('_copy_result_button.keyEquivalent = @"\\r"', app)
        self.assertIn("[_final_text setNextKeyView:_copy_result_button]", app)
        self.assertIn("view.editable = NO", app)
        self.assertIn("view.selectable = YES", app)
        self.assertIn('_copy_warning_label.accessibilityLabel = @"Clipboard privacy warning"', app)
        self.assertIn('_copy_status_label.accessibilityLabel = @"Copy Result status"', app)
        self.assertIn("Copy replaces the clipboard.", app)
        self.assertIn("previous contents may have been cleared", app)
        self.assertIn("result remains selectable", app)

    def test_only_idle_start_and_preparing_worker_ready_can_open_capture(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        start_handler = self.app_method(
            app, "- (void)startPressed:(id)sender {", "- (void)stopPressed:(id)sender {"
        )
        source_handler = self.app_method(
            app, "- (void)beginAudioSourceForSession:", "- (void)handleWorkerEvent:"
        )
        self.assertIn("_state.state() != SessionState::Idle", start_handler)
        self.assertIn("_state.state() != SessionState::Preparing", source_handler)
        self.assertIn("std::make_unique<MacMicrophoneCapture>()", source_handler)

    def test_permission_completion_requires_current_generation_and_live_authorization(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        policy = (PROBE_ROOT / "permission_policy.cpp").read_text(encoding="utf-8")
        start_handler = self.app_method(
            app, "- (void)startPressed:(id)sender {", "- (void)stopPressed:(id)sender {"
        )
        completion = start_handler.split("completionHandler:^(BOOL granted) {", 1)[1].split(
            "    }];", 1
        )[0]
        self.assertIn("request_generation", start_handler)
        self.assertIn("is_current_permission_request(", completion)
        self.assertIn("session_id, strong_self->_state", completion)
        self.assertIn(
            "permission_grant_is_authorized(granted, microphone_permission())", completion
        )
        self.assertNotIn("[strong_self beginSession]", completion)
        self.assertIn("callback_generation == current_generation", policy)
        self.assertIn("state.state() == SessionState::Preparing", policy)

    def test_cancel_invalidates_permission_then_stops_capture_and_joins_worker(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        cancel = self.app_method(
            app, "- (void)cancelPressed:(id)sender {", "- (void)dismissPressed:(id)sender {"
        )
        self.assertLess(cancel.index("[self invalidatePermissionRequest]"),
                        cancel.index("[self stopSourcesForShutdown]"))
        self.assertLess(cancel.index("[self stopSourcesForShutdown]"),
                        cancel.index("_worker->request_cancel()"))
        self.assertLess(cancel.index("_worker->request_cancel()"), cancel.index("_worker->join()"))
        self.assertLess(cancel.index("_worker->join()"), cancel.index("_worker.reset()"))
        self.assertLess(cancel.index("_worker.reset()"), cancel.index("[self refreshControls]"))
        self.assertIn('State: Cancelled — microphone released; result discarded.', cancel)
        self.assertIn("elapsed_milliseconds(cancel_requested_ns, _microphone_release_ns)", cancel)
        self.assertIn("No cancellation metrics file was written", cancel)
        self.assertNotIn("write_metrics_exclusive", cancel)

    def test_terminal_callbacks_cannot_restore_text_or_commit_late_final(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        handler = self.app_method(
            app, "- (void)handleWorkerEvent:(WorkerEvent)event {", "- (void)refreshControls {"
        )
        partial = handler.split("case WorkerEventKind::Partial:", 1)[1].split(
            "case WorkerEventKind::Final:", 1
        )[0]
        final = handler.split("case WorkerEventKind::Final:", 1)[1].split(
            "case WorkerEventKind::Cancelled:", 1
        )[0]
        self.assertIn("if (_closing || !_state.accepts(event.session_id))", handler)
        self.assertIn("clear_sensitive_text(event.text)", handler)
        self.assertIn("_state.state() == SessionState::Listening", partial)
        self.assertIn("if (!_state.ready(event.session_id)) {", final)
        self.assertIn("clear_sensitive_text(event.text);", final)
        self.assertIn("write_metrics_exclusive(_launch.metrics_path", final)
        self.assertIn("clear_sensitive_text(shared_event->text)", app)
        cancelled = handler.split("case WorkerEventKind::Cancelled:", 1)[1].split(
            "case WorkerEventKind::Error:", 1
        )[0]
        self.assertNotIn("write_metrics_exclusive", cancelled)

    def test_dismiss_joins_terminal_worker_before_returning_to_idle(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        dismiss = self.app_method(
            app, "- (void)dismissPressed:(id)sender {", "- (void)settingsPressed:(id)sender {"
        )
        self.assertLess(dismiss.index("_worker->join()"), dismiss.index("_state.dismiss()"))
        self.assertIn("[self stopSourcesForShutdown]", dismiss)


if __name__ == "__main__":
    unittest.main()
