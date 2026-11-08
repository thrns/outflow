import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ShortcutCapabilityPanel } from "./ShortcutCapabilityPanel";

function statusValue(label: string) {
  const labelElement = screen.getByText(label, { selector: "dt" });
  const item = labelElement.parentElement;
  if (item === null) throw new Error(`Missing status item for ${label}.`);
  return within(item).getByText(/.+/, { selector: "dd" });
}

describe("ShortcutCapabilityPanel", () => {
  it("shows the default status as unverified and not connected", () => {
    render(<ShortcutCapabilityPanel />);

    expect(screen.getByRole("status")).toHaveTextContent(
      "Global keyboard shortcuts are not connected.",
    );
    expect(statusValue("Global shortcut registration")).toHaveTextContent(
      "Not checked",
    );
    expect(statusValue("Matching press-and-release events")).toHaveTextContent(
      "Not checked",
    );
    expect(
      statusValue("Recording readiness, including microphone permission"),
    ).toHaveTextContent("Not checked");
    expect(
      screen.getByText(/No shortcut is currently registered by this preview/),
    ).toBeVisible();
    expect(screen.getByText(/^Not verified\./)).toBeVisible();
    expect(screen.getByText(/^Not operational\./)).toBeVisible();
  });

  it.each([
    ["unknown", "Not checked"],
    ["available", "Available — not registered"],
    ["registered", "Registered"],
    ["permission_unavailable", "Permission or authorization unavailable"],
    ["conflict", "Conflict detected"],
    ["failed", "Registration failed"],
    ["unsupported", "Unsupported"],
  ] as const)("renders registration state %s distinctly", (registrationStatus, label) => {
    render(
      <ShortcutCapabilityPanel
        registrationStatus={registrationStatus}
        statusOrigin="simulated"
      />,
    );

    expect(statusValue("Global shortcut registration")).toHaveTextContent(label);
    expect(
      screen.getByText(
        "Simulated scenario — these values are fixed presentation examples, not observations from an operating system.",
      ),
    ).toBeVisible();
  });

  it("represents release-event support separately from shortcut registration", () => {
    render(
      <ShortcutCapabilityPanel
        holdReleaseCapability="unsupported"
        registrationStatus="available"
        statusOrigin="simulated"
      />,
    );

    expect(statusValue("Global shortcut registration")).toHaveTextContent(
      "Available — not registered",
    );
    expect(statusValue("Matching press-and-release events")).toHaveTextContent(
      "Unsupported",
    );
  });

  it("makes push-to-talk unavailable when release events are unsupported", () => {
    render(
      <ShortcutCapabilityPanel
        holdReleaseCapability="unsupported"
        registrationStatus="available"
        statusOrigin="simulated"
      />,
    );

    expect(screen.getByText(/^Unavailable\./)).toBeVisible();
    expect(screen.getByText(/A timeout must never stand in for a key release/)).toBeVisible();
  });

  it("never presents push-to-talk as usable while release events are unknown", () => {
    render(
      <ShortcutCapabilityPanel
        holdReleaseCapability="unknown"
        registrationStatus="registered"
        statusOrigin="native"
        recordingReadiness="ready"
      />,
    );

    const pushToTalk = screen.getByRole("article", { name: "Push-to-talk" });
    expect(within(pushToTalk).getByText(/^Not verified\./)).toBeVisible();
    expect(
      within(pushToTalk).getByText(
        /unavailable until reliable matching key-down and key-up events are confirmed/,
      ),
    ).toBeVisible();
    expect(within(pushToTalk).queryByText(/^Operational\./)).not.toBeInTheDocument();
  });

  it("describes toggle as a possible fallback without claiming recording works", () => {
    render(
      <ShortcutCapabilityPanel
        holdReleaseCapability="unsupported"
        registrationStatus="available"
        statusOrigin="simulated"
      />,
    );

    expect(screen.getByText(/^Unavailable\./)).toBeVisible();
    expect(screen.getByText(/^Possible fallback — not operational\./)).toBeVisible();
    expect(
      screen.getByText(
        /possible fallback pending actual native registration and microphone readiness/i,
      ),
    ).toBeVisible();
    expect(screen.queryByText(/^Operational\./)).not.toBeInTheDocument();
  });

  it("distinguishes a shortcut conflict from unavailable permission", () => {
    const { rerender } = render(
      <ShortcutCapabilityPanel
        holdReleaseCapability="supported"
        registrationStatus="conflict"
        statusOrigin="simulated"
      />,
    );

    expect(statusValue("Global shortcut registration")).toHaveTextContent(
      "Conflict detected",
    );
    expect(screen.getAllByText(/conflict, not a permission denial/)).toHaveLength(2);
    expect(screen.queryByText(/Permission or authorization unavailable/)).not.toBeInTheDocument();

    rerender(
      <ShortcutCapabilityPanel
        holdReleaseCapability="supported"
        registrationStatus="permission_unavailable"
        statusOrigin="simulated"
      />,
    );
    expect(statusValue("Global shortcut registration")).toHaveTextContent(
      "Permission or authorization unavailable",
    );
  });

  it("does not treat registration alone as microphone or recording readiness", () => {
    render(
      <ShortcutCapabilityPanel
        holdReleaseCapability="supported"
        recordingReadiness="unknown"
        registrationStatus="registered"
        statusOrigin="native"
      />,
    );

    expect(statusValue("Global shortcut registration")).toHaveTextContent(
      "Registered",
    );
    expect(
      statusValue("Recording readiness, including microphone permission"),
    ).toHaveTextContent("Not checked");
    expect(screen.getAllByText(/^Not operational — readiness not verified\./)).toHaveLength(2);
    expect(
      screen.getAllByText(
        /microphone permission and recording readiness have not been verified/,
      ),
    ).toHaveLength(2);
  });

  it("does not display a fabricated shortcut assignment or recording control", () => {
    render(<ShortcutCapabilityPanel />);

    expect(screen.queryByText(/(?:Ctrl|Alt|Shift|⌘|Cmd|Command|Control)\s*\+/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /record|dictate|start/i })).not.toBeInTheDocument();
    expect(screen.getAllByRole("button")).toHaveLength(1);
  });

  it("disables status checking without a connected native callback", () => {
    render(<ShortcutCapabilityPanel />);

    expect(
      screen.getByRole("button", { name: "Check shortcut status" }),
    ).toBeDisabled();
    expect(screen.getByText(/unavailable in the browser preview/)).toBeVisible();
  });

  it("invokes an injected status callback only after the explicit action", () => {
    const checkStatus = vi.fn();
    render(
      <ShortcutCapabilityPanel
        nativeAdapter={{ connected: true, checkStatus }}
        statusOrigin="native"
      />,
    );

    const button = screen.getByRole("button", { name: "Check shortcut status" });
    expect(button).toBeEnabled();
    expect(checkStatus).not.toHaveBeenCalled();

    fireEvent.click(button);
    expect(checkStatus).toHaveBeenCalledTimes(1);
  });
});
