use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use outflow_coordinator::{
    CoordinatorAdapters, CoordinatorCallback, CopyAdapter, CopyRequest, EffectObserver,
    PrivatePreview, ResultPanel, SessionCoordinator, SessionSettingsSnapshot,
};
use outflow_core::session_engine::{
    AdapterQueueError, CopyAttemptId, CopyOutcome, EventKind, HoldReleaseCapability,
    InsertionOutcome, MicrophoneStopOutcome, OpaqueTargetId, RecordingMode, SessionId,
    SessionState, SpeechBackend, SpeechCommand, StopOperationId, TargetCommand, TargetEditability,
    TargetSecurity, TargetService, TargetSnapshot, TargetSubmissionError, TargetVerification,
};
use outflow_settings::{CleanupMode, CleanupSettings, Settings, VocabularyEntry};

pub type TestCoordinator =
    SessionCoordinator<MockSpeech, MockTarget, MockPreview, MockPanel, MockCopy>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    PrepareModel,
    StartMicrophone,
    StopMicrophone,
    FinalizeRecognition,
    CancelRecognition,
    VerifyTarget,
    InsertOnce,
    PublishPartial,
    ClearPreview,
    ShowPanel,
    ManualReview,
    CopyFailedNotice,
    DismissPanel,
    Copy,
}

#[derive(Default)]
struct Shared {
    callbacks: VecDeque<CoordinatorCallback>,
    deferred_callbacks: VecDeque<CoordinatorCallback>,
    labels: Vec<&'static str>,
    failures: VecDeque<Operation>,
    verification_results: VecDeque<TargetVerification>,
    insertion_results: VecDeque<InsertionOutcome>,
    target_submission_errors: VecDeque<TargetSubmissionError>,
    stop_outcomes: VecDeque<MicrophoneStopOutcome>,
    copy_results: VecDeque<CopyOutcome>,
    copied_attempts: Vec<CopyAttemptId>,
    inserted_text: Vec<String>,
    insertion_attempts: usize,
    last_stop: Option<(SessionId, StopOperationId)>,
    delay_stop_acks: usize,
    suppress_stop_acks: usize,
    duplicate_stop_acks: usize,
    delay_finalization_acks: usize,
    suppress_finalization_acks: usize,
    fail_finalization_acks: usize,
    duplicate_finalization_acks: usize,
    delay_insertion_acks: usize,
    duplicate_insertion_acks: usize,
    preview_text: Option<String>,
    panel_text: Option<String>,
    delay_close_acks: usize,
    suppress_close_acks: usize,
    duplicate_close_acks: usize,
}

#[derive(Clone, Default)]
pub struct MockHandle(Rc<RefCell<Shared>>);

impl MockHandle {
    pub fn fail_next(&self, operation: Operation) {
        self.0.borrow_mut().failures.push_back(operation);
    }

    pub fn verify_next(&self, result: TargetVerification) {
        self.0.borrow_mut().verification_results.push_back(result);
    }

    pub fn insert_next(&self, result: InsertionOutcome) {
        self.0.borrow_mut().insertion_results.push_back(result);
    }

    pub fn unknown_next_insertion_submission(&self) {
        self.0
            .borrow_mut()
            .target_submission_errors
            .push_back(TargetSubmissionError::OutcomeUnknown);
    }

    pub fn stop_next(&self, outcome: MicrophoneStopOutcome) {
        self.0.borrow_mut().stop_outcomes.push_back(outcome);
    }

    pub fn delay_next_stop_ack(&self) {
        self.0.borrow_mut().delay_stop_acks += 1;
    }

    pub fn suppress_next_stop_ack(&self) {
        self.0.borrow_mut().suppress_stop_acks += 1;
    }

    pub fn duplicate_next_stop_ack(&self) {
        self.0.borrow_mut().duplicate_stop_acks += 1;
    }

    pub fn last_stop(&self) -> Option<(SessionId, StopOperationId)> {
        self.0.borrow().last_stop
    }

    pub fn delay_next_finalization_ack(&self) {
        self.0.borrow_mut().delay_finalization_acks += 1;
    }

