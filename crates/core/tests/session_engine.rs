use outflow_core::session_engine::{
    CopyOutcome, DispatchOutcome, Effect, EventKind, EventSequence, FailureCode,
    HoldReleaseCapability, IgnoredEvent, InsertionOutcome, OpaqueTargetId, RecordingMode,
    ReducerError, SessionEvent, SessionId, SessionReducer, SessionState, ShortcutRegistration,
    TargetCommand, TargetEditability, TargetSecurity, TargetSnapshot, TargetVerification,
};
use std::fmt::Write as _;

const TARGET_ID: u128 = 0xA11CE;

struct MockEffectExecutor {
    trace: Vec<&'static str>,
}

impl MockEffectExecutor {
    fn new() -> Self {
        Self { trace: Vec::new() }
    }

    fn observe(&mut self, effects: &[Effect]) {
        self.trace.extend(effects.iter().map(Effect::label));
    }
}

struct Harness {
    reducer: SessionReducer,
    sequence: u64,
    executor: MockEffectExecutor,
}

impl Harness {
    fn new(capability: HoldReleaseCapability) -> Self {
        let mut harness = Self {
            reducer: SessionReducer::new(capability),
            sequence: 0,
            executor: MockEffectExecutor::new(),
        };
        let initialized = harness.send(EventKind::InitializationSucceeded).unwrap();
        assert_effect_labels(&initialized.effects, &[]);
        harness
    }

    fn send(
        &mut self,
        kind: EventKind,
    ) -> Result<outflow_core::session_engine::DispatchResult, ReducerError> {
        self.sequence += 1;
        let result = self
            .reducer
            .dispatch(SessionEvent::new(EventSequence(self.sequence), kind));
        if let Ok(result) = &result {
            self.executor.observe(&result.effects);
        }
        result
    }

    fn start(&mut self, mode: RecordingMode) -> SessionId {
        let result = self
            .send(EventKind::ShortcutActivated {
                mode,
                target: target(),
            })
            .unwrap();
        assert_effect_labels(&result.effects, &["RequestModelPreparation"]);
        self.reducer.status().session_id.unwrap()
    }

    fn listen(&mut self, mode: RecordingMode) -> SessionId {
        let id = self.start(mode);
        let result = self
            .send(EventKind::ModelPrepared { session_id: id })
            .unwrap();
        assert_effect_labels(&result.effects, &["RequestMicrophoneStart"]);
        let result = self
            .send(EventKind::MicrophoneStarted { session_id: id })
            .unwrap();
        assert_effect_labels(&result.effects, &[]);
        assert_eq!(self.reducer.status().state, SessionState::Listening);
        id
    }
}

fn target() -> TargetSnapshot {
    TargetSnapshot::new(
        OpaqueTargetId::new(TARGET_ID),
        3,
        TargetSecurity::Ordinary,
        TargetEditability::Editable,
    )
}

