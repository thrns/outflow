//! Platform-independent dictation session reducer (C-02).
//!
//! The reducer owns one in-process session and emits typed effects for future
//! speech, target, and private-preview adapters. Adapter work is asynchronous:
//! acknowledgments and failures return as ordered, session-scoped events. This
//! module has no Tauri, filesystem, timer, model, microphone, clipboard, or OS
//! dependency.
//!
//! Final transcript text crosses the boundary through PrepareFinalText. The
//! caller applies the existing C-06 cleanup pipeline with an immutable settings
//! and vocabulary snapshot, then returns one FinalTextPrepared event.
//!
//! A target adapter must verify the captured identity again immediately before
//! insertion. The reducer's earlier verification does not replace that final
//! focus, editability, and protection check.

use std::fmt::{self, Debug, Display, Formatter};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

/// Monotonic identifier unique within the current process.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(NonZeroU64);

impl SessionId {
    /// Numeric value for in-process callback correlation.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl Debug for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "SessionId({})", self.get())
    }
}

/// Checked attempt identifier scoped to one session's explicit Copy actions.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CopyAttemptId {
    session_id: SessionId,
    sequence: NonZeroU64,
}

impl CopyAttemptId {
    fn new(session_id: SessionId, sequence: NonZeroU64) -> Self {
        Self {
            session_id,
            sequence,
        }
    }

    /// Session that issued this explicit Copy attempt.
    pub const fn session_id(self) -> SessionId {
        self.session_id
    }

    /// One-based attempt number within the session.
    pub const fn sequence(self) -> u64 {
        self.sequence.get()
    }
}

impl Debug for CopyAttemptId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CopyAttemptId")
            .field("session_id", &self.session_id)
            .field("sequence", &self.sequence())
            .finish()
    }
}

/// Checked identifier for one microphone-stop operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StopOperationId(NonZeroU64);

impl StopOperationId {
    /// One-based operation number, used only for callback correlation.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Result of the actual microphone-stop operation, separate from queue acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicrophoneStopOutcome {
    /// Capture callbacks have ceased and the microphone stream is released.
    Stopped,
    /// The adapter cannot confirm that capture has stopped.
    Failed,
}

/// Strictly increasing order shared by user and adapter events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventSequence(pub u64);

/// Allocates event sequences without wrapping.
pub struct EventSequencer {
    next: Option<u64>,
}

impl EventSequencer {
    /// Starts at sequence one.
    pub const fn new() -> Self {
        Self { next: Some(1) }
    }

    /// Returns the next sequence or a typed exhaustion error.
    pub fn next_sequence(&mut self) -> Result<EventSequence, ReducerError> {
        let value = self.next.ok_or(ReducerError::EventSequenceExhausted)?;
        self.next = value.checked_add(1);
        Ok(EventSequence(value))
    }
}

fn allocate_session_id(counter: &AtomicU64) -> Result<SessionId, ReducerError> {
    let raw_id = counter
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            if current == 0 {
                None
            } else {
                Some(current.checked_add(1).unwrap_or(0))
            }
        })
        .map_err(|_| ReducerError::SessionIdExhausted)?;
    NonZeroU64::new(raw_id)
        .map(SessionId)
        .ok_or(ReducerError::SessionIdExhausted)
}

impl Default for EventSequencer {
    fn default() -> Self {
        Self::new()
    }
}

/// The recording control that owns a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingMode {
    /// Start on the first press and stop on its matching release.
    PushToTalk,
    /// Start on the first activation and stop on the second.
    Toggle,
}

/// Whether a shortcut adapter can report a matching key release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReleaseCapability {
    /// Release events are available.
    Supported,
    /// Release events are unavailable; push-to-talk must not be offered.
    Unsupported,
}

/// Native shortcut registration status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutRegistration {
    /// This milestone does not install global or native shortcuts.
    NotImplemented,
}

/// Current reducer state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Before local initialization is acknowledged.
    Uninitialized,
    /// Ready for a recording request.
    Idle,
    /// Model preparation or microphone start is in progress.
    Loading,
    /// Microphone capture has been acknowledged as open.
    Listening,
    /// Capture is stopped; final recognition and cleanup are in progress.
    Finalizing,
    /// One guarded insertion request has been issued.
    Committing,
    /// Final text is retained in volatile memory for explicit Copy.
    CopyReady,
    /// Insertion may have had a partial side effect; manual review is required.
    Uncertain,
    /// Session data was discarded and callbacks invalidated.
    Cancelled,
    /// A content-free failure stopped the session.
    Faulted,
}

impl SessionState {
    /// Stable content-free label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Uninitialized => "Uninitialized",
            Self::Idle => "Idle",
            Self::Loading => "Loading",
            Self::Listening => "Listening",
            Self::Finalizing => "Finalizing",
            Self::Committing => "Committing",
            Self::CopyReady => "CopyReady",
            Self::Uncertain => "Uncertain",
            Self::Cancelled => "Cancelled",
            Self::Faulted => "Faulted",
        }
    }
}

/// Fixed-size opaque target identity. It must contain identity metadata only.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpaqueTargetId(u128);

impl OpaqueTargetId {
    /// Wraps an adapter-owned target identity value.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Returns the opaque value so its owning platform adapter can revalidate it.
    pub const fn get(self) -> u128 {
        self.0
    }
}

impl Debug for OpaqueTargetId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("OpaqueTargetId([redacted])")
    }
}

/// Security classification for a focused target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSecurity {
    /// Ordinary user-level target.
    Ordinary,
    /// Password or otherwise protected control.
    SecureOrProtected,
    /// Target behind an elevated privilege boundary.
    ElevatedOrProtected,
    /// Security could not be established.
    Unknown,
}

/// Editability classification for a focused target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetEditability {
    /// Adapter confirmed an editable control.
    Editable,
    /// Adapter confirmed a non-editable control.
    NotEditable,
    /// Editability could not be established.
    Unknown,
}

/// Bounded, non-content identity snapshot captured at session start.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TargetSnapshot {
    identity: OpaqueTargetId,
    generation: u64,
    security: TargetSecurity,
    editability: TargetEditability,
}

impl TargetSnapshot {
    /// Creates a snapshot. Never encode document or selected text in it.
    pub const fn new(
        identity: OpaqueTargetId,
        generation: u64,
        security: TargetSecurity,
        editability: TargetEditability,
    ) -> Self {
        Self {
            identity,
            generation,
            security,
            editability,
        }
    }

