from __future__ import annotations

from pathlib import Path
import unittest


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PROBE_ROOT = REPOSITORY_ROOT / "dev" / "macos-mic-probe"


class MacMicProbePrivacyTests(unittest.TestCase):
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
        implementation = app.split("@implementation MicProbeAppDelegate", 1)[1]
        start_handler = implementation.split("- (void)startPressed:(id)sender {", 1)[1].split(
            "- (void)stopPressed:(id)sender {", 1
        )[0]
        source_handler = implementation.split("- (void)beginAudioSourceForSession:", 1)[1].split(
            "- (void)handleWorkerEvent:", 1
        )[0]
        self.assertIn("_state.state() != SessionState::Idle", start_handler)
        self.assertIn("_state.state() != SessionState::Preparing", source_handler)
        self.assertIn("std::make_unique<MacMicrophoneCapture>()", source_handler)


if __name__ == "__main__":
    unittest.main()