    pub fn suppress_next_finalization_ack(&self) {
        self.0.borrow_mut().suppress_finalization_acks += 1;
    }

    pub fn fail_next_finalization_ack(&self) {
        self.0.borrow_mut().fail_finalization_acks += 1;
    }

    pub fn duplicate_next_finalization_ack(&self) {
        self.0.borrow_mut().duplicate_finalization_acks += 1;
    }

    pub fn delay_next_insertion_ack(&self) {
        self.0.borrow_mut().delay_insertion_acks += 1;
    }

    pub fn duplicate_next_insertion_ack(&self) {
        self.0.borrow_mut().duplicate_insertion_acks += 1;
    }

    pub fn delay_next_close_ack(&self) {
        self.0.borrow_mut().delay_close_acks += 1;
    }

    pub fn copy_next(&self, result: CopyOutcome) {
        self.0.borrow_mut().copy_results.push_back(result);
    }

    pub fn suppress_next_close_ack(&self) {
        self.0.borrow_mut().suppress_close_acks += 1;
    }

    pub fn duplicate_next_close_ack(&self) {
        self.0.borrow_mut().duplicate_close_acks += 1;
    }

    pub fn pop_callback(&self) -> Option<CoordinatorCallback> {
        self.0.borrow_mut().callbacks.pop_front()
    }

    pub fn pending_callbacks(&self) -> usize {
        self.0.borrow().callbacks.len()
    }

    pub fn pending_deferred_callbacks(&self) -> usize {
        self.0.borrow().deferred_callbacks.len()
    }

    pub fn release_next_deferred_callback(&self) -> bool {
        let mut shared = self.0.borrow_mut();
        let Some(callback) = shared.deferred_callbacks.pop_front() else {
            return false;
        };
        shared.callbacks.push_back(callback);
        true
    }

    pub fn enqueue_callback(&self, callback: CoordinatorCallback) {
        self.enqueue(callback);
    }

    pub fn labels(&self) -> Vec<&'static str> {
        self.0.borrow().labels.clone()
    }

    pub fn take_labels(&self) -> Vec<&'static str> {
        std::mem::take(&mut self.0.borrow_mut().labels)
    }

    pub fn copied_attempts(&self) -> Vec<CopyAttemptId> {
        self.0.borrow().copied_attempts.clone()
    }

    pub fn inserted_text(&self) -> Vec<String> {
        self.0.borrow().inserted_text.clone()
    }

    pub fn insertion_attempts(&self) -> usize {
        self.0.borrow().insertion_attempts
    }

    pub fn has_preview(&self) -> bool {
        self.0.borrow().preview_text.is_some()
    }

    pub fn has_panel_result(&self) -> bool {
        self.0.borrow().panel_text.is_some()
    }

    fn fails(&self, operation: Operation) -> bool {
        let mut shared = self.0.borrow_mut();
        if shared.failures.front() == Some(&operation) {
            shared.failures.pop_front();
            true
        } else {
            false
        }
    }

    fn enqueue(&self, callback: CoordinatorCallback) {
        self.0.borrow_mut().callbacks.push_back(callback);
    }

    fn enqueue_controlled(&self, callback: CoordinatorCallback, delay: bool, duplicate: bool) {
        let mut shared = self.0.borrow_mut();
        let queue = if delay {
            &mut shared.deferred_callbacks
        } else {
            &mut shared.callbacks
        };
        queue.push_back(callback.clone());
        if duplicate {
            queue.push_back(callback);
        }
    }

    fn enqueue_close(&self, session_id: SessionId) {
        let mut shared = self.0.borrow_mut();
        if shared.suppress_close_acks > 0 {
            shared.suppress_close_acks -= 1;
        } else {
            let delay = shared.delay_close_acks > 0;
            let duplicate = shared.duplicate_close_acks > 0;
            shared.delay_close_acks = shared.delay_close_acks.saturating_sub(1);
            shared.duplicate_close_acks = shared.duplicate_close_acks.saturating_sub(1);
            let callback = CoordinatorCallback::SpeechResourcesClosed { session_id };
            let queue = if delay {
                &mut shared.deferred_callbacks
            } else {
                &mut shared.callbacks
            };
            queue.push_back(callback.clone());
            if duplicate {
                queue.push_back(callback);
            }
        }
    }
}

