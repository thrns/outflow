//! Synthetic C-02 lifecycle example. It calls no native, model, microphone,
//! clipboard, filesystem, or network interface and prints no transcript text.

use outflow_core::session_engine::{
    Effect, EventKind, EventSequencer, HoldReleaseCapability, InsertionOutcome, OpaqueTargetId,
    RecordingMode, SessionEvent, SessionReducer, SessionState, TargetSecurity, TargetSnapshot,
    TargetVerification,
};
use outflow_settings::{Settings, process_transcript_with_filler_preference};

struct MockEffectExecutor {
    labels: Vec<&'static str>,
}

impl MockEffectExecutor {
    fn observe(&mut self, effects: &[Effect]) {
        self.labels.extend(effects.iter().map(Effect::label));
    }

    fn begin_session(&mut self) {
        self.labels.clear();
    }

    fn summary(&self) -> String {
        self.labels.join(", ")
    }
}

struct Example {
    reducer: SessionReducer,
    sequences: EventSequencer,
    mock: MockEffectExecutor,
}

impl Example {
    fn new() -> Self {
        Self {
            // This is synthetic control injection for the example only.
            // Native/global shortcut registration remains NotImplemented.
            reducer: SessionReducer::new(HoldReleaseCapability::Supported),
            sequences: EventSequencer::new(),
            mock: MockEffectExecutor { labels: Vec::new() },
        }
    }

    fn dispatch(&mut self, kind: EventKind) -> Result<Vec<Effect>, Box<dyn std::error::Error>> {
        let sequence = self.sequences.next_sequence()?;
        let result = self.reducer.dispatch(SessionEvent::new(sequence, kind))?;
        self.mock.observe(&result.effects);
        Ok(result.effects)
    }

    fn start(&mut self, mode: RecordingMode) -> u64 {
        self.dispatch(EventKind::ShortcutActivated {
            mode,
            target: target(),
        })
        .expect("synthetic shortcut event is accepted");
        self.reducer
            .status()
            .session_id
            .expect("session was created")
            .get()
    }

    fn acknowledge_capture_ready(&mut self, session_id: u64) {
        let id = self.session_id(session_id);
        self.dispatch(EventKind::ModelPrepared { session_id: id })
            .expect("synthetic model acknowledgment is accepted");
        self.dispatch(EventKind::MicrophoneStarted { session_id: id })
            .expect("synthetic microphone acknowledgment is accepted");
        self.dispatch(EventKind::InputCaptured { session_id: id })
            .expect("synthetic capture acknowledgment is accepted");
    }

    fn finish_with_mock_cleanup(&mut self, session_id: u64, transcript: &str) {
        let id = self.session_id(session_id);
        self.dispatch(EventKind::KeyReleased { session_id: id })
            .expect("synthetic key release is accepted");
        let effects = self
            .dispatch(EventKind::FinalTranscript {
                session_id: id,
                text: transcript.to_owned(),
            })
            .expect("synthetic final result is accepted");
        let (effect_session, raw_text) = effects
            .into_iter()
            .find_map(|effect| match effect {
                Effect::PrepareFinalText { session_id, text } => Some((session_id, text)),
                _ => None,
            })
            .expect("finalization emitted one cleanup handoff");

        // This is the existing deterministic C-06 adapter. Settings and
        // vocabulary are borrowed; the raw and cleaned strings stay in memory.
        let preferences = Settings::default().cleanup;
        let cleaned =
            process_transcript_with_filler_preference(&raw_text, &[], false, &preferences)
                .expect("default synthetic cleanup preferences are valid");
        self.dispatch(EventKind::FinalTextPrepared {
            session_id: effect_session,
            text: cleaned,
        })
        .expect("one cleanup result is accepted");
    }

    fn session_id(&self, value: u64) -> outflow_core::session_engine::SessionId {
        self.reducer
            .status()
            .session_id
            .filter(|session_id| session_id.get() == value)
            .expect("the expected synthetic session remains current")
    }
}