fn assert_effect_labels(effects: &[Effect], expected: &[&str]) {
    let actual = effects.iter().map(Effect::label).collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

fn assert_state(harness: &Harness, state: SessionState) {
    assert_eq!(harness.reducer.status().state, state);
}

fn finalize_empty(harness: &mut Harness, id: SessionId, mode: RecordingMode) {
    harness
        .send(EventKind::InputCaptured { session_id: id })
        .unwrap();
    match mode {
        RecordingMode::PushToTalk => {
            let stopped = harness
                .send(EventKind::KeyReleased { session_id: id })
                .unwrap();
            assert_effect_labels(
                &stopped.effects,
                &["RequestMicrophoneStop", "RequestAsrFinalization"],
            );
        }
        RecordingMode::Toggle => {
            let key_up = harness
                .send(EventKind::KeyReleased { session_id: id })
                .unwrap();
            assert_eq!(
                key_up.outcome,
                DispatchOutcome::Ignored(IgnoredEvent::ToggleKeyUp)
            );
            assert!(key_up.effects.is_empty());
            let stopped = harness
                .send(EventKind::ShortcutActivated {
                    mode: RecordingMode::Toggle,
                    target: target(),
                })
                .unwrap();
            assert_effect_labels(
                &stopped.effects,
                &["RequestMicrophoneStop", "RequestAsrFinalization"],
            );
        }
    }
    let pending = harness
        .send(EventKind::FinalTranscript {
            session_id: id,
            text: String::new(),
        })
        .unwrap();
    assert_effect_labels(&pending.effects, &["PrepareFinalText"]);
    let ready = harness
        .send(EventKind::FinalTextPrepared {
            session_id: id,
            text: String::new(),
        })
        .unwrap();
    assert_effect_labels(&ready.effects, &["ClearPrivateResult"]);
    assert_state(harness, SessionState::Idle);
}

fn make_copy_ready(harness: &mut Harness, id: SessionId, outcome: InsertionOutcome) {
    let result = harness
        .send(EventKind::InputCaptured { session_id: id })
        .unwrap();
    assert!(result.effects.is_empty());
    harness
        .send(EventKind::KeyReleased { session_id: id })
        .unwrap();
    harness
        .send(EventKind::FinalTranscript {
            session_id: id,
            text: "synthetic final".to_owned(),
        })
        .unwrap();
    let prepared = harness
        .send(EventKind::FinalTextPrepared {
            session_id: id,
            text: "synthetic final".to_owned(),
        })
        .unwrap();
    assert_effect_labels(&prepared.effects, &["RequestTargetVerification"]);
    let attempt = harness
        .send(EventKind::TargetVerified {
            session_id: id,
            result: TargetVerification::Current(target()),
        })
        .unwrap();
    assert_effect_labels(&attempt.effects, &["AttemptInsertionOnce"]);
    let result = harness
        .send(EventKind::InsertionCompleted {
            session_id: id,
            outcome,
        })
        .unwrap();
    if outcome == InsertionOutcome::UncertainOrPartial {
        assert_effect_labels(&result.effects, &["RequireManualReview"]);
        assert_state(harness, SessionState::Uncertain);
    } else {
        assert_effect_labels(&result.effects, &["ShowCopyResult"]);
        assert_state(harness, SessionState::CopyReady);
    }
}

#[test]
fn valid_happy_path_transitions_and_effect_order_are_explicit() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    assert_state(&h, SessionState::Idle);
    let id = h.start(RecordingMode::PushToTalk);
    assert!(id.get() > 0);
    assert_state(&h, SessionState::Loading);
    assert_eq!(
        h.reducer.status().shortcut_registration,
        ShortcutRegistration::NotImplemented
    );

    let model = h.send(EventKind::ModelPrepared { session_id: id }).unwrap();
    assert_effect_labels(&model.effects, &["RequestMicrophoneStart"]);
    let microphone = h
        .send(EventKind::MicrophoneStarted { session_id: id })
        .unwrap();
    assert!(microphone.effects.is_empty());
    assert_state(&h, SessionState::Listening);

    h.send(EventKind::InputCaptured { session_id: id }).unwrap();
    let stop = h.send(EventKind::KeyReleased { session_id: id }).unwrap();
    assert_effect_labels(
        &stop.effects,
        &["RequestMicrophoneStop", "RequestAsrFinalization"],
    );
    assert_state(&h, SessionState::Finalizing);

    let final_text = h
        .send(EventKind::FinalTranscript {
            session_id: id,
            text: "synthetic final".to_owned(),
        })
        .unwrap();
    assert_effect_labels(&final_text.effects, &["PrepareFinalText"]);
    let prepared = h
        .send(EventKind::FinalTextPrepared {
            session_id: id,
            text: "Synthetic final.".to_owned(),
        })
        .unwrap();
    assert_effect_labels(&prepared.effects, &["RequestTargetVerification"]);

    let commit = h
        .send(EventKind::TargetVerified {
            session_id: id,
            result: TargetVerification::Current(target()),
        })
        .unwrap();
    assert_effect_labels(&commit.effects, &["AttemptInsertionOnce"]);
    assert_state(&h, SessionState::Committing);
    assert!(h.reducer.status().insertion_attempted);

    let inserted = h
        .send(EventKind::InsertionCompleted {
            session_id: id,
            outcome: InsertionOutcome::Inserted,
        })
        .unwrap();
    assert_effect_labels(&inserted.effects, &["ClearPrivateResult"]);
    assert_state(&h, SessionState::Idle);
    assert_eq!(h.reducer.status().session_id, None);
    assert_eq!(
        h.executor.trace,
        [
            "RequestModelPreparation",
            "RequestMicrophoneStart",
            "RequestMicrophoneStop",
            "RequestAsrFinalization",
            "PrepareFinalText",
            "RequestTargetVerification",
            "AttemptInsertionOnce",
            "ClearPrivateResult",
        ]
    );
}

#[test]
fn initialization_failure_enters_faulted_and_dismissal_returns_to_idle() {
    let mut reducer = SessionReducer::new(HoldReleaseCapability::Supported);
    let failed = reducer
        .dispatch(SessionEvent::new(
            EventSequence(1),
            EventKind::InitializationFailed,
        ))
        .unwrap();
    assert_eq!(
        failed.effects,
        [Effect::ReportFailure {
            session_id: None,
            code: FailureCode::InitializationFailed,
        }]
    );
    assert_eq!(reducer.status().state, SessionState::Faulted);
    assert_eq!(
        reducer.status().failure,
        Some(FailureCode::InitializationFailed)
    );

    let dismissed = reducer
        .dispatch(SessionEvent::new(
            EventSequence(2),
            EventKind::Dismiss { session_id: None },
        ))
        .unwrap();
    assert_effect_labels(&dismissed.effects, &[]);
    assert_eq!(reducer.status().state, SessionState::Idle);
    assert_eq!(reducer.status().failure, None);
}

