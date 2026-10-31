import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("Outflow foundation screen", () => {
  it("renders without errors", () => {
    expect(() => render(<App />)).not.toThrow();
  });

  it("shows the application name", () => {
    render(<App />);

    expect(screen.getByText("Outflow", { selector: "p" })).toBeVisible();
  });

  it("shows the foundation heading", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { level: 1, name: "Foundation ready" }),
    ).toBeVisible();
  });

  it("shows the supporting description", () => {
    render(<App />);

    expect(
      screen.getByText("Your application is set up and ready for development."),
    ).toBeVisible();
  });

  it("opens Settings from the foundation screen", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(
      screen.getByRole("heading", { level: 1, name: "Settings" }),
    ).toBeVisible();
    expect(
      screen.getByText("Configure your offline dictation experience."),
    ).toBeVisible();
  });

  it("shows all six settings sections in an accessible tab list", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(screen.getByRole("tablist")).toBeVisible();
    expect(screen.getAllByRole("tab")).toHaveLength(6);
    for (const section of [
      "General",
      "Microphone",
      "Shortcuts",
      "Text & Vocabulary",
      "Privacy",
      "About & Diagnostics",
    ]) {
      expect(screen.getByRole("tab", { name: section })).toBeVisible();
    }
  });

  it("starts with General selected", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(
      screen.getByRole("heading", { level: 2, name: "General" }),
    ).toBeVisible();
    expect(
      screen.getByText("Manage general application preferences."),
    ).toBeVisible();
    expect(
      screen.getByText("Start Outflow automatically when you sign in to your computer."),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Preview only — this setting does not change your computer's startup behavior yet.",
      ),
    ).toBeVisible();
    expect(
      screen.getByRole("switch", { name: "Launch at Login" }),
    ).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("Off", { selector: ".settings-switch-state" })).toBeVisible();
  });

  it("toggles Launch at Login on and off with the mouse", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    const launchAtLogin = screen.getByRole("switch", {
      name: "Launch at Login",
    });
    fireEvent.click(launchAtLogin);

    expect(launchAtLogin).toHaveAttribute("aria-checked", "true");
    expect(
      screen.getByText("On", { selector: ".settings-switch-state" }),
    ).toBeVisible();

    fireEvent.click(launchAtLogin);

    expect(launchAtLogin).toHaveAttribute("aria-checked", "false");
    expect(
      screen.getByText("Off", { selector: ".settings-switch-state" }),
    ).toBeVisible();
  });

  it("toggles Launch at Login with Space and ignores a repeated keydown", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    const launchAtLogin = screen.getByRole("switch", {
      name: "Launch at Login",
    });
    launchAtLogin.focus();
    expect(launchAtLogin).toHaveFocus();

    fireEvent.keyDown(launchAtLogin, { key: " " });
    expect(launchAtLogin).toHaveAttribute("aria-checked", "true");

    fireEvent.keyUp(launchAtLogin, { key: " " });
    fireEvent.keyDown(launchAtLogin, { key: " ", repeat: true });
    expect(launchAtLogin).toHaveAttribute("aria-checked", "true");
  });

  it("keeps the preview value when navigating away from and back to General", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Launch at Login" }),
    );
    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));

    expect(
      screen.getByRole("heading", { level: 2, name: "Privacy" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("tab", { name: "General" }));

    expect(
      screen.getByRole("switch", { name: "Launch at Login" }),
    ).toHaveAttribute("aria-checked", "true");
    expect(
      screen.getByText("On", { selector: ".settings-switch-state" }),
    ).toBeVisible();
  });

  it("keeps the preview value when Settings is closed and reopened", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Launch at Login" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(
      screen.getByRole("switch", { name: "Launch at Login" }),
    ).toHaveAttribute("aria-checked", "true");
    expect(
      screen.getByText("On", { selector: ".settings-switch-state" }),
    ).toBeVisible();
  });

  it("keeps the other Settings sections and their placeholders available", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    for (const section of [
      "Microphone",
      "Shortcuts",
      "Text & Vocabulary",
      "Privacy",
      "About & Diagnostics",
    ]) {
      fireEvent.click(screen.getByRole("tab", { name: section }));

      expect(
        screen.getByRole("heading", { level: 2, name: section }),
      ).toBeVisible();
      expect(
        screen.getByText("Settings controls will be added in a later step."),
      ).toBeVisible();
    }
  });

  it("updates section content and selected state when a tab is chosen", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));

    expect(screen.getByRole("tab", { name: "Privacy" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
    expect(
      screen.getByRole("heading", { level: 2, name: "Privacy" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Review how Outflow handles audio, transcripts, and diagnostics.",
      ),
    ).toBeVisible();
  });

  it("supports arrow-key navigation between sections", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    const generalTab = screen.getByRole("tab", { name: "General" });
    generalTab.focus();
    fireEvent.keyDown(generalTab, { key: "ArrowDown" });

    const microphoneTab = screen.getByRole("tab", { name: "Microphone" });
    expect(microphoneTab).toHaveFocus();
    expect(microphoneTab).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByRole("heading", { level: 2, name: "Microphone" }),
    ).toBeVisible();
  });

  it("returns to the foundation screen with Back", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));
    fireEvent.click(screen.getByRole("button", { name: "Back" }));

    expect(
      screen.getByRole("heading", { level: 1, name: "Foundation ready" }),
    ).toBeVisible();
    expect(
      screen.getByText("Your application is set up and ready for development."),
    ).toBeVisible();
  });

  it("opens Settings to General again after returning to the foundation", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(
      screen.getByRole("heading", { level: 2, name: "General" }),
    ).toBeVisible();
  });
});
