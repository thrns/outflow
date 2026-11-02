import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsStatusMessage } from "./SettingsStatusMessage";

describe("SettingsStatusMessage", () => {
  it("renders its title as a heading", () => {
    render(
      <SettingsStatusMessage title="Microphone setup unavailable">
        No audio is captured in this preview.
      </SettingsStatusMessage>,
    );

    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Microphone setup unavailable",
      }),
    ).toBeVisible();
  });

  it("renders explanatory content", () => {
    render(
      <SettingsStatusMessage title="Preview information">
        Microphone access has not been checked.
      </SettingsStatusMessage>,
    );

    expect(
      screen.getByText("Microphone access has not been checked."),
    ).toBeVisible();
  });

  it("supports React children", () => {
    const { container } = render(
      <SettingsStatusMessage title="Preview information">
        <strong>No audio</strong> is captured in this preview.
      </SettingsStatusMessage>,
    );

    expect(screen.getByText("No audio")).toBeVisible();
    expect(container.querySelector("p")).toHaveTextContent(
      "No audio is captured in this preview.",
    );
  });

  it("exposes its title and explanation as accessible text", () => {
    render(
      <SettingsStatusMessage title="Microphone setup unavailable">
        Device availability and permissions have not been checked.
      </SettingsStatusMessage>,
    );

    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Microphone setup unavailable",
      }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Device availability and permissions have not been checked.",
      ),
    ).toBeVisible();
  });

  it("does not use alert or live-region semantics for static information", () => {
    const { container } = render(
      <SettingsStatusMessage title="Preview information">
        This information is present when the page renders.
      </SettingsStatusMessage>,
    );
    const message = container.querySelector(".settings-status-message");

    expect(message).not.toHaveAttribute("role");
    expect(message).not.toHaveAttribute("aria-live");
    expect(container.querySelector("[aria-live]")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("renders without interactive controls by default", () => {
    const { container } = render(
      <SettingsStatusMessage title="Preview information">
        No controls are included in this message.
      </SettingsStatusMessage>,
    );

    expect(
      container.querySelectorAll(
        "a, button, input, select, textarea, [tabindex]",
      ),
    ).toHaveLength(0);
  });
});
