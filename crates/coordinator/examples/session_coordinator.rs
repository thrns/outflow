//! Runnable deterministic example of the reducer-to-adapter effect boundary.
//! The mock speech, target, panel, preview, and Copy adapters never contact an
//! operating system, create a microphone, or write a clipboard.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::error::Error;
use std::rc::Rc;

use outflow_coordinator::{
    CoordinatorAdapters, CoordinatorCallback, CopyAdapter, CopyRequest, EffectObserver,
    PrivatePreview, ResultPanel, SessionCoordinator, SessionSettingsSnapshot,
};
use outflow_core::session_engine::{
    AdapterQueueError, CopyAttemptId, CopyOutcome, EventKind, HoldReleaseCapability,
    InsertionOutcome, MicrophoneStopOutcome, OpaqueTargetId, RecordingMode, SessionId,
    SpeechBackend, SpeechCommand, StopOperationId, TargetCommand, TargetEditability,
    TargetSecurity, TargetService, TargetSnapshot, TargetSubmissionError, TargetVerification,
};
use outflow_settings::Settings;

#[derive(Default)]
struct MockBus {
    callbacks: VecDeque<CoordinatorCallback>,
    effects: Vec<&'static str>,
    target_results: VecDeque<TargetVerification>,
    target_submission_errors: VecDeque<TargetSubmissionError>,
    copy_results: VecDeque<CopyOutcome>,
    pending_stop: Option<(SessionId, StopOperationId)>,
    last_copy_attempt: Option<(SessionId, CopyAttemptId)>,
}

#[derive(Clone, Default)]
struct Shared(Rc<RefCell<MockBus>>);

impl Shared {
    fn event(&self, kind: EventKind) {
        self.0
            .borrow_mut()
            .callbacks
            .push_back(CoordinatorCallback::Session(kind));
    }

    fn closed(&self, session_id: SessionId) {
        self.0
            .borrow_mut()
            .callbacks
            .push_back(CoordinatorCallback::SpeechResourcesClosed { session_id });
    }

    fn acknowledge_stop(&self) -> bool {
        let Some((session_id, operation_id)) = self.0.borrow_mut().pending_stop.take() else {
            return false;
        };
        self.0
            .borrow_mut()
            .callbacks
            .push_back(CoordinatorCallback::MicrophoneStopCompleted {
                session_id,
                operation_id,
                outcome: MicrophoneStopOutcome::Stopped,
            });
        true
    }

    fn take_effects(&self) -> Vec<&'static str> {
        std::mem::take(&mut self.0.borrow_mut().effects)
    }
}

struct Speech(Shared);
struct Target(Shared);
struct Preview;
struct Panel;
struct Copy(Shared);
struct Observer(Shared);

impl EffectObserver for Observer {
    fn observe(&mut self, label: &'static str) {
        self.0.0.borrow_mut().effects.push(label);
    }
}

impl SpeechBackend for Speech {
    fn submit(&mut self, command: SpeechCommand) -> Result<(), AdapterQueueError> {
        match command {
            SpeechCommand::PrepareModel { session_id } => {
                self.0.event(EventKind::ModelPrepared { session_id });
            }
            SpeechCommand::StartMicrophone { session_id } => {
                self.0.event(EventKind::MicrophoneStarted { session_id });
                self.0.event(EventKind::InputCaptured { session_id });
                self.0.event(EventKind::PartialTranscript {
                    session_id,
                    revision: 1,
                    text: "synthetic partial".to_owned(),
                });
                self.0.event(EventKind::PartialTranscript {
                    session_id,
                    revision: 2,
                    text: "revised synthetic partial".to_owned(),
                });
            }
            SpeechCommand::StopMicrophone {
                session_id,
                operation_id,
            } => {
                self.0.0.borrow_mut().pending_stop = Some((session_id, operation_id));
            }
            SpeechCommand::FinalizeRecognition { session_id } => {
                self.0.event(EventKind::FinalTranscript {
                    session_id,
                    text: "um, synthetic alpha sample".to_owned(),
                });
                self.0.closed(session_id);
            }
            SpeechCommand::CancelRecognition { session_id } => self.0.closed(session_id),
        }
        Ok(())
    }
}