    /// Returns the captured security classification.
    pub const fn security(self) -> TargetSecurity {
        self.security
    }

    /// Returns the bounded adapter identity token.
    pub const fn identity(self) -> OpaqueTargetId {
        self.identity
    }

    /// Returns the captured target generation.
    pub const fn generation(self) -> u64 {
        self.generation
    }

    /// Returns the captured editability classification.
    pub const fn editability(self) -> TargetEditability {
        self.editability
    }
}

impl Debug for TargetSnapshot {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TargetSnapshot")
            .field("identity", &"[redacted]")
            .field("generation", &self.generation)
            .field("security", &self.security)
            .field("editability", &self.editability)
            .finish()
    }
}

/// Fresh target/capability query result required before a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetVerification {
    /// New snapshot returned by the target adapter.
    Current(TargetSnapshot),
    /// Intended identity or generation changed.
    Changed,
    /// Secure, protected, elevated, or ambiguously classified.
    SecureOrProtected,
    /// Direct insertion is unsupported.
    Unsupported,
    /// Verification could not establish safe insertion capability.
    Unavailable,
}

/// Result returned from the one target insertion request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertionOutcome {
    /// Insertion completed.
    Inserted,
    /// Definite failure before any target side effect.
    FailedWithoutSideEffect,
    /// A partial or delayed side effect cannot be ruled out.
    UncertainOrPartial,
    /// Immediate pre-insertion check found a changed target.
    TargetChanged,
    /// Immediate pre-insertion check found a protected target.
    SecureOrProtected,
    /// Immediate pre-insertion check found no supported capability.
    Unsupported,
}

/// Result of an explicit user Copy action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyOutcome {
    /// The copy adapter reports success.
    Succeeded,
    /// The copy adapter reports failure; result remains in volatile memory.
    Failed,
}

/// Stable failure categories without transcript or target content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCode {
    /// Local initialization failed.
    InitializationFailed,
    /// Bundled model preparation failed.
    ModelPreparationFailed,
    /// Microphone permission was denied or revoked.
    MicrophonePermissionDenied,
    /// Microphone capture could not start.
    MicrophoneStartFailed,
    /// Input device disappeared.
    AudioDeviceLost,
    /// Bounded audio buffer overflowed.
    AudioBufferOverflow,
    /// Streaming recognition or final drain failed.
    RecognitionFailed,
    /// The caller's existing cleanup pipeline rejected a final result.
    TextPreparationFailed,
}

impl FailureCode {
    /// Stable content-free diagnostic label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::InitializationFailed => "INITIALIZATION_FAILED",
            Self::ModelPreparationFailed => "MODEL_PREPARATION_FAILED",
            Self::MicrophonePermissionDenied => "MICROPHONE_PERMISSION_DENIED",
            Self::MicrophoneStartFailed => "MICROPHONE_START_FAILED",
            Self::AudioDeviceLost => "AUDIO_DEVICE_LOST",
            Self::AudioBufferOverflow => "AUDIO_BUFFER_OVERFLOW",
            Self::RecognitionFailed => "RECOGNITION_FAILED",
            Self::TextPreparationFailed => "TEXT_PREPARATION_FAILED",
        }
    }
}

/// Typed command for a future local ASR/microphone adapter.
#[derive(PartialEq, Eq)]
pub enum SpeechCommand {
    /// Prepare the already-bundled local model.
    PrepareModel { session_id: SessionId },
    /// Start capture after model readiness and permission checks.
    StartMicrophone { session_id: SessionId },
    /// Stop capture; fence off an outstanding start if necessary.
    StopMicrophone {
        session_id: SessionId,
        operation_id: StopOperationId,
    },
    /// Stop input and drain the final recognition result.
    FinalizeRecognition { session_id: SessionId },
    /// Cancel recognition and release session-owned audio/model state.
    CancelRecognition { session_id: SessionId },
}

impl SpeechCommand {
    /// Content-free effect label.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::PrepareModel { .. } => "RequestModelPreparation",
            Self::StartMicrophone { .. } => "RequestMicrophoneStart",
            Self::StopMicrophone { .. } => "RequestMicrophoneStop",
            Self::FinalizeRecognition { .. } => "RequestAsrFinalization",
            Self::CancelRecognition { .. } => "RequestAsrCancellation",
        }
    }
}

impl Debug for SpeechCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Typed command for a future guarded target adapter.
#[derive(PartialEq, Eq)]
pub enum TargetCommand {
    /// Requery target identity and insertion capability before commit.
    VerifyCurrent {
        session_id: SessionId,
        captured: TargetSnapshot,
    },
    /// Attempt once; adapter must revalidate identity immediately before mutation.
    InsertOnce {
        session_id: SessionId,
        target: TargetSnapshot,
        text: String,
    },
}

impl TargetCommand {
    /// Content-free effect label.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::VerifyCurrent { .. } => "RequestTargetVerification",
            Self::InsertOnce { .. } => "AttemptInsertionOnce",
        }
    }
}

impl Debug for TargetCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Typed effect returned by the reducer. Debug output redacts all text.
#[derive(PartialEq, Eq)]
pub enum Effect {
    /// Queue local speech work.
    Speech(SpeechCommand),
    /// Queue a fresh target check or guarded insertion.
    Target(TargetCommand),
    /// Publish a newer partial only to a private in-app preview.
    PublishPrivatePartial {
        session_id: SessionId,
        revision: u64,
        text: String,
    },
    /// Hand the final transcript to the caller's existing cleanup pipeline.
    PrepareFinalText { session_id: SessionId, text: String },
    /// Show an in-memory Copy Result; this never writes a clipboard.
    ShowCopyResult { session_id: SessionId, text: String },
    /// Clear a private preview.
    ClearPrivateResult { session_id: SessionId },
    /// Ask for copy only after an explicit user action, with a checked
    /// session-scoped attempt ID for matching the completion.
    ExplicitCopyRequested {
        session_id: SessionId,
        attempt_id: CopyAttemptId,
        text: String,
    },
    /// Require manual review after uncertain insertion.
    RequireManualReview { session_id: SessionId },
    /// Dismiss a result and clear its volatile buffer.
    DismissAndClearResult { session_id: SessionId },
    /// Report failed explicit copy without dropping the result.
    CopyFailed { session_id: SessionId },
    /// Report only a stable, content-free failure category.
    ReportFailure {
        session_id: Option<SessionId>,
        code: FailureCode,
    },
}

