import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

function openVocabularyPreview() {
  const view = render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "Settings" }));
  fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
  return view;
}

function addEntry(spoken: string, written: string) {
  fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
    target: { value: spoken },
  });
  fireEvent.change(
    screen.getByRole("textbox", { name: "Written replacement" }),
    { target: { value: written } },
  );
  fireEvent.click(screen.getByRole("button", { name: "Add to preview list" }));
}

describe("in-memory vocabulary preview list", () => {
  it("shows an accessible empty state on initial render", () => {
    openVocabularyPreview();

    expect(
      screen.getByRole("heading", {
        level: 4,
        name: "Vocabulary preview list",
      }),
    ).toBeVisible();
    expect(screen.getByRole("status")).toHaveTextContent(
      "No vocabulary entries yet in this development preview. Add a valid pair above to see it here.",
    );
    expect(screen.getByText("0 of 500 entries")).toBeVisible();
  });

  it("adds one valid pair and clears the draft", () => {
    openVocabularyPreview();
    addEntry("north parkade", "North Parkade");

    expect(
      screen.getByRole("list", { name: "Vocabulary entries in preview" }),
    ).toBeVisible();
    expect(screen.getByText("north parkade")).toBeVisible();
    expect(screen.getByText("North Parkade")).toBeVisible();
    expect(screen.getByText("1 of 500 entries")).toBeVisible();
    expect(screen.getByRole("textbox", { name: "Spoken phrase" })).toHaveValue("");
    expect(
      screen.getByRole("textbox", { name: "Written replacement" }),
    ).toHaveValue("");
  });

  it("adds several valid pairs in order", () => {
    openVocabularyPreview();
    addEntry("north parkade", "North Parkade");
    addEntry("rust", "Rust");
    addEntry("outflow", "Outflow");

    const list = screen.getByRole("list", {
      name: "Vocabulary entries in preview",
    });
    expect(list.querySelectorAll("li")).toHaveLength(3);
    expect(screen.getByText("3 of 500 entries")).toBeVisible();
    expect(withinText(list)).toEqual([
      "north parkade",
      "North Parkade",
      "rust",
      "Rust",
      "outflow",
      "Outflow",
    ]);
  });

  it("rejects empty and invalid values without adding an entry", () => {
    openVocabularyPreview();

    fireEvent.click(screen.getByRole("button", { name: "Add to preview list" }));
    expect(screen.getAllByRole("alert")).toHaveLength(2);
    expect(
      screen.queryByRole("list", { name: "Vocabulary entries in preview" }),
    ).not.toBeInTheDocument();

    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "valid phrase" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "x".repeat(129) } },
    );
    fireEvent.click(screen.getByRole("button", { name: "Add to preview list" }));

    expect(screen.getAllByRole("alert")).toHaveLength(1);
    expect(
      screen.getByText("Use 128 Unicode scalar values or fewer."),
    ).toBeVisible();
    expect(screen.getByText("0 of 500 entries")).toBeVisible();
  });

  it("removes an entry and returns to the empty state", () => {
    openVocabularyPreview();
    addEntry("north parkade", "North Parkade");

    const removeButton = screen.getByRole("button", {
      name: "Remove vocabulary entry 1: north parkade",
    });
    expect(removeButton).toHaveTextContent("Remove");
    fireEvent.click(removeButton);

    expect(screen.queryByText("north parkade")).not.toBeInTheDocument();
    expect(screen.getByText("0 of 500 entries")).toBeVisible();
    expect(screen.getByRole("status")).toHaveTextContent("No vocabulary entries yet");
  });

  it("preserves Unicode text exactly as entered", () => {
    openVocabularyPreview();
    addEntry("café 東京 😀", "Café 東京 ✨");

    expect(screen.getByText("café 東京 😀")).toBeVisible();
    expect(screen.getByText("Café 東京 ✨")).toBeVisible();
  });

  it("enforces the 500-entry limit and allows another add after removal", () => {
    openVocabularyPreview();
    const spokenInput = screen.getByRole("textbox", { name: "Spoken phrase" });
    const writtenInput = screen.getByRole("textbox", {
      name: "Written replacement",
    });
    const addButton = screen.getByRole("button", {
      name: "Add to preview list",
    });

    for (let index = 0; index < 500; index += 1) {
      fireEvent.change(spokenInput, { target: { value: `spoken ${index}` } });
      fireEvent.change(writtenInput, { target: { value: `written ${index}` } });
      fireEvent.click(addButton);
    }

    expect(screen.getByText("500 of 500 entries")).toBeVisible();
    expect(addButton).toBeDisabled();
    const limitMessage = screen.getByRole("status");
    expect(limitMessage).toHaveTextContent(
      "The preview list is full at 500 entries. Remove an entry before adding another.",
    );
    expect(addButton).toHaveAttribute("aria-describedby", limitMessage.id);

    fireEvent.click(
      screen.getByRole("button", {
        name: "Remove vocabulary entry 1: spoken 0",
      }),
    );
    expect(screen.getByText("499 of 500 entries")).toBeVisible();
    expect(addButton).toBeEnabled();

    fireEvent.change(spokenInput, { target: { value: "spoken after remove" } });
    fireEvent.change(writtenInput, { target: { value: "written after remove" } });
    fireEvent.click(addButton);
    expect(screen.getByText("500 of 500 entries")).toBeVisible();
  }, 30000);

  it("preserves entries across Settings sections and Home navigation", () => {
    openVocabularyPreview();
    addEntry("north parkade", "North Parkade");

    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    expect(screen.getByText("north parkade")).toBeVisible();
    expect(screen.getByText("North Parkade")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Home" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));
    expect(screen.getByText("north parkade")).toBeVisible();
    expect(screen.getByText("North Parkade")).toBeVisible();
  });

  it("clears entries after a fresh application mount", () => {
    const view = openVocabularyPreview();
    addEntry("north parkade", "North Parkade");

    view.unmount();

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.click(screen.getByRole("tab", { name: "Text & Vocabulary" }));

    expect(screen.getByText("0 of 500 entries")).toBeVisible();
    expect(screen.queryByText("north parkade")).not.toBeInTheDocument();
  });

  it("supports keyboard activation for Add and Remove buttons", () => {
    openVocabularyPreview();
    fireEvent.change(screen.getByRole("textbox", { name: "Spoken phrase" }), {
      target: { value: "keyboard phrase" },
    });
    fireEvent.change(
      screen.getByRole("textbox", { name: "Written replacement" }),
      { target: { value: "Keyboard Phrase" } },
    );

    const addButton = screen.getByRole("button", { name: "Add to preview list" });
    expect(addButton).toHaveAttribute("type", "submit");
    addButton.focus();
    fireEvent.click(addButton, { detail: 0 });
    expect(screen.getByText("keyboard phrase")).toBeVisible();

    const removeButton = screen.getByRole("button", {
      name: "Remove vocabulary entry 1: keyboard phrase",
    });
    expect(removeButton).toHaveAttribute("type", "button");
    removeButton.focus();
    fireEvent.click(removeButton, { detail: 0 });
    expect(screen.getByText("0 of 500 entries")).toBeVisible();
  });
});

function withinText(container: HTMLElement): string[] {
  return Array.from(
    container.querySelectorAll(".vocabulary-preview-entry-value"),
    (element) => element.textContent ?? "",
  );
}
