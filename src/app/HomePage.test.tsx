import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

const originalClipboard = Object.getOwnPropertyDescriptor(navigator, "clipboard");
const originalMediaDevices = Object.getOwnPropertyDescriptor(navigator, "mediaDevices");

afterEach(() => {
  for (const [property, descriptor] of [
    ["clipboard", originalClipboard],
    ["mediaDevices", originalMediaDevices],
  ] as const) {
    if (descriptor) Object.defineProperty(navigator, property, descriptor);
    else Reflect.deleteProperty(navigator, property);
  }
  vi.unstubAllEnvs();
});

function selectScenario(label: string) {
  fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
    target: { value: label },
  });
}

describe("Home session and Copy Result preview", () => {
  it("keeps native dictation clearly unavailable by default", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", {
        level: 2,
        name: "Native dictation is not connected",
      }),
    ).toBeVisible();
    expect(
      screen.getByText(/does not request microphone access or start a recording session/),
    ).toBeVisible();
    expect(
      screen.getByText(/Recording and native dictation are not connected/),
    ).toBeVisible();
  });

  it("labels the fixed scenario selector and every fixture as development-only synthetic content", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { name: "Inspect synthetic scenarios" }),
    ).toBeVisible();
    expect(
      screen.getByText(/Every state and result is synthetic/),
    ).toBeVisible();
    expect(screen.getAllByRole("option")).toHaveLength(11);
  });

  it("renders Preparing and Recording as synthetic without implying an active microphone", () => {
    render(<App />);

    selectScenario("preparing");
    expect(
      screen.getByRole("heading", {
        name: "Preparing offline speech recognition…",
      }),
    ).toBeVisible();

    selectScenario("recording");
    expect(
      screen.getByRole("heading", { name: "Recording locally" }),
    ).toBeVisible();
    expect(
      screen.getByText("Synthetic preview only. No microphone is active."),
    ).toBeVisible();
  });

  it.each([
    ["finalizing", "Finishing transcription…"],
    ["inserted", "Text inserted"],
    ["no-speech", "No speech detected"],
    ["recoverable-error", "Audio input is unavailable"],
  ])("shows the %s Home scenario", (scenario, label) => {
    render(<App />);
    selectScenario(scenario);

    expect(screen.getByRole("heading", { level: 2, name: label })).toBeVisible();
    expect(
      screen.getByText("Synthetic preview only. No microphone is active."),
    ).toBeVisible();
  });

  it("shows a selectable CopyReady result and never copies when the panel opens", () => {
    const clipboardWrite = vi.fn();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: clipboardWrite },
    });
    render(<App />);

    selectScenario("changed-target");

    expect(
      screen.getByRole("heading", {
        name: "Couldn't safely insert in that field",
      }),
    ).toBeVisible();
    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveValue("Synthetic preview result: Outflow keeps this sample in memory.");
    expect(clipboardWrite).not.toHaveBeenCalled();
  });

  it("does not call microphone or clipboard APIs when scenarios change", () => {
    const clipboardWrite = vi.fn();
    const getUserMedia = vi.fn();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: clipboardWrite },
    });
    Object.defineProperty(navigator, "mediaDevices", {
      configurable: true,
      value: { getUserMedia },
    });
    render(<App />);

    for (const scenario of ["preparing", "recording", "finalizing", "changed-target"]) {
      selectScenario(scenario);
    }

    expect(getUserMedia).not.toHaveBeenCalled();
    expect(clipboardWrite).not.toHaveBeenCalled();
  });

  it("keeps Copy disabled in the browser preview and simulates failure without clipboard access", () => {
    const clipboardWrite = vi.fn();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: clipboardWrite },
    });
    render(<App />);

    selectScenario("copy-failure");
    expect(screen.getByRole("button", { name: "Copy" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Simulate Copy action" }));

    expect(
      screen.getByText(/Simulated Copy failed\. No clipboard was accessed/),
    ).toBeVisible();
    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveValue("Synthetic preview result: Outflow keeps this sample in memory.");
    expect(clipboardWrite).not.toHaveBeenCalled();
  });

  it("blocks Copy during uncertain insertion until one explicit review acknowledgment", () => {
    render(<App />);

    selectScenario("uncertain");
    expect(
      screen.getByRole("heading", {
        name: "Review destination before continuing",
      }),
    ).toBeVisible();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(
      screen.getByText(/Text may already have been partially inserted/),
    ).toBeVisible();
    expect(screen.queryByRole("button", { name: "Copy" })).not.toBeInTheDocument();

    const reviewButton = screen.getByRole("button", {
      name: "I have reviewed the destination",
    });
    fireEvent.click(reviewButton);
    fireEvent.click(reviewButton);

    expect(
      screen.getByRole("heading", {
        name: "Couldn't safely insert in that field",
      }),
    ).toBeVisible();
    expect(
      screen.getByText(/Inspect the destination before deciding whether to copy/),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "Copy" })).toBeDisabled();
  });

  it("requires a protected-target acknowledgment before any connected Copy request", () => {
    render(<App />);

    selectScenario("protected-target");

    expect(
      screen.getByRole("checkbox", {
        name: /copied text from a protected destination may be retained/,
      }),
    ).not.toBeChecked();
    expect(screen.getByRole("button", { name: "Copy" })).toBeDisabled();
  });

  it("discards the previous result when another scenario is selected", () => {
    render(<App />);

    selectScenario("changed-target");
    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toBeInTheDocument();

    selectScenario("preparing");
    expect(
      screen.queryByLabelText("Finalized text (kept in memory until dismissed)"),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/Synthetic preview result:/)).not.toBeInTheDocument();
  });

  it("clears the result on Dismiss and does not restore it after a remount", () => {
    const view = render(<App />);
    selectScenario("copy-failure");
    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(
      screen.queryByLabelText("Finalized text (kept in memory until dismissed)"),
    ).not.toBeInTheDocument();

    view.unmount();
    render(<App />);

    expect(
      screen.queryByText(/Synthetic preview result:/),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("heading", {
        name: "Native dictation is not connected",
      }),
    ).toBeVisible();
  });

  it("keeps the scenario controls out of non-development rendering", () => {
    vi.stubEnv("DEV", false);
    try {
      render(<App />);

      expect(
        screen.queryByRole("combobox", { name: "Scenario" }),
      ).not.toBeInTheDocument();
      expect(
        screen.getByRole("heading", {
          name: "Native dictation is not connected",
        }),
      ).toBeVisible();
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it("exposes keyboard-focusable controls and accessible live status", () => {
    render(<App />);

    const scenario = screen.getByRole("combobox", { name: "Scenario" });
    scenario.focus();
    expect(scenario).toHaveFocus();
    expect(scenario.tabIndex).toBe(0);

    selectScenario("changed-target");
    const copyButton = screen.getByRole("button", { name: "Copy" });
    const selectAllButton = screen.getByRole("button", { name: "Select All" });
    const dismissButton = screen.getByRole("button", { name: "Dismiss" });
    expect(copyButton).toBeDisabled();
    for (const control of [selectAllButton, dismissButton]) {
      expect(control.tabIndex).toBe(0);
      control.focus();
      expect(control).toHaveFocus();
    }

    expect(
      screen.getByRole("heading", {
        name: "Couldn't safely insert in that field",
      }),
    ).toHaveAttribute("aria-live", "polite");
    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveAttribute("aria-describedby", "copy-result-memory-note");
  });

  it("preserves Settings navigation and its existing keyboard-focus handoff", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(
      screen.getByRole("heading", { level: 1, name: "Settings" }),
    ).toHaveFocus();
    expect(screen.getByRole("tablist")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Home" }));
    expect(
      screen.getByRole("heading", { level: 1, name: "Home" }),
    ).toHaveFocus();
  });
});