impl Effect {
    /// Stable content-free label for deterministic traces.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Speech(command) => command.label(),
            Self::Target(command) => command.label(),
            Self::PublishPrivatePartial { .. } => "PublishPrivatePartial",
            Self::PrepareFinalText { .. } => "PrepareFinalText",
            Self::ShowCopyResult { .. } => "ShowCopyResult",
            Self::ClearPrivateResult { .. } => "ClearPrivateResult",
            Self::ExplicitCopyRequested { .. } => "ExplicitCopyRequested",
            Self::RequireManualReview { .. } => "RequireManualReview",
            Self::DismissAndClearResult { .. } => "DismissAndClearResult",
            Self::CopyFailed { .. } => "CopyFailed",
            Self::ReportFailure { .. } => "ReportFailure",
        }
    }
}

impl Debug for Effect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Queue-only contract for a future local ASR/microphone adapter.
///
/// Implementations enqueue commands and later return session-scoped events.
/// A queue error is not an acknowledgment. The caller maps it to the
/// operation-specific, session-scoped failure event; reducer transitions do
/// not synchronously depend on worker or microphone success.
pub trait SpeechBackend {
    /// Queues one speech request without blocking the reducer.
    ///
    /// Accepting `StopMicrophone` is not a stop acknowledgment. The adapter
    /// must later report `CoordinatorCallback::MicrophoneStopCompleted` only
    /// after capture callbacks have ceased and the microphone stream is
    /// released. That fact does not confirm that other session resources are
    /// closed; those require a separate `SpeechResourcesClosed` acknowledgment.
    fn submit(&mut self, command: SpeechCommand) -> Result<(), AdapterQueueError>;
}

/// Queue-only contract for a future safe target adapter.
///
/// InsertOnce must revalidate the captured target immediately before mutation.
/// It must not paste blindly into the currently focused control. Clipboard
/// behavior is deliberately outside this interface. Submission errors
/// distinguish work rejected before dispatch from an outcome that may already
/// have reached a native API. The adapter must classify ambiguity
/// conservatively: a failed native API call does not prove that no text was
/// inserted. After acceptance, it must report `UncertainOrPartial` whenever it
/// cannot prove success or a harmless failure.
pub trait TargetService {
    /// Queues one target request without blocking the reducer.
    fn submit(&mut self, command: TargetCommand) -> Result<(), TargetSubmissionError>;
}

/// Certainty available when target submission does not return acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSubmissionError {
    /// The adapter guarantees rejection before dispatch, with no target
    /// mutation attempt.
    DefinitelyNotDispatched,
    /// The request may have reached a native API or produced a partial effect.
    OutcomeUnknown,
}

impl Display for TargetSubmissionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DefinitelyNotDispatched => "target request was rejected before dispatch",
            Self::OutcomeUnknown => "target request outcome is unknown",
        })
    }
}

impl std::error::Error for TargetSubmissionError {}

/// Content-free failure to enqueue adapter work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterQueueError {
    /// Adapter cannot currently accept a bounded request.
    Unavailable,
}

impl Display for AdapterQueueError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("adapter request could not be queued")
    }
}

impl std::error::Error for AdapterQueueError {}

/// Events returned by the caller and future adapters.
///
/// Debug prints only the event kind and sequence. Transcript strings are never
/// formatted into logs or diagnostics.
#[derive(Clone, PartialEq, Eq)]
pub enum EventKind {
    /// Local startup is ready.
    InitializationSucceeded,
    /// Local startup failed.
    InitializationFailed,
    /// Shortcut activation. First toggle starts; second stops.
    ShortcutActivated {
        mode: RecordingMode,
        target: TargetSnapshot,
    },
    /// Matching push-to-talk release; toggle key-up is ignored.
    KeyReleased { session_id: SessionId },
    /// Explicit cancellation before insertion begins.
    Cancel { session_id: SessionId },
    /// Bundled-model preparation acknowledgment or failure.
    ModelPrepared { session_id: SessionId },
    /// Bundled-model preparation failed.
    ModelPreparationFailed { session_id: SessionId },
    /// Microphone-open acknowledgment or failure.
    MicrophoneStarted { session_id: SessionId },
    /// Microphone-open failed.
    MicrophoneStartFailed { session_id: SessionId },
    /// At least one input frame was captured.
    InputCaptured { session_id: SessionId },
    /// Permission denied or revoked.
    PermissionDenied { session_id: SessionId },
    /// Input device disappeared.
    AudioDeviceLost { session_id: SessionId },
    /// Bounded input overflow; must fail safely.
    AudioBufferOverflow { session_id: SessionId },
    /// Recognition or final drain failed.
    RecognitionFailed { session_id: SessionId },
    /// New private partial revision.
    PartialTranscript {
        session_id: SessionId,
        revision: u64,
        text: String,
    },
    /// One final recognition result after stop/drain.
    FinalTranscript { session_id: SessionId, text: String },
    /// Result of one caller-owned cleanup operation.
    FinalTextPrepared { session_id: SessionId, text: String },
    /// The caller-owned cleanup operation could not prepare a final result.
    TextPreparationFailed { session_id: SessionId },
    /// Fresh target/capability query result.
    TargetVerified {
        session_id: SessionId,
        result: TargetVerification,
    },
    /// Result of the single insertion attempt.
    InsertionCompleted {
        session_id: SessionId,
        outcome: InsertionOutcome,
    },
    /// User confirms the uncertainty warning was reviewed.
    UncertaintyReviewed { session_id: SessionId },
    /// Explicit user action on the Copy button. A repeated request while one
    /// attempt is pending is ignored and emits no second Copy effect.
    CopyResultRequested { session_id: SessionId },
    /// Result of that explicit Copy action. The attempt ID must match the
    /// outstanding effect; stale and unsolicited acknowledgments are rejected.
    CopyCompleted {
        session_id: SessionId,
        attempt_id: CopyAttemptId,
        outcome: CopyOutcome,
    },
    /// Explicit result, cancellation, or fault dismissal.
    Dismiss { session_id: Option<SessionId> },
}

