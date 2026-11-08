import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";
import { SETUP_STEP_DEFINITIONS } from "./SetupStepIndicator";

const expectedSteps = [
  {
    view: "welcome",
    label: "Welcome",
    heading: "Welcome to Outflow",
    next: "Review privacy",
    previous: "Back to Home",
  },
  {
    view: "privacy",
    label: "Privacy",
    heading: "Your privacy with Outflow",
    next: "Continue to permissions",
    previous: "Back to Welcome",
  },
  {
    view: "permissions",
    label: "Permissions",
    heading: "Permissions for Outflow",
    next: "Continue to shortcuts",
    previous: "Back to Privacy",
  },
  {
    view: "shortcut-setup",
    label: "Shortcuts",
    heading: "Choose how dictation starts",
    next: "Continue to Try Dictation",
    previous: "Back to Permissions",
  },
  {
    view: "try-dictation",
    label: "Try Dictation",
    heading: "Try Dictation — preview",
    next: "Continue to tray overview",
    previous: "Back to Shortcuts",
  },
  {
    view: "tray-status",
    label: "Tray and App Status",
    heading: "Tray and app status — preview",
    next: undefined,
    previous: "Back to Try Dictation",
  },
] as const;

function indicatorText(index: number) {
  const { label } = expectedSteps[index];
  return `Setup preview · Step ${index + 1} of 6 · ${label}`;
}

describe("setup step indicator", () => {
  it("defines exactly six unique setup views and labels in walkthrough order", () => {
    expect(SETUP_STEP_DEFINITIONS).toEqual(
      expectedSteps.map(({ view, label }) => ({ view, label })),
    );
    expect(new Set(SETUP_STEP_DEFINITIONS.map(({ view }) => view)).size).toBe(6);
  });

  it("renders the right read-only position and keeps heading focus through every direction", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Preview setup" }));

    expectedSteps.forEach((step, index) => {
      const text = indicatorText(index);
      const indicator = screen.getByText(text, { exact: true });

      expect(screen.getAllByText(text, { exact: true })).toHaveLength(1);
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(
        screen.getByRole("heading", { level: 1, name: step.heading }),
      ).toHaveFocus();
      expect(indicator.tagName).toBe("P");
      expect(indicator).not.toHaveAttribute("role", "progressbar");
      expect(indicator).not.toHaveAttribute("aria-live");
      expect(indicator).not.toHaveAttribute("tabindex");
      expect(indicator).not.toHaveTextContent(/complete|completed|success|✓|✔/i);
      expect(
        indicator.querySelectorAll(
          "button, a[href], input, select, textarea, [contenteditable='true']",
        ),
      ).toHaveLength(0);
      expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();

      if (step.next) {
        fireEvent.click(screen.getByRole("button", { name: step.next }));
      }
    });

    for (let index = expectedSteps.length - 1; index >= 0; index -= 1) {
      const step = expectedSteps[index];
      fireEvent.click(screen.getByRole("button", { name: step.previous }));
      const previousIndex = index - 1;

      if (previousIndex >= 0) {
        const previousStep = expectedSteps[previousIndex];
        expect(
          screen.getByText(indicatorText(previousIndex), { exact: true }),
        ).toBeVisible();
        expect(
          screen.getByRole("heading", {
            level: 1,
            name: previousStep.heading,
          }),
        ).toHaveFocus();
      } else {
        expect(screen.getByRole("heading", { level: 1, name: "Home" })).toHaveFocus();
        expect(screen.queryByText(/^Setup preview · Step/)).not.toBeInTheDocument();
      }
    }
  });

  it("does not appear on Home or Settings", () => {
    render(<App />);

    expect(screen.queryByText(/^Setup preview · Step/)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("heading", { level: 1, name: "Settings" })).toHaveFocus();
    expect(screen.queryByText(/^Setup preview · Step/)).not.toBeInTheDocument();
  });
});
