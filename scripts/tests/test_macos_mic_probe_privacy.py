from __future__ import annotations

from pathlib import Path
import unittest


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PROBE_ROOT = REPOSITORY_ROOT / "dev" / "macos-mic-probe"
LAUNCHER = REPOSITORY_ROOT / "scripts" / "run_macos_mic_probe.sh"


class MacMicProbePrivacyTests(unittest.TestCase):
    @staticmethod
    def app_method(app: str, signature: str, next_signature: str) -> str:
        implementation = app.split("@implementation MicProbeAppDelegate", 1)[1]
        return implementation.split(signature, 1)[1].split(next_signature, 1)[0]

    def test_verification_identity_is_visible_read_only_and_omitted_by_default(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        identity = (PROBE_ROOT / "verification_identity.hpp").read_text(encoding="utf-8")
        parser = app.split("LaunchOptions read_launch_options() {", 1)[1].split(
            "NSString* ns_string(", 1
        )[0]
        launch = self.app_method(
            app,
            "- (void)applicationDidFinishLaunching:(NSNotification*)notification {",
            "- (BOOL)applicationShouldTerminateAfterLastWindowClosed:",
        )

        self.assertIn("kMaximumVerificationIdLength = 32", identity)
        self.assertIn("is_valid_verification_id", identity)
        self.assertIn("result.invalid_verification_id = true", parser)
        self.assertIn("result.verification_id = std::string(utf8)", parser)
        self.assertIn("verification_diagnostic(", launch)
        self.assertIn("::getpid()", launch)
        self.assertIn("_window.accessibilityTitle = _window.title", launch)
        self.assertIn("verification_label = make_label(diagnostic_text", launch)
        self.assertIn("verification_label.accessibilityLabel = diagnostic_text", launch)
        self.assertIn('NSTextField* label = [NSTextField labelWithString:text]', app)
        self.assertIn('_window.title = @"Outflow Mic Probe — Development Only"', launch)
        self.assertIn("if (_launch.verification_id.has_value())", launch)
        self.assertIn("} else {\n    _window.title = @\"Outflow Mic Probe — Development Only\";",
                      launch)
        settings_sources = "\n".join(
            path.read_text(encoding="utf-8")
            for path in (REPOSITORY_ROOT / "crates" / "settings").rglob("*.rs")
        )
        self.assertNotIn("verification_id", settings_sources)

    def test_launcher_validates_and_forwards_the_verification_id_once(self) -> None:
        launcher = LAUNCHER.read_text(encoding="utf-8")
        forwarding = 'FORWARDED_ARGUMENTS+=(--verification-id "$VERIFICATION_ID")'
        self.assertEqual(launcher.count(forwarding), 1)
        self.assertIn('[[ "$2" =~ ^[A-Za-z0-9-]{1,32}$ ]] ||', launcher)
        self.assertIn('VERIFICATION_ID_SET == 0', launcher)
        self.assertIn('APP_ARGUMENTS+=(--model-dir "$OUTFLOW_MIC_PROBE_MODEL_DIR")', launcher)
        self.assertIn('FORWARDED_ARGUMENTS+=(--timed-fixture "$2")', launcher)
        self.assertIn('FORWARDED_ARGUMENTS+=("$1")', launcher)
        for option in (
            "--fixture-seconds",
            "--metrics-file",
            "--auto-cancel-after-ms",
            "--auto-exit-after-final",
            "--auto-exit-after-cancel",
        ):
            with self.subTest(option=option):
                self.assertIn(option, launcher)
        self.assertIn('exec /usr/bin/open -n -a "$OUTFLOW_MIC_PROBE_APP" --args "${APP_ARGUMENTS[@]}"',
                      launcher)

    def test_synthetic_diagnostic_launch_is_explicit_and_model_free_at_runtime(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        launch_policy = (PROBE_ROOT / "launch_policy.hpp").read_text(encoding="utf-8")
        synthetic = (PROBE_ROOT / "synthetic_copy_diagnostic.hpp").read_text(encoding="utf-8")
        launcher = LAUNCHER.read_text(encoding="utf-8")
        startup = self.app_method(
            app,
            "- (void)applicationDidFinishLaunching:(NSNotification*)notification {",
            "- (BOOL)applicationShouldTerminateAfterLastWindowClosed:",
        )
        diagnostic_branch = startup.split(
            "if (_launch.synthetic_copy_diagnostic) {", 1
        )[1].split("} else if (_launch.fixture_mode)", 1)[0]
        start = self.app_method(
            app, "- (void)startPressed:(id)sender {", "- (void)stopPressed:(id)sender {"
        )
        worker_start = self.app_method(
            app,
            "- (void)startWorkerForSession:(std::uint64_t)session_id {",
            "- (void)beginAudioSourceForSession:",
        )
        audio_start = self.app_method(
            app,
            "- (void)beginAudioSourceForSession:(std::uint64_t)session_id {",
            "- (void)handleWorkerEvent:",
        )

        self.assertIn("--synthetic-copy-diagnostic", launch_policy)
        self.assertIn("--timed-fixture", launch_policy)
        self.assertIn("--microphone-capture", launch_policy)
        self.assertIn("has_verification_id", launch_policy)
        self.assertIn("return ProbeInputMode::Microphone", launch_policy)
        self.assertIn("establish_synthetic_copy_result(_state, session_id)", diagnostic_branch)
        self.assertIn("kSyntheticCopyDiagnosticSample", diagnostic_branch)
        for forbidden in ("permission_status_text", "microphone_permission", "MacMicrophoneCapture",
                          "SessionWorker", "request_copy_result", "write_plain_text_to_pasteboard"):
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(forbidden, diagnostic_branch)
        self.assertIn("_launch.synthetic_copy_diagnostic || _closing", start)
        self.assertIn("_launch.synthetic_copy_diagnostic || _closing", worker_start)
        self.assertIn("_launch.synthetic_copy_diagnostic ||", audio_start)
        self.assertIn("No microphone, fixture, recognition worker, or model was used.", app)
        self.assertIn("Synthetic clipboard diagnostic", startup)
        self.assertIn("not recognized speech", startup)

        self.assertIn("if ((SYNTHETIC_COPY_DIAGNOSTIC)); then", launcher)
        synthetic_launcher_branch = launcher.split(
            "if ((SYNTHETIC_COPY_DIAGNOSTIC)); then", 1
        )[1].split("else", 1)[0]
        self.assertIn("--verification-id", synthetic_launcher_branch)
        self.assertNotIn("OUTFLOW_MIC_PROBE_MODEL_DIR", synthetic_launcher_branch)
        self.assertIn("APP_ARGUMENTS+=(--model-dir", launcher)
        self.assertIn('FORWARDED_ARGUMENTS+=("$1")', launcher)
        self.assertIn("kSyntheticCopyDiagnosticSample", synthetic)

    def test_synthetic_startup_has_no_clipboard_access_and_dismiss_clears_result(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        startup = self.app_method(
            app,
            "- (void)applicationDidFinishLaunching:(NSNotification*)notification {",
            "- (BOOL)applicationShouldTerminateAfterLastWindowClosed:",
        )
        diagnostic_branch = startup.split(
            "if (_launch.synthetic_copy_diagnostic) {", 1
        )[1].split("} else if (_launch.fixture_mode)", 1)[0]
        dismiss = self.app_method(
            app,
            "- (void)dismissPressed:(id)sender {",
            "- (void)copyResultPressed:(id)sender {",
        )
        self.assertNotIn("NSPasteboard", startup)
        self.assertNotIn("request_copy_result", diagnostic_branch)
        self.assertNotIn("write_plain_text_to_pasteboard", diagnostic_branch)
        self.assertIn("[self clearFinalResult]", dismiss)
        self.assertIn("_state.dismiss()", dismiss)
        self.assertIn("[self refreshControls]", dismiss)
        self.assertIn("_copy_result_button.enabled = copy_result_eligible(", app)

    def test_synthetic_copy_uses_the_existing_mouse_and_return_action(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        action = self.app_method(
            app,
            "- (void)copyResultPressed:(id)sender {",
            "- (void)settingsPressed:(id)sender {",
        )
        self.assertIn('make_button(@"Copy Result", self,', app)
        self.assertIn("@selector(copyResultPressed:)", app)
        self.assertIn('_copy_result_button.keyEquivalent = @"\\r"', app)
        self.assertEqual(action.count("request_copy_result("), 1)
        self.assertIn("string_from_utf8(text)", action)
        self.assertIn("write_plain_text_to_pasteboard(", action)

    def test_final_result_parity_uses_the_actual_text_view_and_fixture_identity_only(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        parity = (PROBE_ROOT / "final_result_parity.hpp").read_text(encoding="utf-8")
        update = self.app_method(
            app,
            "- (void)updateFinalDisplayParity {",
            "- (void)settingsPressed:(id)sender {",
        )
        final_event = self.app_method(
            app,
            "- (void)handleWorkerEvent:(WorkerEvent)event {",
            "- (void)refreshControls {",
        )
        policy = (PROBE_ROOT / "copy_result_policy.hpp").read_text(encoding="utf-8")

        self.assertIn("_launch.fixture_mode", app)
        self.assertIn("!synthetic_copy_diagnostic", parity)
        self.assertIn("verification_id.has_value()", parity)
        self.assertIn("is_valid_verification_id", parity)
        self.assertIn("NSString* displayed_text = _final_text.string", update)
        self.assertIn("dataUsingEncoding:NSUTF8StringEncoding", update)
        self.assertIn("allowLossyConversion:NO", update)
        self.assertIn("compare_final_result_parity(", update)
        self.assertIn("_copy_result_session_id == _state.session_id()", update)
        self.assertIn("[self updateFinalDisplayParity]", final_event)
        self.assertIn("Final display / copy-source parity: MATCH", update)
        self.assertIn("Final display / copy-source parity: MISMATCH", update)
        self.assertIn("Final display / copy-source parity: UNAVAILABLE", update)
        self.assertIn("require_final_result_parity", policy)
        self.assertIn("RejectedByFinalResultParity", policy)

        for forbidden in (
            "NSPasteboard",
            "write_plain_text_to_pasteboard",
            "std::cout",
            "std::cerr",
            "NSLog(",
            "shasum",
            "hash(",
            "ofstream",
        ):
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(forbidden, parity)
                self.assertNotIn(forbidden, update)

    def test_cancel_and_dismiss_clear_parity_result_and_disable_copy(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        cancel = self.app_method(
            app, "- (void)cancelPressed:(id)sender {", "- (void)dismissPressed:(id)sender {"
        )
        dismiss = self.app_method(
            app, "- (void)dismissPressed:(id)sender {", "- (void)copyResultPressed:(id)sender {"
        )
        clear = self.app_method(
            app, "- (void)clearFinalResult {", "- (void)cancelActiveSessionForDismissal {"
        )

        self.assertIn("[self clearFinalResult]", cancel)
        self.assertIn("[self refreshControls]", cancel)
        self.assertIn("[self clearFinalResult]", dismiss)
        self.assertIn("[self refreshControls]", dismiss)
        self.assertIn("_final_text.string = @\"\"", clear)
        self.assertIn("_copy_result_session_id = 0", clear)
        self.assertIn("_final_result_parity = FinalResultParity::Unavailable", clear)
        self.assertIn("_final_display_parity_label.stringValue = @\"\"", clear)
        self.assertIn("_final_result_parity", app.split("- (void)refreshControls {", 1)[1])

    def test_fixed_sample_is_single_source_and_isolated_pasteboard_tests_stay_separate(self) -> None:
        diagnostic = (PROBE_ROOT / "synthetic_copy_diagnostic.hpp").read_text(encoding="utf-8")
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        pasteboard_test = (PROBE_ROOT / "tests" / "pasteboard_integration_test.mm").read_text(
            encoding="utf-8"
        )
        self.assertEqual(diagnostic.count('"OUTFLOW-SYNTHETIC-COPY-START'), 1)
        self.assertEqual(diagnostic.count('"OUTFLOW-SYNTHETIC-COPY-END"'), 1)
        self.assertIn("kSyntheticCopyDiagnosticSample", app)
        self.assertIn("pasteboardWithUniqueName", pasteboard_test)
        self.assertNotIn("[NSPasteboard generalPasteboard]", pasteboard_test)
        self.assertIn("releaseGlobally", pasteboard_test)

    def test_isolated_pasteboard_round_trip_compares_exact_utf8_coverage_bytes(self) -> None:
        pasteboard_test = (PROBE_ROOT / "tests" / "pasteboard_integration_test.mm").read_text(
            encoding="utf-8"
        )

        self.assertIn("std::array<std::string_view, 8>", pasteboard_test)
        for fixture in (
            '"Plain ASCII text."',
            "Whitespace with a tab",
            '"Unicode: Café — naïve façade."',
            '"Emoji: 😀📝"',
            r'"Composed: \xC3\xA9"',
            r'"Decomposed: e\xCC\x81"',
            r'"LF\nCRLF\r\nCR\r"',
            '"Line one.\\n\\n  Indented line \\nLast line.\\n"',
        ):
            with self.subTest(fixture=fixture):
                self.assertIn(fixture, pasteboard_test)

        self.assertIn("NSString* expected = string_from_utf8(text);", pasteboard_test)
        self.assertIn("actual_utf8.length == text.size()", pasteboard_test)
        self.assertIn("std::memcmp(actual_utf8.bytes, text.data(), text.size()) == 0", pasteboard_test)
        self.assertNotIn("isEqualToString", pasteboard_test)
        self.assertNotIn("[NSPasteboard generalPasteboard]", pasteboard_test)

    def test_launch_identity_diagnostics_exclude_session_content_and_model_paths(self) -> None:
        app = (PROBE_ROOT / "app.mm").read_text(encoding="utf-8")
        identity = (PROBE_ROOT / "verification_identity.hpp").read_text(encoding="utf-8")
        launcher = LAUNCHER.read_text(encoding="utf-8")
        diagnostic = identity.split("inline std::string verification_diagnostic(", 1)[1].split(
            "}  // namespace", 1
        )[0]
        launch_diagnostic = launcher.split(
            "printf 'Requesting LaunchServices launch", 1
        )[1].split("# Launch through LaunchServices", 1)[0]

        self.assertIn("std::string_view value", diagnostic)
        self.assertIn("process_id", diagnostic)
        self.assertNotIn("event.text", diagnostic)
        self.assertNotIn("transcript", diagnostic.lower())
        self.assertNotIn("audio", diagnostic.lower())
        self.assertNotIn("clipboard", diagnostic.lower())
        for forbidden in ("transcript", "audio", "clipboard", "MODEL_DIR"):
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(forbidden, launch_diagnostic)
        self.assertIn("open success status alone does not verify the target window", launch_diagnostic)
        startup = self.app_method(
            app,
            "- (void)applicationDidFinishLaunching:(NSNotification*)notification {",
            "- (BOOL)applicationShouldTerminateAfterLastWindowClosed:",
        )
        self.assertNotIn("write_plain_text_to_pasteboard", startup)
        self.assertNotIn("request_copy_result", startup)

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
        self.assertIn(
            "_copy_result_text, [self requiresFinalResultParityVerification],\n      _final_result_parity, [&](std::string_view text)",
            action,
        )
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
