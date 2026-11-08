mod support;

use outflow_coordinator::{
    CoordinatorCallback, CoordinatorError, ResourceCleanupStatus, SessionSettingsSnapshot,
};
use outflow_core::session_engine::{
    CopyOutcome, DispatchOutcome, EventKind, FailureCode, IgnoredEvent, InsertionOutcome,
    RecordingMode, ReducerError, SessionState, TargetSecurity, TargetVerification,
};
use outflow_settings::{CleanupMode, CleanupSettings, VocabularyEntry};
use support::{
    MockHandle, Operation, clean_snapshot, coordinator, current_state, default_settings,
    finish_ptt, initialize, pump, start, stop_toggle, target,
};

fn vocabulary(spoken: &str, written: &str) -> VocabularyEntry {
    VocabularyEntry {
        id: "synthetic-entry".to_owned(),
        spoken: spoken.to_owned(),
        written: written.to_owned(),
    }
}

fn copy_ready(handle: &MockHandle, coordinator: &mut support::TestCoordinator) -> u64 {
    handle.verify_next(TargetVerification::Changed);
    let id = start(coordinator, RecordingMode::PushToTalk);
    pump(coordinator, handle);
    finish_ptt(coordinator, id);
    pump(coordinator, handle);
    assert_eq!(current_state(coordinator), SessionState::CopyReady);
    id.get()
}

#[test]
fn push_to_talk_routes_full_ordered_effects_and_cleans_once() {
    let settings = clean_snapshot(vec![vocabulary("alpha", "Alpha")]);
    let (mut coordinator, handle) = coordinator(settings);
    initialize(&mut coordinator);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::Listening);
    assert_eq!(
        handle.labels(),
        [
            "RequestModelPreparation",
            "RequestMicrophoneStart",
            "PublishPrivatePartial",
            "PublishPrivatePartial",
        ]
    );
    assert!(handle.has_preview());

    finish_ptt(&mut coordinator, id);
    pump(&mut coordinator, &handle);

    assert_eq!(current_state(&coordinator), SessionState::Idle);
    assert_eq!(
        handle.labels(),
        [
            "RequestModelPreparation",
            "RequestMicrophoneStart",
            "PublishPrivatePartial",
            "PublishPrivatePartial",
            "RequestMicrophoneStop",
            "RequestAsrFinalization",
            "PrepareFinalText",
            "RequestTargetVerification",
            "AttemptInsertionOnce",
            "ClearPrivateResult",
        ]
    );
    assert_eq!(
        handle
            .labels()
            .iter()
            .filter(|&&l| l == "PrepareFinalText")
            .count(),
        1
    );
    assert_eq!(
        handle
            .labels()
            .iter()
            .filter(|&&l| l == "AttemptInsertionOnce")
            .count(),
        1
    );
    assert_eq!(handle.inserted_text(), ["Alpha sample"]);
    assert!(!handle.has_preview());
    assert!(
        handle.copied_attempts().is_empty(),
        "insertion success must not copy"
    );
    assert_eq!(
        coordinator.status().cleanup,
        ResourceCleanupStatus::Resolved
    );
}

#[test]
fn toggle_workflow_ignores_key_up_and_stops_on_second_activation() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let id = start(&mut coordinator, RecordingMode::Toggle);
    pump(&mut coordinator, &handle);
    let ignored = coordinator
        .dispatch(EventKind::KeyReleased { session_id: id })
        .unwrap();
    assert_eq!(ignored, DispatchOutcome::Ignored(IgnoredEvent::ToggleKeyUp));
    assert_eq!(current_state(&coordinator), SessionState::Listening);
    stop_toggle(&mut coordinator, id);
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::Idle);
    let labels = handle.labels();
    assert_eq!(
        labels
            .iter()
            .filter(|&&l| l == "RequestMicrophoneStart")
            .count(),
        1
    );
    assert_eq!(
        labels
            .iter()
            .filter(|&&l| l == "RequestMicrophoneStop")
            .count(),
        1
    );
    assert_eq!(
        labels
            .iter()
            .filter(|&&l| l == "RequestAsrFinalization")
            .count(),
        1
    );
    assert_eq!(
        labels
            .iter()
            .filter(|&&l| l == "AttemptInsertionOnce")
            .count(),
        1
    );
}

