import { useEffect, useRef, useState } from "react";

export type CopyFallbackReason =
  | "changed-target"
  | "unsupported-control"
  | "protected-target"
  | "uncertain-reviewed";

export type CopyRequest = {
  /**
   * Opaque UI correlation token, not authorization. A future native bridge must
   * resolve it to the Rust coordinator's actual session and CopyAttemptId.
   */
  requestId: string;
};

export type CopyAcknowledgment = {
  requestId: string;
  outcome: "succeeded" | "failed";
};

export type CopyResult = {
  text: string;
  reason: CopyFallbackReason;
};

type CopyResultPanelProps = {
  result: CopyResult;
  /** Parent-owned scope unique to this result lifecycle for request correlation. */
  requestScope: string;
  onDismiss: () => void;
  onResultCopied: () => void;
  /** Only an explicitly connected clipboard adapter should provide this callback. */
  onCopyRequest?: (request: CopyRequest) => void;
  copyAcknowledgment?: CopyAcknowledgment | null;
  /** Labels the connected callback as a development-only mock, never as clipboard access. */
  developmentCopySimulation?: boolean;
};

const reasonText: Record<CopyFallbackReason, string> = {
  "changed-target":
    "The destination changed while dictation was in progress. No text was inserted.",
  "unsupported-control":
    "This control does not support safe text insertion. No text was inserted.",
  "protected-target":
    "This is a protected destination. Outflow did not type into it.",
  "uncertain-reviewed":
    "The insertion outcome was uncertain. Inspect the destination before deciding whether to copy; some text may already be there.",
};

export function CopyResultPanel({
  result,
  requestScope,
  onDismiss,
  onResultCopied,
  onCopyRequest,
  copyAcknowledgment,
  developmentCopySimulation = false,
}: CopyResultPanelProps) {
  const resultField = useRef<HTMLTextAreaElement | null>(null);
  const nextRequestNumber = useRef(1);
  const pendingRequest = useRef<CopyRequest | null>(null);
  const handledAcknowledgment = useRef<string | null>(null);
  const [pendingRequestId, setPendingRequestId] = useState<string | null>(null);
  const [feedback, setFeedback] = useState("");
  const [selectionFeedback, setSelectionFeedback] = useState("");
  const [protectedCopyAcknowledged, setProtectedCopyAcknowledged] =
    useState(false);

  useEffect(() => {
    const request = pendingRequest.current;
    const acknowledgment = copyAcknowledgment;

    if (
      !request ||
      !acknowledgment ||
      acknowledgment.requestId !== request.requestId ||
      handledAcknowledgment.current === acknowledgment.requestId
    ) {
      return;
    }

    handledAcknowledgment.current = acknowledgment.requestId;
    pendingRequest.current = null;
    setPendingRequestId(null);

    if (acknowledgment.outcome === "succeeded") {
      setFeedback("Copy completed.");
      onResultCopied();
      return;
    }

    setFeedback("Copy failed. Your result is still available in this panel.");
  }, [copyAcknowledgment, onResultCopied]);

  function requestCopy() {
    if (
      !onCopyRequest ||
      pendingRequest.current ||
      (result.reason === "protected-target" && !protectedCopyAcknowledged)
    ) {
      return;
    }

    const request: CopyRequest = {
      requestId: requestScope + ":" + nextRequestNumber.current,
    };
    nextRequestNumber.current += 1;
    pendingRequest.current = request;
    setPendingRequestId(request.requestId);
    setFeedback("Copy request pending.");

    try {
      onCopyRequest(request);
    } catch {
      pendingRequest.current = null;
      setPendingRequestId(null);
      setFeedback("Copy failed. Your result is still available in this panel.");
    }
  }

  function selectAll() {
    resultField.current?.select();
    setSelectionFeedback("The full result is selected. Nothing was copied.");
  }

  const isProtected = result.reason === "protected-target";
  const copyDisabled =
    !onCopyRequest ||
    pendingRequestId !== null ||
    (isProtected && !protectedCopyAcknowledged);
  const copyHelp = !onCopyRequest
    ? "Copy is unavailable because this browser preview has no clipboard adapter connected."
    : isProtected && !protectedCopyAcknowledged
      ? "Acknowledge the protected-destination notice before requesting Copy."
      : developmentCopySimulation
        ? "This development-only mock never accesses the system clipboard."
        : "Copy sends one explicit request to the connected adapter.";

  return (
    <section
      aria-labelledby="copy-result-heading"
      className="home-status-card copy-result-card"
      data-copy-reason={result.reason}
      data-result-lifecycle-id={requestScope}
    >
      <p className="home-status-label">Copy Result</p>
      <h2 id="copy-result-heading">Review finalized text</h2>
      <p className="copy-result-reason">{reasonText[result.reason]}</p>

      <label className="copy-result-field-label" htmlFor="copy-result-text">
        Finalized text (kept in memory until dismissed)
      </label>
      <textarea
        aria-describedby="copy-result-memory-note"
        className="copy-result-text"
        id="copy-result-text"
        readOnly
        ref={resultField}
        rows={4}
        value={result.text}
      />
      <p className="copy-result-memory-note" id="copy-result-memory-note">
        This result is only held in this open view. There is no transcript history.
      </p>

      {isProtected && (
        <label className="copy-result-protected-acknowledgment">
          <input
            checked={protectedCopyAcknowledged}
            onChange={(event) =>
              setProtectedCopyAcknowledged(event.currentTarget.checked)
            }
            type="checkbox"
          />
          <span>
            I understand that copied text from a protected destination may be
            retained outside Outflow.
          </span>
        </label>
      )}

      <p className="copy-result-clipboard-warning">
        Your operating system's clipboard managers or synchronization services
        may retain copied text.
      </p>

      <div
        aria-busy={pendingRequestId !== null}
        aria-label="Copy Result actions"
        className="copy-result-actions"
      >
        <button
          aria-describedby="copy-result-unavailable-note"
          className="copy-result-action copy-result-action-primary"
          disabled={copyDisabled}
          onClick={requestCopy}
          type="button"
        >
          {pendingRequestId
            ? developmentCopySimulation
              ? "Simulated Copy pending…"
              : "Copy pending…"
            : developmentCopySimulation
              ? "Simulate Copy request"
              : "Copy"}
        </button>
        <button
          className="copy-result-action"
          onClick={selectAll}
          type="button"
        >
          Select All
        </button>
        <button
          className="copy-result-action"
          onClick={onDismiss}
          type="button"
        >
          Dismiss
        </button>
      </div>

      <p className="copy-result-unavailable-note" id="copy-result-unavailable-note">
        {copyHelp}
      </p>

      {(feedback || selectionFeedback) && (
        <p aria-atomic="true" aria-live="polite" className="copy-result-feedback">
          {feedback || selectionFeedback}
        </p>
      )}
    </section>
  );
}
