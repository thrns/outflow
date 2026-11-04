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

    def test_probe_has_no_clipboard_or_cross_application_injection_api(self) -> None:
        sources = "\n".join(
            path.read_text(encoding="utf-8")
            for path in PROBE_ROOT.rglob("*")
            if path.is_file() and path.suffix in {".mm", ".cpp", ".hpp"}
        )
        for forbidden in (
            "NSPasteboard",
            "CGEventPost",
            "AXUIElement",
            "CGPreflightPostEventAccess",
        ):
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(forbidden, sources)

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
