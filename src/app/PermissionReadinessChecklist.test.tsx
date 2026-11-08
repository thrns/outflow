import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PermissionReadinessChecklist } from "./PermissionReadinessChecklist";

afterEach(() => {
  vi.unstubAllEnvs();
});

const scenarioCases = [
  ["default", "All unknown (default)", "Permission readiness has not been checked."],
  ["microphone-denied", "Microphone permission denied", "Denied (simulated)"],
  [
    "microphone-no-device",
    "Microphone available, no input device",
    "No available input device (simulated)",
  ],
  ["shortcut-conflict", "Shortcut conflict", "Conflict detected (simulated)"],
  [
    "release-unsupported",
    "Shortcut release events unsupported",
    "Unsupported (simulated)",
  ],
  ["insertion-denied", "Text insertion permission denied", "Denied (simulated)"],
  [
    "insertion-limited",
    "Text insertion unsupported or limited",
    "Not supported or limited (simulated)",
  ],
  [
    "hypothetical-available",
    "All capabilities hypothetically available",
    "Available (simulated)",
  ],
] as const;

function selectScenario(label: string) {
  const selector = screen.getByRole("combobox", {
    name: "Developer scenarios (simulated fixtures)",
  });
  fireEvent.change(selector, {
    target: { value: scenarioCases.find(([, option]) => option === label)?.[0] },
  });
  return selector;
}

