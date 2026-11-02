import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsRow } from "./SettingsRow";

describe("SettingsRow", () => {
  it("renders the setting title", () => {
    render(
      <SettingsRow title="Launch at Login" titleId="setting-title">
        <button type="button">Control</button>
      </SettingsRow>,
    );

    expect(screen.getByText("Launch at Login")).toBeVisible();
  });

  it("renders an optional description", () => {
    render(
      <SettingsRow
        title="Launch at Login"
        titleId="setting-title"
        description="Start Outflow when you sign in."
        descriptionId="setting-description"
      >
        <button type="button">Control</button>
      </SettingsRow>,
    );

    expect(screen.getByText("Start Outflow when you sign in.")).toBeVisible();
  });

  it("renders its control children", () => {
    render(
      <SettingsRow title="Launch at Login" titleId="setting-title">
        <button type="button">Control</button>
      </SettingsRow>,
    );

    expect(screen.getByRole("button", { name: "Control" })).toBeVisible();
  });

  it("omits the description when it is not supplied", () => {
    const { container } = render(
      <SettingsRow title="Launch at Login" titleId="setting-title">
        <button type="button">Control</button>
      </SettingsRow>,
    );

    expect(container.querySelector(".settings-row-description")).toBeNull();
  });

  it("names the setting group without treating its control as a labelable element", () => {
    render(
      <SettingsRow title="Launch at Login" titleId="setting-title">
        <button aria-label="Launch at Login switch" role="switch" type="button" />
      </SettingsRow>,
    );

    expect(
      screen.getByRole("group", { name: "Launch at Login" }),
    ).toBeVisible();
    expect(
      screen.getByRole("switch", { name: "Launch at Login switch" }),
    ).toBeVisible();
  });

  it("supports explicit accessible name and description associations on the control", () => {
    render(
      <SettingsRow
        title="Launch at Login"
        titleId="setting-title"
        description="Start Outflow when you sign in."
        descriptionId="setting-description"
      >
        <button
          aria-describedby="setting-description"
          aria-labelledby="setting-title"
          aria-checked="false"
          role="switch"
          type="button"
        />
      </SettingsRow>,
    );

    expect(
      screen.getByRole("switch", {
        name: "Launch at Login",
        description: "Start Outflow when you sign in.",
      }),
    ).toBeVisible();
  });
});
