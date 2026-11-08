import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

describe("Outflow Home and Settings preview", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("renders without errors", () => {
    expect(() => render(<App />)).not.toThrow();
  });

  it("shows the application name", () => {
    render(<App />);

    expect(screen.getByText("Outflow", { selector: "p" })).toBeVisible();
  });

  it("shows the Home heading", () => {
    render(<App />);

    expect(screen.getByRole("heading", { level: 1, name: "Home" })).toBeVisible();
  });

  it("uses the shared page container with one main landmark on both views", () => {
    const { container } = render(<App />);
    const expectOneContainedMain = () => {
      const main = screen.getAllByRole("main");

      expect(main).toHaveLength(1);
      expect(main[0].querySelectorAll(".page-container")).toHaveLength(1);
    };

    expectOneContainedMain();
    expect(container.querySelector(".home-shell")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expectOneContainedMain();
    expect(container.querySelector(".settings-shell")).not.toBeInTheDocument();
  });

  it("uses one shared header and marks its active page", () => {
    const { container } = render(<App />);
    const applicationNavigation = () =>
      screen.getAllByRole("navigation", { name: "Application navigation" });

    expect(container.querySelectorAll(".app-header")).toHaveLength(1);
    expect(applicationNavigation()).toHaveLength(1);
    expect(screen.getByText("Outflow", { selector: "p" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Home" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(
      screen.getByRole("button", { name: "Settings" }),
    ).not.toHaveAttribute("aria-current");

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(container.querySelectorAll(".app-header")).toHaveLength(1);
    expect(applicationNavigation()).toHaveLength(1);
    expect(screen.getByText("Outflow", { selector: "p" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Settings" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(
      screen.getByRole("button", { name: "Home" }),
    ).not.toHaveAttribute("aria-current");
  });

  it("shows the offline desktop dictation description", () => {
    render(<App />);

    expect(
      screen.getByText("Outflow is designed for offline desktop dictation."),
    ).toBeVisible();
  });

  it("opens Settings from Home", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    const settingsHeading = screen.getByRole("heading", {
      level: 1,
      name: "Settings",
    });
    expect(settingsHeading).toBeVisible();
    expect(settingsHeading).toHaveFocus();
    expect(
      screen.getByText("Configure your offline dictation experience."),
    ).toBeVisible();
  });

  it("shows all six settings sections in an accessible tab list", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(screen.getByRole("tablist")).toBeVisible();
    expect(screen.getAllByRole("tab")).toHaveLength(6);
    for (const section of [
      "General",
      "Microphone",
      "Shortcuts",
      "Text & Vocabulary",
      "Privacy",
      "About & Diagnostics",
    ]) {
      expect(screen.getByRole("tab", { name: section })).toBeVisible();
    }
  });

  it("shows the cleanup mode selector and preview-only vocabulary form", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(
      screen.getByRole("heading", { level: 2, name: "Text & Vocabulary" }),
    ).toBeVisible();
    expect(
      screen.getByText("Set text cleanup preferences and custom terms."),
    ).toBeVisible();
    expect(
      screen.getByRole("group", { name: "Text cleanup mode" }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { level: 3, name: "Custom vocabulary" }),
    ).toBeVisible();
    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toBeVisible();
    const writtenInput = screen.getByRole("textbox", {
      name: "Written replacement",
    });
    expect(writtenInput).toBeVisible();
    expect(
      screen.getByRole("heading", {
        level: 4,
        name: "Illustrative example",
      }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Enter a spoken phrase and written replacement to see an illustrative example.",
      ),
    ).toBeVisible();
    const previewHeading = screen.getByRole("heading", {
      level: 4,
      name: "Illustrative example",
    });
    expect(
      writtenInput.compareDocumentPosition(previewHeading) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      screen.getByText(
        "This is a synthetic illustration of your draft. Entries in the development preview list are not persisted or applied to actual speech.",
      ),
    ).toBeVisible();
  });

  it("updates the draft illustration with both fields and keeps form validation", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    const spoken = screen.getByRole("textbox", { name: "Spoken phrase" });
    const written = screen.getByRole("textbox", {
      name: "Written replacement",
    });

    fireEvent.change(spoken, { target: { value: "north parkade" } });
    expect(
      screen.getByText(
        "Enter both a spoken phrase and written replacement to see the illustrative example.",
      ),
    ).toBeVisible();

    fireEvent.change(written, { target: { value: "North Parkade" } });
    expect(
      screen.getByText("I mentioned “north parkade” today."),
    ).toBeVisible();
    expect(
      screen.getByText("I mentioned “North Parkade” today."),
    ).toBeVisible();

    fireEvent.change(spoken, { target: { value: "   " } });
    expect(screen.queryByText(/I mentioned/)).not.toBeInTheDocument();
    fireEvent.blur(spoken);
    expect(spoken).toHaveAttribute("aria-invalid", "true");
    expect(
      screen.getByText("Enter at least one non-whitespace character."),
    ).toBeVisible();

    fireEvent.change(spoken, { target: { value: "south station" } });
    expect(spoken).not.toHaveAttribute("aria-invalid", "true");
    expect(
      screen.getByText("I mentioned “south station” today."),
    ).toBeVisible();
  });

  it("preserves vocabulary draft values when switching Settings sections", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "north parkade" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "North Parkade" } },
    );

    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));
    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Privacy implementation in progress",
      }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "north parkade",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("North Parkade");
    expect(
      screen.getByText("I mentioned “north parkade” today."),
    ).toBeVisible();
    expect(
      screen.getByText("I mentioned “North Parkade” today."),
    ).toBeVisible();
  });

  it("preserves vocabulary draft values across Home and Settings navigation", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "café" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "Café" } },
    );

    fireEvent.click(screen.getByRole("button", { name: "Home" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "café",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("Café");
    expect(screen.getByText("I mentioned “café” today.")).toBeVisible();
    expect(screen.getByText("I mentioned “Café” today.")).toBeVisible();
  });

  it("resets vocabulary drafts after a fresh application mount", () => {
    const { unmount } = render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "north parkade" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "North Parkade" } },
    );
    unmount();

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("");
    expect(
      screen.getByText(
        "Enter a spoken phrase and written replacement to see an illustrative example.",
      ),
    ).toBeVisible();
  });

  it("selects Clean by default and leaves Verbatim unselected", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    const clean = screen.getByRole("radio", { name: "Clean" });
    const verbatim = screen.getByRole("radio", { name: "Verbatim" });

    expect(clean).toBeChecked();
    expect(verbatim).not.toBeChecked();
    expect(screen.getAllByRole("radio", { checked: true })).toHaveLength(1);
  });

  it("selects Verbatim with the mouse and keeps exactly one mode checked", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));

    expect(screen.getByRole("radio", { name: "Verbatim" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Clean" })).not.toBeChecked();
    expect(screen.getAllByRole("radio", { checked: true })).toHaveLength(1);
  });

  it("restores Clean when its radio is selected again", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));
    fireEvent.click(screen.getByRole("radio", { name: "Clean" }));

    expect(screen.getByRole("radio", { name: "Clean" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Verbatim" })).not.toBeChecked();
    expect(screen.getAllByRole("radio", { checked: true })).toHaveLength(1);
  });

  it("gives the radio group and each option accessible names and descriptions", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(
      screen.getByRole("group", { name: "Text cleanup mode" }),
    ).toBeVisible();
    expect(screen.getByRole("radio", { name: "Clean" })).toHaveAccessibleDescription(
      /Intended to remove supported filler words and apply deterministic text cleanup\./,
    );
    expect(
      screen.getByRole("radio", { name: "Verbatim" }),
    ).toHaveAccessibleDescription(
      /Intended to preserve filler words rather than removing them\./,
    );
  });

  it("uses one native radio group so browser keyboard behavior is available", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    const clean = screen.getByRole("radio", { name: "Clean" });
    const verbatim = screen.getByRole("radio", { name: "Verbatim" });

    expect(clean).toHaveAttribute("type", "radio");
    expect(verbatim).toHaveAttribute("type", "radio");
    expect(clean).toHaveAttribute("name", "text-cleanup-mode");
    expect(verbatim).toHaveAttribute("name", "text-cleanup-mode");
    verbatim.focus();
    expect(verbatim).toHaveFocus();
  });

  it("keeps cleanup mode when switching between Settings sections", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));
    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByRole("radio", { name: "Verbatim" })).toBeChecked();
  });

  it("keeps cleanup mode when navigating between Home and Settings", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));
    fireEvent.click(screen.getByRole("button", { name: "Home" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByRole("radio", { name: "Verbatim" })).toBeChecked();
  });

  it("defaults to Clean after a fresh application mount", () => {
    const { unmount } = render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));
    unmount();

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByRole("radio", { name: "Clean" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Verbatim" })).not.toBeChecked();
  });

  it("clearly states that mode selection is not connected to processing", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(
      screen.getByText(
        "Mode selection is available in this development preview. It is not yet connected to dictation or text processing.",
      ),
    ).toBeVisible();
  });

  it("starts with General selected", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(
      screen.getByRole("heading", { level: 2, name: "General" }),
    ).toBeVisible();
    expect(
      screen.getByText("Manage general application preferences."),
    ).toBeVisible();
    expect(
      screen.getByText("Start Outflow automatically when you sign in to your computer."),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Preview only — this setting does not change your computer's startup behavior yet.",
      ),
    ).toBeVisible();
    expect(
      screen.getByRole("switch", { name: "Launch at Login" }),
    ).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("Off", { selector: ".settings-switch-state" })).toBeVisible();
  });

  it("toggles Launch at Login on and off with the mouse", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    const launchAtLogin = screen.getByRole("switch", {
      name: "Launch at Login",
    });
    fireEvent.click(launchAtLogin);

    expect(launchAtLogin).toHaveAttribute("aria-checked", "true");
    expect(
      screen.getByText("On", { selector: ".settings-switch-state" }),
    ).toBeVisible();

    fireEvent.click(launchAtLogin);

    expect(launchAtLogin).toHaveAttribute("aria-checked", "false");
    expect(
      screen.getByText("Off", { selector: ".settings-switch-state" }),
    ).toBeVisible();
  });

  it("starts Launch at Login OFF in a fresh application mount", () => {
    const { unmount } = render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("switch", { name: "Launch at Login" }));
    expect(screen.getByRole("switch", { name: "Launch at Login" })).toHaveAttribute(
      "aria-checked",
      "true",
    );

    unmount();
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(screen.getByRole("switch", { name: "Launch at Login" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
  });

  it("places the permission checklist below Launch at Login without adding a settings tab", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    const launchSwitch = screen.getByRole("switch", { name: "Launch at Login" });
    const checklist = screen.getByRole("region", { name: "Permission readiness" });
    const tabList = screen.getByRole("tablist");

    expect(launchSwitch.compareDocumentPosition(checklist) & Node.DOCUMENT_POSITION_FOLLOWING).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
    expect(within(tabList).getAllByRole("tab")).toHaveLength(6);
    expect(launchSwitch).toBeVisible();
  });

  it("keeps permission fixtures local to General and independent from Microphone and Shortcuts", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.change(
      screen.getByRole("combobox", {
        name: "Developer scenarios (simulated fixtures)",
      }),
      { target: { value: "microphone-denied" } },
    );

    fireEvent.click(screen.getByRole("tab", { name: "Microphone" }));
    expect(screen.getByRole("status")).toHaveTextContent(
      "Microphone status has not been checked.",
    );
    expect(screen.queryByText("Denied (simulated)")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));
    expect(screen.getByRole("status")).toHaveTextContent(
      "Global keyboard shortcuts are not connected.",
    );
  });

  it("toggles Launch at Login with Space and ignores a repeated keydown", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    const launchAtLogin = screen.getByRole("switch", {
      name: "Launch at Login",
    });
    launchAtLogin.focus();
    expect(launchAtLogin).toHaveFocus();

    fireEvent.keyDown(launchAtLogin, { key: " " });
    expect(launchAtLogin).toHaveAttribute("aria-checked", "true");

    fireEvent.keyUp(launchAtLogin, { key: " " });
    fireEvent.keyDown(launchAtLogin, { key: " ", repeat: true });
    expect(launchAtLogin).toHaveAttribute("aria-checked", "true");
  });

  it("keeps the preview value when navigating away from and back to General", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Launch at Login" }),
    );
    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));

    expect(
      screen.getByRole("heading", { level: 2, name: "Privacy" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("tab", { name: "General" }));

    expect(
      screen.getByRole("switch", { name: "Launch at Login" }),
    ).toHaveAttribute("aria-checked", "true");
    expect(
      screen.getByText("On", { selector: ".settings-switch-state" }),
    ).toBeVisible();
  });

  it("keeps the preview value when Settings is closed and reopened", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Launch at Login" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Home" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(
      screen.getByRole("switch", { name: "Launch at Login" }),
    ).toHaveAttribute("aria-checked", "true");
    expect(
      screen.getByText("On", { selector: ".settings-switch-state" }),
    ).toBeVisible();
  });

  it("keeps the other Settings sections available", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    for (const section of [
      "General",
      "Microphone",
      "Shortcuts",
      "Text & Vocabulary",
      "Privacy",
      "About & Diagnostics",
    ]) {
      fireEvent.click(screen.getByRole("tab", { name: section }));

      const tab = screen.getByRole("tab", { name: section });
      const panels = screen.getAllByRole("tabpanel");
      expect(panels).toHaveLength(1);
      const panel = screen.getByRole("tabpanel", { name: section });
      const heading = within(panel).getByRole("heading", {
        level: 2,
        name: section,
      });

      expect(heading).toBeVisible();
      expect(tab).toHaveAttribute("aria-selected", "true");
      expect(tab).toHaveAttribute("aria-controls", panel.id);
      expect(panel).toHaveAttribute("aria-labelledby", tab.id);
      expect(document.querySelectorAll(`#${heading.id}`)).toHaveLength(1);
      expect(document.querySelectorAll(`#${tab.id}`)).toHaveLength(1);
      if (section === "Microphone") {
        expect(
          within(panel).getByRole("heading", {
            level: 3,
            name: "Microphone capability",
          }),
        ).toBeVisible();
      } else if (section === "Shortcuts") {
        expect(
          within(panel).getByRole("heading", {
            level: 3,
            name: "Global shortcut capability",
          }),
        ).toBeVisible();
      } else if (section === "Text & Vocabulary") {
        expect(
          within(panel).getByRole("group", { name: "Text cleanup mode" }),
        ).toBeVisible();
        expect(
          within(panel).getByRole("textbox", { name: "Spoken phrase" }),
        ).toBeVisible();
        expect(
          within(panel).getByRole("textbox", {
            name: "Written replacement",
          }),
        ).toBeVisible();
      } else if (section === "Privacy") {
        expect(
          within(panel).getByRole("heading", {
            level: 3,
            name: "Privacy implementation in progress",
          }),
        ).toBeVisible();
        expect(
          within(panel).getByRole("heading", {
            level: 3,
            name: "Offline processing",
          }),
        ).toBeVisible();
      } else if (section === "About & Diagnostics") {
        expect(
          within(panel).getByRole("heading", {
            level: 3,
            name: "Troubleshooting",
          }),
        ).toBeVisible();
        expect(
          within(panel).getByRole("heading", {
            level: 3,
            name: "Diagnostics export unavailable",
          }),
        ).toBeVisible();
        expect(
          within(panel).getByText(
            "Update checking is not implemented in this development preview.",
          ),
        ).toBeVisible();
      }
    }
  });

  it("shows the default microphone preview as unchecked and not connected", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Microphone" }));

    const panel = screen.getByRole("tabpanel", { name: "Microphone" });
    expect(within(panel).getByRole("status")).toHaveTextContent(
      "Microphone status has not been checked.",
    );
    expect(panel).toHaveTextContent("Permission statusNot checked");
    expect(panel).toHaveTextContent("Input-device availabilityNot checked");
    expect(panel).toHaveTextContent("Native microphone integrationNot connected");
    expect(panel).toHaveTextContent("No audio is captured");
    expect(panel).toHaveTextContent("only after you intentionally start dictation");
    expect(within(panel).getByRole("button", { name: "Check again" })).toBeDisabled();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("opens Microphone settings without calling browser permission or capture APIs", () => {
    const forbiddenCall = vi.fn(() => {
      throw new Error("The preview must not access microphone or permission APIs.");
    });
    const descriptors: Array<{ target: object; key: string; prior?: PropertyDescriptor }> = [];
    const installTripwire = (target: object, key: string, value: unknown) => {
      descriptors.push({ target, key, prior: Object.getOwnPropertyDescriptor(target, key) });
      Object.defineProperty(target, key, { configurable: true, value });
    };

    installTripwire(navigator, "mediaDevices", {
      getUserMedia: forbiddenCall,
      enumerateDevices: forbiddenCall,
    });
    installTripwire(navigator, "permissions", { query: forbiddenCall });

    try {
      render(<App />);
      fireEvent.click(screen.getByRole("button", { name: "Settings" }));
      fireEvent.click(screen.getByRole("tab", { name: "Microphone" }));

      expect(screen.getByRole("status")).toHaveTextContent(
        "Microphone status has not been checked.",
      );
      expect(forbiddenCall).not.toHaveBeenCalled();
    } finally {
      for (const { target, key, prior } of descriptors.reverse()) {
        if (prior) Object.defineProperty(target, key, prior);
        else Reflect.deleteProperty(target, key);
      }
    }
  });

  it("omits development microphone scenarios when Vite reports production mode", () => {
    vi.stubEnv("DEV", false);
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Microphone" }));

    expect(screen.queryByRole("combobox", { name: "Scenario" })).not.toBeInTheDocument();
  });

  it("omits the shortcut scenario inspector when Vite reports production mode", () => {
    vi.stubEnv("DEV", false);
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));

    const panel = screen.getByRole("tabpanel", { name: "Shortcuts" });
    expect(
      within(panel).getByRole("heading", {
        level: 3,
        name: "Global shortcut capability",
      }),
    ).toBeVisible();
    expect(
      within(panel).queryByRole("group", {
        name: "Developer scenarios (simulated)",
      }),
    ).not.toBeInTheDocument();
    expect(
      within(panel).queryByRole("combobox", { name: "Scenario" }),
    ).not.toBeInTheDocument();
  });

  it("shows unverified global shortcut status without implying registration", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));

    const panel = screen.getByRole("tabpanel", { name: "Shortcuts" });
    expect(
      within(panel).getByRole("heading", {
        level: 3,
        name: "Global shortcut capability",
      }),
    ).toBeVisible();
    expect(within(panel).getByRole("status")).toHaveTextContent(
      "Global keyboard shortcuts are not connected.",
    );
    expect(panel).toHaveTextContent("Global shortcut registrationNot checked");
    expect(panel).toHaveTextContent("Matching press-and-release eventsNot checked");
    expect(panel).toHaveTextContent(
      "No shortcut is currently registered by this preview.",
    );
    expect(panel).toHaveTextContent("No recording mode is operational.");
    expect(within(panel).getByRole("status")).toHaveAttribute(
      "aria-live",
      "polite",
    );
    expect(within(panel).queryByRole("alert")).not.toBeInTheDocument();
    expect(
      within(panel).getByRole("button", { name: "Check shortcut status" }),
    ).toBeDisabled();
    expect(
      within(panel).getByRole("group", {
        name: "Developer scenarios (simulated)",
      }),
    ).toBeVisible();
  });

  it("switches between Shortcuts and Microphone status messages", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));

    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Global shortcut capability",
      }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole("tab", { name: "Microphone" }));
    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Microphone capability",
      }),
    ).toBeVisible();
    expect(
      screen.queryByRole("heading", {
        level: 3,
        name: "Global shortcut capability",
      }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));
    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Global shortcut capability",
      }),
    ).toBeVisible();
  });

  it("updates section content and selected state when a tab is chosen", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));

    expect(screen.getByRole("tab", { name: "Privacy" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
    expect(
      screen.getByRole("heading", { level: 2, name: "Privacy" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Review how Outflow handles audio, transcripts, and diagnostics.",
      ),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Privacy implementation in progress",
      }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { level: 3, name: "Offline processing" }),
    ).toBeVisible();
  });

  it("renders About & Diagnostics in the existing Settings panel and navigates between sections", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "About & Diagnostics" }));

    expect(
      screen.getByRole("tab", { name: "About & Diagnostics" }),
    ).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByRole("tabpanel", { name: "About & Diagnostics" }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { level: 3, name: "Diagnostics" }),
    ).toBeVisible();
    expect(
      screen.getByText("Update checking is not implemented in this development preview."),
    ).toBeVisible();

    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));
    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Privacy implementation in progress",
      }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("tab", { name: "About & Diagnostics" }));
    expect(
      screen.getByRole("heading", { level: 3, name: "Diagnostics" }),
    ).toBeVisible();
  });

  it("preserves Settings values while visiting About & Diagnostics and keeps page heading focus", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(
      screen.getByRole("heading", { level: 1, name: "Settings" }),
    ).toHaveFocus();
    fireEvent.click(screen.getByRole("switch", { name: "Launch at Login" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "north parkade" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "North Parkade" } },
    );
    fireEvent.click(screen.getByRole("tab", { name: "About & Diagnostics" }));

    expect(
      screen.getByRole("heading", { level: 3, name: "Troubleshooting" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Home" }));
    expect(screen.getByRole("heading", { level: 1, name: "Home" })).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(
      screen.getByRole("heading", { level: 1, name: "Settings" }),
    ).toHaveFocus();

    expect(
      screen.getByRole("switch", { name: "Launch at Login" }),
    ).toHaveAttribute("aria-checked", "true");
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    expect(screen.getByRole("radio", { name: "Verbatim" })).toBeChecked();
    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "north parkade",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("North Parkade");
  });

  it("supports arrow-key navigation between sections", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    const generalTab = screen.getByRole("tab", { name: "General" });
    generalTab.focus();
    fireEvent.keyDown(generalTab, { key: "ArrowDown" });

    const microphoneTab = screen.getByRole("tab", { name: "Microphone" });
    expect(microphoneTab).toHaveFocus();
    expect(microphoneTab).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByRole("heading", { level: 2, name: "Microphone" }),
    ).toBeVisible();
  });

  it("shows that this development preview has no native dictation", () => {
    render(<App />);

    expect(screen.getByText("Development preview")).toBeVisible();
    expect(
      screen.getByRole("heading", {
        level: 2,
        name: "Native dictation is not connected",
      }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "This preview does not request microphone access or start a recording session. No speech model is connected.",
      ),
    ).toBeVisible();
  });

  it("does not offer a working dictation or model-download control", () => {
    render(<App />);

    expect(
      screen.queryByRole("button", { name: /dictate|record|download model/i }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/transcript history/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/download a model/i)).not.toBeInTheDocument();
  });

  it("uses a labeled navigation landmark and native Settings buttons", () => {
    render(<App />);

    const navigation = screen.getByRole("navigation", {
      name: "Application navigation",
    });
    expect(navigation).toBeVisible();
    const homeButton = screen.getByRole("button", { name: "Home" });
    const settingsButton = screen.getByRole("button", { name: "Settings" });
    expect(homeButton).toHaveAttribute("type", "button");
    expect(settingsButton).toHaveAttribute("type", "button");
    homeButton.focus();
    expect(homeButton).toHaveFocus();
    settingsButton.focus();
    expect(settingsButton).toHaveFocus();
    expect(
      screen.getByRole("button", { name: "Open Settings" }),
    ).toHaveAttribute("type", "button");
  });

  it("returns to the Home screen from Settings", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));
    fireEvent.click(screen.getByRole("button", { name: "Home" }));

    const homeHeading = screen.getByRole("heading", {
      level: 1,
      name: "Home",
    });
    expect(homeHeading).toBeVisible();
    expect(homeHeading).toHaveFocus();
  });

  it("opens Settings to General again after returning Home", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));
    fireEvent.click(screen.getByRole("button", { name: "Home" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(
      screen.getByRole("heading", { level: 2, name: "General" }),
    ).toBeVisible();
  });

  it("clears both vocabulary fields and preview without changing the active section or mode", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "north parkade" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "North Parkade" } },
    );
    expect(screen.getByText("I mentioned “north parkade” today.")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Clear draft" }));

    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("");
    expect(
      screen.getByText(
        "Enter a spoken phrase and written replacement to see an illustrative example.",
      ),
    ).toBeVisible();
    expect(screen.getByRole("radio", { name: "Verbatim" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Clean" })).not.toBeChecked();
    expect(screen.getByRole("tab", { name: "Text & Vocabulary" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getAllByRole("tab")).toHaveLength(6);
  });

  it("keeps Launch at Login unchanged and the cleared draft empty across navigation", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("switch", { name: "Launch at Login" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "north parkade" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "North Parkade" } },
    );

    fireEvent.click(screen.getByRole("button", { name: "Clear draft" }));
    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("");

    fireEvent.click(screen.getByRole("button", { name: "Home" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("switch", { name: "Launch at Login" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("");

    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "south station" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "South Station" } },
    );
    expect(screen.getByText("I mentioned “south station” today.")).toBeVisible();
  });
});

describe("Welcome setup introduction preview", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  function openWelcome() {
    fireEvent.click(screen.getByRole("button", { name: "Preview setup" }));
  }

  it("offers Preview setup in Home's Getting Started area", () => {
    render(<App />);

    const gettingStarted = screen.getByRole("region", {
      name: "Dictation that stays local",
    });
    const previewButton = within(gettingStarted).getByRole("button", {
      name: "Preview setup",
    });

    expect(previewButton).toHaveAttribute("type", "button");
    expect(previewButton.tabIndex).toBe(0);
  });

  it("opens Welcome with the planned offline and privacy principles and preview disclaimer", () => {
    render(<App />);
    openWelcome();

    const heading = screen.getByRole("heading", {
      level: 1,
      name: "Welcome to Outflow",
    });
    expect(heading).toBeVisible();
    expect(heading).toHaveFocus();
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(
      within(
        screen.getByRole("navigation", { name: "Application navigation" }),
      ).getAllByRole("button"),
    ).toHaveLength(2);
    expect(
      screen.getByText(
        "Speech stays on this device. No account. No transcript history.",
      ),
    ).toBeVisible();
    expect(
      screen.getByText(/intended privacy design for the planned MVP/i),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { level: 2, name: "Development preview" }),
    ).toBeVisible();
    expect(
      screen.getByText(/does not prove that offline speech processing is complete/i),
    ).toBeVisible();
    expect(
      screen.getByText(/functioning Outflow desktop application and microphone permission/i),
    ).toBeVisible();
  });

  it("does not claim that recording, setup, or dictation is ready", () => {
    render(<App />);
    openWelcome();

    expect(
      screen.queryByText(/ready to dictate|recording started|microphone connected|setup complete/i),
    ).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /record|dictate/i })).not.toBeInTheDocument();
    expect(screen.queryByText(/listening/i)).not.toBeInTheDocument();
  });

  it("returns to Home and moves focus to the Home heading", () => {
    render(<App />);
    openWelcome();

    const backButton = screen.getByRole("button", { name: "Back to Home" });
    expect(backButton).toHaveAttribute("type", "button");
    backButton.focus();
    expect(backButton).toHaveFocus();
    fireEvent.click(backButton);

    const homeHeading = screen.getByRole("heading", { level: 1, name: "Home" });
    expect(homeHeading).toBeVisible();
    expect(homeHeading).toHaveFocus();
  });

  it("opens Settings to General with the existing checklist and six tabs", () => {
    render(<App />);
    openWelcome();
    fireEvent.click(
      screen.getByRole("button", { name: "Review permission readiness" }),
    );

    const settingsHeading = screen.getByRole("heading", {
      level: 1,
      name: "Settings",
    });
    expect(settingsHeading).toHaveFocus();
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(
      screen.getByRole("region", { name: "Permission readiness" }),
    ).toBeVisible();
    expect(screen.getAllByRole("tab")).toHaveLength(6);
    for (const tab of [
      "General",
      "Microphone",
      "Shortcuts",
      "Text & Vocabulary",
      "Privacy",
      "About & Diagnostics",
    ]) {
      expect(screen.getByRole("tab", { name: tab })).toBeVisible();
    }
  });

  it("discards Home's volatile Copy Result and acknowledgment but keeps lifecycle IDs unique", async () => {
    render(<App />);
    fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
      target: { value: "copy-failure" },
    });
    const firstResult = screen.getByRole("region", { name: "Review finalized text" });
    const firstLifecycleId = firstResult.getAttribute("data-result-lifecycle-id");
    fireEvent.click(screen.getByRole("button", { name: "Simulate Copy request" }));
    expect(
      await screen.findByText("Copy failed. Your result is still available in this panel."),
    ).toBeVisible();

    openWelcome();
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Back to Home" }));
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Scenario" })).toHaveValue(
      "unavailable",
    );

    fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
      target: { value: "copy-failure" },
    });
    const secondResult = screen.getByRole("region", { name: "Review finalized text" });
    const secondLifecycleId = secondResult.getAttribute("data-result-lifecycle-id");
    expect(firstLifecycleId).not.toBeNull();
    expect(secondLifecycleId).not.toBe(firstLifecycleId);

    fireEvent.click(screen.getByRole("button", { name: "Simulate Copy request" }));
    expect(
      await screen.findByText("Copy failed. Your result is still available in this panel."),
    ).toBeVisible();
    expect(screen.getByRole("region", { name: "Review finalized text" })).toBeVisible();
  });

  it("does not call native, keyboard, clipboard, network, or persistence APIs while navigating", () => {
    const forbiddenCall = vi.fn(() => {
      throw new Error("Welcome navigation must remain browser-only and temporary.");
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
    installTripwire(navigator, "clipboard", apiTrap);
    installTripwire(window, "fetch", forbiddenCall);
    installTripwire(window, "XMLHttpRequest", networkConstructorTrap);
    installTripwire(window, "WebSocket", networkConstructorTrap);
    installTripwire(window, "EventSource", networkConstructorTrap);
    installTripwire(navigator, "sendBeacon", forbiddenCall);
    installTripwire(window, "__TAURI__", apiTrap);
    installTripwire(window, "__TAURI_INTERNALS__", apiTrap);
    installTripwire(Storage.prototype, "setItem", forbiddenCall);

    try {
      render(<App />);
      openWelcome();
      fireEvent.click(screen.getByRole("button", { name: "Review permission readiness" }));
      expect(screen.getByRole("region", { name: "Permission readiness" })).toBeVisible();
      expect(forbiddenCall).not.toHaveBeenCalled();
      expect(
        addEventListenerSpy.mock.calls.some(([type]) => type === "keydown" || type === "keyup"),
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