#[test]
fn invalid_transitions_and_duplicate_or_out_of_order_sequences_are_rejected() {
    let mut reducer = SessionReducer::new(HoldReleaseCapability::Supported);
    let invalid = reducer
        .dispatch(SessionEvent::new(
            EventSequence(1),
            EventKind::ModelPreparationFailed {
                session_id: fake_id_for_compile(),
            },
        ))
        .unwrap_err();
    assert_eq!(invalid, ReducerError::StaleSession);
    assert_eq!(reducer.status().last_event_sequence, Some(EventSequence(1)));
    assert_eq!(
        reducer
            .dispatch(SessionEvent::new(
                EventSequence(1),
                EventKind::InitializationSucceeded,
            ))
            .unwrap_err(),
        ReducerError::NonMonotonicEventSequence
    );
    assert_eq!(
        reducer
            .dispatch(SessionEvent::new(
                EventSequence(0),
                EventKind::InitializationSucceeded,
            ))
            .unwrap_err(),
        ReducerError::NonMonotonicEventSequence
    );
    assert_state_raw(&reducer, SessionState::Uninitialized);

    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.start(RecordingMode::PushToTalk);
    let start_without_request = h
        .send(EventKind::MicrophoneStarted { session_id: id })
        .unwrap_err();
    assert_eq!(start_without_request, ReducerError::InvalidTransition);
    assert_state(&h, SessionState::Loading);
    assert_eq!(
        h.send(EventKind::PartialTranscript {
            session_id: id,
            revision: 1,
            text: "ignored synthetic value".to_owned(),
        })
        .unwrap_err(),
        ReducerError::InvalidTransition
    );
}

fn fake_id_for_compile() -> SessionId {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    h.start(RecordingMode::PushToTalk)
}

fn assert_state_raw(reducer: &SessionReducer, expected: SessionState) {
    assert_eq!(reducer.status().state, expected);
}

#[test]
fn repeated_push_to_talk_key_down_is_idempotent_and_modes_cannot_overlap() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.start(RecordingMode::PushToTalk);
    let repeated = h
        .send(EventKind::ShortcutActivated {
            mode: RecordingMode::PushToTalk,
            target: target(),
        })
        .unwrap();
    assert_eq!(
        repeated.outcome,
        DispatchOutcome::Ignored(IgnoredEvent::RepeatedPushToTalkActivation)
    );
    assert!(repeated.effects.is_empty());
    assert_eq!(h.reducer.status().session_id, Some(id));
    assert_eq!(
        h.send(EventKind::ShortcutActivated {
            mode: RecordingMode::Toggle,
            target: target(),
        })
        .unwrap_err(),
        ReducerError::RecordingModeConflict
    );
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|effect| **effect == "RequestModelPreparation")
            .count(),
        1
    );
}

#[test]
fn session_ids_increase_and_old_callbacks_cannot_reenter_a_new_session() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let first = h.start(RecordingMode::PushToTalk);
    let cancelled = h.send(EventKind::Cancel { session_id: first }).unwrap();
    assert_effect_labels(
        &cancelled.effects,
        &["RequestAsrCancellation", "ClearPrivateResult"],
    );
    assert_state(&h, SessionState::Cancelled);
    h.send(EventKind::Dismiss {
        session_id: Some(first),
    })
    .unwrap();
    let second = h.start(RecordingMode::PushToTalk);
    assert!(second.get() > first.get());
    assert_eq!(
        h.send(EventKind::ModelPrepared { session_id: first })
            .unwrap_err(),
        ReducerError::StaleSession
    );
    assert_eq!(h.reducer.status().session_id, Some(second));
    assert_state(&h, SessionState::Loading);
}

#[test]
fn session_ids_are_unique_across_reducers_and_foreign_callbacks_are_rejected() {
    let mut first_reducer = Harness::new(HoldReleaseCapability::Supported);
    let first_id = first_reducer.start(RecordingMode::PushToTalk);
    let mut second_reducer = Harness::new(HoldReleaseCapability::Supported);
    let second_id = second_reducer.start(RecordingMode::PushToTalk);

    assert!(second_id.get() > first_id.get());
    assert_eq!(
        second_reducer
            .send(EventKind::ModelPrepared {
                session_id: first_id,
            })
            .unwrap_err(),
        ReducerError::StaleSession
    );
    assert_eq!(second_reducer.reducer.status().session_id, Some(second_id));
    assert_state(&second_reducer, SessionState::Loading);
}

#[test]
fn one_hundred_push_to_talk_cycles_have_one_prepare_and_finalize_each() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    for _ in 0..100 {
        let id = h.listen(RecordingMode::PushToTalk);
        finalize_empty(&mut h, id, RecordingMode::PushToTalk);
    }
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "RequestModelPreparation")
            .count(),
        100
    );
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "RequestMicrophoneStart")
            .count(),
        100
    );
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "RequestAsrFinalization")
            .count(),
        100
    );
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "AttemptInsertionOnce")
            .count(),
        0
    );
    assert_state(&h, SessionState::Idle);
}

#[test]
fn one_hundred_toggle_cycles_ignore_key_up_and_stop_on_second_activation() {
    let mut h = Harness::new(HoldReleaseCapability::Unsupported);
    for _ in 0..100 {
        let id = h.listen(RecordingMode::Toggle);
        finalize_empty(&mut h, id, RecordingMode::Toggle);
    }
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "RequestModelPreparation")
            .count(),
        100
    );
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "RequestAsrFinalization")
            .count(),
        100
    );
    assert_eq!(
        h.reducer.status().hold_release_capability,
        HoldReleaseCapability::Unsupported
    );
    assert_state(&h, SessionState::Idle);
}