impl EventKind {
    const fn label(&self) -> &'static str {
        match self {
            Self::InitializationSucceeded => "InitializationSucceeded",
            Self::InitializationFailed => "InitializationFailed",
            Self::ShortcutActivated { .. } => "ShortcutActivated",
            Self::KeyReleased { .. } => "KeyReleased",
            Self::Cancel { .. } => "Cancel",
            Self::ModelPrepared { .. } => "ModelPrepared",
            Self::ModelPreparationFailed { .. } => "ModelPreparationFailed",
            Self::MicrophoneStarted { .. } => "MicrophoneStarted",
            Self::MicrophoneStartFailed { .. } => "MicrophoneStartFailed",
            Self::InputCaptured { .. } => "InputCaptured",
            Self::PermissionDenied { .. } => "PermissionDenied",
            Self::AudioDeviceLost { .. } => "AudioDeviceLost",
            Self::AudioBufferOverflow { .. } => "AudioBufferOverflow",
            Self::RecognitionFailed { .. } => "RecognitionFailed",
            Self::PartialTranscript { .. } => "PartialTranscript",
            Self::FinalTranscript { .. } => "FinalTranscript",
            Self::FinalTextPrepared { .. } => "FinalTextPrepared",
            Self::TextPreparationFailed { .. } => "TextPreparationFailed",
            Self::TargetVerified { .. } => "TargetVerified",
            Self::InsertionCompleted { .. } => "InsertionCompleted",
            Self::UncertaintyReviewed { .. } => "UncertaintyReviewed",
            Self::CopyResultRequested { .. } => "CopyResultRequested",
            Self::CopyCompleted { .. } => "CopyCompleted",
            Self::Dismiss { .. } => "Dismiss",
        }
    }
}

/// One ordered event.
#[derive(PartialEq, Eq)]
pub struct SessionEvent {
    /// Caller-assigned monotonic sequence.
    pub sequence: EventSequence,
    /// Typed payload.
    pub kind: EventKind,
}

impl SessionEvent {
    /// Creates one sequenced event.
    pub const fn new(sequence: EventSequence, kind: EventKind) -> Self {
        Self { sequence, kind }
    }
}

impl Debug for SessionEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionEvent")
            .field("sequence", &self.sequence)
            .field("kind", &self.kind.label())
            .finish()
    }
}

/// Content-free reducer failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReducerError {
    /// Duplicate or out-of-order event sequence.
    NonMonotonicEventSequence,
    /// Session ID counter reached its checked limit.
    SessionIdExhausted,
    /// Event sequence counter reached its checked limit.
    EventSequenceExhausted,
    /// Microphone stop operation counter reached its checked limit.
    StopOperationIdExhausted,
    /// Event belongs to an old or unknown session.
    StaleSession,
    /// Current session has been cancelled or faulted.
    SessionClosed,
    /// Event is invalid for the current state.
    InvalidTransition,
    /// PTT is unavailable without release-event capability.
    PushToTalkUnsupported,
    /// A different recording mode owns the session.
    RecordingModeConflict,
    /// Partial revision is duplicate or out of order.
    NonMonotonicPartialRevision,
    /// Cancellation arrived after insertion began.
    CancellationTooLate,
    /// Session state violated the one-attempt insertion invariant.
    InsertionAlreadyAttempted,
    /// A Copy acknowledgment does not match the outstanding explicit attempt.
    CopyAttemptMismatch,
    /// The checked Copy attempt counter reached its limit.
    CopyAttemptExhausted,
}

impl Display for ReducerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonMonotonicEventSequence => "event sequence is stale or duplicated",
            Self::SessionIdExhausted => "session identifier space is exhausted",
            Self::EventSequenceExhausted => "event sequence space is exhausted",
            Self::StopOperationIdExhausted => "microphone stop operation space is exhausted",
            Self::StaleSession => "event belongs to a stale or unknown session",
            Self::SessionClosed => "session is already closed",
            Self::InvalidTransition => "event is invalid for the current session state",
            Self::PushToTalkUnsupported => "push-to-talk release events are unsupported",
            Self::RecordingModeConflict => "another recording mode owns the active session",
            Self::NonMonotonicPartialRevision => "partial revision is stale or duplicated",
            Self::CancellationTooLate => "session cancellation is too late",
            Self::InsertionAlreadyAttempted => "insertion was already attempted for this session",
            Self::CopyAttemptMismatch => {
                "copy acknowledgment does not match the outstanding attempt"
            }
            Self::CopyAttemptExhausted => "copy attempt identifier space is exhausted",
        })
    }
}

impl std::error::Error for ReducerError {}

/// Accepted, idempotent no-op reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoredEvent {
    /// Repeated PTT key-down did not start another session.
    RepeatedPushToTalkActivation,
    /// Stop arrived after finalization was already requested.
    AlreadyStopping,
    /// Key-up does not stop toggle mode.
    ToggleKeyUp,
    /// A Copy request arrived while the current explicit attempt is pending.
    CopyAlreadyPending,
    /// Dismiss was requested while idle.
    AlreadyIdle,
}

/// Outcome of one accepted event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchOutcome {
    /// Valid state change or expected effects.
    Applied,
    /// Valid event intentionally made no lifecycle change.
    Ignored(IgnoredEvent),
}

/// Content-free public state, safe for status and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionStatus {
    /// Lifecycle state.
    pub state: SessionState,
    /// Current or tombstoned identifier.
    pub session_id: Option<SessionId>,
    /// Current recording mode while session data remains.
    pub recording_mode: Option<RecordingMode>,
    /// Last accepted event sequence.
    pub last_event_sequence: Option<EventSequence>,
    /// Latest accepted revision number, without partial text.
    pub partial_revision: Option<u64>,
    /// Whether insertion has been attempted for this session.
    pub insertion_attempted: bool,
    /// Reported release-event capability.
    pub hold_release_capability: HoldReleaseCapability,
    /// Explicit shortcut installation state.
    pub shortcut_registration: ShortcutRegistration,
    /// Stable failure category, if any.
    pub failure: Option<FailureCode>,
}

