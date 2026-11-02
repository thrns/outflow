import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("Outflow Home and Settings preview", () => {
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
        "This is a synthetic illustration of your vocabulary draft. No vocabulary entry is saved, and no speech recognition or text replacement is performed.",
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
            name: "Microphone setup unavailable",
          }),
        ).toBeVisible();
      } else if (section === "Shortcuts") {
        expect(
          within(panel).getByRole("heading", {
            level: 3,
            name: "Keyboard shortcuts unavailable",
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

  it("describes the unavailable Microphone preview without reporting device state", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Microphone" }));

    const panel = screen.getByRole("tabpanel", { name: "Microphone" });
    const statusMessage = within(panel).getByRole("heading", {
      level: 3,
      name: "Microphone setup unavailable",
    }).parentElement;

    expect(statusMessage).toHaveTextContent(
      "Microphone selection is not connected in this development preview.",
    );
    expect(statusMessage).toHaveTextContent("No audio is captured");
    expect(statusMessage).toHaveTextContent(
      "device availability and microphone permissions have not been checked",
    );
    expect(statusMessage).toHaveTextContent(
      "Selecting an input will require future native integration.",
    );
    expect(statusMessage).not.toHaveTextContent(
      /connected microphone|permissions? (?:are )?granted/i,
    );
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("explains that global shortcuts are unavailable without implying registration", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));

    const panel = screen.getByRole("tabpanel", { name: "Shortcuts" });
    const title = within(panel).getByRole("heading", {
      level: 3,
      name: "Keyboard shortcuts unavailable",
    });
    if (title.parentElement === null) {
      throw new Error("The Shortcuts message heading must have a container.");
    }
    const message = title.parentElement;

    expect(message).toHaveTextContent(
      "Global keyboard shortcuts are not connected in this development preview.",
    );
    expect(message).toHaveTextContent(
      "No system-wide shortcuts are registered or monitored.",
    );
    expect(message).toHaveTextContent(
      "Shortcut configuration will become available after native integration.",
    );
    expect(message).not.toHaveTextContent(
      /global shortcuts? (?:are|is) registered/i,
    );
    expect(panel.querySelector("[aria-live]")).toBeNull();
    expect(within(panel).queryByRole("alert")).not.toBeInTheDocument();
    expect(
      message.querySelectorAll(
        "a, button, input, select, textarea, [tabindex]",
      ),
    ).toHaveLength(0);
  });

  it("switches between Shortcuts and Microphone status messages", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));

    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Keyboard shortcuts unavailable",
      }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole("tab", { name: "Microphone" }));
    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Microphone setup unavailable",
      }),
    ).toBeVisible();
    expect(
      screen.queryByRole("heading", {
        level: 3,
        name: "Keyboard shortcuts unavailable",
      }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));
    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Keyboard shortcuts unavailable",
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