fn target() -> TargetSnapshot {
    TargetSnapshot::new(
        OpaqueTargetId::new(0x0F10),
        1,
        TargetSecurity::Ordinary,
        outflow_core::session_engine::TargetEditability::Editable,
    )
}

fn main() {
    let mut example = Example::new();
    example
        .dispatch(EventKind::InitializationSucceeded)
        .expect("synthetic initialization is accepted");

    example.mock.begin_session();
    let first = example.start(RecordingMode::PushToTalk);
    example.acknowledge_capture_ready(first);
    let first_id = example.session_id(first);
    example
        .dispatch(EventKind::PartialTranscript {
            session_id: first_id,
            revision: 1,
            text: "first synthetic partial".to_owned(),
        })
        .expect("first partial revision is accepted");
    example
        .dispatch(EventKind::PartialTranscript {
            session_id: first_id,
            revision: 2,
            text: "revised synthetic partial".to_owned(),
        })
        .expect("second partial revision is accepted");
    example.finish_with_mock_cleanup(first, "um, synthetic example");
    example
        .dispatch(EventKind::TargetVerified {
            session_id: first_id,
            result: TargetVerification::Current(target()),
        })
        .expect("mock target verification is accepted");
    example
        .dispatch(EventKind::InsertionCompleted {
            session_id: first_id,
            outcome: InsertionOutcome::Inserted,
        })
        .expect("mock insertion acknowledgment is accepted");
    let first_effects = example.mock.summary();
    let first_state = example.reducer.status().state;
    let first_inserted = example.mock.labels.contains(&"AttemptInsertionOnce");

    example.mock.begin_session();
    let second = example.start(RecordingMode::PushToTalk);
    example.acknowledge_capture_ready(second);
    example.finish_with_mock_cleanup(second, "synthetic changed target");
    let second_id = example.session_id(second);
    example
        .dispatch(EventKind::TargetVerified {
            session_id: second_id,
            result: TargetVerification::Changed,
        })
        .expect("mock changed-target result is accepted");
    let fallback_state = example.reducer.status().state;
    example
        .dispatch(EventKind::CopyResultRequested {
            session_id: second_id,
        })
        .expect("explicit mock Copy request is accepted");
    example
        .dispatch(EventKind::Dismiss {
            session_id: Some(second_id),
        })
        .expect("explicit dismissal is accepted");
    let second_effects = example.mock.summary();
    let second_state = example.reducer.status().state;

    example.mock.begin_session();
    let third = example.start(RecordingMode::PushToTalk);
    let third_id = example.session_id(third);
    example
        .dispatch(EventKind::Cancel {
            session_id: third_id,
        })
        .expect("synthetic cancellation is accepted");
    let late_final_rejected = example
        .dispatch(EventKind::FinalTranscript {
            session_id: third_id,
            text: "late private synthetic final".to_owned(),
        })
        .is_err();
    let third_state = example.reducer.status().state;
    let third_effects = example.mock.summary();
    example
        .dispatch(EventKind::Dismiss {
            session_id: Some(third_id),
        })
        .expect("cancelled session dismissal is accepted");

    assert_eq!(first_state, SessionState::Idle);
    assert!(first_inserted);
    assert_eq!(fallback_state, SessionState::CopyReady);
    assert_eq!(second_state, SessionState::Idle);
    assert_eq!(third_state, SessionState::Cancelled);
    assert!(late_final_rejected);
    println!("Outflow Rust session engine example (synthetic)");
    println!("Native shortcuts: NOT INSTALLED");
    println!("Session {first}: {first_state:?}; insertion_attempted={first_inserted}");
    println!("  effects: {first_effects}");
    println!(
        "Session {second}: {fallback_state:?} -> {second_state:?}; target changed; explicit Copy requested"
    );
    println!("  effects: {second_effects}");
    println!("Session {third}: {third_state:?}; late final rejected={late_final_rejected}");
    println!("  effects: {third_effects}");
    println!("Native model, microphone, insertion, clipboard, persistence, network: NOT USED");
}