struct SessionData {
    mode: RecordingMode,
    target: TargetSnapshot,
    release_pending: bool,
    microphone_start_pending: bool,
    microphone_open: bool,
    input_captured: bool,
    partial_revision: Option<u64>,
    final_transcript_requested: bool,
    awaiting_cleanup: bool,
    target_verification_pending: bool,
    final_text: Option<String>,
    insertion_attempted: bool,
    next_copy_attempt: Option<u64>,
    outstanding_copy_attempt: Option<CopyAttemptId>,
}

impl SessionData {
    fn new(mode: RecordingMode, target: TargetSnapshot) -> Self {
        Self {
            mode,
            target,
            release_pending: false,
            microphone_start_pending: false,
            microphone_open: false,
            input_captured: false,
            partial_revision: None,
            final_transcript_requested: false,
            awaiting_cleanup: false,
            target_verification_pending: false,
            final_text: None,
            insertion_attempted: false,
            next_copy_attempt: Some(1),
            outstanding_copy_attempt: None,
        }
    }
}

/// Deterministic single-session reducer. IDs are unique within this process.
pub struct SessionReducer {
    state: SessionState,
    hold_release_capability: HoldReleaseCapability,
    shortcut_registration: ShortcutRegistration,
    last_event_sequence: Option<EventSequence>,
    next_stop_operation: Option<u64>,
    current_session_id: Option<SessionId>,
    session: Option<SessionData>,
    failure: Option<FailureCode>,
}

impl SessionReducer {
    /// Creates an uninitialized reducer; native shortcuts are not installed.
    pub const fn new(hold_release_capability: HoldReleaseCapability) -> Self {
        Self {
            state: SessionState::Uninitialized,
            hold_release_capability,
            shortcut_registration: ShortcutRegistration::NotImplemented,
            last_event_sequence: None,
            next_stop_operation: Some(1),
            current_session_id: None,
            session: None,
            failure: None,
        }
    }

    /// Returns content-free state.
    pub fn status(&self) -> SessionStatus {
        SessionStatus {
            state: self.state,
            session_id: self.current_session_id,
            recording_mode: self.session.as_ref().map(|s| s.mode),
            last_event_sequence: self.last_event_sequence,
            partial_revision: self.session.as_ref().and_then(|s| s.partial_revision),
            insertion_attempted: self.session.as_ref().is_some_and(|s| s.insertion_attempted),
            hold_release_capability: self.hold_release_capability,
            shortcut_registration: self.shortcut_registration,
            failure: self.failure,
        }
    }

    fn stop_microphone_effect(&mut self, session_id: SessionId) -> Result<Effect, ReducerError> {
        let raw_id = self
            .next_stop_operation
            .ok_or(ReducerError::StopOperationIdExhausted)?;
        self.next_stop_operation = raw_id.checked_add(1);
        let operation_id = NonZeroU64::new(raw_id)
            .map(StopOperationId)
            .ok_or(ReducerError::StopOperationIdExhausted)?;
        Ok(Effect::Speech(SpeechCommand::StopMicrophone {
            session_id,
            operation_id,
        }))
    }

    /// Applies a sequenced event and returns effects in execution order.
    ///
    /// A greater sequence is consumed even if the event is invalid for the
    /// current state, preventing replay under the same sequence with a changed
    /// payload. Duplicate and out-of-order sequences are rejected unchanged.
    pub fn dispatch(&mut self, event: SessionEvent) -> Result<DispatchResult, ReducerError> {
        if self
            .last_event_sequence
            .is_some_and(|last| event.sequence <= last)
        {
            return Err(ReducerError::NonMonotonicEventSequence);
        }
        self.last_event_sequence = Some(event.sequence);
        self.reduce(event.kind)
    }

    fn reduce(&mut self, event: EventKind) -> Result<DispatchResult, ReducerError> {
        match event {
            EventKind::InitializationSucceeded if self.state == SessionState::Uninitialized => {
                self.state = SessionState::Idle;
                self.failure = None;
                Ok(DispatchResult::applied(Vec::new()))
            }
            EventKind::InitializationFailed if self.state == SessionState::Uninitialized => {
                self.state = SessionState::Faulted;
                self.failure = Some(FailureCode::InitializationFailed);
                Ok(DispatchResult::applied(vec![Effect::ReportFailure {
                    session_id: None,
                    code: FailureCode::InitializationFailed,
                }]))
            }
            EventKind::ShortcutActivated { mode, target } => self.activate(mode, target),
            EventKind::KeyReleased { session_id } => self.key_released(session_id),
            EventKind::Cancel { session_id } => self.cancel(session_id),
            EventKind::ModelPrepared { session_id } => self.model_prepared(session_id),
            EventKind::ModelPreparationFailed { session_id } => {
                self.fail_active(session_id, FailureCode::ModelPreparationFailed)
            }
            EventKind::MicrophoneStarted { session_id } => self.microphone_started(session_id),
            EventKind::MicrophoneStartFailed { session_id } => {
                self.fail_active(session_id, FailureCode::MicrophoneStartFailed)
            }
            EventKind::InputCaptured { session_id } => self.input_captured(session_id),
            EventKind::PermissionDenied { session_id } => {
                self.fail_active(session_id, FailureCode::MicrophonePermissionDenied)
            }
            EventKind::AudioDeviceLost { session_id } => {
                self.cancel_for_fault(session_id, FailureCode::AudioDeviceLost)
            }
            EventKind::AudioBufferOverflow { session_id } => {
                self.cancel_for_fault(session_id, FailureCode::AudioBufferOverflow)
            }
            EventKind::RecognitionFailed { session_id } => {
                self.fail_active(session_id, FailureCode::RecognitionFailed)
            }
            EventKind::PartialTranscript {
                session_id,
                revision,
                text,
            } => self.partial(session_id, revision, text),
            EventKind::FinalTranscript { session_id, text } => {
                self.final_transcript(session_id, text)
            }
            EventKind::FinalTextPrepared { session_id, text } => {
                self.final_text_prepared(session_id, text)
            }
            EventKind::TextPreparationFailed { session_id } => {
                self.fail_active(session_id, FailureCode::TextPreparationFailed)
            }
            EventKind::TargetVerified { session_id, result } => {
                self.target_verified(session_id, result)
            }
            EventKind::InsertionCompleted {
                session_id,
                outcome,
            } => self.insertion_completed(session_id, outcome),
            EventKind::UncertaintyReviewed { session_id } => self.uncertainty_reviewed(session_id),
            EventKind::CopyResultRequested { session_id } => self.copy_requested(session_id),
            EventKind::CopyCompleted {
                session_id,
                attempt_id,
                outcome,
            } => self.copy_completed(session_id, attempt_id, outcome),
            EventKind::Dismiss { session_id } => self.dismiss(session_id),
            _ => Err(ReducerError::InvalidTransition),
        }
    }

