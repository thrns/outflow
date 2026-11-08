//! Ordered effect routing for the platform-independent dictation reducer.
//!
//! The coordinator owns one reducer and event sequencer, applies an immutable
//! cleanup/vocabulary snapshot for each session, and routes reducer effects to
//! narrow adapters. Speech and target adapters keep the existing queue-only
//! [`SpeechBackend`] and [`TargetService`] contracts. Adapter callbacks return
//! through [`SessionCoordinator::dispatch`] or [`SessionCoordinator::accept_callback`].
//!
//! Speech commands are dispatched in reducer order. In particular, Stop is
//! queued before Finalize, and cancellation queues Stop before Cancel before
//! clearing preview state. A session remains marked as having unresolved
//! cleanup until its speech adapter confirms resource closure. Queue failures
//! use the reducer's operation-specific failure events or conservative
//! fallback outcomes; they never imply that resources were released.
//!
//! Transcript text is passed only to the cleanup pipeline and the relevant
//! private adapter call. Status, errors, observer callbacks, and formatting
//! implementations contain metadata only.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use outflow_core::session_engine::{
    AdapterQueueError, CopyAttemptId, CopyOutcome, DispatchOutcome, Effect, EventKind,
    EventSequencer, HoldReleaseCapability, InsertionOutcome, ReducerError, SessionEvent, SessionId,
    SessionReducer, SessionState, SessionStatus, SpeechBackend, SpeechCommand, TargetCommand,
    TargetService, TargetVerification,
};
use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_settings::{
    CleanupSettings, Settings, VocabularyEntry, process_transcript_with_filler_preference,
};

/// Private partial-preview boundary. Success confirms that the in-memory UI
/// operation was applied; implementations must not log or persist the text.
pub trait PrivatePreview {
    /// Publishes one newer partial revision to a session-owned private preview.
    fn publish_partial(
        &mut self,
        session_id: SessionId,
        revision: u64,
        text: &str,
    ) -> Result<(), AdapterQueueError>;

    /// Clears the session's private partial preview.
    fn clear(&mut self, session_id: SessionId) -> Result<(), AdapterQueueError>;
}

/// In-memory Copy Result panel boundary. Calls complete synchronously and
/// success confirms application of the in-memory UI operation. It has no
/// clipboard API.
pub trait ResultPanel {
    /// Displays the finalized text and safe-fallback reason in volatile UI state.
    fn show_copy_result(
        &mut self,
        session_id: SessionId,
        text: &str,
    ) -> Result<(), AdapterQueueError>;

    /// Displays the mandatory manual-review step for uncertain insertion.
    fn require_manual_review(&mut self, session_id: SessionId) -> Result<(), AdapterQueueError>;

    /// Reports an unsuccessful explicit Copy while preserving the in-memory result.
    fn copy_failed(&mut self, session_id: SessionId) -> Result<(), AdapterQueueError>;

    /// Dismisses the panel and clears its volatile result.
    fn dismiss_and_clear(&mut self, session_id: SessionId) -> Result<(), AdapterQueueError>;
}

/// One explicit Copy request, including the reducer-issued correlation ID.
pub struct CopyRequest {
    session_id: SessionId,
    attempt_id: CopyAttemptId,
    text: String,
}

impl CopyRequest {
    /// Session for which the user explicitly requested Copy.
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Checked ID that must accompany the completion callback.
    pub const fn attempt_id(&self) -> CopyAttemptId {
        self.attempt_id
    }

    /// Volatile result text to copy. This accessor is for the adapter call only.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Debug for CopyRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CopyRequest")
            .field("session_id", &self.session_id)
            .field("attempt_id", &self.attempt_id)
            .field("text", &"[redacted]")
            .finish()
    }
}

/// Explicit user Copy adapter. Production clipboard integration is outside
/// this crate; the current runnable example uses a deterministic mock only.
pub trait CopyAdapter {
    /// Queues one user-requested copy operation.
    fn submit(&mut self, request: CopyRequest) -> Result<(), AdapterQueueError>;
}