pub struct MockSpeech(MockHandle);
pub struct MockTarget(MockHandle);
pub struct MockPreview(MockHandle);
pub struct MockPanel(MockHandle);
pub struct MockCopy(MockHandle);

pub struct MockObserver(MockHandle);

fn take_flag(counter: &mut usize) -> bool {
    if *counter == 0 {
        false
    } else {
        *counter -= 1;
        true
    }
}

impl EffectObserver for MockObserver {
    fn observe(&mut self, label: &'static str) {
        self.0.0.borrow_mut().labels.push(label);
    }
}

impl SpeechBackend for MockSpeech {
    fn submit(&mut self, command: SpeechCommand) -> Result<(), AdapterQueueError> {
        let operation = match &command {
            SpeechCommand::PrepareModel { .. } => Operation::PrepareModel,
            SpeechCommand::StartMicrophone { .. } => Operation::StartMicrophone,
            SpeechCommand::StopMicrophone { .. } => Operation::StopMicrophone,
            SpeechCommand::FinalizeRecognition { .. } => Operation::FinalizeRecognition,
            SpeechCommand::CancelRecognition { .. } => Operation::CancelRecognition,
        };
        if self.0.fails(operation) {
            return Err(AdapterQueueError::Unavailable);
        }
        match command {
            SpeechCommand::PrepareModel { session_id } => {
                self.0
                    .enqueue(CoordinatorCallback::Session(EventKind::ModelPrepared {
                        session_id,
                    }))
            }
            SpeechCommand::StartMicrophone { session_id } => {
                self.0
                    .enqueue(CoordinatorCallback::Session(EventKind::MicrophoneStarted {
                        session_id,
                    }));
                self.0
                    .enqueue(CoordinatorCallback::Session(EventKind::InputCaptured {
                        session_id,
                    }));
                self.0
                    .enqueue(CoordinatorCallback::Session(EventKind::PartialTranscript {
                        session_id,
                        revision: 1,
                        text: "um, synthetic partial".to_owned(),
                    }));
                self.0
                    .enqueue(CoordinatorCallback::Session(EventKind::PartialTranscript {
                        session_id,
                        revision: 2,
                        text: "um, revised synthetic partial".to_owned(),
                    }));
            }
            SpeechCommand::StopMicrophone {
                session_id,
                operation_id,
            } => {
                let (outcome, suppressed, delayed, duplicate) = {
                    let mut shared = self.0.0.borrow_mut();
                    shared.last_stop = Some((session_id, operation_id));
                    let outcome = shared
                        .stop_outcomes
                        .pop_front()
                        .unwrap_or(MicrophoneStopOutcome::Stopped);
                    let suppressed = take_flag(&mut shared.suppress_stop_acks);
                    let delayed = take_flag(&mut shared.delay_stop_acks);
                    let duplicate = take_flag(&mut shared.duplicate_stop_acks);
                    (outcome, suppressed, delayed, duplicate)
                };
                if !suppressed {
                    self.0.enqueue_controlled(
                        CoordinatorCallback::MicrophoneStopCompleted {
                            session_id,
                            operation_id,
                            outcome,
                        },
                        delayed,
                        duplicate,
                    );
                }
            }
            SpeechCommand::FinalizeRecognition { session_id } => {
                let (suppressed, delayed, failed, duplicate) = {
                    let mut shared = self.0.0.borrow_mut();
                    (
                        take_flag(&mut shared.suppress_finalization_acks),
                        take_flag(&mut shared.delay_finalization_acks),
                        take_flag(&mut shared.fail_finalization_acks),
                        take_flag(&mut shared.duplicate_finalization_acks),
                    )
                };
                if !suppressed {
                    let event = if failed {
                        EventKind::RecognitionFailed { session_id }
                    } else {
                        EventKind::FinalTranscript {
                            session_id,
                            text: "um, alpha sample".to_owned(),
                        }
                    };
                    self.0.enqueue_controlled(
                        CoordinatorCallback::Session(event),
                        delayed,
                        duplicate,
                    );
                }
                self.0.enqueue_close(session_id);
            }
            SpeechCommand::CancelRecognition { session_id } => self.0.enqueue_close(session_id),
        }
        Ok(())
    }
}

