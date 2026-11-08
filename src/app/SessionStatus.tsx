import type { ReactNode } from "react";

export type SessionPresentation =
  | { state: "ready" }
  | { state: "preparing" }
  | { state: "recording" }
  | { state: "finalizing" }
  | { state: "inserted" }
  | { state: "no-speech" }
  | { state: "copy-ready" }
  | { state: "uncertain" }
  | {
      state: "error";
      reason: "microphone-permission" | "audio-device" | "speech-engine";
    }
  | { state: "unavailable" };

type StatusDetails = {
  label: string;
  description: ReactNode;
};

const errorDetails = {
  "microphone-permission": {
    label: "Microphone permission is unavailable",
    description:
      "Review microphone access in system settings, then try again.",
  },
  "audio-device": {
    label: "Audio input is unavailable",
    description: "Reconnect the input device, then try again.",
  },
  "speech-engine": {
    label: "Offline speech recognition could not start",
    description: "Restart Outflow. If the problem continues, reinstall the app.",
  },
} as const;

function getStatusDetails(presentation: SessionPresentation): StatusDetails {
  switch (presentation.state) {
    case "ready":
      return {
        label: "Ready to dictate",
        description: "No session is active.",
      };
    case "preparing":
      return {
        label: "Preparing offline speech recognition…",
        description: "Speech recognition is getting ready for this session.",
      };
    case "recording":
      return {
        label: "Recording locally",
        description: "Audio stays on this device during dictation.",
      };
    case "finalizing":
      return {
        label: "Finishing transcription…",
        description: "The current session is being finalized.",
      };
    case "inserted":
      return {
        label: "Text inserted",
        description: "The session finished successfully.",
      };
    case "no-speech":
      return {
        label: "No speech detected",
        description:
          "No text was produced. You can try again when dictation is connected.",
      };
    case "copy-ready":
      return {
        label: "Couldn't safely insert in that field",
        description: "Review the reason and final text below.",
      };
    case "uncertain":
      return {
        label: "Review destination before continuing",
        description:
          "Manual review is required before any insertion retry or Copy request.",
      };
    case "error":
      return errorDetails[presentation.reason];
    case "unavailable":
      return {
        label: "Native dictation is not connected",
        description:
          "This preview does not request microphone access or start a recording session. No speech model is connected.",
      };
  }
}

export type SessionStatusProps = {
  presentation: SessionPresentation;
  synthetic?: boolean;
};

/** Renders content-free session status separately from any native event transport. */
export function SessionStatus({
  presentation,
  synthetic = false,
}: SessionStatusProps) {
  const details = getStatusDetails(presentation);

  return (
    <section
      aria-labelledby="home-status-heading"
      className="home-status-card"
      data-session-state={presentation.state}
    >
      <p className="home-status-label">Development preview</p>
      <h2
        aria-atomic="true"
        aria-live="polite"
        id="home-status-heading"
      >
        {details.label}
      </h2>
      <p className="home-status-description">{details.description}</p>
      {synthetic && (
        <p className="session-status-simulation-note">
          Synthetic preview only. No microphone is active.
        </p>
      )}
    </section>
  );
}