/// Receives only stable effect labels, never transcript or target text.
pub trait EffectObserver {
    /// Observes one effect in reducer dispatch order.
    fn observe(&mut self, label: &'static str);
}

/// The narrow adapter set owned by one coordinator instance.
pub struct CoordinatorAdapters<S, T, P, R, C> {
    speech: S,
    target: T,
    preview: P,
    panel: R,
    copy: C,
}

impl<S, T, P, R, C> CoordinatorAdapters<S, T, P, R, C> {
    /// Groups adapters that share one coordinator lifecycle.
    pub const fn new(speech: S, target: T, preview: P, panel: R, copy: C) -> Self {
        Self {
            speech,
            target,
            preview,
            panel,
            copy,
        }
    }
}

/// No-op production observer.
pub struct NoopEffectObserver;

impl EffectObserver for NoopEffectObserver {
    fn observe(&mut self, _label: &'static str) {}
}

/// Immutable cleanup and vocabulary values captured at session start.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionSettingsSnapshot {
    cleanup: CleanupSettings,
    vocabulary: Vec<VocabularyEntry>,
    commands_enabled: bool,
}

impl SessionSettingsSnapshot {
    /// Copies the settings that affect the existing deterministic final cleanup.
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            cleanup: settings.cleanup.clone(),
            vocabulary: settings.vocabulary.clone(),
            // Dictation commands are not enabled by the persisted MVP schema.
            commands_enabled: false,
        }
    }

    /// Creates a cleanup snapshot from existing settings types. Vocabulary is
    /// cloned and dictation commands remain disabled because the persisted MVP
    /// settings schema has no command opt-in field.
    pub fn new(cleanup: CleanupSettings, vocabulary: Vec<VocabularyEntry>) -> Self {
        Self {
            cleanup,
            vocabulary,
            commands_enabled: false,
        }
    }
}

impl Debug for SessionSettingsSnapshot {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionSettingsSnapshot")
            .field("cleanup_mode", &self.cleanup.mode)
            .field("remove_fillers", &self.cleanup.remove_fillers)
            .field("commands_enabled", &self.commands_enabled)
            .field("vocabulary_entries", &self.vocabulary.len())
            .field("vocabulary", &"[redacted]")
            .finish()
    }
}

/// Callbacks delivered by an adapter or an example event loop.
#[derive(PartialEq, Eq)]
pub enum CoordinatorCallback {
    /// Session-scoped reducer acknowledgment or failure.
    Session(EventKind),
    /// Confirms that session-owned speech and capture resources are closed.
    SpeechResourcesClosed { session_id: SessionId },
}

impl Debug for CoordinatorCallback {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(_) => f.write_str("Session([redacted event payload])"),
            Self::SpeechResourcesClosed { session_id } => f
                .debug_struct("SpeechResourcesClosed")
                .field("session_id", session_id)
                .finish(),
        }
    }
}

/// Content-free resource status. Unresolved remains set until an exact session
/// closure acknowledgment arrives, including after cancellation or finalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceCleanupStatus {
    /// No speech resources are awaiting release.
    Resolved,
    /// A resource owner has not confirmed release for this session.
    Unresolved { session_id: SessionId },
}

impl ResourceCleanupStatus {
    /// Stable content-free label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Resolved => "RESOLVED",
            Self::Unresolved { .. } => "UNRESOLVED",
        }
    }
}

/// Content-free status for preview, result panel, and volatile UI clearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PresentationStatus {
    /// The preview adapter failed to apply or queue its latest requested state.
    pub preview_unresolved: bool,
    /// The result-panel adapter failed to apply or queue its latest requested state.
    pub panel_unresolved: bool,
    /// The panel did not confirm that a Copy failure was announced.
    pub copy_failure_notice_unresolved: bool,
    /// A clear/dismiss operation did not confirm all volatile UI copies cleared.
    pub volatile_content_clear_unresolved: bool,
}

/// Public status safe for UI and diagnostics. It contains no transcript data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinatorStatus {
    /// Reducer state and content-free session metadata.
    pub session: SessionStatus,
    /// Speech resource ownership status.
    pub cleanup: ResourceCleanupStatus,
    /// UI adapter state summarized as booleans only.
    pub presentation: PresentationStatus,
}

/// Content-free coordinator failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinatorError {
    /// Reducer rejected this event or its sequence.
    Reducer(ReducerError),
    /// A new session cannot start until the prior speech owner confirms closure.
    CleanupUnresolved,
    /// A new session cannot start while a volatile UI result may still remain.
    VolatileContentUnresolved,
    /// A user action requires a result panel that is not confirmed available.
    PresentationUnresolved,
    /// Closure acknowledgment does not match the outstanding resource owner.
    StaleCleanupAcknowledgment,
}