describe("PermissionReadinessChecklist", () => {
  it("starts with every capability unknown and names the unchecked default", () => {
    render(<PermissionReadinessChecklist />);

    expect(screen.getByRole("status")).toHaveTextContent(
      "Permission readiness has not been checked.",
    );
    expect(screen.getAllByText("Not checked")).toHaveLength(5);
    expect(screen.getByText(/cannot inspect operating-system permissions/i)).toBeVisible();
  });

  it("represents microphone, global shortcuts, and text insertion independently", () => {
    render(<PermissionReadinessChecklist />);

    const checklist = screen.getByRole("list", {
      name: "Permission capability checklist",
    });
    expect(within(checklist).getAllByRole("listitem")).toHaveLength(3);
    expect(within(checklist).getByRole("heading", { name: "Microphone" })).toBeVisible();
    expect(within(checklist).getByRole("heading", { name: "Global shortcuts" })).toBeVisible();
    expect(within(checklist).getByRole("heading", { name: "Text insertion" })).toBeVisible();
  });

  it("keeps microphone permission separate from input-device availability", () => {
    render(<PermissionReadinessChecklist />);
    selectScenario("Microphone available, no input device");

    expect(screen.getByText("Available (simulated)")).toBeVisible();
    expect(screen.getByText("No available input device (simulated)")).toBeVisible();
    expect(screen.getByText(/Connect or enable an input device/i)).toBeVisible();
  });

  it("does not infer microphone readiness from shortcut registration", () => {
    render(<PermissionReadinessChecklist />);
    selectScenario("All capabilities hypothetically available");

    const capabilityItems = within(
      screen.getByRole("list", { name: "Permission capability checklist" }),
    ).getAllByRole("listitem");
    const microphone = capabilityItems[0];
    expect(within(microphone).getByText("Available (simulated)")).toBeVisible();
    expect(within(microphone).getByText("Device detected (simulated)")).toBeVisible();
    expect(screen.getByRole("status")).toHaveTextContent(/not operating-system observations/i);
    expect(screen.queryByText(/^Ready$/)).not.toBeInTheDocument();
  });

  it("does not imply push-to-talk works when release-event support is unknown", () => {
    render(<PermissionReadinessChecklist />);

    expect(screen.getByText(/Unknown release support does not mean push-to-talk works/i)).toBeVisible();
    expect(screen.getByText(/a timeout cannot stand in for a release/i)).toBeVisible();
  });

  it("keeps shortcut conflicts distinct from authorization denial", () => {
    render(<PermissionReadinessChecklist />);
    selectScenario("Shortcut conflict");

    expect(screen.getByText("Conflict detected (simulated)")).toBeVisible();
    expect(screen.getByText(/A conflict is different from authorization denial/i)).toBeVisible();
    expect(screen.queryByText(/Authorization unavailable/)).not.toBeInTheDocument();
  });

  it("does not treat insertion capability as target safety", () => {
    render(<PermissionReadinessChecklist />);
    selectScenario("All capabilities hypothetically available");

    expect(screen.getByText(/does not establish that any particular destination is editable/i)).toBeVisible();
    expect(screen.getByText(/Capability does not establish target eligibility/i)).toBeVisible();
  });

  it("never describes protected destinations as universally insertable", () => {
    render(<PermissionReadinessChecklist />);

    expect(screen.getByText(/password-protected or otherwise protected/i)).toBeVisible();
    expect(screen.getByText(/Accessibility permission alone does not make every field safe/i)).toBeVisible();
  });

  it("explains the explicit Copy Result fallback for insertion denial without offering Copy", () => {
    render(<PermissionReadinessChecklist />);
    selectScenario("Text insertion permission denied");

    expect(screen.getByText(/intended recovery is an explicit Copy Result action/i)).toBeVisible();
    expect(screen.queryByRole("button", { name: /copy/i })).not.toBeInTheDocument();
  });

  it("makes clear that Copy-only fallback has no browser clipboard integration", () => {
    render(<PermissionReadinessChecklist />);
    selectScenario("Text insertion permission denied");

    expect(screen.getByText(/real clipboard integration is not connected in this browser preview/i)).toBeVisible();
  });

  it("keeps unknown Wayland portal capabilities unverified and limited", () => {
    render(<PermissionReadinessChecklist />);
    selectScenario("Text insertion unsupported or limited");

    expect(screen.getByText("Not supported or limited (simulated)")).toBeVisible();
    expect(screen.getByText(/Wayland portal support.*unverified here and must be treated as limited/i)).toBeVisible();
    expect(screen.getByText(/Wayland portal capabilities remain unverified/i)).toBeVisible();
    expect(screen.queryByText(/Wayland.*Supported/)).not.toBeInTheDocument();
  });

  it.each(scenarioCases)("renders the fixed fixture %s with an unmistakable label", (id, label, expected) => {
    render(<PermissionReadinessChecklist />);

    if (id !== "default") selectScenario(label);

    if (id === "default") {
      expect(screen.getByRole("status")).toHaveTextContent(expected);
    } else {
      expect(screen.getByRole("status")).toHaveTextContent(`Simulated scenario: ${label}`);
      expect(screen.getAllByText(expected).length).toBeGreaterThan(0);
      expect(screen.getByText(/Simulated values are independent fixtures/i)).toBeVisible();
    }
  });

  it("labels the selector, supports keyboard focus, and announces scenario changes politely", () => {
    render(<PermissionReadinessChecklist />);

    const selector = screen.getByRole("combobox", {
      name: "Developer scenarios (simulated fixtures)",
    });
    expect(selector).toHaveAttribute(
      "aria-describedby",
      "permission-readiness-scenario-help",
    );
    expect(screen.getByRole("status")).toHaveAttribute("aria-live", "polite");
    expect(screen.getByRole("status")).toHaveAttribute("aria-atomic", "true");
    selector.focus();
    expect(selector).toHaveFocus();
    selectScenario("Microphone permission denied");
    expect(selector).toHaveFocus();
    expect(screen.getByRole("status")).toHaveTextContent(/Simulated scenario/);
  });

  it("does not call microphone, permission, keyboard, clipboard, Tauri, or network APIs", () => {
    const forbiddenCall = vi.fn(() => {
      throw new Error("A permission fixture called a native or browser capability API.");
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
    const windowListener = vi.spyOn(window, "addEventListener");

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
    installTripwire(window, "__TAURI__", { invoke: forbiddenCall, core: { invoke: forbiddenCall } });
    installTripwire(window, "__TAURI_INTERNALS__", { invoke: forbiddenCall });
    installTripwire(globalThis, "fetch", forbiddenCall);
    installTripwire(globalThis, "XMLHttpRequest", class {
      constructor() {
        forbiddenCall();
      }
    });

    try {
      render(<PermissionReadinessChecklist />);
      for (const [, label] of scenarioCases.slice(1)) selectScenario(label);
      expect(forbiddenCall).not.toHaveBeenCalled();
      expect(windowListener.mock.calls.filter(([type]) => type === "keydown")).toHaveLength(0);
    } finally {
      windowListener.mockRestore();
      for (const restore of restorers.reverse()) restore();
    }
  });

  it("does not persist scenario selection and resets to the unknown default after remount", () => {
    const storageWrite = vi.spyOn(Storage.prototype, "setItem");
    const first = render(<PermissionReadinessChecklist />);
    selectScenario("Shortcut conflict");
    first.unmount();

    render(<PermissionReadinessChecklist />);

    expect(screen.getByRole("status")).toHaveTextContent(
      "Permission readiness has not been checked.",
    );
    expect(storageWrite).not.toHaveBeenCalled();
    storageWrite.mockRestore();
  });

  it("omits the development scenario selector when Vite is in production mode", () => {
    vi.stubEnv("DEV", false);
    render(<PermissionReadinessChecklist />);

    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Permission readiness has not been checked.",
    );
  });
});
