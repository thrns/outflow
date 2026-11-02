import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { SettingsSwitch } from "./SettingsSwitch";

describe("SettingsSwitch", () => {
  function renderSwitch(checked = false, describedBy?: string) {
    return render(
      <>
        <span id="switch-label">Launch at Login</span>
        <span id="switch-description">Start Outflow when you sign in.</span>
        <SettingsSwitch
          checked={checked}
          describedBy={describedBy}
          labelledBy="switch-label"
          onCheckedChange={vi.fn()}
        />
      </>,
    );
  }

  it("renders as an accessible switch", () => {
    renderSwitch();

    expect(screen.getByRole("switch", { name: "Launch at Login" })).toBeVisible();
  });

  it("reflects checked=false", () => {
    renderSwitch(false);

    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "false");
  });

  it("reflects checked=true", () => {
    renderSwitch(true);

    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  });

  it("calls the change callback when activated", () => {
    const onCheckedChange = vi.fn();
    render(
      <SettingsSwitch
        checked={false}
        labelledBy="switch-label"
        onCheckedChange={onCheckedChange}
      />,
    );
    fireEvent.click(screen.getByRole("switch"));

    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });

  it("supports Space activation once per press", () => {
    const onCheckedChange = vi.fn();
    render(
      <SettingsSwitch
        checked={false}
        labelledBy="switch-label"
        onCheckedChange={onCheckedChange}
      />,
    );
    const control = screen.getByRole("switch");
    fireEvent.keyDown(control, { key: " " });
    fireEvent.keyUp(control, { key: " " });

    expect(onCheckedChange).toHaveBeenCalledTimes(1);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });

  it("uses the supplied accessible label", () => {
    renderSwitch();

    expect(screen.getByRole("switch", { name: "Launch at Login" })).toHaveAttribute(
      "aria-labelledby",
      "switch-label",
    );
  });

  it("uses the optional accessible description", () => {
    renderSwitch(false, "switch-description");

    expect(
      screen.getByRole("switch", {
        name: "Launch at Login",
        description: "Start Outflow when you sign in.",
      }),
    ).toHaveAttribute("aria-describedby", "switch-description");
  });

  it("follows updated checked props without internal checked state", () => {
    function ControlledHarness() {
      const [checked, setChecked] = useState(false);
      return (
        <>
          <SettingsSwitch
            checked={checked}
            labelledBy="switch-label"
            onCheckedChange={setChecked}
          />
          <button type="button" onClick={() => setChecked(true)}>
            Set on externally
          </button>
        </>
      );
    }
    render(
      <>
        <span id="switch-label">Launch at Login</span>
        <ControlledHarness />
      </>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Set on externally" }));

    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
  });
});