impl TargetService for MockTarget {
    fn submit(&mut self, command: TargetCommand) -> Result<(), TargetSubmissionError> {
        match command {
            TargetCommand::VerifyCurrent {
                session_id,
                captured,
            } => {
                if self.0.fails(Operation::VerifyTarget) {
                    return Err(TargetSubmissionError::DefinitelyNotDispatched);
                }
                let result = self
                    .0
                    .0
                    .borrow_mut()
                    .verification_results
                    .pop_front()
                    .unwrap_or(TargetVerification::Current(captured));
                self.0
                    .enqueue(CoordinatorCallback::Session(EventKind::TargetVerified {
                        session_id,
                        result,
                    }));
            }
            TargetCommand::InsertOnce {
                session_id, text, ..
            } => {
                self.0.0.borrow_mut().insertion_attempts += 1;
                if self.0.fails(Operation::InsertOnce) {
                    return Err(TargetSubmissionError::DefinitelyNotDispatched);
                }
                let (submission_error, outcome, delayed, duplicate) = {
                    let mut shared = self.0.0.borrow_mut();
                    let submission_error = shared.target_submission_errors.pop_front();
                    let outcome = shared
                        .insertion_results
                        .pop_front()
                        .unwrap_or(InsertionOutcome::Inserted);
                    let delayed = take_flag(&mut shared.delay_insertion_acks);
                    let duplicate = take_flag(&mut shared.duplicate_insertion_acks);
                    if submission_error == Some(TargetSubmissionError::OutcomeUnknown)
                        || submission_error.is_none()
                            && matches!(
                                outcome,
                                InsertionOutcome::Inserted | InsertionOutcome::UncertainOrPartial
                            )
                    {
                        shared.inserted_text.push(text.clone());
                    }
                    (submission_error, outcome, delayed, duplicate)
                };
                if let Some(error) = submission_error {
                    return Err(error);
                }
                self.0.enqueue_controlled(
                    CoordinatorCallback::Session(EventKind::InsertionCompleted {
                        session_id,
                        outcome,
                    }),
                    delayed,
                    duplicate,
                );
            }
        }
        Ok(())
    }
}

impl PrivatePreview for MockPreview {
    fn publish_partial(
        &mut self,
        _session_id: SessionId,
        _revision: u64,
        text: &str,
    ) -> Result<(), AdapterQueueError> {
        if self.0.fails(Operation::PublishPartial) {
            return Err(AdapterQueueError::Unavailable);
        }
        self.0.0.borrow_mut().preview_text = Some(text.to_owned());
        Ok(())
    }

    fn clear(&mut self, _session_id: SessionId) -> Result<(), AdapterQueueError> {
        if self.0.fails(Operation::ClearPreview) {
            return Err(AdapterQueueError::Unavailable);
        }
        self.0.0.borrow_mut().preview_text = None;
        Ok(())
    }
}

impl ResultPanel for MockPanel {
    fn show_copy_result(
        &mut self,
        _session_id: SessionId,
        text: &str,
    ) -> Result<(), AdapterQueueError> {
        if self.0.fails(Operation::ShowPanel) {
            return Err(AdapterQueueError::Unavailable);
        }
        self.0.0.borrow_mut().panel_text = Some(text.to_owned());
        Ok(())
    }

    fn require_manual_review(&mut self, _session_id: SessionId) -> Result<(), AdapterQueueError> {
        if self.0.fails(Operation::ManualReview) {
            return Err(AdapterQueueError::Unavailable);
        }
        Ok(())
    }

    fn copy_failed(&mut self, _session_id: SessionId) -> Result<(), AdapterQueueError> {
        if self.0.fails(Operation::CopyFailedNotice) {
            return Err(AdapterQueueError::Unavailable);
        }
        Ok(())
    }