#[test]
fn unsupported_release_capability_is_visible_and_rejects_push_to_talk() {
    let mut h = Harness::new(HoldReleaseCapability::Unsupported);
    assert_eq!(
        h.reducer.status().hold_release_capability,
        HoldReleaseCapability::Unsupported
    );
    assert_eq!(
        h.reducer.status().shortcut_registration,
        ShortcutRegistration::NotImplemented
    );
    assert_eq!(
        h.send(EventKind::ShortcutActivated {
            mode: RecordingMode::PushToTalk,
            target: target(),
        })
        .unwrap_err(),
        ReducerError::PushToTalkUnsupported
    );
    assert_eq!(h.reducer.status().state, SessionState::Idle);
    assert_eq!(h.reducer.status().session_id, None);
    assert!(h.executor.trace.is_empty());
}

#[test]
fn release_during_model_loading_cancels_without_starting_microphone() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.start(RecordingMode::PushToTalk);
    let released = h.send(EventKind::KeyReleased { session_id: id }).unwrap();
    assert_effect_labels(
        &released.effects,
        &["RequestAsrCancellation", "ClearPrivateResult"],
    );
    assert_state(&h, SessionState::Cancelled);
    assert_eq!(
        h.send(EventKind::ModelPrepared { session_id: id })
            .unwrap_err(),
        ReducerError::SessionClosed
    );
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "RequestMicrophoneStart")
            .count(),
        0
    );
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "RequestAsrFinalization")
            .count(),
        0
    );
}

#[test]
fn release_while_microphone_start_is_pending_stops_it_and_finalizes_only_captured_audio() {
    let mut captured = Harness::new(HoldReleaseCapability::Supported);
    let id = captured.start(RecordingMode::PushToTalk);
    captured
        .send(EventKind::ModelPrepared { session_id: id })
        .unwrap();
    let released = captured
        .send(EventKind::KeyReleased { session_id: id })
        .unwrap();
    assert!(released.effects.is_empty());
    captured
        .send(EventKind::InputCaptured { session_id: id })
        .unwrap();
    let started = captured
        .send(EventKind::MicrophoneStarted { session_id: id })
        .unwrap();
    assert_effect_labels(
        &started.effects,
        &["RequestMicrophoneStop", "RequestAsrFinalization"],
    );
    assert_state(&captured, SessionState::Finalizing);

    let mut empty = Harness::new(HoldReleaseCapability::Supported);
    let id = empty.start(RecordingMode::PushToTalk);
    empty
        .send(EventKind::ModelPrepared { session_id: id })
        .unwrap();
    empty
        .send(EventKind::KeyReleased { session_id: id })
        .unwrap();
    let started = empty
        .send(EventKind::MicrophoneStarted { session_id: id })
        .unwrap();
    assert_effect_labels(
        &started.effects,
        &[
            "RequestMicrophoneStop",
            "RequestAsrCancellation",
            "ClearPrivateResult",
        ],
    );
    assert_state(&empty, SessionState::Cancelled);
    assert_eq!(
        empty
            .executor
            .trace
            .iter()
            .filter(|e| **e == "RequestAsrFinalization")
            .count(),
        0
    );
}

#[test]
fn cancellation_during_loading_and_listening_discards_data_and_rejects_late_results() {
    let mut loading = Harness::new(HoldReleaseCapability::Supported);
    let id = loading.start(RecordingMode::PushToTalk);
    let cancelled = loading.send(EventKind::Cancel { session_id: id }).unwrap();
    assert_effect_labels(
        &cancelled.effects,
        &["RequestAsrCancellation", "ClearPrivateResult"],
    );
    assert_state(&loading, SessionState::Cancelled);
    assert_eq!(
        loading
            .send(EventKind::FinalTranscript {
                session_id: id,
                text: "private synthetic late result".to_owned(),
            })
            .unwrap_err(),
        ReducerError::SessionClosed
    );
    assert!(!loading.executor.trace.contains(&"PrepareFinalText"));
    assert!(!loading.executor.trace.contains(&"AttemptInsertionOnce"));

    let mut listening = Harness::new(HoldReleaseCapability::Supported);
    let id = listening.listen(RecordingMode::PushToTalk);
    listening
        .send(EventKind::PartialTranscript {
            session_id: id,
            revision: 1,
            text: "private partial".to_owned(),
        })
        .unwrap();
    let cancelled = listening
        .send(EventKind::Cancel { session_id: id })
        .unwrap();
    assert_effect_labels(
        &cancelled.effects,
        &[
            "RequestMicrophoneStop",
            "RequestAsrCancellation",
            "ClearPrivateResult",
        ],
    );
    assert_state(&listening, SessionState::Cancelled);
    assert!(!listening.executor.trace.contains(&"AttemptInsertionOnce"));
}