    fn activate(
        &mut self,
        mode: RecordingMode,
        target: TargetSnapshot,
    ) -> Result<DispatchResult, ReducerError> {
        if matches!(self.state, SessionState::Loading | SessionState::Listening) {
            let active = self
                .session
                .as_ref()
                .ok_or(ReducerError::InvalidTransition)?;
            if active.mode != mode {
                return Err(ReducerError::RecordingModeConflict);
            }
            return match mode {
                RecordingMode::PushToTalk => Ok(DispatchResult::ignored(
                    IgnoredEvent::RepeatedPushToTalkActivation,
                )),
                RecordingMode::Toggle => self.stop_current(),
            };
        }
        if self.state != SessionState::Idle {
            return Err(ReducerError::InvalidTransition);
        }
        if mode == RecordingMode::PushToTalk
            && self.hold_release_capability == HoldReleaseCapability::Unsupported
        {
            return Err(ReducerError::PushToTalkUnsupported);
        }
        let id = allocate_session_id(&NEXT_SESSION_ID)?;
        self.current_session_id = Some(id);
        self.session = Some(SessionData::new(mode, target));
        self.state = SessionState::Loading;
        self.failure = None;
        Ok(DispatchResult::applied(vec![Effect::Speech(
            SpeechCommand::PrepareModel { session_id: id },
        )]))
    }

    fn key_released(&mut self, id: SessionId) -> Result<DispatchResult, ReducerError> {
        if self.active_session(id)?.mode == RecordingMode::Toggle {
            return Ok(DispatchResult::ignored(IgnoredEvent::ToggleKeyUp));
        }
        self.stop_current()
    }

    fn stop_current(&mut self) -> Result<DispatchResult, ReducerError> {
        let id = self.current_session_id.ok_or(ReducerError::StaleSession)?;
        match self.state {
            SessionState::Loading => {
                let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
                if session.microphone_start_pending {
                    session.release_pending = true;
                    return Ok(DispatchResult::applied(Vec::new()));
                }
                self.cancel_active(id, false)
            }
            SessionState::Listening => {
                let input_captured = self
                    .session
                    .as_ref()
                    .ok_or(ReducerError::SessionClosed)?
                    .input_captured;
                if input_captured {
                    let stop_effect = self.stop_microphone_effect(id)?;
                    let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
                    session.microphone_open = false;
                    self.state = SessionState::Finalizing;
                    Ok(DispatchResult::applied(vec![
                        stop_effect,
                        Effect::Speech(SpeechCommand::FinalizeRecognition { session_id: id }),
                    ]))
                } else {
                    self.cancel_active(id, true)
                }
            }
            SessionState::Finalizing => Ok(DispatchResult::ignored(IgnoredEvent::AlreadyStopping)),
            _ => Err(ReducerError::InvalidTransition),
        }
    }

