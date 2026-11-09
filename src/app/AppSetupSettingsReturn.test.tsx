import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";
import {
  SETUP_STEP_DEFINITIONS,
  type SetupStepView,
} from "./SetupStepIndicator";

const setupOrigins: Array<{
  view: SetupStepView;
  label: string;
  heading: string;
  settingsAction: string;
  settingsSection: string;
}> = [
  {
    view: "welcome",
    label: "Welcome",
    heading: "Welcome to Outflow",
    settingsAction: "Review permission readiness",
    settingsSection: "General",
  },
  {
    view: "privacy",
    label: "Privacy",
    heading: "Your privacy with Outflow",
    settingsAction: "Review permission readiness",
    settingsSection: "General",
  },
  {
    view: "permissions",
    label: "Permissions",
    heading: "Permissions for Outflow",
    settingsAction: "View permission details in Settings",
    settingsSection: "General",
  },
  {
    view: "shortcut-setup",
    label: "Shortcuts",
    heading: "Choose how dictation starts",
    settingsAction: "View shortcut details in Settings",
    settingsSection: "Shortcuts",
  },
];

function openSetup(view: SetupStepView) {
  fireEvent.click(screen.getByRole("button", { name: "Preview setup" }));
  if (view === "welcome") return;

  fireEvent.click(screen.getByRole("button", { name: "Review privacy" }));
  if (view === "privacy") return;

  fireEvent.click(
    screen.getByRole("button", { name: "Continue to permissions" }),
  );
  if (view === "permissions") return;

  fireEvent.click(
    screen.getByRole("button", { name: "Continue to shortcuts" }),
  );
}

function getPrimaryNavigationButton(name: "Home" | "Settings") {
  return within(
    screen.getByRole("navigation", { name: "Application navigation" }),
  ).getByRole("button", { name });
}