#[test]
fn model_preparation_enqueue_failure_never_starts_microphone() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    handle.fail_next(Operation::PrepareModel);
    start(&mut coordinator, RecordingMode::PushToTalk);
    assert_eq!(current_state(&coordinator), SessionState::Faulted);
    assert_eq!(
        handle.labels(),
        [
            "RequestModelPreparation",
            "RequestAsrCancellation",
            "ClearPrivateResult",
            "ReportFailure",
        ]
    );
    assert!(!handle.labels().contains(&"RequestMicrophoneStart"));
    assert_eq!(
        coordinator.status().session.failure,
        Some(FailureCode::ModelPreparationFailed)
    );
    pump(&mut coordinator, &handle);
    assert_eq!(
        coordinator.status().cleanup,
        ResourceCleanupStatus::Resolved
    );
}

#[test]
fn microphone_start_enqueue_failure_cancels_in_order_without_listening() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    handle.fail_next(Operation::StartMicrophone);
    start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::Faulted);
    assert_eq!(
        handle.labels(),
        [
            "RequestModelPreparation",
            "RequestMicrophoneStart",
            "RequestMicrophoneStop",
            "RequestAsrCancellation",
            "ClearPrivateResult",
            "ReportFailure",
        ]
    );
    assert_eq!(
        coordinator.status().session.failure,
        Some(FailureCode::MicrophoneStartFailed)
    );
    assert!(!handle.labels().contains(&"AttemptInsertionOnce"));
    assert_eq!(
        coordinator.status().cleanup,
        ResourceCleanupStatus::Resolved
    );
}

#[test]
fn failed_microphone_stop_prevents_finalization_and_reports_failure() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    handle.fail_next(Operation::StopMicrophone);
    finish_ptt(&mut coordinator, id);
    assert_eq!(current_state(&coordinator), SessionState::Faulted);
    assert_eq!(
        handle.labels(),
        [
            "RequestModelPreparation",
            "RequestMicrophoneStart",
            "PublishPrivatePartial",
            "PublishPrivatePartial",
            "RequestMicrophoneStop",
            "RequestAsrCancellation",
            "ClearPrivateResult",
            "ReportFailure",
        ]
    );
    assert!(!handle.labels().contains(&"RequestAsrFinalization"));
    assert!(!handle.labels().contains(&"AttemptInsertionOnce"));
}

#[test]
fn failed_recognition_finalize_never_verifies_or_inserts() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    handle.fail_next(Operation::FinalizeRecognition);
    finish_ptt(&mut coordinator, id);
    assert_eq!(current_state(&coordinator), SessionState::Faulted);
    assert!(handle.labels().contains(&"RequestAsrCancellation"));
    assert!(handle.labels().contains(&"ReportFailure"));
    assert!(!handle.labels().contains(&"RequestTargetVerification"));
    assert!(!handle.labels().contains(&"AttemptInsertionOnce"));
}

#[test]
fn failed_target_verification_falls_back_to_copy_without_insertion() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    handle.fail_next(Operation::VerifyTarget);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    finish_ptt(&mut coordinator, id);
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::CopyReady);
    assert!(handle.has_panel_result());
    assert!(!handle.labels().contains(&"AttemptInsertionOnce"));
    assert!(handle.copied_attempts().is_empty());
}

#[test]
fn failed_insert_enqueue_is_definite_fallback_and_never_retried() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    handle.fail_next(Operation::InsertOnce);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    finish_ptt(&mut coordinator, id);
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::CopyReady);
    assert_eq!(
        handle
            .labels()
            .iter()
            .filter(|&&l| l == "AttemptInsertionOnce")
            .count(),
        1
    );
    assert_eq!(handle.inserted_text().len(), 0);
    assert!(handle.copied_attempts().is_empty());
}

#[test]
fn uncertain_insertion_requires_manual_review_and_never_copies_automatically() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    handle.insert_next(InsertionOutcome::UncertainOrPartial);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    finish_ptt(&mut coordinator, id);
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::Uncertain);
    assert!(handle.labels().contains(&"RequireManualReview"));
    assert!(!handle.labels().contains(&"ShowCopyResult"));
    assert!(handle.copied_attempts().is_empty());
    coordinator
        .dispatch(EventKind::UncertaintyReviewed { session_id: id })
        .unwrap();
    assert_eq!(current_state(&coordinator), SessionState::CopyReady);
    assert!(handle.labels().contains(&"ShowCopyResult"));
    assert!(handle.copied_attempts().is_empty());
}

