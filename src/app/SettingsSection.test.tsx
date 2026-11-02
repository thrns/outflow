import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsSection } from "./SettingsSection";

describe("SettingsSection", () => {
  it("renders its section heading", () => {
    render(
      <SettingsSection id="microphone" title="Microphone">
        <p>Input settings</p>
      </SettingsSection>,
    );

    expect(
      screen.getByRole("heading", { level: 2, name: "Microphone" }),
    ).toBeVisible();
  });

  it("renders an optional description", () => {
    render(
      <SettingsSection
        id="shortcuts"
        title="Shortcuts"
        description="Choose how dictation starts and stops."
      >
        <p>Shortcut settings</p>
      </SettingsSection>,
    );

    expect(
      screen.getByText("Choose how dictation starts and stops."),
    ).toBeVisible();
  });

  it("renders children inside the section", () => {
    render(
      <SettingsSection id="privacy" title="Privacy">
        <button type="button">Review privacy settings</button>
      </SettingsSection>,
    );

    const panel = screen.getByRole("tabpanel", { name: "Privacy" });

    expect(
      within(panel).getByRole("button", { name: "Review privacy settings" }),
    ).toBeVisible();
  });

  it("omits the description when it is not provided", () => {
    render(
      <SettingsSection id="general" title="General">
        <p>General settings</p>
      </SettingsSection>,
    );

    expect(
      screen.getByRole("heading", { level: 2, name: "General" }),
    ).toBeVisible();
    expect(screen.queryByText("Choose how dictation starts and stops.")).not.toBeInTheDocument();
  });

  it("provides an accessible tab panel named by its heading", () => {
    render(
      <SettingsSection id="text-vocabulary" title="Text & Vocabulary">
        <p>Text settings</p>
      </SettingsSection>,
    );

    const panel = screen.getByRole("tabpanel", { name: "Text & Vocabulary" });
    const heading = screen.getByRole("heading", {
      level: 2,
      name: "Text & Vocabulary",
    });

    expect(panel).toHaveAttribute("aria-labelledby", heading.id);
    expect(heading.id).toBe("text-vocabulary-heading");
  });

  it("preserves a supplied heading ID", () => {
    render(
      <SettingsSection
        id="settings-panel"
        headingId="about-diagnostics-heading"
        title="About & Diagnostics"
      >
        <p>Application details</p>
      </SettingsSection>,
    );

    const heading = screen.getByRole("heading", {
      level: 2,
      name: "About & Diagnostics",
    });
    const panel = screen.getByRole("tabpanel", {
      name: "About & Diagnostics",
    });

    expect(heading).toHaveAttribute("id", "about-diagnostics-heading");
    expect(panel).toHaveAttribute("aria-labelledby", "about-diagnostics-heading");
  });
});