    fn model_prepared(&mut self, id: SessionId) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::Loading {
            return Err(ReducerError::InvalidTransition);
        }
        let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
        if session.release_pending {
            return self.cancel_active(id, false);
        }
        if session.microphone_start_pending || session.microphone_open {
            return Err(ReducerError::InvalidTransition);
        }
        session.microphone_start_pending = true;
        Ok(DispatchResult::applied(vec![Effect::Speech(
            SpeechCommand::StartMicrophone { session_id: id },
        )]))
    }

    fn microphone_started(&mut self, id: SessionId) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::Loading {
            return Err(ReducerError::InvalidTransition);
        }
        let session = self.session.as_ref().ok_or(ReducerError::SessionClosed)?;
        if !session.microphone_start_pending {
            return Err(ReducerError::InvalidTransition);
        }
        let release_pending = session.release_pending;
        let captured = session.input_captured;
        let stop_effect = if release_pending && captured {
            Some(self.stop_microphone_effect(id)?)
        } else {
            None
        };
        let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
        session.microphone_start_pending = false;
        session.microphone_open = true;
        if release_pending {
            if let Some(stop_effect) = stop_effect {
                session.microphone_open = false;
                self.state = SessionState::Finalizing;
                return Ok(DispatchResult::applied(vec![
                    stop_effect,
                    Effect::Speech(SpeechCommand::FinalizeRecognition { session_id: id }),
                ]));
            }
            return self.cancel_active(id, true);
        }
        self.state = SessionState::Listening;
        Ok(DispatchResult::applied(Vec::new()))
    }

    fn input_captured(&mut self, id: SessionId) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
        match self.state {
            SessionState::Listening => session.input_captured = true,
            SessionState::Loading if session.microphone_start_pending => {
                session.input_captured = true
            }
            _ => return Err(ReducerError::InvalidTransition),
        }
        Ok(DispatchResult::applied(Vec::new()))
    }

    fn partial(
        &mut self,
        id: SessionId,
        revision: u64,
        text: String,
    ) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::Listening {
            return Err(ReducerError::InvalidTransition);
        }
        let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
        if session.partial_revision.is_some_and(|old| revision <= old) {
            return Err(ReducerError::NonMonotonicPartialRevision);
        }
        session.partial_revision = Some(revision);
        Ok(DispatchResult::applied(vec![
            Effect::PublishPrivatePartial {
                session_id: id,
                revision,
                text,
            },
        ]))
    }

    fn final_transcript(
        &mut self,
        id: SessionId,
        text: String,
    ) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::Finalizing {
            return Err(ReducerError::InvalidTransition);
        }
        let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
        if session.final_transcript_requested {
            return Err(ReducerError::InvalidTransition);
        }
        session.final_transcript_requested = true;
        session.awaiting_cleanup = true;
        Ok(DispatchResult::applied(vec![Effect::PrepareFinalText {
            session_id: id,
            text,
        }]))
    }

    fn final_text_prepared(
        &mut self,
        id: SessionId,
        text: String,
    ) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::Finalizing {
            return Err(ReducerError::InvalidTransition);
        }
        let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
        if !session.awaiting_cleanup || session.final_text.is_some() {
            return Err(ReducerError::InvalidTransition);
        }
        session.awaiting_cleanup = false;
        if text.trim().is_empty() {
            self.finish_without_history(id);
            return Ok(DispatchResult::applied(vec![Effect::ClearPrivateResult {
                session_id: id,
            }]));
        }
        session.final_text = Some(text);
        session.target_verification_pending = true;
        Ok(DispatchResult::applied(vec![Effect::Target(
            TargetCommand::VerifyCurrent {
                session_id: id,
                captured: session.target,
            },
        )]))
    }

    fn target_verified(
        &mut self,
        id: SessionId,
        result: TargetVerification,
    ) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::Finalizing {
            return Err(ReducerError::InvalidTransition);
        }
        let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
        if !session.target_verification_pending {
            return Err(ReducerError::InvalidTransition);
        }
        session.target_verification_pending = false;
        let current = match result {
            TargetVerification::Current(current) => current,
            TargetVerification::Changed
            | TargetVerification::SecureOrProtected
            | TargetVerification::Unsupported
            | TargetVerification::Unavailable => return Ok(self.make_copy_ready(id)),
        };
        if current != session.target
            || current.security() != TargetSecurity::Ordinary
            || current.editability() != TargetEditability::Editable
        {
            return Ok(self.make_copy_ready(id));
        }
        if session.insertion_attempted {
            return Err(ReducerError::InsertionAlreadyAttempted);
        }
        let text = session
            .final_text
            .as_ref()
            .ok_or(ReducerError::InvalidTransition)?
            .clone();
        session.insertion_attempted = true;
        self.state = SessionState::Committing;
        Ok(DispatchResult::applied(vec![Effect::Target(
            TargetCommand::InsertOnce {
                session_id: id,
                target: current,
                text,
            },
        )]))
    }

    fn insertion_completed(
        &mut self,
        id: SessionId,
        outcome: InsertionOutcome,
    ) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::Committing {
            return Err(ReducerError::InvalidTransition);
        }
        if !self.session.as_ref().is_some_and(|s| s.insertion_attempted) {
            return Err(ReducerError::InsertionAlreadyAttempted);
        }
        match outcome {
            InsertionOutcome::Inserted => {
                self.finish_without_history(id);
                Ok(DispatchResult::applied(vec![Effect::ClearPrivateResult {
                    session_id: id,
                }]))
            }
            InsertionOutcome::FailedWithoutSideEffect
            | InsertionOutcome::TargetChanged
            | InsertionOutcome::SecureOrProtected
            | InsertionOutcome::Unsupported => Ok(self.make_copy_ready(id)),
            InsertionOutcome::UncertainOrPartial => {
                self.state = SessionState::Uncertain;
                Ok(DispatchResult::applied(vec![Effect::RequireManualReview {
                    session_id: id,
                }]))
            }
        }
    }

    fn uncertainty_reviewed(&mut self, id: SessionId) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::Uncertain {
            return Err(ReducerError::InvalidTransition);
        }
        Ok(self.make_copy_ready(id))
    }

    fn copy_requested(&mut self, id: SessionId) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::CopyReady {
            return Err(ReducerError::InvalidTransition);
        }
        let session = self.session.as_mut().ok_or(ReducerError::SessionClosed)?;
        if session.outstanding_copy_attempt.is_some() {
            return Ok(DispatchResult::ignored(IgnoredEvent::CopyAlreadyPending));
        }
        let text = session
            .final_text
            .as_ref()
            .cloned()
            .ok_or(ReducerError::InvalidTransition)?;
        let sequence = session
            .next_copy_attempt
            .ok_or(ReducerError::CopyAttemptExhausted)?;
        let sequence_nonzero =
            NonZeroU64::new(sequence).ok_or(ReducerError::CopyAttemptExhausted)?;
        session.next_copy_attempt = sequence.checked_add(1);
        let attempt_id = CopyAttemptId::new(id, sequence_nonzero);
        session.outstanding_copy_attempt = Some(attempt_id);
        Ok(DispatchResult::applied(vec![
            Effect::ExplicitCopyRequested {
                session_id: id,
                attempt_id,
                text,
            },
        ]))
    }

    fn copy_completed(
        &mut self,
        id: SessionId,
        attempt_id: CopyAttemptId,
        outcome: CopyOutcome,
    ) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if self.state != SessionState::CopyReady {
            return Err(ReducerError::InvalidTransition);
        }
        if attempt_id.session_id() != id
            || self
                .session
                .as_ref()
                .and_then(|session| session.outstanding_copy_attempt)
                != Some(attempt_id)
        {
            return Err(ReducerError::CopyAttemptMismatch);
        }
        if let Some(session) = self.session.as_mut() {
            session.outstanding_copy_attempt = None;
        }
        match outcome {
            CopyOutcome::Succeeded => {
                self.finish_without_history(id);
                Ok(DispatchResult::applied(vec![
                    Effect::DismissAndClearResult { session_id: id },
                ]))
            }
            CopyOutcome::Failed => Ok(DispatchResult::applied(vec![Effect::CopyFailed {
                session_id: id,
            }])),
        }
    }

    fn dismiss(&mut self, expected: Option<SessionId>) -> Result<DispatchResult, ReducerError> {
        if self.state == SessionState::Idle {
            return if expected.is_some() {
                Err(ReducerError::StaleSession)
            } else {
                Ok(DispatchResult::ignored(IgnoredEvent::AlreadyIdle))
            };
        }
        if self.state == SessionState::Uninitialized {
            return Err(ReducerError::InvalidTransition);
        }
        if self.current_session_id != expected {
            return Err(ReducerError::StaleSession);
        }
        if !matches!(
            self.state,
            SessionState::CopyReady
                | SessionState::Uncertain
                | SessionState::Cancelled
                | SessionState::Faulted
        ) {
            return Err(ReducerError::InvalidTransition);
        }
        let had_result = matches!(
            self.state,
            SessionState::CopyReady | SessionState::Uncertain
        );
        let effects = match (self.current_session_id, had_result) {
            (Some(id), true) => vec![Effect::DismissAndClearResult { session_id: id }],
            (Some(id), false) => vec![Effect::ClearPrivateResult { session_id: id }],
            (None, _) => Vec::new(),
        };
        self.session = None;
        self.current_session_id = None;
        self.failure = None;
        self.state = SessionState::Idle;
        Ok(DispatchResult::applied(effects))
    }

    fn cancel(&mut self, id: SessionId) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if matches!(
            self.state,
            SessionState::Committing | SessionState::Uncertain
        ) {
            return Err(ReducerError::CancellationTooLate);
        }
        if self.state == SessionState::CopyReady {
            self.session = None;
            self.state = SessionState::Cancelled;
            return Ok(DispatchResult::applied(vec![
                Effect::DismissAndClearResult { session_id: id },
            ]));
        }
        if !matches!(
            self.state,
            SessionState::Loading | SessionState::Listening | SessionState::Finalizing
        ) {
            return Err(ReducerError::InvalidTransition);
        }
        let stop = self
            .session
            .as_ref()
            .is_some_and(|s| s.microphone_open || s.microphone_start_pending);
        self.cancel_active(id, stop)
    }

    fn cancel_for_fault(
        &mut self,
        id: SessionId,
        code: FailureCode,
    ) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if !matches!(
            self.state,
            SessionState::Loading | SessionState::Listening | SessionState::Finalizing
        ) {
            return Err(ReducerError::InvalidTransition);
        }
        let mut effects = self.cancel_effects(id)?;
        effects.push(Effect::ReportFailure {
            session_id: Some(id),
            code,
        });
        self.failure = Some(code);
        self.session = None;
        self.state = SessionState::Cancelled;
        Ok(DispatchResult::applied(effects))
    }

    fn cancel_active(
        &mut self,
        id: SessionId,
        stop_microphone: bool,
    ) -> Result<DispatchResult, ReducerError> {
        let mut effects = Vec::new();
        if stop_microphone {
            effects.push(self.stop_microphone_effect(id)?);
        }
        effects.push(Effect::Speech(SpeechCommand::CancelRecognition {
            session_id: id,
        }));
        effects.push(Effect::ClearPrivateResult { session_id: id });
        self.session = None;
        self.failure = None;
        self.state = SessionState::Cancelled;
        Ok(DispatchResult::applied(effects))
    }

    fn fail_active(
        &mut self,
        id: SessionId,
        code: FailureCode,
    ) -> Result<DispatchResult, ReducerError> {
        self.active_session(id)?;
        if !matches!(
            self.state,
            SessionState::Loading | SessionState::Listening | SessionState::Finalizing
        ) {
            return Err(ReducerError::InvalidTransition);
        }
        let stop = self
            .session
            .as_ref()
            .is_some_and(|s| s.microphone_open || s.microphone_start_pending);
        let mut effects = Vec::new();
        if stop {
            effects.push(self.stop_microphone_effect(id)?);
        }
        effects.push(Effect::Speech(SpeechCommand::CancelRecognition {
            session_id: id,
        }));
        effects.push(Effect::ClearPrivateResult { session_id: id });
        effects.push(Effect::ReportFailure {
            session_id: Some(id),
            code,
        });
        self.session = None;
        self.failure = Some(code);
        self.state = SessionState::Faulted;
        Ok(DispatchResult::applied(effects))
    }

    fn cancel_effects(&mut self, id: SessionId) -> Result<Vec<Effect>, ReducerError> {
        let mut effects = Vec::new();
        if self
            .session
            .as_ref()
            .is_some_and(|s| s.microphone_open || s.microphone_start_pending)
        {
            effects.push(self.stop_microphone_effect(id)?);
        }
        effects.push(Effect::Speech(SpeechCommand::CancelRecognition {
            session_id: id,
        }));
        effects.push(Effect::ClearPrivateResult { session_id: id });
        Ok(effects)
    }

    fn make_copy_ready(&mut self, id: SessionId) -> DispatchResult {
        let text = self.session.as_ref().and_then(|s| s.final_text.clone());
        match text {
            Some(text) => {
                self.state = SessionState::CopyReady;
                DispatchResult::applied(vec![Effect::ShowCopyResult {
                    session_id: id,
                    text,
                }])
            }
            None => {
                self.state = SessionState::Faulted;
                self.failure = Some(FailureCode::RecognitionFailed);
                DispatchResult::applied(vec![Effect::ReportFailure {
                    session_id: Some(id),
                    code: FailureCode::RecognitionFailed,
                }])
            }
        }
    }

    fn finish_without_history(&mut self, id: SessionId) {
        if self.current_session_id == Some(id) {
            self.session = None;
            self.current_session_id = None;
            self.failure = None;
            self.state = SessionState::Idle;
        }
    }

    fn active_session(&self, id: SessionId) -> Result<&SessionData, ReducerError> {
        if self.current_session_id != Some(id) {
            return Err(ReducerError::StaleSession);
        }
        if self.session.is_none()
            || matches!(self.state, SessionState::Cancelled | SessionState::Faulted)
        {
            return Err(ReducerError::SessionClosed);
        }
        self.session.as_ref().ok_or(ReducerError::SessionClosed)
    }
}