#[test]
fn changed_target_copy_success_uses_matching_explicit_attempt() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let raw_id = copy_ready(&handle, &mut coordinator);
    let id = coordinator.status().session.session_id.unwrap();
    assert_eq!(id.get(), raw_id);
    coordinator
        .dispatch(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::Idle);
    assert_eq!(handle.copied_attempts().len(), 1);
    assert!(handle.labels().contains(&"ExplicitCopyRequested"));
    assert!(handle.labels().contains(&"DismissAndClearResult"));
    assert!(!handle.has_panel_result());
}

#[test]
fn copy_failure_retains_text_and_only_a_new_explicit_action_retries() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let _ = copy_ready(&handle, &mut coordinator);
    let id = coordinator.status().session.session_id.unwrap();
    handle.copy_next(CopyOutcome::Failed);
    coordinator
        .dispatch(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::CopyReady);
    assert!(handle.has_panel_result());
    let first = handle.copied_attempts()[0];

    let duplicate = coordinator
        .dispatch(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    assert_eq!(duplicate, DispatchOutcome::Applied);
    let second = handle.copied_attempts().last().copied().unwrap();
    assert_ne!(first, second);
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::Idle);
    assert_eq!(handle.copied_attempts(), [first, second]);
}

#[test]
fn repeated_pending_copy_clicks_do_not_enqueue_another_attempt() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let _ = copy_ready(&handle, &mut coordinator);
    let id = coordinator.status().session.session_id.unwrap();
    coordinator
        .dispatch(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    let before = handle.labels().len();
    let outcome = coordinator
        .dispatch(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    assert_eq!(
        outcome,
        DispatchOutcome::Ignored(IgnoredEvent::CopyAlreadyPending)
    );
    assert_eq!(handle.labels().len(), before);
    assert_eq!(handle.copied_attempts().len(), 1);
}

#[test]
fn mismatched_copy_attempt_and_completion_without_request_are_rejected() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let first_raw_id = copy_ready(&handle, &mut coordinator);
    let first_id = coordinator.status().session.session_id.unwrap();
    coordinator
        .dispatch(EventKind::CopyResultRequested {
            session_id: first_id,
        })
        .unwrap();
    let first_callback = handle.pop_callback().expect("mock Copy callback is queued");
    let first_attempt = match first_callback {
        CoordinatorCallback::Session(EventKind::CopyCompleted { attempt_id, .. }) => attempt_id,
        _ => panic!("mock Copy returns its correlated attempt"),
    };
    assert_eq!(first_id.get(), first_raw_id);
    coordinator
        .dispatch(EventKind::CopyCompleted {
            session_id: first_id,
            attempt_id: first_attempt,
            outcome: CopyOutcome::Succeeded,
        })
        .unwrap();
    assert_eq!(current_state(&coordinator), SessionState::Idle);

    let _ = copy_ready(&handle, &mut coordinator);
    let second_id = coordinator.status().session.session_id.unwrap();
    assert_eq!(
        coordinator.dispatch(EventKind::CopyCompleted {
            session_id: second_id,
            attempt_id: first_attempt,
            outcome: CopyOutcome::Succeeded,
        }),
        Err(CoordinatorError::Reducer(ReducerError::CopyAttemptMismatch))
    );
    assert_eq!(current_state(&coordinator), SessionState::CopyReady);
}

#[test]
fn copy_ack_after_dismissal_is_rejected_and_cannot_clear_a_new_result() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let _ = copy_ready(&handle, &mut coordinator);
    let id = coordinator.status().session.session_id.unwrap();
    coordinator
        .dispatch(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    let late_ack = handle.pop_callback().expect("mock Copy callback is queued");
    coordinator
        .dispatch(EventKind::Dismiss {
            session_id: Some(id),
        })
        .unwrap();
    assert!(coordinator.accept_callback(late_ack).is_err());
    assert_eq!(current_state(&coordinator), SessionState::Idle);
    assert!(!handle.has_panel_result());
}

#[test]
fn cancellation_rejects_late_recognition_and_cleanup_completions() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    finish_ptt(&mut coordinator, id);
    coordinator
        .dispatch(EventKind::Cancel { session_id: id })
        .unwrap();
    let summary = pump(&mut coordinator, &handle);
    assert!(summary.rejected >= 1, "late final should be rejected");
    assert_eq!(current_state(&coordinator), SessionState::Cancelled);
    assert!(!handle.labels().contains(&"RequestTargetVerification"));
    assert!(!handle.labels().contains(&"AttemptInsertionOnce"));
    assert_eq!(
        coordinator.dispatch(EventKind::FinalTextPrepared {
            session_id: id,
            text: "late private cleanup result".to_owned(),
        }),
        Err(CoordinatorError::Reducer(ReducerError::SessionClosed))
    );
    assert_eq!(current_state(&coordinator), SessionState::Cancelled);
}

