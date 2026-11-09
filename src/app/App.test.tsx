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

  function openPrivacy() {
    fireEvent.click(screen.getByRole("button", { name: "Review privacy" }));
  }

  function openPermissions() {
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to permissions" }),
    );
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

  it("offers Review privacy while preserving the existing Welcome actions", () => {
    render(<App />);
    openWelcome();

    for (const label of [
      "Back to Home",
      "Review privacy",
      "Review permission readiness",
    ]) {
      expect(screen.getByRole("button", { name: label })).toBeVisible();
    }
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

  it("opens the Privacy explanation, focuses its heading, and keeps primary navigation to Home and Settings", () => {
    render(<App />);
    openWelcome();
    openPrivacy();

    const heading = screen.getByRole("heading", {
      level: 1,
      name: "Your privacy with Outflow",
    });
    expect(heading).toBeVisible();
    expect(heading).toHaveFocus();
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(screen.queryByRole("heading", { name: "Home" })).not.toBeInTheDocument();
    expect(
      within(
        screen.getByRole("navigation", { name: "Application navigation" }),
      ).getAllByRole("button").map((button) => button.textContent),
    ).toEqual(["Home", "Settings"]);
    expect(
      screen.getByRole("heading", {
        level: 2,
        name: "Planned privacy requirements",
      }),
    ).toBeVisible();
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
    expect(screen.queryByText(/accept (the )?privacy|agree to terms/i)).not.toBeInTheDocument();
  });

  it("adds Continue to permissions without removing the existing Privacy actions", () => {
    render(<App />);
    openWelcome();
    openPrivacy();

    expect(
      screen.getByRole("button", { name: "Back to Welcome" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Review permission readiness" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Continue to permissions" }),
    ).toBeVisible();
  });

  it("opens Permissions, focuses its single h1, and keeps only Home and Settings as primary destinations", () => {
    render(<App />);
    openWelcome();
    openPrivacy();
    openPermissions();

    const heading = screen.getByRole("heading", {
      level: 1,
      name: "Permissions for Outflow",
    });
    expect(heading).toBeVisible();
    expect(heading).toHaveFocus();
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(
      within(
        screen.getByRole("navigation", { name: "Application navigation" }),
      )
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Home", "Settings"]);
    expect(screen.queryByRole("heading", { level: 1, name: "Home" })).not.toBeInTheDocument();
    for (const name of [
      "Microphone",
      "Global shortcuts",
      "Text insertion",
      "Review the separate capability details",
    ]) {
      expect(screen.getByRole("heading", { level: 2, name })).toBeVisible();
    }
    expect(
      screen.getByRole("heading", { level: 3, name: "Permission readiness" }),
    ).toBeVisible();
    expect(
      screen.getByText(
        "This is an unconnected browser preview. No permission has been granted, checked, or requested.",
      ),
    ).toBeVisible();
  });

  it("explains intentional microphone use and separates permission from device availability", () => {
    render(<App />);
    openWelcome();
    openPrivacy();
    openPermissions();

    const microphone = screen.getByRole("region", { name: "Microphone" });
    expect(microphone).toHaveTextContent(/only after you intentionally start dictation/i);
    expect(microphone).toHaveTextContent(/Permission and an available input device are separate/i);
    expect(microphone).toHaveTextContent(/permission alone does not mean a working microphone is available/i);
  });

  it("explains separate shortcut registration and release-event requirements", () => {
    render(<App />);
    openWelcome();
    openPrivacy();
    openPermissions();

    const shortcuts = screen.getByRole("region", { name: "Global shortcuts" });
    expect(shortcuts).toHaveTextContent(/intended to start and stop dictation/i);
    expect(shortcuts).toHaveTextContent(/requires reliable matching press-and-release events/i);
    expect(shortcuts).toHaveTextContent(/registration alone is not enough/i);
    expect(
      screen.getByText(/Unknown release support does not mean push-to-talk works/i),
    ).toBeVisible();
  });

  it("separates insertion permission, target eligibility, and the future Copy-only fallback", () => {
    render(<App />);
    openWelcome();
    openPrivacy();
    openPermissions();

    const insertion = screen.getByRole("region", { name: "Text insertion" });
    expect(insertion).toHaveTextContent(/place finalized text in supported destinations/i);
    expect(insertion).toHaveTextContent(/does not establish that a specific editor is safe or eligible/i);
    expect(insertion).toHaveTextContent(/optional for a future Copy-only workflow/i);
    expect(insertion).toHaveTextContent(/result panel that waits for an explicit Copy Result action/i);
    expect(insertion).toHaveTextContent(/no clipboard connection and does not enable Copy/i);
    expect(insertion).toHaveTextContent(/Microphone permission remains necessary for actual speech recording/i);
    expect(
      screen.queryByRole("button", { name: /^Copy$/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Next|Finish|Enable|Allow|Start Dictation/i }),
    ).not.toBeInTheDocument();
  });

  it("renders the existing checklist in its unverified default and keeps its fixtures synthetic", () => {
    render(<App />);
    openWelcome();
    openPrivacy();
    openPermissions();

    const checklist = screen.getByRole("region", { name: "Permission readiness" });
    expect(checklist).toBeVisible();
    expect(checklist).toHaveTextContent("Permission readiness has not been checked.");
    const selector = within(checklist).getByRole("combobox", {
      name: "Developer scenarios (simulated fixtures)",
    });
    fireEvent.change(selector, {
      target: { value: "hypothetical-available" },
    });
    expect(within(checklist).getByRole("status")).toHaveTextContent(
      /Simulated scenario: All capabilities hypothetically available/i,
    );
    expect(
      within(checklist).getByText(
        /Simulated values are independent fixtures, not shared operating-system observations/i,
      ),
    ).toBeVisible();
  });

  it("omits the developer scenario selector from the Permissions production rendering", () => {
    vi.stubEnv("DEV", false);
    render(<App />);
    openWelcome();
    openPrivacy();
    openPermissions();

    expect(screen.queryByRole("combobox", { name: /Developer scenarios/i })).not.toBeInTheDocument();
    expect(
      screen.getByRole("status", { name: "" }),
    ).toHaveTextContent("Permission readiness has not been checked.");
  });

  it("keeps setup checklist fixtures independent from Settings and returns to General with heading focus", () => {
    render(<App />);
    openWelcome();
    openPrivacy();
    openPermissions();
    fireEvent.change(
      screen.getByRole("combobox", {
        name: "Developer scenarios (simulated fixtures)",
      }),
      { target: { value: "hypothetical-available" } },
    );

    fireEvent.click(
      screen.getByRole("button", {
        name: "View permission details in Settings",
      }),
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
    ).toHaveTextContent("Permission readiness has not been checked.");
  });

  it("returns to Privacy and focuses its heading", () => {
    render(<App />);
    openWelcome();
    openPrivacy();
    openPermissions();

    const backButton = screen.getByRole("button", { name: "Back to Privacy" });
    expect(backButton).toHaveAttribute("type", "button");
    backButton.focus();
    fireEvent.click(backButton);

    const heading = screen.getByRole("heading", {
      level: 1,
      name: "Your privacy with Outflow",
    });
    expect(heading).toBeVisible();
    expect(heading).toHaveFocus();
  });

  it("opens Settings General from Permissions and focuses its heading", () => {
    render(<App />);
    openWelcome();
    openPrivacy();
    openPermissions();
    fireEvent.click(
      screen.getByRole("button", {
        name: "View permission details in Settings",
      }),
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
  });

  it("describes local speech recognition, text cleanup, and the no-account offline requirement", () => {
    render(<App />);
    openWelcome();
    openPrivacy();

    const section = screen.getByRole("region", { name: "Offline processing" });
    expect(section).toHaveTextContent(
      /speech recognition and text cleanup locally/i,
    );
    expect(section).toHaveTextContent(/without an Internet connection/i);
    expect(section).toHaveTextContent(/an account, or cloud inference/i);
    expect(section).toHaveTextContent(/not intended to be sent to a cloud transcription service/i);
    expect(section).toHaveTextContent(/not verified behavior in this frontend preview/i);
  });

  it("distinguishes prohibited audio and transcript retention from local preferences and vocabulary", () => {
    render(<App />);
    openWelcome();
    openPrivacy();

    const retention = screen.getByRole("region", {
      name: "What Outflow is designed not to retain",
    });
    expect(retention).toHaveTextContent("Saved audio recordings.");
    expect(retention).toHaveTextContent(/transcript history or persistent transcript caches/i);
    expect(retention).toHaveTextContent(/requirements prohibit retaining or tracking/i);

    const storage = screen.getByRole("region", { name: "What may be stored" });
    expect(storage).toHaveTextContent(/Application settings may be stored locally/i);
    expect(storage).toHaveTextContent(/vocabulary entries may eventually be stored locally/i);
    expect(storage).toHaveTextContent(/Recording audio and transcript history must not be stored/i);
    expect(storage).toHaveTextContent(/vocabulary draft is not persisted/i);
  });

  it("discloses explicit Copy Result, external clipboard retention, and memory or swap erasure limits", () => {
    render(<App />);
    openWelcome();
    openPrivacy();

    const section = screen.getByRole("region", {
      name: "Clipboard, operating system, and updates",
    });
    expect(section).toHaveTextContent(/explicitly choose Copy Result/i);
    expect(section).toHaveTextContent(/system clipboard/i);
    expect(section).toHaveTextContent(
      /destination application, operating system swap, and clipboard managers may retain content/i,
    );
    expect(section).toHaveTextContent(
      /cannot promise cryptographic erasure from memory or operating-system swap/i,
    );
  });

  it("separates user-initiated signed updates from normal offline dictation", () => {
    render(<App />);
    openWelcome();
    openPrivacy();

    const section = screen.getByRole("region", {
      name: "Clipboard, operating system, and updates",
    });
    expect(section).toHaveTextContent(
      /optional, explicit, user-initiated check for signed full-application updates/i,
    );
    expect(section).toHaveTextContent(/Normal dictation must not require update checks/i);
    expect(section).toHaveTextContent(/Normal dictation must not require Internet access/i);
    expect(section).toHaveTextContent(/runtime model downloads/i);
    expect(section).toHaveTextContent(/signed updates are not implemented in this preview/i);
  });

  it("labels the native and privacy guarantees as unimplemented and unverified", () => {
    render(<App />);
    openWelcome();
    openPrivacy();

    const status = screen.getByRole("heading", {
      level: 3,
      name: "Privacy implementation in progress",
    }).parentElement;
    expect(status).not.toBeNull();
    expect(status).toHaveTextContent(/native recording/i);
    expect(status).toHaveTextContent(/complete offline operation/i);
    expect(status).toHaveTextContent(/clipboard integration/i);
    expect(status).toHaveTextContent(/privacy safeguards have not yet been fully implemented and verified/i);
    expect(status).toHaveTextContent(/does not implement microphone recording/i);
  });

  it("returns to Welcome and moves focus to its heading", () => {
    render(<App />);
    openWelcome();
    openPrivacy();

    const backButton = screen.getByRole("button", { name: "Back to Welcome" });
    expect(backButton).toHaveAttribute("type", "button");
    backButton.focus();
    fireEvent.click(backButton);

    const heading = screen.getByRole("heading", {
      level: 1,
      name: "Welcome to Outflow",
    });
    expect(heading).toBeVisible();
    expect(heading).toHaveFocus();
  });

  it("opens Settings General from Privacy with the existing accessible checklist", () => {
    render(<App />);
    openWelcome();
    openPrivacy();
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
    const checklist = screen.getByRole("region", {
      name: "Permission readiness",
    });
    expect(checklist).toBeVisible();
    expect(checklist).toHaveTextContent("Permission readiness has not been checked");
    expect(screen.getAllByRole("tab")).toHaveLength(6);
    expect(screen.queryByText(/setup complete/i)).not.toBeInTheDocument();
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
    openPrivacy();
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    openPermissions();
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", {
        name: "View permission details in Settings",
      }),
    );
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Return to setup: Permissions" }),
    );
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    expect(screen.queryByText("Copy failed. Your result is still available in this panel.")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Continue to shortcuts" }));
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    expect(screen.queryByText("Copy failed. Your result is still available in this panel.")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { level: 1, name: "Home" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Back to Permissions" }));
    expect(screen.getByRole("heading", { level: 1, name: "Permissions for Outflow" })).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "Continue to shortcuts" }));
    fireEvent.click(screen.getByRole("button", { name: "Continue to Try Dictation" }));
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    expect(screen.queryByText("Copy failed. Your result is still available in this panel.")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { level: 1, name: "Home" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Show example text" }));
    expect(screen.getByRole("textbox", { name: "Example text (read-only, selectable)" })).toHaveValue(
      "So I met North Parkade at three.",
    );
    fireEvent.click(screen.getByRole("button", { name: "Continue to tray overview" }));
    expect(screen.queryByRole("textbox", { name: /example text/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    expect(screen.queryByText("Copy failed. Your result is still available in this panel.")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { level: 1, name: "Home" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Back to Try Dictation" }));
    expect(screen.getByRole("heading", { level: 1, name: "Try Dictation — preview" })).toHaveFocus();
    expect(screen.queryByRole("textbox", { name: /example text/i })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Continue to tray overview" }));
    fireEvent.click(screen.getByRole("button", { name: "Return to Home" }));
    expect(screen.getByRole("heading", { level: 1, name: "Home" })).toHaveFocus();
    expect(screen.queryByRole("region", { name: "Review finalized text" })).not.toBeInTheDocument();
    expect(screen.queryByText("Copy failed. Your result is still available in this panel.")).not.toBeInTheDocument();
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
      throw new Error("Setup navigation must remain browser-only and temporary.");
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
    installTripwire(Storage.prototype, "setItem", forbiddenCall);
    installTripwire(Storage.prototype, "getItem", forbiddenCall);
    installTripwire(Storage.prototype, "removeItem", forbiddenCall);

    try {
      render(<App />);
      openWelcome();
      openPrivacy();
      openPermissions();
      expect(screen.getByRole("region", { name: "Permission readiness" })).toHaveTextContent(
        "Permission readiness has not been checked.",
      );
      fireEvent.click(screen.getByRole("button", { name: "Back to Privacy" }));
      fireEvent.click(screen.getByRole("button", { name: "Back to Welcome" }));
      fireEvent.click(screen.getByRole("button", { name: "Review permission readiness" }));
      expect(screen.getByRole("region", { name: "Permission readiness" })).toBeVisible();
      expect(
        screen.getByRole("button", { name: "Return to setup: Welcome" }),
      ).toBeVisible();
      fireEvent.click(
        screen.getByRole("button", { name: "Return to setup: Welcome" }),
      );
      expect(
        screen.getByRole("heading", {
          level: 1,
          name: "Welcome to Outflow",
        }),
      ).toHaveFocus();
      fireEvent.click(
        within(
          screen.getByRole("navigation", { name: "Application navigation" }),
        ).getByRole("button", { name: "Settings" }),
      );
      fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));
      fireEvent.click(
        within(
          screen.getByRole("navigation", { name: "Application navigation" }),
        ).getByRole("button", { name: "Settings" }),
      );
      expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(
        screen.queryByRole("button", { name: /^Return to setup:/ }),
      ).not.toBeInTheDocument();
      expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
      expect(screen.queryByText(/accept (the )?privacy|agree to terms/i)).not.toBeInTheDocument();
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

describe("Shortcut setup explanation preview", () => {
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

  it("adds Continue to shortcuts without removing the existing Permissions actions", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Preview setup" }));
    fireEvent.click(screen.getByRole("button", { name: "Review privacy" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Continue to permissions" }),
    );

    expect(
      screen.getByRole("button", { name: "Back to Privacy" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", {
        name: "View permission details in Settings",
      }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Continue to shortcuts" }),
    ).toBeVisible();
  });

  it("opens the single Shortcut setup h1, focuses it, and keeps Home unmounted", () => {
    render(<App />);
    openShortcutSetup();

    const heading = screen.getByRole("heading", {
      level: 1,
      name: "Choose how dictation starts",
    });
    expect(heading).toBeVisible();
    expect(heading).toHaveFocus();
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(
      screen.queryByRole("heading", { level: 1, name: "Home" }),
    ).not.toBeInTheDocument();
    expect(
      within(
        screen.getByRole("navigation", { name: "Application navigation" }),
      )
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["Home", "Settings"]);
  });

  it("explains push-to-talk hold and release requirements", () => {
    render(<App />);
    openShortcutSetup();

    const pushToTalk = screen.getByRole("article", { name: "Push-to-talk" });
    expect(pushToTalk).toHaveTextContent(
      "Hold the configured shortcut to record; release to finalize.",
    );
    expect(pushToTalk).toHaveTextContent(
      "This mode requires reliable matching key-down and key-up events.",
    );
    expect(pushToTalk).toHaveTextContent(/Not verified/i);
  });

  it("explains toggle activation and that key release does not stop recording", () => {
    render(<App />);
    openShortcutSetup();

    const toggle = screen.getByRole("article", { name: "Toggle" });
    expect(toggle).toHaveTextContent(
      "Activate the configured shortcut to begin; activate it again to stop.",
    );
    expect(toggle).toHaveTextContent(
      "Key release is ignored for recording control.",
    );
    expect(toggle).toHaveTextContent(/Not operational/i);
  });

  it("never enables push-to-talk when release support is unknown or unsupported", () => {
    render(<App />);
    openShortcutSetup();

    const pushToTalk = () =>
      screen.getByRole("article", { name: "Push-to-talk" });
    expect(within(pushToTalk()).getByText(/^Not verified\./)).toBeVisible();
    expect(within(pushToTalk()).queryByText(/^Operational\./)).not.toBeInTheDocument();

    fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
      target: { value: "available-release-unsupported" },
    });
    expect(within(pushToTalk()).getByText(/^Unavailable\./)).toBeVisible();
    expect(within(pushToTalk()).queryByText(/^Operational\./)).not.toBeInTheDocument();
  });

  it("presents toggle only as a non-operational fallback when release events are unsupported", () => {
    render(<App />);
    openShortcutSetup();
    fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
      target: { value: "available-release-unsupported" },
    });

    const toggle = screen.getByRole("article", { name: "Toggle" });
    expect(
      within(toggle).getByText(/^Possible fallback — not operational\./),
    ).toBeVisible();
    expect(toggle).toHaveTextContent(
      /pending actual native registration and microphone readiness/i,
    );
    expect(
      screen.getByText(/no actual recording mode is operational/i),
    ).toBeVisible();
  });

  it("keeps shortcut registration separate from microphone readiness", () => {
    render(<App />);
    openShortcutSetup();

    const capabilityPanel = screen.getByRole("region", {
      name: "Global shortcut capability",
    });
    expect(capabilityPanel).toHaveTextContent(
      "Shortcut registration has not been checked.",
    );
    expect(capabilityPanel).toHaveTextContent(
      "Recording readiness, including microphone permission",
    );
    expect(capabilityPanel).toHaveTextContent("Not checked");
    expect(capabilityPanel).toHaveTextContent(
      /registered shortcut alone does not establish microphone permission or recording readiness/i,
    );
  });

  it("keeps conflict and authorization guidance distinct", () => {
    render(<App />);
    openShortcutSetup();
    const selector = screen.getByRole("combobox", { name: "Scenario" });

    fireEvent.change(selector, { target: { value: "conflict" } });
    expect(screen.getByRole("region", { name: "Global shortcut capability" })).toHaveTextContent(
      /shortcut conflict, not a permission denial/i,
    );

    fireEvent.change(selector, { target: { value: "permission-unavailable" } });
    expect(screen.getByRole("region", { name: "Global shortcut capability" })).toHaveTextContent(
      /authorization is unavailable/i,
    );
    expect(screen.getByRole("region", { name: "Global shortcut capability" })).not.toHaveTextContent(
      /shortcut conflict, not a permission denial/i,
    );
  });

  it("shows no shortcut assignment and reuses the existing capability panel", () => {
    render(<App />);
    openShortcutSetup();

    expect(
      screen.getByText(
        /no global shortcut is registered, no key combination is assigned or captured/i,
      ),
    ).toBeVisible();
    expect(screen.getByRole("region", { name: "Global shortcut capability" })).toBeVisible();
    expect(screen.queryByRole("term", { name: /assigned shortcut/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /finish setup|start recording|start dictation|enable shortcut/i })).not.toBeInTheDocument();
  });

  it("starts unconnected and unverified and marks all seven development scenarios simulated", () => {
    render(<App />);
    openShortcutSetup();

    expect(screen.getByRole("status")).toHaveTextContent(
      "Global keyboard shortcuts are not connected.",
    );
    expect(screen.getByRole("region", { name: "Global shortcut capability" })).toHaveTextContent(
      "Simulated",
    );
    expect(screen.getByRole("combobox", { name: "Scenario" })).toHaveValue("default");
    expect(screen.getByText("Simulated scenario — these values are fixed presentation examples, not observations from an operating system.")).toBeVisible();

    const scenarios = within(
      screen.getByRole("combobox", { name: "Scenario" }),
    ).getAllByRole("option");
    expect(scenarios).toHaveLength(7);
    expect(scenarios.every((option) => option.textContent?.includes("(simulated)"))).toBe(true);
  });

  it("omits the development scenario selector in production and keeps default status unconnected", () => {
    vi.stubEnv("DEV", false);
    render(<App />);
    openShortcutSetup();

    expect(screen.queryByRole("combobox", { name: "Scenario" })).not.toBeInTheDocument();
    expect(screen.queryByText("Developer scenarios (simulated)")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Global keyboard shortcuts are not connected.",
    );
    expect(screen.getByRole("region", { name: "Global shortcut capability" })).toHaveTextContent(
      "Browser preview — not verified",
    );
  });

  it("keeps setup and Settings shortcut scenarios independent", () => {
    render(<App />);
    openShortcutSetup();
    fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
      target: { value: "conflict" },
    });

    fireEvent.click(
      screen.getByRole("button", { name: "View shortcut details in Settings" }),
    );
    expect(
      screen.getByRole("heading", { level: 1, name: "Settings" }),
    ).toHaveFocus();
    expect(screen.getByRole("tab", { name: "Shortcuts" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    const settingsScenario = screen.getByRole("combobox", { name: "Scenario" });
    expect(settingsScenario).toHaveValue("default");
    expect(screen.getByRole("status")).toHaveTextContent(
      "Global keyboard shortcuts are not connected.",
    );

    fireEvent.change(settingsScenario, {
      target: { value: "permission-unavailable" },
    });
    expect(screen.getByRole("status")).toHaveTextContent(
      /registration is permission or authorization unavailable/i,
    );
  });

  it("returns to Permissions and focuses its heading", () => {
    render(<App />);
    openShortcutSetup();

    fireEvent.click(screen.getByRole("button", { name: "Back to Permissions" }));

    const heading = screen.getByRole("heading", {
      level: 1,
      name: "Permissions for Outflow",
    });
    expect(heading).toHaveFocus();
    expect(screen.getByRole("button", { name: "Continue to shortcuts" })).toBeVisible();
  });

  it("opens Settings Shortcuts only from the explicit detail action and focuses Settings", () => {
    render(<App />);
    openShortcutSetup();
    fireEvent.click(
      screen.getByRole("button", { name: "View shortcut details in Settings" }),
    );

    expect(
      screen.getByRole("heading", { level: 1, name: "Settings" }),
    ).toHaveFocus();
    expect(screen.getByRole("tab", { name: "Shortcuts" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("region", { name: "Global shortcut capability" })).toBeVisible();
  });

  it("keeps ordinary Settings navigation on General", () => {
    render(<App />);
    openShortcutSetup();
    fireEvent.click(
      within(
        screen.getByRole("navigation", { name: "Application navigation" }),
      ).getByRole("button", { name: "Settings" }),
    );

    expect(
      screen.getByRole("heading", { level: 1, name: "Settings" }),
    ).toHaveFocus();
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: "Shortcuts" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
  });

  it("preserves preference and vocabulary previews across setup navigation", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("switch", { name: "Launch at Login" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "shortcut preview term" },
    });
    fireEvent.change(screen.getByRole("textbox", { name: "Written replacement" }), {
      target: { value: "Shortcut Preview Term" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Home" }));

    openShortcutSetup();
    fireEvent.click(screen.getByRole("button", { name: "Continue to Try Dictation" }));
    fireEvent.click(screen.getByRole("button", { name: "Continue to tray overview" }));
    fireEvent.click(screen.getByRole("button", { name: "Return to Home" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(
      screen.getByRole("switch", { name: "Launch at Login" }),
    ).toHaveAttribute("aria-checked", "true");
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByRole("radio", { name: "Verbatim" })).toBeChecked();
    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "shortcut preview term",
    );
    expect(screen.getByRole("textbox", { name: "Written replacement" })).toHaveValue(
      "Shortcut Preview Term",
    );
  });

  it("does not install keyboard listeners or invoke native, microphone, permission, clipboard, storage, or network APIs", () => {
    const forbiddenCall = vi.fn(() => {
      throw new Error("Shortcut setup must remain presentation-only.");
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
    const apiTrap = new Proxy({}, { get: forbiddenCall, apply: forbiddenCall });
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
    installTripwire(window, "XMLHttpRequest", apiTrap);
    installTripwire(window, "WebSocket", apiTrap);
    installTripwire(window, "EventSource", apiTrap);
    installTripwire(Storage.prototype, "setItem", forbiddenCall);
    installTripwire(Storage.prototype, "getItem", forbiddenCall);
    installTripwire(Storage.prototype, "removeItem", forbiddenCall);

    try {
      render(<App />);
      openShortcutSetup();
      fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
        target: { value: "available-release-unsupported" },
      });

      const keyboardEvents = [
        ...windowListeners.mock.calls,
        ...documentListeners.mock.calls,
      ].filter(([type]) => type === "keydown" || type === "keyup" || type === "keypress");
      expect(keyboardEvents).toHaveLength(0);
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