/// Ordered effects from one event.
#[derive(Debug, PartialEq, Eq)]
pub struct DispatchResult {
    /// State change or accepted idempotent no-op.
    pub outcome: DispatchOutcome,
    /// Typed effects in required order.
    pub effects: Vec<Effect>,
}

impl DispatchResult {
    fn applied(effects: Vec<Effect>) -> Self {
        Self {
            outcome: DispatchOutcome::Applied,
            effects,
        }
    }

    fn ignored(reason: IgnoredEvent) -> Self {
        Self {
            outcome: DispatchOutcome::Ignored(reason),
            effects: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventSequence, EventSequencer, ReducerError, allocate_session_id};
    use std::sync::atomic::AtomicU64;

    #[test]
    fn event_sequence_overflow_is_explicit_and_never_wraps() {
        let mut sequence = EventSequencer {
            next: Some(u64::MAX),
        };
        assert_eq!(sequence.next_sequence(), Ok(EventSequence(u64::MAX)));
        assert_eq!(
            sequence.next_sequence(),
            Err(ReducerError::EventSequenceExhausted)
        );
    }

    #[test]
    fn session_identifier_overflow_is_explicit_and_does_not_reuse_an_id() {
        let counter = AtomicU64::new(u64::MAX);
        assert_eq!(allocate_session_id(&counter).unwrap().get(), u64::MAX);
        assert_eq!(
            allocate_session_id(&counter),
            Err(ReducerError::SessionIdExhausted)
        );
    }
}
