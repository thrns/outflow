use outflow_core::session_engine::{
    Effect, EventKind, EventSequencer, HoldReleaseCapability, OpaqueTargetId, RecordingMode,
    SessionEvent, SessionReducer, TargetEditability, TargetSecurity, TargetSnapshot,
    TargetVerification,
};
use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_settings::cleanup_policy::process_transcript_with_filler_preference;
use outflow_settings::{CleanupMode, CleanupSettings};

fn target() -> TargetSnapshot {
    TargetSnapshot::new(
        OpaqueTargetId::new(17),
        2,
        TargetSecurity::Ordinary,
        TargetEditability::Editable,
    )
}

struct Harness {
    reducer: SessionReducer,
    sequence: EventSequencer,
}

impl Harness {
    fn new() -> Self {
        let mut harness = Self {
            reducer: SessionReducer::new(HoldReleaseCapability::Supported),
            sequence: EventSequencer::new(),
        };
        harness.send(EventKind::InitializationSucceeded);
        harness
    }

    fn send(&mut self, kind: EventKind) -> Vec<Effect> {
        let sequence = self
            .sequence
            .next_sequence()
            .expect("synthetic sequence remains available");
        self.reducer
            .dispatch(SessionEvent::new(sequence, kind))
            .expect("synthetic lifecycle event is accepted")
            .effects
    }

    fn finish(&mut self, mode: CleanupMode) -> String {
        let start = self.send(EventKind::ShortcutActivated {
            mode: RecordingMode::PushToTalk,
            target: target(),
        });
        assert_eq!(
            start.iter().map(Effect::label).collect::<Vec<_>>(),
            ["RequestModelPreparation"]
        );
        let id = self
            .reducer
            .status()
            .session_id
            .expect("session was allocated");
        self.send(EventKind::ModelPrepared { session_id: id });
        self.send(EventKind::MicrophoneStarted { session_id: id });
        self.send(EventKind::InputCaptured { session_id: id });
        self.send(EventKind::KeyReleased { session_id: id });
        let finalization = self.send(EventKind::FinalTranscript {
            session_id: id,
            text: "  um, rust  new line  at ubc .".to_owned(),
        });
        assert_eq!(
            finalization.iter().map(Effect::label).collect::<Vec<_>>(),
            ["PrepareFinalText"]
        );
        let raw_text = match finalization.as_slice() {
            [Effect::PrepareFinalText { session_id, text }] if *session_id == id => text,
            _ => panic!("finalization must hand one session-scoped text value to cleanup"),
        };

        let vocabulary = [
            VocabularyEntryRef {
                spoken: "rust",
                written: "Rust",
            },
            VocabularyEntryRef {
                spoken: "ubc",
                written: "UBC",
            },
        ];
        let preferences = CleanupSettings {
            mode,
            remove_fillers: true,
        };
        let cleaned =
            process_transcript_with_filler_preference(raw_text, &vocabulary, true, &preferences)
                .expect("existing cleanup pipeline accepts this established case");
        let prepared = self.send(EventKind::FinalTextPrepared {
            session_id: id,
            text: cleaned.clone(),
        });
        assert_eq!(
            prepared.iter().map(Effect::label).collect::<Vec<_>>(),
            ["RequestTargetVerification"]
        );
        let copy_ready = self.send(EventKind::TargetVerified {
            session_id: id,
            result: TargetVerification::Changed,
        });
        assert_eq!(copy_ready.len(), 1);
        match &copy_ready[0] {
            Effect::ShowCopyResult { text, .. } => assert_eq!(text, &cleaned),
            effect => panic!("expected CopyReady effect, got {}", effect.label()),
        }
        assert!(
            self.reducer
                .dispatch(SessionEvent::new(
                    self.sequence
                        .next_sequence()
                        .expect("sequence remains available"),
                    EventKind::FinalTextPrepared {
                        session_id: id,
                        text: cleaned.clone(),
                    },
                ))
                .is_err()
        );
        assert_eq!(
            self.reducer.status().state,
            outflow_core::session_engine::SessionState::CopyReady
        );
        cleaned
    }
}

#[test]
fn session_cleanup_handoff_preserves_existing_clean_and_verbatim_modes_once() {
    let mut clean = Harness::new();
    assert_eq!(clean.finish(CleanupMode::Clean), "Rust\nat UBC.");

    let mut verbatim = Harness::new();
    assert_eq!(verbatim.finish(CleanupMode::Verbatim), "um, Rust\nat UBC.");
}
