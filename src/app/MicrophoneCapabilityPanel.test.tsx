import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  MicrophoneCapabilityPanel,
  type InputDeviceAvailability,
  type MicrophonePermissionStatus,
} from "./MicrophoneCapabilityPanel";
import { MicrophoneScenarioPreview } from "./MicrophoneScenarioPreview";

describe("MicrophoneCapabilityPanel", () => {
  it("defaults to an unchecked preview with no device or native integration claim", () => {
    render(<MicrophoneCapabilityPanel />);

    expect(screen.getByRole("status")).toHaveTextContent(
      "Microphone status has not been checked.",
    );
    expect(screen.getByText("Permission status").nextElementSibling).toHaveTextContent(
      "Not checked",
    );
    expect(
      screen.getByText("Input-device availability").nextElementSibling,
    ).toHaveTextContent("Not checked");
    expect(
      screen.getByText("Native microphone integration").nextElementSibling,
    ).toHaveTextContent("Not connected");
    expect(
      screen.getByText(/No audio is captured, no devices are enumerated/),
    ).toBeVisible();
  });

  it.each([
    ["unknown", "Microphone status has not been checked."],
    ["available", "Microphone permission is available."],
    ["denied", "Microphone access is off."],
    ["needs_user_action", "Microphone permission needs your action."],
    ["not_supported", "Microphone capability is not supported."],
  ] as const)("presents %s permission with recovery guidance", (permission, summary) => {
    render(
      <MicrophoneCapabilityPanel
        permissionStatus={permission}
        statusOrigin="native"
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent(summary);
    expect(screen.getByText("Why Outflow needs microphone access")).toBeVisible();
    expect(
      screen.getByText(/only after you intentionally start dictation/),
    ).toBeVisible();
    expect(screen.getByText("What you can do next")).toBeVisible();
  });

  it("keeps permission and input-device availability independent", () => {
    render(
      <MicrophoneCapabilityPanel
        inputDeviceAvailability="none"
        permissionStatus="available"
        statusOrigin="simulated"
      />,
    );

    expect(screen.getByText("Permission status").nextElementSibling).toHaveTextContent(
      "Available",
    );
    expect(
      screen.getByText("Input-device availability").nextElementSibling,
    ).toHaveTextContent("No available input device");
    expect(screen.getByText(/No input device is available/)).toBeVisible();
    expect(screen.getByRole("status")).toHaveTextContent("Simulated scenario");
    expect(screen.queryByText(/Ready to record|Recording ready/i)).not.toBeInTheDocument();
  });

  it.each([
    ["unknown", "Not checked"],
    ["available", "Device detected"],
    ["none", "No available input device"],
    ["disconnected", "Disconnected or unavailable"],
  ] as const)("presents %s device availability separately", (device, label) => {
    render(
      <MicrophoneCapabilityPanel
        inputDeviceAvailability={device}
        permissionStatus="available"
        statusOrigin="simulated"
      />,
    );

    expect(
      screen.getByText("Input-device availability").nextElementSibling,
    ).toHaveTextContent(label);
  });

  it("keeps denied permission distinct from a required user action", () => {
    const { rerender } = render(
      <MicrophoneCapabilityPanel
        permissionStatus="denied"
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent("Microphone access is off.");
    expect(screen.getByText(/Recording remains unavailable until permission is granted/)).toBeVisible();

    rerender(
      <MicrophoneCapabilityPanel
        permissionStatus="needs_user_action"
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      "Microphone permission needs your action.",
    );
    expect(screen.getByRole("status")).not.toHaveTextContent("Microphone access is off.");
  });

  it("does not imply a usable microphone for unsupported capability", () => {
    render(
      <MicrophoneCapabilityPanel
        permissionStatus="not_supported"
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("not supported");
    expect(screen.getByText(/Microphone use is unavailable through this capability/)).toBeVisible();
    expect(screen.queryByText(/Ready to record|Recording ready/i)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Check again" })).toBeDisabled();
  });

  it("keeps retry unavailable and explains the missing native callback", () => {
    render(<MicrophoneCapabilityPanel />);

    const retry = screen.getByRole("button", { name: "Check again" });
    expect(retry).toBeDisabled();
    expect(retry).toHaveAttribute("aria-describedby", "microphone-check-again-help");
    expect(screen.getByText(/no connected native permission-check callback is provided/i)).toBeVisible();
  });

  it("keeps System Settings unavailable without a connected callback", () => {
    render(
      <MicrophoneCapabilityPanel
        permissionStatus="denied"
      />,
    );

    const settings = screen.getByRole("button", { name: "Open System Settings" });
    expect(settings).toBeDisabled();
    expect(settings).toHaveAttribute(
      "aria-describedby",
      "microphone-system-settings-help",
    );
    expect(screen.getByText(/no connected native settings callback is provided/i)).toBeVisible();
  });

  it("runs injected native callbacks only after their explicit actions", () => {
    const checkAgain = vi.fn();
    const openSystemSettings = vi.fn();
    render(
      <MicrophoneCapabilityPanel
        nativeAdapter={{
          connected: true,
          checkAgain,
          openSystemSettings,
        }}
        permissionStatus="denied"
        statusOrigin="native"
      />,
    );

    expect(screen.getByText("Native microphone integration").nextElementSibling).toHaveTextContent(
      "Connected",
    );
    expect(checkAgain).not.toHaveBeenCalled();
    expect(openSystemSettings).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Check again" }));
    expect(checkAgain).toHaveBeenCalledTimes(1);
    expect(openSystemSettings).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Open System Settings" }));
    expect(openSystemSettings).toHaveBeenCalledTimes(1);
  });

  it("labels the development selector and its selected fixture as simulated", () => {
    render(<MicrophoneScenarioPreview />);

    expect(screen.getByText("Developer scenarios (simulated)")).toBeVisible();
    const selector = screen.getByRole("combobox", { name: "Scenario" });
    expect(selector).toBeVisible();
    fireEvent.change(selector, { target: { value: "available" } });

    expect(screen.getByRole("status")).toHaveTextContent("Simulated scenario");
    expect(screen.getByText("Simulated scenario — these values are not observations from a real microphone or operating-system permission check.")).toBeVisible();
    expect(screen.getByText("Native microphone integration").nextElementSibling).toHaveTextContent(
      "Not connected",
    );
    expect(screen.getByText(/This does not confirm that an input device is present or that recording has started/)).toBeVisible();
    expect(screen.getByText(/No audio is captured/)).toBeVisible();
  });

  it("keeps the scenario selector out of production rendering", () => {
    render(<MicrophoneCapabilityPanel />);

    expect(screen.queryByRole("combobox", { name: "Scenario" })).not.toBeInTheDocument();
  });

  it("does not touch microphone, permission, Tauri, clipboard, or network APIs in scenarios", () => {
    const forbiddenCall = vi.fn(() => {
      throw new Error("A presentation scenario accessed a runtime capability API.");
    });
    const restorers: Array<() => void> = [];
    const installTripwire = (target: object, key: string, value: unknown) => {
      const prior = Object.getOwnPropertyDescriptor(target, key);
      Object.defineProperty(target, key, { configurable: true, value });
      restorers.push(() => {
        if (prior) Object.defineProperty(target, key, prior);
        else Reflect.deleteProperty(target, key);
      });
    };

    installTripwire(navigator, "mediaDevices", {
      getUserMedia: forbiddenCall,
      enumerateDevices: forbiddenCall,
    });
    installTripwire(navigator, "permissions", { query: forbiddenCall });
    installTripwire(navigator, "clipboard", {
      readText: forbiddenCall,
      writeText: forbiddenCall,
      read: forbiddenCall,
      write: forbiddenCall,
    });
    installTripwire(window, "__TAURI__", {
      core: { invoke: forbiddenCall },
      invoke: forbiddenCall,
    });
    installTripwire(window, "__TAURI_INTERNALS__", { invoke: forbiddenCall });
    installTripwire(globalThis, "fetch", forbiddenCall);
    installTripwire(globalThis, "XMLHttpRequest", class {
      constructor() {
        forbiddenCall();
      }
    });

    try {
      render(<MicrophoneScenarioPreview />);
      const selector = screen.getByRole("combobox", { name: "Scenario" });
      for (const scenario of [
        "available",
        "denied",
        "needs-user-action",
        "unsupported",
        "no-device",
        "disconnected",
        "default",
      ]) {
        fireEvent.change(selector, { target: { value: scenario } });
      }
      expect(forbiddenCall).not.toHaveBeenCalled();
    } finally {
      for (const restore of restorers.reverse()) restore();
    }
  });

  it("exposes polite status, labels, descriptions, and keyboard-operable controls", () => {
    render(<MicrophoneScenarioPreview />);

    const status = screen.getByRole("status");
    expect(status).toHaveAttribute("aria-live", "polite");
    expect(status).toHaveAttribute("aria-atomic", "true");
    expect(screen.getByRole("heading", { level: 3, name: "Microphone capability" })).toBeVisible();
    expect(screen.getByLabelText("Scenario")).toHaveAttribute(
      "aria-describedby",
      "microphone-scenario-help",
    );

    const selector = screen.getByRole("combobox", { name: "Scenario" });
    selector.focus();
    expect(selector).toHaveFocus();
  });

  it("keeps unknown permission and device statuses typed independently", () => {
    const permission: MicrophonePermissionStatus = "unknown";
    const device: InputDeviceAvailability = "disconnected";
    render(
      <MicrophoneCapabilityPanel
        inputDeviceAvailability={device}
        permissionStatus={permission}
      />,
    );

    expect(screen.getByText("Permission status").nextElementSibling).toHaveTextContent("Not checked");
    expect(screen.getByText("Input-device availability").nextElementSibling).toHaveTextContent(
      "Disconnected or unavailable",
    );
  });
});
