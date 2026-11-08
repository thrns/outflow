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
    AdapterQueueError, CopyOutcome, EventKind, HoldReleaseCapability, InsertionOutcome,
    OpaqueTargetId, RecordingMode, SessionId, SpeechBackend, SpeechCommand, TargetCommand,
    TargetEditability, TargetSecurity, TargetService, TargetSnapshot, TargetVerification,
};
use outflow_settings::Settings;

#[derive(Default)]
struct MockBus {
    callbacks: VecDeque<CoordinatorCallback>,
    effects: Vec<&'static str>,
    target_results: VecDeque<TargetVerification>,
    copy_results: VecDeque<CopyOutcome>,
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
            SpeechCommand::StopMicrophone { .. } => {}
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
    fn submit(&mut self, command: TargetCommand) -> Result<(), AdapterQueueError> {
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
    pump(&mut coordinator, &shared)?;
    let insertion_effects = shared.take_effects();
    println!(
        "Push-to-talk: {}; outcome=guarded_insertion_succeeded",
        coordinator.status().session.state.label()
    );
    print_trace("effects", &insertion_effects);

    shared
        .0
        .borrow_mut()
        .target_results
        .push_back(TargetVerification::Changed);
    let changed = start(&mut coordinator, RecordingMode::PushToTalk)?;
    pump(&mut coordinator, &shared)?;
    coordinator.dispatch(EventKind::KeyReleased {
        session_id: changed,
    })?;
    pump(&mut coordinator, &shared)?;
    let before_copy = coordinator.status().session.state;
    coordinator.dispatch(EventKind::CopyResultRequested {
        session_id: changed,
    })?;
    pump(&mut coordinator, &shared)?;
    let changed_effects = shared.take_effects();
    println!(
        "Changed target: {} -> {}; outcome=explicit_copy_succeeded",
        before_copy.label(),
        coordinator.status().session.state.label()
    );
    print_trace("effects", &changed_effects);

    shared
        .0
        .borrow_mut()
        .target_results
        .push_back(TargetVerification::Unsupported);
    shared
        .0
        .borrow_mut()
        .copy_results
        .extend([CopyOutcome::Failed, CopyOutcome::Succeeded]);
    let retry = start(&mut coordinator, RecordingMode::Toggle)?;
    pump(&mut coordinator, &shared)?;
    coordinator.dispatch(EventKind::ShortcutActivated {
        mode: RecordingMode::Toggle,
        target: target(),
    })?;
    pump(&mut coordinator, &shared)?;
    let first_copy = coordinator.status().session.state;
    coordinator.dispatch(EventKind::CopyResultRequested { session_id: retry })?;
    pump(&mut coordinator, &shared)?;
    let after_failure = coordinator.status().session.state;
    coordinator.dispatch(EventKind::CopyResultRequested { session_id: retry })?;
    pump(&mut coordinator, &shared)?;
    let retry_effects = shared.take_effects();
    println!(
        "Copy retry: {} -> {} -> {}; outcome=first_failed_text_retained_then_retry_succeeded",
        first_copy.label(),
        after_failure.label(),
        coordinator.status().session.state.label()
    );
    print_trace("effects", &retry_effects);

    let cancelled = start(&mut coordinator, RecordingMode::PushToTalk)?;
    coordinator.dispatch(EventKind::Cancel {
        session_id: cancelled,
    })?;
    let rejected = pump(&mut coordinator, &shared)?;
    println!(
        "Pending cancellation: {}; outcome=late_ack_rejected:{}; cleanup={}",
        coordinator.status().session.state.label(),
        rejected > 0,
        match coordinator.status().cleanup {
            outflow_coordinator::ResourceCleanupStatus::Resolved => "RESOLVED",
            outflow_coordinator::ResourceCleanupStatus::Unresolved { .. } => "UNRESOLVED",
        }
    );
    print_trace("effects", &shared.take_effects());
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