#[test]
fn permission_and_microphone_start_failures_fault_without_insertion() {
    for permission_failure in [true, false] {
        let mut h = Harness::new(HoldReleaseCapability::Supported);
        let id = h.start(RecordingMode::PushToTalk);
        let (failure, expected) = if permission_failure {
            (
                EventKind::PermissionDenied { session_id: id },
                FailureCode::MicrophonePermissionDenied,
            )
        } else {
            h.send(EventKind::ModelPrepared { session_id: id }).unwrap();
            (
                EventKind::MicrophoneStartFailed { session_id: id },
                FailureCode::MicrophoneStartFailed,
            )
        };
        let result = h.send(failure).unwrap();
        let expected_effects = if permission_failure {
            &[
                "RequestAsrCancellation",
                "ClearPrivateResult",
                "ReportFailure",
            ][..]
        } else {
            &[
                "RequestMicrophoneStop",
                "RequestAsrCancellation",
                "ClearPrivateResult",
                "ReportFailure",
            ][..]
        };
        assert_effect_labels(&result.effects, expected_effects);
        assert_state(&h, SessionState::Faulted);
        assert_eq!(h.reducer.status().failure, Some(expected));
        assert!(!h.executor.trace.contains(&"AttemptInsertionOnce"));
        h.send(EventKind::Dismiss {
            session_id: Some(id),
        })
        .unwrap();
        assert_state(&h, SessionState::Idle);
    }
}

#[test]
fn device_loss_and_bounded_buffer_overflow_cancel_and_clear_safely() {
    for overflow in [false, true] {
        let mut h = Harness::new(HoldReleaseCapability::Supported);
        let id = h.listen(RecordingMode::PushToTalk);
        h.send(EventKind::InputCaptured { session_id: id }).unwrap();
        h.send(EventKind::PartialTranscript {
            session_id: id,
            revision: 1,
            text: "private partial".to_owned(),
        })
        .unwrap();
        let event = if overflow {
            EventKind::AudioBufferOverflow { session_id: id }
        } else {
            EventKind::AudioDeviceLost { session_id: id }
        };
        let result = h.send(event).unwrap();
        assert_effect_labels(
            &result.effects,
            &[
                "RequestMicrophoneStop",
                "RequestAsrCancellation",
                "ClearPrivateResult",
                "ReportFailure",
            ],
        );
        assert_state(&h, SessionState::Cancelled);
        assert_eq!(
            h.reducer.status().failure,
            Some(if overflow {
                FailureCode::AudioBufferOverflow
            } else {
                FailureCode::AudioDeviceLost
            })
        );
        assert!(!h.executor.trace.contains(&"AttemptInsertionOnce"));
    }
}

#[test]
fn mock_executor_observes_order_and_failure_acknowledgments_without_os_calls() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.start(RecordingMode::Toggle);
    let failed = h
        .send(EventKind::ModelPreparationFailed { session_id: id })
        .unwrap();
    assert_effect_labels(
        &failed.effects,
        &[
            "RequestAsrCancellation",
            "ClearPrivateResult",
            "ReportFailure",
        ],
    );
    assert_eq!(
        h.executor.trace,
        [
            "RequestModelPreparation",
            "RequestAsrCancellation",
            "ClearPrivateResult",
            "ReportFailure",
        ]
    );
    assert_eq!(
        h.reducer.status().failure,
        Some(FailureCode::ModelPreparationFailed)
    );
}

#[test]
fn empty_final_transcript_returns_idle_without_target_or_insert_effects() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    finalize_empty(&mut h, id, RecordingMode::PushToTalk);
    assert!(!h.executor.trace.contains(&"RequestTargetVerification"));
    assert!(!h.executor.trace.contains(&"AttemptInsertionOnce"));
    assert!(!h.executor.trace.contains(&"ShowCopyResult"));
}

#[test]
fn cleanup_failure_is_session_scoped_and_discards_final_text() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    h.send(EventKind::InputCaptured { session_id: id }).unwrap();
    h.send(EventKind::KeyReleased { session_id: id }).unwrap();
    let pending = h
        .send(EventKind::FinalTranscript {
            session_id: id,
            text: "private cleanup input".to_owned(),
        })
        .unwrap();
    assert_effect_labels(&pending.effects, &["PrepareFinalText"]);

    let failed = h
        .send(EventKind::TextPreparationFailed { session_id: id })
        .unwrap();
    assert_effect_labels(
        &failed.effects,
        &[
            "RequestAsrCancellation",
            "ClearPrivateResult",
            "ReportFailure",
        ],
    );
    assert_eq!(h.reducer.status().state, SessionState::Faulted);
    assert_eq!(
        h.reducer.status().failure,
        Some(FailureCode::TextPreparationFailed)
    );
    assert!(!h.executor.trace.contains(&"AttemptInsertionOnce"));
    assert!(!format!("{:?} {:?}", failed, h.reducer.status()).contains("private cleanup input"));
    assert!(matches!(
        h.send(EventKind::FinalTextPrepared {
            session_id: id,
            text: "late cleanup result".to_owned(),
        }),
        Err(ReducerError::SessionClosed)
    ));
    assert_eq!(h.reducer.status().state, SessionState::Faulted);
}