impl TargetService for Target {
    fn submit(&mut self, command: TargetCommand) -> Result<(), TargetSubmissionError> {
        match command {
            TargetCommand::VerifyCurrent {
                session_id,
                captured,
            } => {
                let result = self
                    .0
                    .0
                    .borrow_mut()
                    .target_results
                    .pop_front()
                    .unwrap_or(TargetVerification::Current(captured));
                self.0
                    .event(EventKind::TargetVerified { session_id, result });
            }
            TargetCommand::InsertOnce { session_id, .. } => {
                if let Some(error) = self.0.0.borrow_mut().target_submission_errors.pop_front() {
                    return Err(error);
                }
                self.0.event(EventKind::InsertionCompleted {
                    session_id,
                    outcome: InsertionOutcome::Inserted,
                });
            }
        }
        Ok(())
    }
}

impl PrivatePreview for Preview {
    fn publish_partial(
        &mut self,
        _session_id: SessionId,
        _revision: u64,
        _text: &str,
    ) -> Result<(), AdapterQueueError> {
        Ok(())
    }

    fn clear(&mut self, _session_id: SessionId) -> Result<(), AdapterQueueError> {
        Ok(())
    }
}

impl ResultPanel for Panel {
    fn show_copy_result(
        &mut self,
        _session_id: SessionId,
        _text: &str,
    ) -> Result<(), AdapterQueueError> {
        Ok(())
    }

    fn require_manual_review(&mut self, _session_id: SessionId) -> Result<(), AdapterQueueError> {
        Ok(())
    }

    fn copy_failed(&mut self, _session_id: SessionId) -> Result<(), AdapterQueueError> {
        Ok(())
    }

    fn dismiss_and_clear(&mut self, _session_id: SessionId) -> Result<(), AdapterQueueError> {
        Ok(())
    }
}

impl CopyAdapter for Copy {
    fn submit(&mut self, request: CopyRequest) -> Result<(), AdapterQueueError> {
        self.0.0.borrow_mut().last_copy_attempt =
            Some((request.session_id(), request.attempt_id()));
        let outcome = self
            .0
            .0
            .borrow_mut()
            .copy_results
            .pop_front()
            .unwrap_or(CopyOutcome::Succeeded);
        self.0.event(EventKind::CopyCompleted {
            session_id: request.session_id(),
            attempt_id: request.attempt_id(),
            outcome,
        });
        Ok(())
    }
}

type Coordinator = SessionCoordinator<Speech, Target, Preview, Panel, Copy>;

fn new_coordinator() -> (Coordinator, Shared) {
    let shared = Shared::default();
    let coordinator = SessionCoordinator::with_observer(
        HoldReleaseCapability::Supported,
        SessionSettingsSnapshot::from_settings(&Settings::default()),
        CoordinatorAdapters::new(
            Speech(shared.clone()),
            Target(shared.clone()),
            Preview,
            Panel,
            Copy(shared.clone()),
        ),
        Box::new(Observer(shared.clone())),
    );
    (coordinator, shared)
}

fn target() -> TargetSnapshot {
    TargetSnapshot::new(
        OpaqueTargetId::new(0x0F10),
        1,
        TargetSecurity::Ordinary,
        TargetEditability::Editable,
    )
}

fn start(coordinator: &mut Coordinator, mode: RecordingMode) -> Result<SessionId, Box<dyn Error>> {
    coordinator.dispatch(EventKind::ShortcutActivated {
        mode,
        target: target(),
    })?;
    Ok(coordinator
        .status()
        .session
        .session_id
        .expect("start created a session"))
}

fn pump(coordinator: &mut Coordinator, shared: &Shared) -> Result<usize, Box<dyn Error>> {
    let mut rejected = 0;
    let mut count = 0;
    loop {
        let callback = shared.0.borrow_mut().callbacks.pop_front();
        let Some(callback) = callback else { break };
        count += 1;
        if count > 256 {
            return Err("mock event queue failed to quiesce".into());
        }
        if coordinator.accept_callback(callback).is_err() {
            rejected += 1;
        }
    }
    Ok(rejected)
}

