import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

describe("Try Dictation preview", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  function openShortcutSetup() {
    fireEvent.click(screen.getByRole("button", { name: "Preview setup" }));
    fireEvent.click(screen.getByRole("button", { name: "Review privacy" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to permissions" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to shortcuts" }),
    );
  }

  function openTryDictation() {
    openShortcutSetup();
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to Try Dictation" }),
    );
  }

  it("offers Continue to Try Dictation from Shortcut setup", () => {
    render(<App />);
    openShortcutSetup();

    expect(
      screen.getByRole("button", { name: "Continue to Try Dictation" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Back to Permissions" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "View shortcut details in Settings" }),
    ).toBeVisible();
  });

  it("offers Continue to tray overview without removing existing actions", () => {
    render(<App />);
    openTryDictation();

    expect(
      screen.getByRole("button", { name: "Continue to tray overview" }),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "Back to Shortcuts" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Return to Home" })).toBeVisible();
  });

  it("opens the page, focuses its single h1, and keeps Home unmounted", () => {
    render(<App />);
    openTryDictation();

    const heading = screen.getByRole("heading", {
      level: 1,
      name: "Try Dictation — preview",
    });
    expect(heading).toBeVisible();
    expect(heading).toHaveFocus();
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(
      screen.queryByRole("heading", { level: 1, name: "Home" }),
    ).not.toBeInTheDocument();
  });

  it("describes the intended Start, Speak, Stop, and Review workflow", () => {
    render(<App />);
    openTryDictation();

    const stages = screen.getByRole("list");
    expect(within(stages).getAllByRole("listitem")).toHaveLength(4);
    expect(stages).toHaveTextContent(
      "Start. Begin dictation with a registered shortcut or supported native control.",
    );
    expect(stages).toHaveTextContent(
      "Speak. Speak while the microphone is open and speech is processed locally.",
    );
    expect(stages).toHaveTextContent(
      "Stop. Stop recording and finalize the result.",
    );
    expect(stages).toHaveTextContent(
      "Review. Verify the destination before inserting text, or offer explicit Copy Result when safe insertion is unavailable.",
    );
  });

  it("states that the browser preview cannot perform native dictation", () => {
    render(<App />);
    openTryDictation();

    expect(
      screen.getByText(
        /this browser preview cannot execute any of these native steps/i,
      ),
    ).toBeVisible();
    expect(screen.getByText(/dictation, microphone access, and destination checks are not connected/i)).toBeVisible();
    expect(
      screen.queryByRole("button", { name: /start recording|start dictation|try dictation/i }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/recording started|ready to dictate/i)).not.toBeInTheDocument();
  });

  it("blocks protected and changed-focus targets and never retries uncertain insertion", () => {
    render(<App />);
    openTryDictation();

    const safety = screen.getByRole("note");
    expect(safety).toHaveTextContent(/protected controls/i);
    expect(safety).toHaveTextContent(/unexpected focus changes block insertion/i);
    expect(safety).toHaveTextContent(/insertion outcome is uncertain/i);
    expect(safety).toHaveTextContent(/never retry automatically/i);
  });

  it("shows no sample before activation and reveals one fixed synthetic result", () => {
    render(<App />);
    openTryDictation();

    expect(screen.getByRole("button", { name: "Show example text" })).toBeVisible();
    expect(screen.queryByRole("textbox", { name: /example text/i })).not.toBeInTheDocument();
    expect(screen.queryByText("So I met North Parkade at three.")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Show example text" }));

    const example = screen.getByRole("textbox", {
      name: "Example text (read-only, selectable)",
    });
    expect(example).toHaveValue("So I met North Parkade at three.");
    expect(example).toHaveAttribute("readonly");
    expect(example).toHaveAttribute("aria-readonly", "true");
    expect(example).toHaveClass("synthetic-example-text");
    expect(
      screen.getByText("Synthetic example — no speech was recorded or transcribed."),
    ).toBeVisible();
  });

  it("announces changes politely without reading the sample aloud", () => {
    render(<App />);
    openTryDictation();
    fireEvent.click(screen.getByRole("button", { name: "Show example text" }));

    const announcement = screen.getByRole("status");
    expect(announcement).toHaveAttribute("aria-live", "polite");
    expect(announcement).toHaveTextContent("Synthetic example is shown.");
    expect(announcement).not.toHaveTextContent("So I met North Parkade at three.");

    fireEvent.click(screen.getByRole("button", { name: "Clear example" }));
    expect(announcement).toHaveTextContent("Synthetic example cleared.");
    expect(announcement).not.toHaveTextContent("So I met North Parkade at three.");
  });

  it("clears the sample and keeps repeated Show and Clear actions idempotent", () => {
    render(<App />);
    openTryDictation();

    fireEvent.click(screen.getByRole("button", { name: "Show example text" }));
    fireEvent.click(screen.getByRole("button", { name: "Show example text" }));
    expect(
      screen.getAllByRole("textbox", { name: "Example text (read-only, selectable)" }),
    ).toHaveLength(1);

    fireEvent.click(screen.getByRole("button", { name: "Clear example" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear example" }));
    expect(screen.queryByRole("textbox", { name: /example text/i })).not.toBeInTheDocument();
    expect(screen.queryByText("So I met North Parkade at three.")).not.toBeInTheDocument();
  });

  it("discards local example state when leaving and reopening the page", () => {
    render(<App />);
    openTryDictation();
    fireEvent.click(screen.getByRole("button", { name: "Show example text" }));

    fireEvent.click(screen.getByRole("button", { name: "Back to Shortcuts" }));
    expect(
      screen.getByRole("heading", { level: 1, name: "Choose how dictation starts" }),
    ).toHaveFocus();
    expect(screen.queryByText("So I met North Parkade at three.")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Continue to Try Dictation" }));
    expect(
      screen.getByRole("heading", { level: 1, name: "Try Dictation — preview" }),
    ).toHaveFocus();
    expect(screen.queryByRole("textbox", { name: /example text/i })).not.toBeInTheDocument();
  });

  it("omits development controls and fixture in production rendering", () => {
    vi.stubEnv("DEV", false);
    render(<App />);
    openTryDictation();

    expect(screen.queryByRole("button", { name: "Show example text" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Clear example" })).not.toBeInTheDocument();
    expect(screen.queryByText("So I met North Parkade at three.")).not.toBeInTheDocument();
    expect(
      screen.getByText(
        "Native dictation is not connected. This example is available only in the development preview.",
      ),
    ).toBeVisible();
  });

  it("returns to Shortcut setup and focuses its heading", () => {
    render(<App />);
    openTryDictation();
    fireEvent.click(screen.getByRole("button", { name: "Back to Shortcuts" }));

    expect(
      screen.getByRole("heading", { level: 1, name: "Choose how dictation starts" }),
    ).toHaveFocus();
  });

  it("returns Home to its unavailable default and focuses its heading", () => {
    render(<App />);
    openTryDictation();
    fireEvent.click(screen.getByRole("button", { name: "Return to Home" }));

    expect(screen.getByRole("heading", { level: 1, name: "Home" })).toHaveFocus();
    expect(screen.getByRole("combobox", { name: "Scenario" })).toHaveValue("unavailable");
    expect(
      screen.getByRole("region", { name: "Native dictation is not connected" }),
    ).toHaveTextContent("This preview does not request microphone access or start a recording session.");
  });

  it("keeps Home and Settings as the only primary destinations", () => {
    render(<App />);
    openTryDictation();

    expect(
      within(screen.getByRole("navigation", { name: "Application navigation" }))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Home", "Settings"]);
  });

  it("opens ordinary Settings navigation to General", () => {
    render(<App />);
    openTryDictation();
    fireEvent.click(
      within(screen.getByRole("navigation", { name: "Application navigation" }))
        .getByRole("button", { name: "Settings" }),
    );

    expect(screen.getByRole("heading", { level: 1, name: "Settings" })).toHaveFocus();
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute("aria-selected", "true");
  });

  it("keeps the explicit Shortcut setup Settings action on Shortcuts", () => {
    render(<App />);
    openShortcutSetup();
    fireEvent.click(
      screen.getByRole("button", { name: "View shortcut details in Settings" }),
    );

    expect(screen.getByRole("heading", { level: 1, name: "Settings" })).toHaveFocus();
    expect(screen.getByRole("tab", { name: "Shortcuts" })).toHaveAttribute("aria-selected", "true");
  });

  it("does not invoke microphone, permissions, keyboard capture, Tauri, clipboard, storage, telemetry, or network APIs", () => {
    const forbiddenCall = vi.fn(() => {
      throw new Error("Try Dictation preview must stay presentation-only.");
    });
    const descriptors: Array<{
      target: object;
      key: PropertyKey;
      prior?: PropertyDescriptor;
    }> = [];
    const installTripwire = (target: object, key: PropertyKey, value: unknown) => {
      descriptors.push({ target, key, prior: Object.getOwnPropertyDescriptor(target, key) });
      Object.defineProperty(target, key, { configurable: true, value });
    };
    const apiTrap = new Proxy(
      {},
      {
        get: forbiddenCall,
        apply: forbiddenCall,
      },
    );
    const networkConstructorTrap = new Proxy(function () {}, {
      apply: forbiddenCall,
      construct: forbiddenCall,
    });
    const windowListeners = vi.spyOn(window, "addEventListener");
    const documentListeners = vi.spyOn(document, "addEventListener");
    installTripwire(navigator, "mediaDevices", apiTrap);
    installTripwire(navigator, "permissions", apiTrap);
    installTripwire(navigator, "keyboard", apiTrap);
    installTripwire(navigator, "clipboard", apiTrap);
    installTripwire(navigator, "sendBeacon", forbiddenCall);
    installTripwire(window, "__TAURI__", apiTrap);
    installTripwire(window, "__TAURI_INTERNALS__", apiTrap);
    installTripwire(window, "fetch", forbiddenCall);
    installTripwire(window, "XMLHttpRequest", networkConstructorTrap);
    installTripwire(window, "WebSocket", networkConstructorTrap);
    installTripwire(window, "EventSource", networkConstructorTrap);
    installTripwire(Storage.prototype, "setItem", forbiddenCall);
    installTripwire(Storage.prototype, "getItem", forbiddenCall);
    installTripwire(Storage.prototype, "removeItem", forbiddenCall);

    try {
      render(<App />);
      openTryDictation();
      fireEvent.click(screen.getByRole("button", { name: "Show example text" }));
      fireEvent.click(screen.getByRole("button", { name: "Clear example" }));

      const keyboardListeners = [
        ...windowListeners.mock.calls,
        ...documentListeners.mock.calls,
      ].filter(([type]) => type === "keydown" || type === "keyup" || type === "keypress");
      expect(keyboardListeners).toHaveLength(0);
      expect(forbiddenCall).not.toHaveBeenCalled();
    } finally {
      windowListeners.mockRestore();
      documentListeners.mockRestore();
      for (const { target, key, prior } of descriptors.reverse()) {
        if (prior) Object.defineProperty(target, key, prior);
        else Reflect.deleteProperty(target, key);
      }
    }
  });
});