describe("return to setup from Settings", () => {
  it.each(setupOrigins)(
    "$label returns to the exact originating setup page and focuses its heading",
    ({ view, label, heading, settingsAction, settingsSection }) => {
      render(<App />);
      openSetup(view);

      fireEvent.click(screen.getByRole("button", { name: settingsAction }));

      expect(
        screen.getByRole("heading", { level: 1, name: "Settings" }),
      ).toHaveFocus();
      expect(
        screen.getByRole("tab", { name: settingsSection }),
      ).toHaveAttribute("aria-selected", "true");

      const returnButton = screen.getByRole("button", {
        name: `Return to setup: ${label}`,
      });
      expect(returnButton).toHaveAttribute("type", "button");
      expect(returnButton).toHaveAccessibleName(`Return to setup: ${label}`);
      fireEvent.click(returnButton);

      expect(
        screen.getByRole("heading", { level: 1, name: heading }),
      ).toHaveFocus();
      const stepIndex = SETUP_STEP_DEFINITIONS.findIndex(
        (step) => step.view === view,
      );
      expect(
        screen.getByText(
          `Setup preview · Step ${stepIndex + 1} of 6 · ${label}`,
          { exact: true },
        ),
      ).toBeVisible();
      expect(
        screen.queryByRole("button", { name: /^Return to setup:/ }),
      ).not.toBeInTheDocument();
    },
  );

  it("keeps the contextual return action while Settings tabs change", () => {
    render(<App />);
    openSetup("privacy");
    fireEvent.click(
      screen.getByRole("button", { name: "Review permission readiness" }),
    );

    fireEvent.click(screen.getByRole("tab", { name: "Microphone" }));
    expect(
      screen.getByRole("button", { name: "Return to setup: Privacy" }),
    ).toBeVisible();
    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));
    expect(
      screen.getByRole("button", { name: "Return to setup: Privacy" }),
    ).toBeVisible();

    fireEvent.click(
      screen.getByRole("button", { name: "Return to setup: Privacy" }),
    );
    expect(
      screen.getByRole("heading", {
        level: 1,
        name: "Your privacy with Outflow",
      }),
    ).toHaveFocus();
  });

  it.each(["Shortcuts", "Privacy"] as const)(
    "same-view primary Settings navigation from %s selects General",
    (section) => {
      render(<App />);
      fireEvent.click(getPrimaryNavigationButton("Settings"));
      fireEvent.click(screen.getByRole("tab", { name: section }));

      expect(screen.getByRole("tab", { name: section })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      fireEvent.click(getPrimaryNavigationButton("Settings"));

      expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(screen.getByRole("tab", { name: section })).toHaveAttribute(
        "aria-selected",
        "false",
      );
    },
  );

  it("focuses the Settings heading after same-view primary navigation", () => {
    render(<App />);
    fireEvent.click(getPrimaryNavigationButton("Settings"));
    fireEvent.click(screen.getByRole("tab", { name: "Shortcuts" }));

    const settingsButton = getPrimaryNavigationButton("Settings");
    settingsButton.focus();
    fireEvent.click(settingsButton);

    expect(
      screen.getByRole("heading", { level: 1, name: "Settings" }),
    ).toHaveFocus();
  });

  it.each([
    {
      view: "welcome" as const,
      label: "Welcome",
      action: "Review permission readiness",
      tab: "Shortcuts",
    },
    {
      view: "shortcut-setup" as const,
      label: "Shortcuts",
      action: "View shortcut details in Settings",
      tab: "General",
    },
  ])(
    "same-view primary Settings clears the $label return action and tab changes do not restore it",
    ({ view, label, action, tab }) => {
      render(<App />);
      openSetup(view);
      fireEvent.click(screen.getByRole("button", { name: action }));

      expect(
        screen.getByRole("button", { name: `Return to setup: ${label}` }),
      ).toBeVisible();
      fireEvent.click(getPrimaryNavigationButton("Settings"));

      expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(
        screen.queryByRole("button", { name: /^Return to setup:/ }),
      ).not.toBeInTheDocument();
      expect(
        screen.getByRole("heading", { level: 1, name: "Settings" }),
      ).toHaveFocus();

      fireEvent.click(screen.getByRole("tab", { name: tab }));
      expect(
        screen.queryByRole("button", { name: /^Return to setup:/ }),
      ).not.toBeInTheDocument();
    },
  );

  it("preserves Settings preferences and vocabulary preview values on same-view navigation", () => {
    render(<App />);
    fireEvent.click(getPrimaryNavigationButton("Settings"));
    fireEvent.click(screen.getByRole("switch", { name: "Launch at Login" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "re-entry preview term" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "Re-entry Preview Term" } },
    );
    fireEvent.click(screen.getByRole("button", { name: "Add to preview list" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "unsaved re-entry draft" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "Unsaved Re-entry Draft" } },
    );

    fireEvent.click(getPrimaryNavigationButton("Settings"));

    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("switch", { name: "Launch at Login" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByRole("radio", { name: "Verbatim" })).toBeChecked();
    expect(screen.getByText("1 of 500 entries")).toBeVisible();
    expect(screen.getByText("re-entry preview term")).toBeVisible();
    expect(screen.getByText("Re-entry Preview Term")).toBeVisible();
    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "unsaved re-entry draft",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("Unsaved Re-entry Draft");
  });

  it.each(setupOrigins)(
    "ordinary primary Settings navigation from $label has no return action and opens General",
    ({ view }) => {
      render(<App />);
      openSetup(view);

      fireEvent.click(getPrimaryNavigationButton("Settings"));

      expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(
        screen.queryByRole("button", { name: /^Return to setup:/ }),
      ).not.toBeInTheDocument();
    },
  );

  it.each(["Settings", "Open Settings"] as const)(
    "does not show a return action for ordinary Home to %s navigation",
    (action) => {
      render(<App />);
      fireEvent.click(screen.getByRole("button", { name: action }));

      expect(
        screen.getByRole("heading", { level: 1, name: "Settings" }),
      ).toHaveFocus();
      expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(
        screen.queryByRole("button", { name: /^Return to setup:/ }),
      ).not.toBeInTheDocument();
    },
  );

  it("clears the return origin when leaving Settings for Home", () => {
    render(<App />);
    openSetup("permissions");
    fireEvent.click(
      screen.getByRole("button", {
        name: "View permission details in Settings",
      }),
    );

    fireEvent.click(getPrimaryNavigationButton("Home"));
    expect(screen.getByRole("heading", { level: 1, name: "Home" })).toHaveFocus();
    fireEvent.click(getPrimaryNavigationButton("Settings"));

    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(
      screen.queryByRole("button", { name: /^Return to setup:/ }),
    ).not.toBeInTheDocument();
  });

  it("starts without return context after a fresh App mount", () => {
    const firstMount = render(<App />);
    openSetup("welcome");
    fireEvent.click(
      screen.getByRole("button", { name: "Review permission readiness" }),
    );
    expect(
      screen.getByRole("button", { name: "Return to setup: Welcome" }),
    ).toBeVisible();

    firstMount.unmount();
    render(<App />);
    fireEvent.click(getPrimaryNavigationButton("Settings"));

    expect(
      screen.queryByRole("button", { name: /^Return to setup:/ }),
    ).not.toBeInTheDocument();
  });

  it("discards unmounted setup capability fixtures and keeps Settings fixtures separate", () => {
    render(<App />);
    openSetup("permissions");
    const setupFixture = screen.getByRole("combobox", {
      name: "Developer scenarios (simulated fixtures)",
    });
    fireEvent.change(setupFixture, { target: { value: "hypothetical-available" } });
    fireEvent.click(
      screen.getByRole("button", {
        name: "View permission details in Settings",
      }),
    );

    const settingsFixture = screen.getByRole("combobox", {
      name: "Developer scenarios (simulated fixtures)",
    });
    expect(settingsFixture).toHaveValue("default");
    fireEvent.change(settingsFixture, {
      target: { value: "insertion-denied" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Return to setup: Permissions" }),
    );

    expect(
      screen.getByRole("combobox", {
        name: "Developer scenarios (simulated fixtures)",
      }),
    ).toHaveValue("default");
  });

  it("keeps shortcut fixtures independent and resets the unmounted setup fixture", () => {
    render(<App />);
    openSetup("shortcut-setup");
    fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
      target: { value: "conflict" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "View shortcut details in Settings" }),
    );

    const settingsFixture = screen.getByRole("combobox", { name: "Scenario" });
    expect(settingsFixture).toHaveValue("default");
    fireEvent.change(settingsFixture, {
      target: { value: "permission-unavailable" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Return to setup: Shortcuts" }),
    );

    expect(screen.getByRole("combobox", { name: "Scenario" })).toHaveValue(
      "default",
    );
  });

  it("preserves Settings preference and vocabulary preview state across return navigation", () => {
    render(<App />);
    openSetup("privacy");
    fireEvent.click(
      screen.getByRole("button", { name: "Review permission readiness" }),
    );
    fireEvent.click(screen.getByRole("switch", { name: "Launch at Login" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    fireEvent.click(screen.getByRole("radio", { name: "Verbatim" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "return preview term" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "Return Preview Term" } },
    );

    expect(
      screen.getByRole("button", { name: "Return to setup: Privacy" }),
    ).toBeVisible();
    fireEvent.click(
      screen.getByRole("button", { name: "Return to setup: Privacy" }),
    );
    fireEvent.click(getPrimaryNavigationButton("Settings"));
    expect(screen.getByRole("switch", { name: "Launch at Login" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByRole("radio", { name: "Verbatim" })).toBeChecked();
    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "return preview term",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("Return Preview Term");
  });
});