fn print_trace(label: &str, effects: &[&'static str]) {
    println!("  {label}: {}", effects.join(", "));
}

fn run() -> Result<(), Box<dyn Error>> {
    let (mut coordinator, shared) = new_coordinator();
    coordinator.dispatch(EventKind::InitializationSucceeded)?;

    shared
        .0
        .borrow_mut()
        .target_results
        .push_back(TargetVerification::Current(target()));
    let ptt = start(&mut coordinator, RecordingMode::PushToTalk)?;
    pump(&mut coordinator, &shared)?;
    coordinator.dispatch(EventKind::KeyReleased { session_id: ptt })?;
    let stop_effects = shared.take_effects();
    println!(
        "Push-to-talk before stop ack: {}; finalization_submitted={}; outcome=waiting_for_capture_release",
        coordinator.status().session.state.label(),
        stop_effects.contains(&"RequestAsrFinalization")
    );
    print_trace("submitted", &stop_effects);
    if !shared.acknowledge_stop() {
        return Err("synthetic stop acknowledgment was not pending".into());
    }
    pump(&mut coordinator, &shared)?;
    let finalized_effects = shared.take_effects();
    println!(
        "Push-to-talk after stop ack: {}; finalization_submitted={}; outcome=guarded_insertion_succeeded",
        coordinator.status().session.state.label(),
        finalized_effects.contains(&"RequestAsrFinalization")
    );
    print_trace("after acknowledgment", &finalized_effects);

    let cancelled = start(&mut coordinator, RecordingMode::PushToTalk)?;
    pump(&mut coordinator, &shared)?;
    coordinator.dispatch(EventKind::KeyReleased {
        session_id: cancelled,
    })?;
    let before_cancel = shared.take_effects();
    coordinator.dispatch(EventKind::Cancel {
        session_id: cancelled,
    })?;
    pump(&mut coordinator, &shared)?;
    let late_stop_delivered = shared.acknowledge_stop();
    let late_stop_rejected = late_stop_delivered && pump(&mut coordinator, &shared)? > 0;
    println!(
        "Cancel before stop ack: {}; finalization_submitted={}; late_stop_ack_rejected={}; outcome=cancelled_without_finalization",
        coordinator.status().session.state.label(),
        before_cancel.contains(&"RequestAsrFinalization"),
        late_stop_rejected
    );
    print_trace("operations", &shared.take_effects());
    coordinator.dispatch(EventKind::Dismiss {
        session_id: Some(cancelled),
    })?;

    shared
        .0
        .borrow_mut()
        .target_submission_errors
        .push_back(TargetSubmissionError::OutcomeUnknown);
    let uncertain = start(&mut coordinator, RecordingMode::PushToTalk)?;
    pump(&mut coordinator, &shared)?;
    coordinator.dispatch(EventKind::KeyReleased {
        session_id: uncertain,
    })?;
    shared.acknowledge_stop();
    pump(&mut coordinator, &shared)?;
    let state_before_review = coordinator.status().session.state;
    let copy_blocked = coordinator
        .dispatch(EventKind::CopyResultRequested {
            session_id: uncertain,
        })
        .is_err();
    coordinator.dispatch(EventKind::UncertaintyReviewed {
        session_id: uncertain,
    })?;
    println!(
        "Unknown insertion submission: {} -> {}; copy_before_review_blocked={}; outcome=manual_review_required",
        state_before_review.label(),
        coordinator.status().session.state.label(),
        copy_blocked
    );
    print_trace("operations", &shared.take_effects());
    coordinator.dispatch(EventKind::Dismiss {
        session_id: Some(uncertain),
    })?;

    shared
        .0
        .borrow_mut()
        .target_results
        .push_back(TargetVerification::Changed);
    let copy_ready = start(&mut coordinator, RecordingMode::Toggle)?;
    pump(&mut coordinator, &shared)?;
    coordinator.dispatch(EventKind::ShortcutActivated {
        mode: RecordingMode::Toggle,
        target: target(),
    })?;
    shared.acknowledge_stop();
    pump(&mut coordinator, &shared)?;
    coordinator.dispatch(EventKind::CopyResultRequested {
        session_id: copy_ready,
    })?;
    let (expected_session, expected_attempt) = shared
        .0
        .borrow()
        .last_copy_attempt
        .ok_or("explicit Copy operation did not submit")?;
    let copy_ack = shared
        .0
        .borrow_mut()
        .callbacks
        .pop_front()
        .ok_or("explicit Copy acknowledgment was not queued")?;
    let matching_copy_ack = matches!(
        &copy_ack,
        CoordinatorCallback::Session(EventKind::CopyCompleted {
            session_id,
            attempt_id,
            outcome: CopyOutcome::Succeeded,
        }) if *session_id == expected_session && *attempt_id == expected_attempt
    );
    coordinator.accept_callback(copy_ack)?;
    println!(
        "Explicit Copy from CopyReady: {}; copy_ack=matching_attempt:{}; outcome=success",
        coordinator.status().session.state.label(),
        matching_copy_ack
    );
    print_trace("operations", &shared.take_effects());
    println!(
        "Native microphone, model, insertion, clipboard, UI shell, network, persistence: NOT USED"
    );
    Ok(())
}

fn main() {
    println!("Outflow session coordinator example (synthetic)");
    if let Err(error) = run() {
        eprintln!("synthetic coordinator failed: {error}");
        std::process::exit(1);
    }
}
