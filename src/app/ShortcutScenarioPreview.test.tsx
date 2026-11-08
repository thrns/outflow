import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ShortcutScenarioPreview } from "./ShortcutScenarioPreview";

function installTripwire(target: object, key: string, value: unknown) {
  const prior = Object.getOwnPropertyDescriptor(target, key);
  Object.defineProperty(target, key, { configurable: true, value });
  return () => {
    if (prior) Object.defineProperty(target, key, prior);
    else Reflect.deleteProperty(target, key);
  };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ShortcutScenarioPreview", () => {
  it("labels the fixed scenarios as simulated and starts from the unknown default", () => {
    render(<ShortcutScenarioPreview />);

    const selector = screen.getByRole("combobox", { name: "Scenario" });
    const options = within(selector).getAllByRole("option");
    expect(options).toHaveLength(7);
    expect(options.every((option) => option.textContent?.includes("(simulated)"))).toBe(true);
    expect(screen.getByText("Developer scenarios (simulated)")).toBeVisible();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Global keyboard shortcuts are not connected.",
    );
  });

  it.each([
    [
      "available-release-supported",
      "Registration available, release supported (simulated)",
      "Available — not registered",
      "Supported",
    ],
    [
      "available-release-unsupported",
      "Registration available, release unsupported (simulated)",
      "Available — not registered",
      "Unsupported",
    ],
    ["conflict", "Registration conflict (simulated)", "Conflict detected", "Not checked"],
    [
      "permission-unavailable",
      "Permission / authorization unavailable (simulated)",
      "Permission or authorization unavailable",
      "Not checked",
    ],
    ["unsupported", "Shortcut capability unsupported (simulated)", "Unsupported", "Not checked"],
    ["registration-failed", "Registration failed (simulated)", "Registration failed", "Not checked"],
  ] as const)("renders scenario %s with separate registration and release values", (id, _label, registration, release) => {
    render(<ShortcutScenarioPreview />);
    fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
      target: { value: id },
    });

    const registrationLabel = screen.getByText("Global shortcut registration", { selector: "dt" });
    const registrationValue = within(registrationLabel.parentElement as HTMLElement).getByText(/.+/, { selector: "dd" });
    const releaseLabel = screen.getByText("Matching press-and-release events", { selector: "dt" });
    const releaseValue = within(releaseLabel.parentElement as HTMLElement).getByText(/.+/, { selector: "dd" });
    expect(registrationValue).toHaveTextContent(registration);
    expect(releaseValue).toHaveTextContent(release);
    expect(
      screen.getByText(
        "Simulated scenario — these values are fixed presentation examples, not observations from an operating system.",
      ),
    ).toBeVisible();
  });

  it("makes release-unsupported guidance explicit and keeps focus on the selector", () => {
    render(<ShortcutScenarioPreview />);
    const selector = screen.getByRole("combobox", { name: "Scenario" });
    selector.focus();
    fireEvent.change(selector, {
      target: { value: "available-release-unsupported" },
    });

    expect(selector).toHaveFocus();
    expect(screen.getByText(/^Unavailable\./)).toBeVisible();
    expect(screen.getByText(/^Possible fallback — not operational\./)).toBeVisible();
    expect(
      screen.getByText(
        /possible fallback pending actual native registration and microphone readiness/i,
      ),
    ).toBeVisible();
    expect(screen.getByRole("status")).toHaveAttribute("aria-live", "polite");
    expect(screen.getByRole("status")).toHaveTextContent("press-and-release events are unsupported");
  });

  it("does not add global keyboard listeners or call browser, native, clipboard, or network APIs", () => {
    const forbiddenCall = vi.fn(() => {
      throw new Error("Shortcut scenarios must remain presentation-only.");
    });
    const restore: Array<() => void> = [];
    const windowListenerSpy = vi.spyOn(window, "addEventListener");
    const documentListenerSpy = vi.spyOn(document, "addEventListener");
    restore.push(
      installTripwire(navigator, "mediaDevices", {
        getUserMedia: forbiddenCall,
        enumerateDevices: forbiddenCall,
      }),
      installTripwire(navigator, "permissions", { query: forbiddenCall }),
      installTripwire(navigator, "keyboard", {
        getLayoutMap: forbiddenCall,
        lock: forbiddenCall,
      }),
      installTripwire(navigator, "clipboard", { writeText: forbiddenCall }),
      installTripwire(navigator, "sendBeacon", forbiddenCall),
      installTripwire(window, "__TAURI_INTERNALS__", { invoke: forbiddenCall }),
      installTripwire(globalThis, "fetch", forbiddenCall),
      installTripwire(window, "fetch", forbiddenCall),
      installTripwire(globalThis, "XMLHttpRequest", forbiddenCall),
      installTripwire(globalThis, "WebSocket", forbiddenCall),
      installTripwire(globalThis, "EventSource", forbiddenCall),
    );

    try {
      render(<ShortcutScenarioPreview />);
      const selector = screen.getByRole("combobox", { name: "Scenario" });
      for (const value of [
        "available-release-supported",
        "available-release-unsupported",
        "conflict",
        "permission-unavailable",
        "unsupported",
        "registration-failed",
        "default",
      ]) {
        fireEvent.change(selector, { target: { value } });
      }

      const globalKeyboardListeners = [
        ...windowListenerSpy.mock.calls,
        ...documentListenerSpy.mock.calls,
      ].filter(([type]) => type === "keydown" || type === "keyup" || type === "keypress");
      expect(globalKeyboardListeners).toHaveLength(0);
      expect(forbiddenCall).not.toHaveBeenCalled();
    } finally {
      for (const undo of restore.reverse()) undo();
    }
  });
});