#[test]
fn settings_and_vocabulary_are_snapshotted_per_session() {
    let original = clean_snapshot(vec![vocabulary("alpha", "InitialName")]);
    let (mut coordinator, handle) = coordinator(original);
    initialize(&mut coordinator);
    let first = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    coordinator.update_settings(SessionSettingsSnapshot::new(
        CleanupSettings {
            mode: CleanupMode::Verbatim,
            remove_fillers: false,
        },
        vec![vocabulary("alpha", "ChangedName")],
    ));
    finish_ptt(&mut coordinator, first);
    pump(&mut coordinator, &handle);
    assert_eq!(handle.inserted_text()[0], "InitialName sample");

    let second = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    finish_ptt(&mut coordinator, second);
    pump(&mut coordinator, &handle);
    let inserted = handle.inserted_text();
    assert!(inserted[1].contains("ChangedName"));
    assert!(inserted[1].starts_with("um"));
}

#[test]
fn missing_speech_closure_ack_is_exposed_as_unresolved_and_blocks_new_session() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    handle.suppress_next_close_ack();
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    finish_ptt(&mut coordinator, id);
    pump(&mut coordinator, &handle);
    assert_eq!(
        coordinator.status().cleanup,
        ResourceCleanupStatus::Unresolved { session_id: id }
    );
    assert_eq!(
        coordinator.dispatch(EventKind::ShortcutActivated {
            mode: RecordingMode::PushToTalk,
            target: target(),
        }),
        Err(CoordinatorError::CleanupUnresolved)
    );
    assert_eq!(current_state(&coordinator), SessionState::Idle);
}

#[test]
fn uncertain_insert_side_effect_never_retries_or_copies_without_review() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    handle.insert_next(InsertionOutcome::UncertainOrPartial);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    finish_ptt(&mut coordinator, id);
    pump(&mut coordinator, &handle);
    let labels = handle.labels();
    assert_eq!(
        labels
            .iter()
            .filter(|&&label| label == "AttemptInsertionOnce")
            .count(),
        1
    );
    assert!(!labels.contains(&"ExplicitCopyRequested"));
    assert!(!labels.contains(&"ShowCopyResult"));
    assert!(handle.copied_attempts().is_empty());
    assert_eq!(coordinator.status().session.state, SessionState::Uncertain);
    assert_eq!(coordinator.status().session.failure, None);
    assert_eq!(
        coordinator.dispatch(EventKind::CopyResultRequested { session_id: id }),
        Err(CoordinatorError::Reducer(ReducerError::InvalidTransition))
    );
    assert!(handle.copied_attempts().is_empty());
}

#[test]
fn adapter_errors_status_and_trace_never_expose_transcript_content() {
    const SECRET: &str = "do-not-print-this-transcript";
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    finish_ptt(&mut coordinator, id);
    coordinator
        .dispatch(EventKind::Cancel { session_id: id })
        .unwrap();
    let error = coordinator
        .dispatch(EventKind::FinalTranscript {
            session_id: id,
            text: SECRET.to_owned(),
        })
        .unwrap_err();
    let diagnostics = format!("{:?} {} {:?}", coordinator.status(), error, handle.labels());
    assert!(!diagnostics.contains(SECRET));
    assert!(
        !format!(
            "{:?}",
            CoordinatorCallback::Session(EventKind::FinalTranscript {
                session_id: id,
                text: SECRET.to_owned(),
            })
        )
        .contains(SECRET)
    );
}

#[test]
fn one_hundred_complete_mock_sessions_leave_no_pending_state() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    for _ in 0..100 {
        let id = start(&mut coordinator, RecordingMode::PushToTalk);
        pump(&mut coordinator, &handle);
        finish_ptt(&mut coordinator, id);
        pump(&mut coordinator, &handle);
        assert_eq!(current_state(&coordinator), SessionState::Idle);
        assert_eq!(coordinator.status().session.session_id, None);
        assert_eq!(
            coordinator.status().cleanup,
            ResourceCleanupStatus::Resolved
        );
        assert_eq!(handle.pending_callbacks(), 0);
        assert!(!handle.has_preview());
        assert!(!handle.has_panel_result());
        handle.take_labels();
    }
}