    fn dismiss_and_clear(&mut self, _session_id: SessionId) -> Result<(), AdapterQueueError> {
        if self.0.fails(Operation::DismissPanel) {
            return Err(AdapterQueueError::Unavailable);
        }
        self.0.0.borrow_mut().panel_text = None;
        Ok(())
    }
}

impl CopyAdapter for MockCopy {
    fn submit(&mut self, request: CopyRequest) -> Result<(), AdapterQueueError> {
        if self.0.fails(Operation::Copy) {
            return Err(AdapterQueueError::Unavailable);
        }
        let session_id = request.session_id();
        let attempt_id = request.attempt_id();
        // This deterministic mock proves request delivery without writing a system clipboard.
        let _synthetic_text_was_supplied = !request.text().is_empty();
        self.0.0.borrow_mut().copied_attempts.push(attempt_id);
        let outcome = self
            .0
            .0
            .borrow_mut()
            .copy_results
            .pop_front()
            .unwrap_or(CopyOutcome::Succeeded);
        self.0
            .enqueue(CoordinatorCallback::Session(EventKind::CopyCompleted {
                session_id,
                attempt_id,
                outcome,
            }));
        Ok(())
    }
}

pub fn coordinator(settings: SessionSettingsSnapshot) -> (TestCoordinator, MockHandle) {
    let handle = MockHandle::default();
    let make_handle = || handle.clone();
    let coordinator = SessionCoordinator::with_observer(
        HoldReleaseCapability::Supported,
        settings,
        CoordinatorAdapters::new(
            MockSpeech(make_handle()),
            MockTarget(make_handle()),
            MockPreview(make_handle()),
            MockPanel(make_handle()),
            MockCopy(make_handle()),
        ),
        Box::new(MockObserver(make_handle())),
    );
    (coordinator, handle)
}

pub fn default_settings() -> SessionSettingsSnapshot {
    SessionSettingsSnapshot::from_settings(&Settings::default())
}

pub fn clean_snapshot(vocabulary: Vec<VocabularyEntry>) -> SessionSettingsSnapshot {
    SessionSettingsSnapshot::new(
        CleanupSettings {
            mode: CleanupMode::Clean,
            remove_fillers: true,
        },
        vocabulary,
    )
}

pub fn target() -> TargetSnapshot {
    TargetSnapshot::new(
        OpaqueTargetId::new(0x0F10),
        1,
        TargetSecurity::Ordinary,
        TargetEditability::Editable,
    )
}

pub fn start(coordinator: &mut TestCoordinator, mode: RecordingMode) -> SessionId {
    coordinator
        .dispatch(EventKind::ShortcutActivated {
            mode,
            target: target(),
        })
        .expect("synthetic shortcut action is accepted");
    coordinator.status().session.session_id.unwrap()
}

pub fn initialize(coordinator: &mut TestCoordinator) {
    coordinator
        .dispatch(EventKind::InitializationSucceeded)
        .expect("synthetic initialization is accepted");
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PumpSummary {
    pub accepted: usize,
    pub rejected: usize,
}

pub fn pump(coordinator: &mut TestCoordinator, handle: &MockHandle) -> PumpSummary {
    let mut summary = PumpSummary::default();
    let mut limit = 0;
    while let Some(callback) = handle.pop_callback() {
        limit += 1;
        assert!(limit < 1000, "mock callbacks must quiesce");
        match coordinator.accept_callback(callback) {
            Ok(_) => summary.accepted += 1,
            Err(_) => summary.rejected += 1,
        }
    }
    summary
}

pub fn finish_ptt(coordinator: &mut TestCoordinator, session_id: SessionId) {
    coordinator
        .dispatch(EventKind::KeyReleased { session_id })
        .expect("synthetic PTT release is accepted");
}

pub fn stop_toggle(coordinator: &mut TestCoordinator, session_id: SessionId) {
    coordinator
        .dispatch(EventKind::ShortcutActivated {
            mode: RecordingMode::Toggle,
            target: target(),
        })
        .expect("second toggle action is accepted");
    assert_eq!(coordinator.status().session.session_id, Some(session_id));
}

pub fn current_state(coordinator: &TestCoordinator) -> SessionState {
    coordinator.status().session.state
}
