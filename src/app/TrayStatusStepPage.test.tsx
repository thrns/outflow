import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

describe("Tray and app status preview", () => {
  function openTryDictation() {
    fireEvent.click(screen.getByRole("button", { name: "Preview setup" }));
    fireEvent.click(screen.getByRole("button", { name: "Review privacy" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to permissions" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to shortcuts" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to Try Dictation" }),
    );
  }

  function openTrayStatus() {
    openTryDictation();
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to tray overview" }),
    );
  }

  it("opens from Try Dictation with one focused page heading and Home unmounted", () => {
    render(<App />);
    openTryDictation();

    fireEvent.click(
      screen.getByRole("button", { name: "Continue to tray overview" }),
    );

    const heading = screen.getByRole("heading", {
      level: 1,
      name: "Tray and app status — preview",
    });
    expect(heading).toBeVisible();
    expect(heading).toHaveFocus();
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(
      screen.queryByRole("heading", { level: 1, name: "Home" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Back to Try Dictation" }),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "Return to Home" })).toBeVisible();
  });

  it("states plainly that the current browser preview is not connected", () => {
    render(<App />);
    openTrayStatus();

    expect(
      screen.getByText("System tray integration is not connected."),
    ).toBeVisible();
    expect(
      screen.getByText(/this browser preview is not connected to a native Outflow application/i),
    ).toBeVisible();
    expect(
      screen.getByText(/planned desktop application will use a system tray or platform-appropriate system menu/i),
    ).toBeVisible();
  });

  it("explains planned tray functions as an informational list, not menu actions", () => {
    render(<App />);
    openTrayStatus();

    const list = screen.getByRole("list", { name: "Planned tray functions" });
    expect(within(list).getAllByRole("listitem")).toHaveLength(4);
    expect(list).toHaveTextContent(
      "Start / Stop. Control a dictation session when native recording is available.",
    );
    expect(list).toHaveTextContent(
      "Open Settings. Open the application's Settings interface.",
    );
    expect(list).toHaveTextContent(
      "Status / Permissions. Explain current recording readiness, capability limitations, and permission recovery.",
    );
    expect(list).toHaveTextContent(
      "Quit. Safely stop active resources and exit the application.",
    );
    expect(within(list).queryByRole("button")).not.toBeInTheDocument();
    expect(
      screen.getByText(/descriptions of future controls, not interactive menu items/i),
    ).toBeVisible();
  });

  it("offers no Start, Stop, Quit, Enable Tray, or setup-completion action", () => {
    render(<App />);
    openTrayStatus();

    for (const name of [/^Start$/, /^Stop$/, /^Quit$/, /^Enable Tray$/]) {
      expect(screen.queryByRole("button", { name })).not.toBeInTheDocument();
    }
    expect(screen.queryByRole("heading", { name: "Ready" })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: "Setup Complete" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/^Setup Complete$/)).not.toBeInTheDocument();
    expect(screen.queryByText(/^Installation Successful$/)).not.toBeInTheDocument();
    expect(screen.queryByText(/^Connected$/)).not.toBeInTheDocument();
  });

  it("lists model, microphone, session, control, target-safety, and platform prerequisites", () => {
    render(<App />);
    openTrayStatus();

    const section = screen.getByRole("region", {
      name: "What must work before Outflow is ready",
    });
    expect(section).toHaveTextContent(/valid bundled offline speech-model assets and working native recognition/i);
    expect(section).toHaveTextContent(/microphone permission and an available input device/i);
    expect(section).toHaveTextContent(/native recording and session lifecycle integration/i);
    expect(section).toHaveTextContent(/a supported shortcut or another recording control that is actually implemented/i);
    expect(section).toHaveTextContent(/safe target handling or an explicit Copy-only fallback/i);
    expect(section).toHaveTextContent(/successful platform integration and testing/i);
    expect(
      within(
        screen.getByRole("list", { name: "Tray readiness prerequisites" }),
      ).getAllByRole("listitem"),
    ).toHaveLength(6);
  });

  it("separates opening Settings and registering a shortcut from recording readiness", () => {
    render(<App />);
    openTrayStatus();

    const section = screen.getByRole("region", {
      name: "What must work before Outflow is ready",
    });
    expect(section).toHaveTextContent(
      /Being able to open Settings verifies only that the Settings interface opens; it does not prove that Outflow can record\./,
    );
    expect(section).toHaveTextContent(
      /A registered shortcut alone does not prove microphone permission, input-device availability, valid model assets, or working recognition\./,
    );
  });

  it("keeps insertion limitations and the explicit Copy-only fallback accurate", () => {
    render(<App />);
    openTrayStatus();

    const section = screen.getByRole("region", {
      name: "What must work before Outflow is ready",
    });
    expect(section).toHaveTextContent(/Text insertion may remain unavailable even when recording works/i);
    expect(section).toHaveTextContent(/Unsupported or protected destinations must never receive forced insertion/i);
    expect(section).toHaveTextContent(/explicit Copy-only fallback/i);
  });

  it("describes Settings close behavior as a future native requirement", () => {
    render(<App />);
    openTrayStatus();

    const section = screen.getByRole("region", {
      name: "Intended background behavior",
    });
    expect(section).toHaveTextContent(/future desktop application must keep its native core and tray active when a Settings window closes/i);
    expect(section).toHaveTextContent(/releasing resources owned by that Settings window/i);
    expect(section).toHaveTextContent(/closing this browser tab does not preserve a native Outflow service/i);
    expect(section).toHaveTextContent(/does not create a background process, register a tray icon, control microphone resources, or manage a native window lifecycle/i);
  });

  it("returns to Try Dictation with heading focus and its synthetic example hidden", () => {
    render(<App />);
    openTryDictation();
    fireEvent.click(screen.getByRole("button", { name: "Show example text" }));
    expect(
      screen.getByRole("textbox", { name: "Example text (read-only, selectable)" }),
    ).toBeVisible();

    fireEvent.click(
      screen.getByRole("button", { name: "Continue to tray overview" }),
    );
    expect(
      screen.queryByRole("textbox", { name: /example text/i }),
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Back to Try Dictation" }));

    expect(
      screen.getByRole("heading", {
        level: 1,
        name: "Try Dictation — preview",
      }),
    ).toHaveFocus();
    expect(screen.getByRole("button", { name: "Show example text" })).toBeVisible();
    expect(screen.queryByRole("textbox", { name: /example text/i })).not.toBeInTheDocument();
  });

  it("returns Home with heading focus and its unavailable default", () => {
    render(<App />);
    openTrayStatus();
    fireEvent.click(screen.getByRole("button", { name: "Return to Home" }));

    expect(screen.getByRole("heading", { level: 1, name: "Home" })).toHaveFocus();
    expect(screen.getByRole("combobox", { name: "Scenario" })).toHaveValue("unavailable");
    expect(
      screen.getByRole("region", { name: "Native dictation is not connected" }),
    ).toHaveTextContent("This preview does not request microphone access or start a recording session.");
  });

  it("keeps only Home and Settings in primary navigation and Settings opens General", () => {
    render(<App />);
    openTrayStatus();

    const navigation = screen.getByRole("navigation", {
      name: "Application navigation",
    });
    expect(within(navigation).getAllByRole("button").map((button) => button.textContent)).toEqual([
      "Home",
      "Settings",
    ]);
    fireEvent.click(within(navigation).getByRole("button", { name: "Settings" }));

    expect(screen.getByRole("heading", { level: 1, name: "Settings" })).toHaveFocus();
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("does not call native, microphone, permission, keyboard, tray, clipboard, storage, telemetry, or network APIs", () => {
    const forbiddenCall = vi.fn(() => {
      throw new Error("Tray status preview must remain presentation-only.");
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
    const addEventListenerSpy = vi.spyOn(window, "addEventListener");
    installTripwire(navigator, "mediaDevices", apiTrap);
    installTripwire(navigator, "permissions", apiTrap);
    installTripwire(navigator, "keyboard", apiTrap);
    installTripwire(navigator, "clipboard", apiTrap);
    installTripwire(window, "fetch", forbiddenCall);
    installTripwire(window, "XMLHttpRequest", networkConstructorTrap);
    installTripwire(window, "WebSocket", networkConstructorTrap);
    installTripwire(window, "EventSource", networkConstructorTrap);
    installTripwire(navigator, "sendBeacon", forbiddenCall);
    installTripwire(window, "__TAURI__", apiTrap);
    installTripwire(window, "__TAURI_INTERNALS__", apiTrap);
    installTripwire(window, "Notification", networkConstructorTrap);
    installTripwire(window, "localStorage", apiTrap);
    installTripwire(window, "sessionStorage", apiTrap);
    installTripwire(Storage.prototype, "setItem", forbiddenCall);
    installTripwire(Storage.prototype, "getItem", forbiddenCall);
    installTripwire(Storage.prototype, "removeItem", forbiddenCall);
    installTripwire(Storage.prototype, "clear", forbiddenCall);

    try {
      render(<App />);
      openTrayStatus();
      expect(
        screen.getByText("System tray integration is not connected."),
      ).toBeVisible();
      expect(screen.queryByText(/setup complete/i)).not.toBeInTheDocument();
      expect(forbiddenCall).not.toHaveBeenCalled();
      expect(
        addEventListenerSpy.mock.calls.some(([type]) =>
          ["keydown", "keyup", "beforeunload"].includes(type),
        ),
      ).toBe(false);
    } finally {
      addEventListenerSpy.mockRestore();
      for (const { target, key, prior } of descriptors.reverse()) {
        if (prior) Object.defineProperty(target, key, prior);
        else Reflect.deleteProperty(target, key);
      }
    }
  });
});
