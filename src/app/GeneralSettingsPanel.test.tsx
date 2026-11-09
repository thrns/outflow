import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GeneralSettingsPanel } from "./GeneralSettingsPanel";

afterEach(() => {
  vi.unstubAllEnvs();
});

function renderPanel(
  runAtLogin = false,
  onRunAtLoginChange = vi.fn(),
) {
  return render(
    <GeneralSettingsPanel
      runAtLogin={runAtLogin}
      onRunAtLoginChange={onRunAtLoginChange}
    />,
  );
}

function selectScenario(label: string) {
  const selector = screen.getByRole("combobox", {
    name: "Developer scenarios (simulated fixtures)",
  }) as HTMLSelectElement;
  const option = Array.from(selector.options).find(
    (candidate) => candidate.textContent === label,
  );

  if (!option) throw new Error(`Missing permission scenario: ${label}`);

  fireEvent.change(selector, { target: { value: option.value } });
  return selector;
}

describe("GeneralSettingsPanel", () => {
  it("renders the Launch at Login setting row", () => {
    renderPanel();

    expect(
      screen.getByRole("group", { name: "Launch at Login" }),
    ).toBeVisible();
    expect(screen.getByText("Launch at Login")).toHaveAttribute(
      "id",
      "launch-at-login-label",
    );
  });

  it("shows Off when runAtLogin is false", () => {
    renderPanel(false);

    expect(
      screen.getByText("Off", { selector: ".settings-switch-state" }),
    ).toBeVisible();
  });

  it("shows On when runAtLogin is true", () => {
    renderPanel(true);

    expect(
      screen.getByText("On", { selector: ".settings-switch-state" }),
    ).toBeVisible();
  });

  it("reflects updated props without owning the Launch at Login value", () => {
    const onRunAtLoginChange = vi.fn();
    const panel = renderPanel(false, onRunAtLoginChange);

    panel.rerender(
      <GeneralSettingsPanel
        runAtLogin={true}
        onRunAtLoginChange={onRunAtLoginChange}
      />,
    );

    expect(screen.getByRole("switch", { name: "Launch at Login" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(
      screen.getByText("On", { selector: ".settings-switch-state" }),
    ).toBeVisible();
    expect(onRunAtLoginChange).not.toHaveBeenCalled();
  });

  it("preserves the switch accessible name, checked state, and ARIA references", () => {
    renderPanel(true);

    const control = screen.getByRole("switch", { name: "Launch at Login" });
    expect(control).toHaveAttribute("aria-checked", "true");
    expect(control).toHaveAttribute("aria-labelledby", "launch-at-login-label");
    expect(control).toHaveAttribute(
      "aria-describedby",
      "launch-at-login-description launch-at-login-preview-note",
    );
  });

  it("notifies App with the next value when clicked", () => {
    const onRunAtLoginChange = vi.fn();
    renderPanel(false, onRunAtLoginChange);

    fireEvent.click(screen.getByRole("switch", { name: "Launch at Login" }));

    expect(onRunAtLoginChange).toHaveBeenCalledTimes(1);
    expect(onRunAtLoginChange).toHaveBeenCalledWith(true);
  });

  it("activates with Space", () => {
    const onRunAtLoginChange = vi.fn();
    renderPanel(false, onRunAtLoginChange);
    const control = screen.getByRole("switch", { name: "Launch at Login" });

    control.focus();
    fireEvent.keyDown(control, { key: " " });

    expect(control).toHaveFocus();
    expect(onRunAtLoginChange).toHaveBeenCalledTimes(1);
    expect(onRunAtLoginChange).toHaveBeenCalledWith(true);
  });

  it("ignores repeated Space keydown events", () => {
    const onRunAtLoginChange = vi.fn();
    renderPanel(false, onRunAtLoginChange);
    const control = screen.getByRole("switch", { name: "Launch at Login" });

    fireEvent.keyDown(control, { key: " " });
    fireEvent.keyUp(control, { key: " " });
    fireEvent.keyDown(control, { key: " ", repeat: true });

    expect(onRunAtLoginChange).toHaveBeenCalledTimes(1);
    expect(onRunAtLoginChange).toHaveBeenCalledWith(true);
  });

  it("preserves the existing description and preview-only disclaimer", () => {
    renderPanel();

    expect(
      screen.getByText(
        "Start Outflow automatically when you sign in to your computer.",
      ),
    ).toHaveAttribute("id", "launch-at-login-description");
    expect(
      screen.getByText(
        "Preview only — this setting does not change your computer's startup behavior yet.",
      ),
    ).toHaveAttribute("id", "launch-at-login-preview-note");
  });

  it("renders Permission readiness below the Launch at Login row", () => {
    renderPanel();

    const row = screen.getByRole("group", { name: "Launch at Login" });
    const checklist = screen.getByRole("region", {
      name: "Permission readiness",
    });

    expect(
      row.compareDocumentPosition(checklist) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  });

  it("starts with every checklist capability unverified", () => {
    renderPanel();

    expect(screen.getByRole("status")).toHaveTextContent(
      "Permission readiness has not been checked.",
    );
    expect(screen.getAllByText("Not checked")).toHaveLength(5);
    expect(screen.queryByText(/Available \(simulated\)/)).not.toBeInTheDocument();
  });

  it("keeps development scenario controls local and explicitly simulated", () => {
    renderPanel();
    selectScenario("All capabilities hypothetically available");

    expect(screen.getByRole("status")).toHaveTextContent(
      "Simulated scenario: All capabilities hypothetically available.",
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      /independent fixtures, not operating-system observations/,
    );
    expect(screen.getAllByText("Available (simulated)")).toHaveLength(2);
    expect(screen.getByText("Registered (simulated)")).toBeVisible();
    expect(screen.getByText("Supported (simulated)")).toBeVisible();
    expect(
      screen.getByText(/does not establish that any particular destination is editable/i),
    ).toBeVisible();
  });

  it("resets its checklist fixture after the panel is unmounted and remounted", () => {
    const panel = renderPanel();
    selectScenario("Shortcut conflict");
    expect(screen.getByRole("status")).toHaveTextContent(
      "Simulated scenario: Shortcut conflict.",
    );

    panel.rerender(null);
    panel.rerender(
      <GeneralSettingsPanel runAtLogin={false} onRunAtLoginChange={vi.fn()} />,
    );

    expect(screen.getByRole("status")).toHaveTextContent(
      "Permission readiness has not been checked.",
    );
    expect(
      screen.getByRole("combobox", {
        name: "Developer scenarios (simulated fixtures)",
      }),
    ).toHaveValue("default");
  });

  it("omits development-only scenario selection in production mode", () => {
    vi.stubEnv("DEV", false);
    renderPanel();

    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Permission readiness has not been checked.",
    );
  });

  it("does not call native, permission, capture, storage, clipboard, telemetry, or network APIs", () => {
    const forbiddenCall = vi.fn(() => {
      throw new Error("General Settings called a browser or native capability API.");
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
    const storageWrite = vi.spyOn(Storage.prototype, "setItem");

    installTripwire(navigator, "mediaDevices", {
      getUserMedia: forbiddenCall,
      enumerateDevices: forbiddenCall,
    });
    installTripwire(navigator, "permissions", { query: forbiddenCall });
    installTripwire(navigator, "keyboard", {
      lock: forbiddenCall,
      unlock: forbiddenCall,
      getLayoutMap: forbiddenCall,
    });
    installTripwire(navigator, "clipboard", {
      readText: forbiddenCall,
      writeText: forbiddenCall,
      read: forbiddenCall,
      write: forbiddenCall,
    });
    installTripwire(navigator, "sendBeacon", forbiddenCall);
    installTripwire(window, "__TAURI__", {
      invoke: forbiddenCall,
      core: { invoke: forbiddenCall },
    });
    installTripwire(window, "__TAURI_INTERNALS__", { invoke: forbiddenCall });
    installTripwire(globalThis, "fetch", forbiddenCall);
    installTripwire(
      globalThis,
      "XMLHttpRequest",
      class {
        constructor() {
          forbiddenCall();
        }
      },
    );
    installTripwire(globalThis, "WebSocket", class {
      constructor() {
        forbiddenCall();
      }
    });
    installTripwire(globalThis, "EventSource", class {
      constructor() {
        forbiddenCall();
      }
    });

    try {
      renderPanel();
      selectScenario("Microphone permission denied");
      const control = screen.getByRole("switch", { name: "Launch at Login" });
      fireEvent.click(control);
      fireEvent.keyDown(control, { key: " " });

      expect(forbiddenCall).not.toHaveBeenCalled();
      expect(storageWrite).not.toHaveBeenCalled();
      expect(
        windowListener.mock.calls.filter(([type]) => type === "keydown"),
      ).toHaveLength(0);
    } finally {
      windowListener.mockRestore();
      storageWrite.mockRestore();
      for (const restore of restorers.reverse()) restore();
    }
  });
});