#[test]
fn unicode_final_is_preserved_and_cleanup_result_is_used_once() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    h.send(EventKind::InputCaptured { session_id: id }).unwrap();
    h.send(EventKind::KeyReleased { session_id: id }).unwrap();
    h.send(EventKind::FinalTranscript {
        session_id: id,
        text: "café 東京".to_owned(),
    })
    .unwrap();
    let prepared = h
        .send(EventKind::FinalTextPrepared {
            session_id: id,
            text: "café 東京".to_owned(),
        })
        .unwrap();
    assert_eq!(prepared.effects.len(), 1);
    assert_eq!(prepared.effects[0].label(), "RequestTargetVerification");
    let commit = h
        .send(EventKind::TargetVerified {
            session_id: id,
            result: TargetVerification::Current(target()),
        })
        .unwrap();
    let insertion_text = match &commit.effects[0] {
        Effect::Target(TargetCommand::InsertOnce { text, .. }) => text,
        _ => panic!("expected one guarded insert effect"),
    };
    assert_eq!(insertion_text, "café 東京");
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "PrepareFinalText")
            .count(),
        1
    );
}

#[test]
fn partial_revisions_must_increase_and_never_trigger_insertion() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    let one = h
        .send(EventKind::PartialTranscript {
            session_id: id,
            revision: 1,
            text: "private revision one".to_owned(),
        })
        .unwrap();
    assert_effect_labels(&one.effects, &["PublishPrivatePartial"]);
    let two = h
        .send(EventKind::PartialTranscript {
            session_id: id,
            revision: 2,
            text: "private revision two".to_owned(),
        })
        .unwrap();
    assert_effect_labels(&two.effects, &["PublishPrivatePartial"]);
    assert_eq!(
        h.send(EventKind::PartialTranscript {
            session_id: id,
            revision: 2,
            text: "duplicate".to_owned(),
        })
        .unwrap_err(),
        ReducerError::NonMonotonicPartialRevision
    );
    assert_eq!(
        h.send(EventKind::PartialTranscript {
            session_id: id,
            revision: 1,
            text: "old".to_owned(),
        })
        .unwrap_err(),
        ReducerError::NonMonotonicPartialRevision
    );
    assert_eq!(h.reducer.status().partial_revision, Some(2));
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "PublishPrivatePartial")
            .count(),
        2
    );
    assert!(!h.executor.trace.contains(&"AttemptInsertionOnce"));
}

#[test]
fn target_change_secure_target_and_unsupported_target_fall_back_without_insert() {
    for verification in [
        TargetVerification::Changed,
        TargetVerification::SecureOrProtected,
        TargetVerification::Unsupported,
        TargetVerification::Unavailable,
        TargetVerification::Current(TargetSnapshot::new(
            OpaqueTargetId::new(TARGET_ID + 1),
            3,
            TargetSecurity::Ordinary,
            TargetEditability::Editable,
        )),
        TargetVerification::Current(TargetSnapshot::new(
            OpaqueTargetId::new(TARGET_ID),
            4,
            TargetSecurity::Ordinary,
            TargetEditability::Editable,
        )),
        TargetVerification::Current(TargetSnapshot::new(
            OpaqueTargetId::new(TARGET_ID),
            3,
            TargetSecurity::SecureOrProtected,
            TargetEditability::Editable,
        )),
        TargetVerification::Current(TargetSnapshot::new(
            OpaqueTargetId::new(TARGET_ID),
            3,
            TargetSecurity::Ordinary,
            TargetEditability::Unknown,
        )),
    ] {
        let mut h = Harness::new(HoldReleaseCapability::Supported);
        let id = h.listen(RecordingMode::PushToTalk);
        h.send(EventKind::InputCaptured { session_id: id }).unwrap();
        h.send(EventKind::KeyReleased { session_id: id }).unwrap();
        h.send(EventKind::FinalTranscript {
            session_id: id,
            text: "synthetic private result".to_owned(),
        })
        .unwrap();
        h.send(EventKind::FinalTextPrepared {
            session_id: id,
            text: "synthetic private result".to_owned(),
        })
        .unwrap();
        let fallback = h
            .send(EventKind::TargetVerified {
                session_id: id,
                result: verification,
            })
            .unwrap();
        assert_effect_labels(&fallback.effects, &["ShowCopyResult"]);
        assert_state(&h, SessionState::CopyReady);
        assert!(!h.executor.trace.contains(&"AttemptInsertionOnce"));
        assert!(!h.executor.trace.contains(&"ExplicitCopyRequested"));
    }
}

