import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import VocabularyEntryForm, {
  type VocabularyDraft,
  validateVocabularyValue,
} from "./VocabularyEntryForm";

const emptyDraft: VocabularyDraft = { spoken: "", written: "" };

function ControlledVocabularyEntryForm({
  initialValue = emptyDraft,
  onClearDraft = vi.fn(),
}: {
  initialValue?: VocabularyDraft;
  onClearDraft?: () => void;
}) {
  const [value, setValue] = useState<VocabularyDraft>(initialValue);

  function clearDraft() {
    setValue(emptyDraft);
    onClearDraft();
  }

  return (
    <VocabularyEntryForm
      onChange={setValue}
      onClearDraft={clearDraft}
      value={value}
    />
  );
}

describe("VocabularyEntryForm", () => {
  it("renders both fields with accessible labels", () => {
    render(<ControlledVocabularyEntryForm />);

    const spoken = screen.getByRole("textbox", { name: "Spoken phrase" });
    expect(spoken).toBeVisible();
    expect(spoken).toHaveAccessibleDescription(/expect to dictate/);
    const written = screen.getByRole("textbox", {
      name: "Written replacement",
    });
    expect(written).toBeVisible();
    expect(written).toHaveAccessibleDescription(
      /eventually appear in finalized output/,
    );
  });

  it("starts both fields empty", () => {
    render(<ControlledVocabularyEntryForm />);

    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows controlled values as the user types", () => {
    render(<ControlledVocabularyEntryForm />);

    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "café" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "North Parkade" } },
    );

    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "café",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("North Parkade");
  });

  it("preserves user-entered spaces and capitalization", () => {
    render(<ControlledVocabularyEntryForm />);

    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "  North  Parkade " },
    });

    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "  North  Parkade ",
    );
  });

  it("accepts valid English and non-ASCII phrases without changing them", () => {
    expect(validateVocabularyValue("North Parkade")).toBeNull();
    expect(validateVocabularyValue("café")).toBeNull();
    expect(validateVocabularyValue("North  Parkade ")).toBeNull();
  });

  it("rejects empty and whitespace-only values", () => {
    expect(validateVocabularyValue("")).toBe("required");
    expect(validateVocabularyValue("   ")).toBe("whitespace");
    expect(validateVocabularyValue("\t\n")).toBe("whitespace");
  });

  it("accepts exactly 128 ASCII scalar values", () => {
    expect(validateVocabularyValue("a".repeat(128))).toBeNull();
  });

  it("rejects 129 ASCII scalar values", () => {
    expect(validateVocabularyValue("a".repeat(129))).toBe("too-long");
  });

  it("accepts exactly 128 emoji Unicode scalar values", () => {
    expect(validateVocabularyValue("😀".repeat(128))).toBeNull();
  });

  it("rejects 129 emoji Unicode scalar values", () => {
    expect(validateVocabularyValue("😀".repeat(129))).toBe("too-long");
  });

  it("counts combining characters as separate Unicode scalar values", () => {
    expect(validateVocabularyValue("e\u0301")).toBeNull();
    expect(validateVocabularyValue("e\u0301".repeat(64))).toBeNull();
    expect(validateVocabularyValue("e\u0301".repeat(65))).toBe("too-long");
  });

  it("rejects an unpaired UTF-16 surrogate", () => {
    expect(validateVocabularyValue("\uD83D")).toBe("invalid-unicode");
  });

  it("shows a scalar-length error after an overlong value is blurred", () => {
    render(<ControlledVocabularyEntryForm />);
    const spoken = screen.getByRole("textbox", { name: "Spoken phrase" });

    fireEvent.change(spoken, { target: { value: "😀".repeat(129) } });
    fireEvent.blur(spoken);

    expect(spoken).toHaveAttribute("aria-invalid", "true");
    expect(
      screen.getByText("Use 128 Unicode scalar values or fewer."),
    ).toBeVisible();
  });

  it("shows an invalid-Unicode error for an unpaired surrogate", () => {
    render(<ControlledVocabularyEntryForm />);
    const spoken = screen.getByRole("textbox", { name: "Spoken phrase" });

    fireEvent.change(spoken, { target: { value: "\uD83D" } });
    fireEvent.blur(spoken);

    expect(spoken).toHaveAttribute("aria-invalid", "true");
    expect(
      screen.getByText("This value contains invalid Unicode text."),
    ).toBeVisible();
  });

  it("associates inline errors with invalid fields accessibly", () => {
    render(<ControlledVocabularyEntryForm />);
    const spoken = screen.getByRole("textbox", { name: "Spoken phrase" });

    fireEvent.blur(spoken);

    const error = screen.getByRole("alert");
    expect(error).toHaveTextContent("Enter a spoken phrase.");
    expect(spoken).toHaveAttribute("aria-invalid", "true");
    expect(spoken.getAttribute("aria-describedby")).toContain(error.id);
    expect(spoken).toHaveAccessibleDescription(/word or phrase you expect/);
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).not.toHaveAttribute("aria-invalid", "true");
  });

  it("clears a field error as soon as the user corrects the value", () => {
    render(<ControlledVocabularyEntryForm />);
    const spoken = screen.getByRole("textbox", { name: "Spoken phrase" });

    fireEvent.blur(spoken);
    expect(spoken).toHaveAttribute("aria-invalid", "true");

    fireEvent.change(spoken, { target: { value: "north parkade" } });

    expect(spoken).not.toHaveAttribute("aria-invalid", "true");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("does not introduce Save, Delete, or Import actions", () => {
    render(<ControlledVocabularyEntryForm />);

    expect(screen.queryByRole("button", { name: /save|delete|import/i })).toBeNull();
  });

  it("shows Clear draft as a native button with an accessible name", () => {
    render(<ControlledVocabularyEntryForm />);

    const clearButton = screen.getByRole("button", { name: "Clear draft" });
    expect(clearButton).toHaveAttribute("type", "button");
    expect(clearButton).toBeVisible();
  });

  it("disables Clear draft when both fields are empty", () => {
    render(<ControlledVocabularyEntryForm />);

    expect(screen.getByRole("button", { name: "Clear draft" })).toBeDisabled();
  });

  it("enables Clear draft when Spoken phrase contains text", () => {
    render(<ControlledVocabularyEntryForm />);

    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "north parkade" },
    });

    expect(screen.getByRole("button", { name: "Clear draft" })).toBeEnabled();
  });

  it("enables Clear draft when Written replacement contains text", () => {
    render(<ControlledVocabularyEntryForm />);

    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "North Parkade" } },
    );

    expect(screen.getByRole("button", { name: "Clear draft" })).toBeEnabled();
  });

  it("clears the controlled draft and invokes its callback on click", () => {
    const onClearDraft = vi.fn();
    render(
      <ControlledVocabularyEntryForm
        initialValue={{ spoken: "north parkade", written: "North Parkade" }}
        onClearDraft={onClearDraft}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Clear draft" }));

    expect(onClearDraft).toHaveBeenCalledOnce();
    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue(
      "",
    );
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("");
  });

  it("handles a keyboard-generated native button activation", () => {
    const onClearDraft = vi.fn();
    render(
      <ControlledVocabularyEntryForm
        initialValue={{ spoken: "north parkade", written: "" }}
        onClearDraft={onClearDraft}
      />,
    );
    const clearButton = screen.getByRole("button", { name: "Clear draft" });
    clearButton.focus();

    // Browsers dispatch a click with detail 0 for native Enter/Space activation.
    fireEvent.click(clearButton, { detail: 0 });

    expect(onClearDraft).toHaveBeenCalledOnce();
  });

  it("enables Clear draft for whitespace-only and invalid input", () => {
    render(<ControlledVocabularyEntryForm />);
    const spoken = screen.getByRole("textbox", { name: "Spoken phrase" });
    const clearButton = screen.getByRole("button", { name: "Clear draft" });

    fireEvent.change(spoken, { target: { value: "   " } });
    expect(clearButton).toBeEnabled();

    fireEvent.change(spoken, { target: { value: "😀".repeat(129) } });
    fireEvent.blur(spoken);
    expect(spoken).toHaveAttribute("aria-invalid", "true");
    expect(clearButton).toBeEnabled();
  });

  it("clears invalid values and their displayed validation errors", () => {
    render(
      <ControlledVocabularyEntryForm
        initialValue={{
          spoken: "😀".repeat(129),
          written: "",
        }}
      />,
    );
    const spoken = screen.getByRole("textbox", { name: "Spoken phrase" });
    const written = screen.getByRole("textbox", {
      name: "Written replacement",
    });
    fireEvent.blur(spoken);
    fireEvent.blur(written);
    expect(screen.getAllByRole("alert")).toHaveLength(2);

    fireEvent.click(screen.getByRole("button", { name: "Clear draft" }));

    expect(spoken).toHaveValue("");
    expect(written).toHaveValue("");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(spoken).not.toHaveAttribute("aria-invalid", "true");
    expect(written).not.toHaveAttribute("aria-invalid", "true");
  });

  it("returns validation to untouched presentation and validates again after blur", () => {
    render(
      <ControlledVocabularyEntryForm
        initialValue={{ spoken: "north parkade", written: "North Parkade" }}
      />,
    );
    const spoken = screen.getByRole("textbox", { name: "Spoken phrase" });
    const written = screen.getByRole("textbox", {
      name: "Written replacement",
    });
    fireEvent.blur(spoken);
    fireEvent.blur(written);

    fireEvent.click(screen.getByRole("button", { name: "Clear draft" }));

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(spoken).not.toHaveAttribute("aria-invalid", "true");
    fireEvent.blur(spoken);
    expect(screen.getByRole("alert")).toHaveTextContent("Enter a spoken phrase.");
  });

  it("moves focus to Spoken phrase after clearing", () => {
    render(
      <ControlledVocabularyEntryForm
        initialValue={{ spoken: "north parkade", written: "North Parkade" }}
      />,
    );
    const clearButton = screen.getByRole("button", { name: "Clear draft" });
    clearButton.focus();

    fireEvent.click(clearButton);

    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveFocus();
    expect(clearButton).toBeDisabled();
  });
});