#[test]
fn panel_clear_failure_is_visible_and_blocks_next_session_until_recovered() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let id = copy_ready(&handle, &mut coordinator);
    handle.fail_next(Operation::DismissPanel);
    coordinator
        .dispatch(EventKind::Dismiss {
            session_id: Some(coordinator.status().session.session_id.unwrap()),
        })
        .unwrap();
    assert!(
        coordinator
            .status()
            .presentation
            .volatile_content_clear_unresolved
    );
    assert_eq!(
        coordinator.dispatch(EventKind::ShortcutActivated {
            mode: RecordingMode::PushToTalk,
            target: target(),
        }),
        Err(CoordinatorError::VolatileContentUnresolved)
    );
    assert_eq!(coordinator.status().session.state, SessionState::Idle);
    assert!(handle.has_panel_result());
    let _ = id;
}

#[test]
fn target_protection_falls_back_without_any_insertion_or_automatic_copy() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    handle.verify_next(TargetVerification::SecureOrProtected);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    finish_ptt(&mut coordinator, id);
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::CopyReady);
    assert_eq!(handle.inserted_text().len(), 0);
    assert!(handle.copied_attempts().is_empty());
    assert_eq!(target().security(), TargetSecurity::Ordinary);
}

#[test]
fn duplicate_model_ack_is_rejected_without_duplicate_start_effect() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    let original = handle.pop_callback().expect("model ack is queued");
    coordinator.accept_callback(original).unwrap();
    assert_eq!(
        coordinator.accept_callback(CoordinatorCallback::Session(EventKind::ModelPrepared {
            session_id: id,
        })),
        Err(CoordinatorError::Reducer(ReducerError::InvalidTransition))
    );
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::Listening);
    assert_eq!(
        handle
            .labels()
            .iter()
            .filter(|&&l| l == "RequestMicrophoneStart")
            .count(),
        1
    );
}

#[test]
fn cancellation_stop_and_cancel_queue_failures_remain_unresolved() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let id = start(&mut coordinator, RecordingMode::PushToTalk);
    pump(&mut coordinator, &handle);
    handle.fail_next(Operation::StopMicrophone);
    handle.fail_next(Operation::CancelRecognition);
    coordinator
        .dispatch(EventKind::Cancel { session_id: id })
        .unwrap();
    assert_eq!(
        handle.labels(),
        [
            "RequestModelPreparation",
            "RequestMicrophoneStart",
            "PublishPrivatePartial",
            "PublishPrivatePartial",
            "RequestMicrophoneStop",
            "RequestAsrCancellation",
            "ClearPrivateResult",
        ]
    );
    assert_eq!(
        coordinator.status().cleanup,
        ResourceCleanupStatus::Unresolved { session_id: id }
    );
    coordinator
        .dispatch(EventKind::Dismiss {
            session_id: Some(id),
        })
        .unwrap();
    assert_eq!(
        coordinator.dispatch(EventKind::ShortcutActivated {
            mode: RecordingMode::PushToTalk,
            target: target(),
        }),
        Err(CoordinatorError::CleanupUnresolved)
    );
}

#[test]
fn copy_enqueue_failure_is_a_failed_attempt_that_allows_explicit_retry() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let _ = copy_ready(&handle, &mut coordinator);
    let id = coordinator.status().session.session_id.unwrap();
    handle.fail_next(Operation::Copy);
    coordinator
        .dispatch(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    assert_eq!(current_state(&coordinator), SessionState::CopyReady);
    assert!(
        !coordinator
            .status()
            .presentation
            .copy_failure_notice_unresolved
    );
    assert!(handle.has_panel_result());
    assert!(handle.copied_attempts().is_empty());
    coordinator
        .dispatch(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    pump(&mut coordinator, &handle);
    assert_eq!(current_state(&coordinator), SessionState::Idle);
    assert_eq!(handle.copied_attempts().len(), 1);
}

#[test]
fn copy_ack_after_cancellation_is_rejected() {
    let (mut coordinator, handle) = coordinator(default_settings());
    initialize(&mut coordinator);
    let _ = copy_ready(&handle, &mut coordinator);
    let id = coordinator.status().session.session_id.unwrap();
    coordinator
        .dispatch(EventKind::CopyResultRequested { session_id: id })
        .unwrap();
    let late_ack = handle.pop_callback().expect("mock Copy callback is queued");
    coordinator
        .dispatch(EventKind::Cancel { session_id: id })
        .unwrap();
    assert!(coordinator.accept_callback(late_ack).is_err());
    assert_eq!(current_state(&coordinator), SessionState::Cancelled);
    assert!(!handle.has_panel_result());
}