impl Display for CoordinatorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Reducer(error) => return Display::fmt(error, f),
            Self::CleanupUnresolved => "speech resource cleanup is unresolved",
            Self::VolatileContentUnresolved => "volatile result clearing is unresolved",
            Self::PresentationUnresolved => "result presentation is unresolved",
            Self::StaleCleanupAcknowledgment => "cleanup acknowledgment is stale or mismatched",
        })
    }
}

impl Error for CoordinatorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Reducer(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ReducerError> for CoordinatorError {
    fn from(value: ReducerError) -> Self {
        Self::Reducer(value)
    }
}

/// Reducer plus ordered, mockable effect dispatch and per-session cleanup.
pub struct SessionCoordinator<S, T, P, R, C> {
    reducer: SessionReducer,
    sequences: EventSequencer,
    speech: S,
    target: T,
    preview: P,
    panel: R,
    copy: C,
    observer: Box<dyn EffectObserver>,
    current_settings: SessionSettingsSnapshot,
    active_settings: Option<SessionSettingsSnapshot>,
    cleanup: ResourceCleanupStatus,
    presentation: PresentationStatus,
}

impl<S, T, P, R, C> SessionCoordinator<S, T, P, R, C>
where
    S: SpeechBackend,
    T: TargetService,
    P: PrivatePreview,
    R: ResultPanel,
    C: CopyAdapter,
{
    /// Creates a single-session coordinator with a no-op, content-free observer.
    pub fn new(
        hold_release_capability: HoldReleaseCapability,
        settings: SessionSettingsSnapshot,
        adapters: CoordinatorAdapters<S, T, P, R, C>,
    ) -> Self {
        Self::with_observer(
            hold_release_capability,
            settings,
            adapters,
            Box::new(NoopEffectObserver),
        )
    }

    /// Creates a coordinator with a content-free effect observer.
    pub fn with_observer(
        hold_release_capability: HoldReleaseCapability,
        settings: SessionSettingsSnapshot,
        adapters: CoordinatorAdapters<S, T, P, R, C>,
        observer: Box<dyn EffectObserver>,
    ) -> Self {
        Self {
            reducer: SessionReducer::new(hold_release_capability),
            sequences: EventSequencer::new(),
            speech: adapters.speech,
            target: adapters.target,
            preview: adapters.preview,
            panel: adapters.panel,
            copy: adapters.copy,
            observer,
            current_settings: settings,
            active_settings: None,
            cleanup: ResourceCleanupStatus::Resolved,
            presentation: PresentationStatus::default(),
        }
    }

    /// Replaces settings for future sessions. An active session keeps its clone.
    pub fn update_settings(&mut self, settings: SessionSettingsSnapshot) {
        self.current_settings = settings;
    }

    /// Returns content-free lifecycle and cleanup state.
    pub fn status(&self) -> CoordinatorStatus {
        CoordinatorStatus {
            session: self.reducer.status(),
            cleanup: self.cleanup,
            presentation: self.presentation,
        }
    }

    /// Applies a user action or adapter event, then dispatches effects in order.
    pub fn dispatch(&mut self, kind: EventKind) -> Result<DispatchOutcome, CoordinatorError> {
        let starting = self.reducer.status().state == SessionState::Idle
            && matches!(&kind, EventKind::ShortcutActivated { .. });
        if starting {
            self.require_clean_boundary()?;
        }
        if matches!(
            &kind,
            EventKind::CopyResultRequested { .. } | EventKind::UncertaintyReviewed { .. }
        ) && self.presentation.panel_unresolved
        {
            return Err(CoordinatorError::PresentationUnresolved);
        }

        let captured_settings = starting.then(|| self.current_settings.clone());
        let result = self.apply_event(kind)?;
        if self.reducer.status().state == SessionState::Loading
            && let Some(snapshot) = captured_settings
        {
            self.active_settings = Some(snapshot);
        }
        self.route_effects(result.effects)?;
        self.release_finished_settings();
        Ok(result.outcome)
    }

    /// Routes one callback and rejects stale session or resource acknowledgments.
    pub fn accept_callback(
        &mut self,
        callback: CoordinatorCallback,
    ) -> Result<Option<DispatchOutcome>, CoordinatorError> {
        match callback {
            CoordinatorCallback::Session(kind) => self.dispatch(kind).map(Some),
            CoordinatorCallback::SpeechResourcesClosed { session_id } => {
                if self.cleanup != (ResourceCleanupStatus::Unresolved { session_id }) {
                    return Err(CoordinatorError::StaleCleanupAcknowledgment);
                }
                self.cleanup = ResourceCleanupStatus::Resolved;
                Ok(None)
            }
        }
    }

    fn require_clean_boundary(&self) -> Result<(), CoordinatorError> {
        if matches!(self.cleanup, ResourceCleanupStatus::Unresolved { .. }) {
            return Err(CoordinatorError::CleanupUnresolved);
        }
        if self.presentation.volatile_content_clear_unresolved {
            return Err(CoordinatorError::VolatileContentUnresolved);
        }
        if self.presentation.preview_unresolved || self.presentation.panel_unresolved {
            return Err(CoordinatorError::PresentationUnresolved);
        }
        Ok(())
    }

    fn apply_event(
        &mut self,
        kind: EventKind,
    ) -> Result<outflow_core::session_engine::DispatchResult, CoordinatorError> {
        let sequence = self.sequences.next_sequence()?;
        Ok(self.reducer.dispatch(SessionEvent::new(sequence, kind))?)
    }

    fn route_effects(&mut self, effects: Vec<Effect>) -> Result<(), CoordinatorError> {
        let mut pending = VecDeque::from(effects);
        while let Some(effect) = pending.pop_front() {
            self.observer.observe(effect.label());
            match effect {
                Effect::Speech(command) => {
                    let session_id = speech_session_id(&command);
                    let stop_in_finalizing =
                        matches!(&command, SpeechCommand::StopMicrophone { .. })
                            && self.reducer.status().state == SessionState::Finalizing;
                    let failure_event = if stop_in_finalizing {
                        Some(EventKind::RecognitionFailed { session_id })
                    } else {
                        speech_queue_failure_event(&command)
                    };
                    self.cleanup = ResourceCleanupStatus::Unresolved { session_id };
                    if self.speech.submit(command).is_err() {
                        if stop_in_finalizing {
                            // Do not drain recognition until the microphone
                            // stop request has been accepted.
                            pending.retain(|effect| {
                                !matches!(
                                    effect,
                                    Effect::Speech(SpeechCommand::FinalizeRecognition {
                                        session_id: pending_id,
                                    }) if *pending_id == session_id
                                )
                            });
                        }
                        if let Some(kind) = failure_event {
                            self.prepend_effects(&mut pending, kind)?;
                        }
                    }
                }
                Effect::Target(command) => {
                    let queue_failure = match &command {
                        TargetCommand::VerifyCurrent { session_id, .. } => {
                            EventKind::TargetVerified {
                                session_id: *session_id,
                                result: TargetVerification::Unavailable,
                            }
                        }
                        TargetCommand::InsertOnce { session_id, .. } => {
                            EventKind::InsertionCompleted {
                                session_id: *session_id,
                                outcome: InsertionOutcome::FailedWithoutSideEffect,
                            }
                        }
                    };
                    if self.target.submit(command).is_err() {
                        self.prepend_effects(&mut pending, queue_failure)?;
                    }
                }
                Effect::PublishPrivatePartial {
                    session_id,
                    revision,
                    text,
                } => {
                    self.presentation.preview_unresolved = self
                        .preview
                        .publish_partial(session_id, revision, &text)
                        .is_err();
                }
                Effect::PrepareFinalText { session_id, text } => {
                    let prepared = self.prepare_final_text(&text);
                    let kind = match prepared {
                        Ok(cleaned) => EventKind::FinalTextPrepared {
                            session_id,
                            text: cleaned,
                        },
                        Err(()) => EventKind::TextPreparationFailed { session_id },
                    };
                    self.prepend_effects(&mut pending, kind)?;
                }
                Effect::ShowCopyResult { session_id, text } => {
                    let preview_failed = self.preview.clear(session_id).is_err();
                    let panel_failed = self.panel.show_copy_result(session_id, &text).is_err();
                    self.presentation.preview_unresolved = preview_failed;
                    self.presentation.panel_unresolved = panel_failed;
                    self.presentation.volatile_content_clear_unresolved = preview_failed;
                }
                Effect::ClearPrivateResult { session_id } => {
                    let failed = self.preview.clear(session_id).is_err();
                    self.presentation.preview_unresolved = failed;
                    self.presentation.volatile_content_clear_unresolved = failed;
                }
                Effect::ExplicitCopyRequested {
                    session_id,
                    attempt_id,
                    text,
                } => {
                    let request = CopyRequest {
                        session_id,
                        attempt_id,
                        text,
                    };
                    if self.copy.submit(request).is_err() {
                        self.prepend_effects(
                            &mut pending,
                            EventKind::CopyCompleted {
                                session_id,
                                attempt_id,
                                outcome: CopyOutcome::Failed,
                            },
                        )?;
                    }
                }
                Effect::RequireManualReview { session_id } => {
                    let preview_failed = self.preview.clear(session_id).is_err();
                    let panel_failed = self.panel.require_manual_review(session_id).is_err();
                    self.presentation.preview_unresolved = preview_failed;
                    self.presentation.panel_unresolved = panel_failed;
                    self.presentation.volatile_content_clear_unresolved = preview_failed;
                }
                Effect::DismissAndClearResult { session_id } => {
                    let panel_failed = self.panel.dismiss_and_clear(session_id).is_err();
                    let preview_failed = self.preview.clear(session_id).is_err();
                    self.presentation.panel_unresolved = panel_failed;
                    self.presentation.preview_unresolved = preview_failed;
                    self.presentation.volatile_content_clear_unresolved =
                        panel_failed || preview_failed;
                    if !panel_failed {
                        self.presentation.copy_failure_notice_unresolved = false;
                    }
                }
                Effect::CopyFailed { session_id } => {
                    self.presentation.copy_failure_notice_unresolved =
                        self.panel.copy_failed(session_id).is_err();
                }
                Effect::ReportFailure { code, .. } => {
                    // Reducer status already carries the stable code; no content is sent onward.
                    debug_assert!(code.label().is_ascii());
                }
            }
        }
        Ok(())
    }

    fn prepend_effects(
        &mut self,
        pending: &mut VecDeque<Effect>,
        kind: EventKind,
    ) -> Result<(), CoordinatorError> {
        let result = self.apply_event(kind)?;
        for effect in result.effects.into_iter().rev() {
            pending.push_front(effect);
        }
        Ok(())
    }

    fn prepare_final_text(&self, text: &str) -> Result<String, ()> {
        let snapshot = self.active_settings.as_ref().ok_or(())?;
        let entries = snapshot
            .vocabulary
            .iter()
            .map(|entry| VocabularyEntryRef {
                spoken: &entry.spoken,
                written: &entry.written,
            })
            .collect::<Vec<_>>();
        process_transcript_with_filler_preference(
            text,
            &entries,
            snapshot.commands_enabled,
            &snapshot.cleanup,
        )
        .map_err(|_| ())
    }

    fn release_finished_settings(&mut self) {
        if matches!(
            self.reducer.status().state,
            SessionState::Idle
                | SessionState::CopyReady
                | SessionState::Uncertain
                | SessionState::Cancelled
                | SessionState::Faulted
        ) {
            self.active_settings = None;
        }
    }
}

fn speech_session_id(command: &SpeechCommand) -> SessionId {
    match command {
        SpeechCommand::PrepareModel { session_id }
        | SpeechCommand::StartMicrophone { session_id }
        | SpeechCommand::StopMicrophone { session_id }
        | SpeechCommand::FinalizeRecognition { session_id }
        | SpeechCommand::CancelRecognition { session_id } => *session_id,
    }
}

fn speech_queue_failure_event(command: &SpeechCommand) -> Option<EventKind> {
    let session_id = speech_session_id(command);
    match command {
        SpeechCommand::PrepareModel { .. } => {
            Some(EventKind::ModelPreparationFailed { session_id })
        }
        SpeechCommand::StartMicrophone { .. } => {
            Some(EventKind::MicrophoneStartFailed { session_id })
        }
        SpeechCommand::FinalizeRecognition { .. } => {
            Some(EventKind::RecognitionFailed { session_id })
        }
        SpeechCommand::StopMicrophone { .. } | SpeechCommand::CancelRecognition { .. } => None,
    }
}