#[test]
fn secure_target_captured_at_start_never_gets_a_direct_insert() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let secure = TargetSnapshot::new(
        OpaqueTargetId::new(TARGET_ID),
        3,
        TargetSecurity::SecureOrProtected,
        TargetEditability::Editable,
    );
    h.send(EventKind::ShortcutActivated {
        mode: RecordingMode::PushToTalk,
        target: secure,
    })
    .unwrap();
    let id = h.reducer.status().session_id.unwrap();
    h.send(EventKind::ModelPrepared { session_id: id }).unwrap();
    h.send(EventKind::MicrophoneStarted { session_id: id })
        .unwrap();
    h.send(EventKind::InputCaptured { session_id: id }).unwrap();
    h.send(EventKind::KeyReleased { session_id: id }).unwrap();
    h.send(EventKind::FinalTranscript {
        session_id: id,
        text: "synthetic private result".to_owned(),
    })
    .unwrap();
    h.send(EventKind::FinalTextPrepared {
        session_id: id,
        text: "synthetic private result".to_owned(),
    })
    .unwrap();
    let result = h
        .send(EventKind::TargetVerified {
            session_id: id,
            result: TargetVerification::Current(secure),
        })
        .unwrap();
    assert_effect_labels(&result.effects, &["ShowCopyResult"]);
    assert!(!h.executor.trace.contains(&"AttemptInsertionOnce"));
}

#[test]
fn target_snapshot_exposes_only_bounded_identity_metadata_and_redacts_debug() {
    let snapshot = target();
    assert_eq!(snapshot.identity().get(), TARGET_ID);
    assert_eq!(snapshot.generation(), 3);
    let diagnostics = format!("{snapshot:?}");
    assert!(!diagnostics.contains(&TARGET_ID.to_string()));
    assert!(diagnostics.contains("[redacted]"));
}

#[test]
fn definite_insert_failure_is_copy_ready_and_never_retries_insert() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    make_copy_ready(&mut h, id, InsertionOutcome::FailedWithoutSideEffect);
    assert!(h.reducer.status().insertion_attempted);
    let attempts = h
        .executor
        .trace
        .iter()
        .filter(|e| **e == "AttemptInsertionOnce")
        .count();
    assert_eq!(attempts, 1);
    assert_eq!(
        h.send(EventKind::TargetVerified {
            session_id: id,
            result: TargetVerification::Current(target()),
        })
        .unwrap_err(),
        ReducerError::InvalidTransition
    );
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "AttemptInsertionOnce")
            .count(),
        1
    );
}

#[test]
fn uncertain_insertion_requires_review_and_has_no_automatic_retry_or_copy() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    make_copy_ready(&mut h, id, InsertionOutcome::UncertainOrPartial);
    assert!(!h.executor.trace.contains(&"ShowCopyResult"));
    assert!(!h.executor.trace.contains(&"ExplicitCopyRequested"));
    assert_eq!(
        h.send(EventKind::Cancel { session_id: id }).unwrap_err(),
        ReducerError::CancellationTooLate
    );
    let reviewed = h
        .send(EventKind::UncertaintyReviewed { session_id: id })
        .unwrap();
    assert_effect_labels(&reviewed.effects, &["ShowCopyResult"]);
    assert_state(&h, SessionState::CopyReady);
    assert_eq!(
        h.executor
            .trace
            .iter()
            .filter(|e| **e == "AttemptInsertionOnce")
            .count(),
        1
    );
}

