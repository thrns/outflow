import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SessionStatus, type SessionPresentation } from "./SessionStatus";

const presentations: Array<[string, SessionPresentation]> = [
  ["Ready to dictate", { state: "ready" }],
  ["Preparing offline speech recognition…", { state: "preparing" }],
  ["Recording locally", { state: "recording" }],
  ["Finishing transcription…", { state: "finalizing" }],
  ["Text inserted", { state: "inserted" }],
  ["No speech detected", { state: "no-speech" }],
  ["Couldn't safely insert in that field", { state: "copy-ready" }],
  ["Review destination before continuing", { state: "uncertain" }],
  [
    "Audio input is unavailable",
    { state: "error", reason: "audio-device" },
  ],
  ["Native dictation is not connected", { state: "unavailable" }],
];

describe("SessionStatus", () => {
  it.each(presentations)("shows the %s presentation", (label, presentation) => {
    render(<SessionStatus presentation={presentation} />);

    const heading = screen.getByRole("heading", { level: 2, name: label });
    expect(heading).toBeVisible();
    expect(heading).toHaveAttribute("aria-live", "polite");
    expect(heading).toHaveAttribute("aria-atomic", "true");
  });

  it.each([
    [
      "microphone-permission",
      "Microphone permission is unavailable",
      "Review microphone access in system settings, then try again.",
    ],
    [
      "audio-device",
      "Audio input is unavailable",
      "Reconnect the input device, then try again.",
    ],
    [
      "speech-engine",
      "Offline speech recognition could not start",
      "Restart Outflow. If the problem continues, reinstall the app.",
    ],
  ] as const)(
    "gives stable recovery guidance for %s errors",
    (reason, label, guidance) => {
      render(
        <SessionStatus presentation={{ state: "error", reason }} />,
      );

      expect(screen.getByRole("heading", { name: label })).toBeVisible();
      expect(screen.getByText(guidance)).toBeVisible();
    },
  );

  it("marks simulated recording as synthetic and states that no microphone is active", () => {
    render(
      <SessionStatus presentation={{ state: "recording" }} synthetic />,
    );

    expect(
      screen.getByText("Synthetic preview only. No microphone is active."),
    ).toBeVisible();
  });
});