#[test]
fn explicit_copy_is_correlated_single_flight_and_retryable_after_failure() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    make_copy_ready(&mut h, id, InsertionOutcome::Unsupported);
    assert!(!h.executor.trace.contains(&"ExplicitCopyRequested"));

    let first = h
        .send(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    assert_effect_labels(&first.effects, &["ExplicitCopyRequested"]);
    let first_attempt = match first.effects.as_slice() {
        [Effect::ExplicitCopyRequested { attempt_id, .. }] => *attempt_id,
        _ => panic!("one explicit Copy action must issue one correlated attempt"),
    };
    let repeated = h
        .send(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    assert_eq!(
        repeated.outcome,
        DispatchOutcome::Ignored(IgnoredEvent::CopyAlreadyPending)
    );
    assert!(repeated.effects.is_empty());

    let failed = h
        .send(EventKind::CopyCompleted {
            session_id: id,
            attempt_id: first_attempt,
            outcome: CopyOutcome::Failed,
        })
        .unwrap();
    assert_effect_labels(&failed.effects, &["CopyFailed"]);
    assert_state(&h, SessionState::CopyReady);

    let retry = h
        .send(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    let second_attempt = match retry.effects.as_slice() {
        [Effect::ExplicitCopyRequested { attempt_id, .. }] => *attempt_id,
        _ => panic!("a failed Copy allows a new explicit attempt"),
    };
    assert_ne!(first_attempt, second_attempt);
    assert_eq!(first_attempt.session_id(), id);
    assert_eq!(second_attempt.sequence(), first_attempt.sequence() + 1);
    assert_eq!(
        h.send(EventKind::CopyCompleted {
            session_id: id,
            attempt_id: first_attempt,
            outcome: CopyOutcome::Succeeded,
        })
        .unwrap_err(),
        ReducerError::CopyAttemptMismatch
    );
    assert_state(&h, SessionState::CopyReady);
    let completed = h
        .send(EventKind::CopyCompleted {
            session_id: id,
            attempt_id: second_attempt,
            outcome: CopyOutcome::Succeeded,
        })
        .unwrap();
    assert_effect_labels(&completed.effects, &["DismissAndClearResult"]);
    assert_state(&h, SessionState::Idle);
    assert_eq!(h.reducer.status().session_id, None);
}

#[test]
fn copy_completion_without_current_request_is_rejected_by_attempt_correlation() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let first_id = h.listen(RecordingMode::PushToTalk);
    make_copy_ready(&mut h, first_id, InsertionOutcome::Unsupported);
    let first_request = h
        .send(EventKind::CopyResultRequested {
            session_id: first_id,
        })
        .unwrap();
    let first_attempt = match first_request.effects.as_slice() {
        [Effect::ExplicitCopyRequested { attempt_id, .. }] => *attempt_id,
        _ => panic!("explicit Copy must issue one attempt"),
    };
    h.send(EventKind::CopyCompleted {
        session_id: first_id,
        attempt_id: first_attempt,
        outcome: CopyOutcome::Failed,
    })
    .unwrap();
    h.send(EventKind::Dismiss {
        session_id: Some(first_id),
    })
    .unwrap();

    let second_id = h.listen(RecordingMode::PushToTalk);
    make_copy_ready(&mut h, second_id, InsertionOutcome::Unsupported);
    assert_eq!(
        h.send(EventKind::CopyCompleted {
            session_id: second_id,
            attempt_id: first_attempt,
            outcome: CopyOutcome::Succeeded,
        })
        .unwrap_err(),
        ReducerError::CopyAttemptMismatch
    );
    assert_state(&h, SessionState::CopyReady);
}

#[test]
fn late_copy_acknowledgment_after_dismissal_is_rejected() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    make_copy_ready(&mut h, id, InsertionOutcome::Unsupported);
    let request = h
        .send(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    let attempt_id = match request.effects.as_slice() {
        [Effect::ExplicitCopyRequested { attempt_id, .. }] => *attempt_id,
        _ => panic!("explicit Copy must issue one attempt"),
    };
    h.send(EventKind::Dismiss {
        session_id: Some(id),
    })
    .unwrap();
    assert_eq!(
        h.send(EventKind::CopyCompleted {
            session_id: id,
            attempt_id,
            outcome: CopyOutcome::Succeeded,
        })
        .unwrap_err(),
        ReducerError::StaleSession
    );
    assert_state(&h, SessionState::Idle);
}

#[test]
fn explicit_copy_success_clears_result_and_returns_idle() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    make_copy_ready(&mut h, id, InsertionOutcome::TargetChanged);
    let request = h
        .send(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    let attempt_id = match request.effects.as_slice() {
        [Effect::ExplicitCopyRequested { attempt_id, .. }] => *attempt_id,
        _ => panic!("explicit Copy must issue one attempt"),
    };
    let result = h
        .send(EventKind::CopyCompleted {
            session_id: id,
            attempt_id,
            outcome: CopyOutcome::Succeeded,
        })
        .unwrap();
    assert_effect_labels(&result.effects, &["DismissAndClearResult"]);
    assert_state(&h, SessionState::Idle);
    assert!(h.executor.trace.contains(&"ExplicitCopyRequested"));
}

#[test]
fn cancellation_from_copy_ready_discards_text_and_cannot_be_copied_later() {
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    make_copy_ready(&mut h, id, InsertionOutcome::FailedWithoutSideEffect);
    let cancelled = h.send(EventKind::Cancel { session_id: id }).unwrap();
    assert_effect_labels(&cancelled.effects, &["DismissAndClearResult"]);
    assert_state(&h, SessionState::Cancelled);
    assert_eq!(
        h.send(EventKind::CopyResultRequested { session_id: id })
            .unwrap_err(),
        ReducerError::SessionClosed
    );
    h.send(EventKind::Dismiss {
        session_id: Some(id),
    })
    .unwrap();
    assert_state(&h, SessionState::Idle);
}

#[test]
fn no_transcript_values_appear_in_errors_status_events_effect_debug_or_diagnostics() {
    let sentinel = "private-synthetic-sentinel-73f1";
    let mut h = Harness::new(HoldReleaseCapability::Supported);
    let id = h.listen(RecordingMode::PushToTalk);
    let partial = h
        .send(EventKind::PartialTranscript {
            session_id: id,
            revision: 1,
            text: sentinel.to_owned(),
        })
        .unwrap();
    let event = SessionEvent::new(
        EventSequence(h.sequence + 1),
        EventKind::FinalTranscript {
            session_id: id,
            text: sentinel.to_owned(),
        },
    );
    let formatted_event = format!("{event:?}");
    let formatted_effect = format!("{:?}", partial.effects[0]);
    let formatted_status = format!("{:?}", h.reducer.status());
    let error = ReducerError::InvalidTransition;
    let mut formatted_error = error.to_string();
    write!(&mut formatted_error, " {error:?}").unwrap();
    for diagnostic in [
        formatted_event,
        formatted_effect,
        formatted_status,
        formatted_error,
    ] {
        assert!(!diagnostic.contains(sentinel));
    }
    assert_eq!(h.reducer.status().partial_revision, Some(1));
}
