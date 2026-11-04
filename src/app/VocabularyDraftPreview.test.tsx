import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { VocabularyDraft } from "./VocabularyEntryForm";
import VocabularyDraftPreview from "./VocabularyDraftPreview";

const emptyDraft: VocabularyDraft = { spoken: "", written: "" };

describe("VocabularyDraftPreview", () => {
  it("shows a neutral empty-preview message when both values are empty", () => {
    render(<VocabularyDraftPreview value={emptyDraft} />);

    expect(
      screen.getByText(
        "Enter a spoken phrase and written replacement to see an illustrative example.",
      ),
    ).toBeVisible();
    expect(screen.queryByText(/I mentioned/)).not.toBeInTheDocument();
  });

  it("asks for both fields when only the spoken phrase is filled", () => {
    render(
      <VocabularyDraftPreview
        value={{ spoken: "north parkade", written: "" }}
      />,
    );

    expect(
      screen.getByText(
        "Enter both a spoken phrase and written replacement to see the illustrative example.",
      ),
    ).toBeVisible();
    expect(screen.queryByText(/I mentioned/)).not.toBeInTheDocument();
  });

  it("asks for both fields when only the written replacement is filled", () => {
    render(
      <VocabularyDraftPreview
        value={{ spoken: "", written: "North Parkade" }}
      />,
    );

    expect(
      screen.getByText(
        "Enter both a spoken phrase and written replacement to see the illustrative example.",
      ),
    ).toBeVisible();
    expect(screen.queryByText(/I mentioned/)).not.toBeInTheDocument();
  });

  it("constructs the two synthetic sentences from valid English values", () => {
    render(
      <VocabularyDraftPreview
        value={{ spoken: "north parkade", written: "North Parkade" }}
      />,
    );

    expect(
      screen.getByText("I mentioned “north parkade” today."),
    ).toBeVisible();
    expect(
      screen.getByText("I mentioned “North Parkade” today."),
    ).toBeVisible();
    expect(
      screen.getByText("Synthetic spoken sentence"),
    ).toBeVisible();
    expect(
      screen.getByText("Intended written sentence"),
    ).toBeVisible();
  });

  it("updates the spoken sentence when its prop changes", () => {
    const { rerender } = render(
      <VocabularyDraftPreview
        value={{ spoken: "north parkade", written: "North Parkade" }}
      />,
    );

    rerender(
      <VocabularyDraftPreview
        value={{ spoken: "south station", written: "North Parkade" }}
      />,
    );

    expect(
      screen.getByText("I mentioned “south station” today."),
    ).toBeVisible();
    expect(screen.queryByText(/“north parkade”/)).not.toBeInTheDocument();
  });

  it("updates the written sentence when its prop changes", () => {
    const { rerender } = render(
      <VocabularyDraftPreview
        value={{ spoken: "north parkade", written: "North Parkade" }}
      />,
    );

    rerender(
      <VocabularyDraftPreview
        value={{ spoken: "north parkade", written: "North Parkade Park" }}
      />,
    );

    expect(
      screen.getByText("I mentioned “North Parkade Park” today."),
    ).toBeVisible();
    expect(screen.queryByText(/“North Parkade”/)).not.toBeInTheDocument();
  });

  it("does not show a completed example for whitespace-only values", () => {
    render(
      <VocabularyDraftPreview
        value={{ spoken: " \t ", written: "North Parkade" }}
      />,
    );

    expect(
      screen.getByText("Correct both values to see the illustrative example."),
    ).toBeVisible();
    expect(screen.queryByText(/I mentioned/)).not.toBeInTheDocument();
  });

  it("does not show a completed example for a value over 128 Unicode scalars", () => {
    render(
      <VocabularyDraftPreview
        value={{ spoken: "😀".repeat(129), written: "North Parkade" }}
      />,
    );

    expect(
      screen.getByText("Correct both values to see the illustrative example."),
    ).toBeVisible();
    expect(screen.queryByText(/I mentioned/)).not.toBeInTheDocument();
  });

  it("does not show a completed example for an unpaired surrogate", () => {
    render(
      <VocabularyDraftPreview
        value={{ spoken: "\uD83D", written: "North Parkade" }}
      />,
    );

    expect(
      screen.getByText("Correct both values to see the illustrative example."),
    ).toBeVisible();
    expect(screen.queryByText(/I mentioned/)).not.toBeInTheDocument();
  });

  it("renders valid emoji and combining characters as text", () => {
    const spoken = "cafe\u0301 😀";
    const written = "Café 🌲";
    render(<VocabularyDraftPreview value={{ spoken, written }} />);

    expect(
      screen.getByText(`I mentioned “${spoken}” today.`),
    ).toBeVisible();
    expect(
      screen.getByText(`I mentioned “${written}” today.`),
    ).toBeVisible();
  });

  it("renders HTML-like values literally instead of creating markup", () => {
    const spoken = "<img src=x onerror=alert(1)>";
    const { container } = render(
      <VocabularyDraftPreview
        value={{ spoken, written: "<b>literal text</b>" }}
      />,
    );

    expect(
      screen.getByText(`I mentioned “${spoken}” today.`),
    ).toBeVisible();
    expect(
      screen.getByText("I mentioned “<b>literal text</b>” today."),
    ).toBeVisible();
    expect(container.querySelector("img, b")).toBeNull();
  });

  it("restores the synthetic example as soon as invalid input is corrected", () => {
    const { rerender } = render(
      <VocabularyDraftPreview
        value={{ spoken: "   ", written: "North Parkade" }}
      />,
    );

    expect(screen.queryByText(/I mentioned/)).not.toBeInTheDocument();
    rerender(
      <VocabularyDraftPreview
        value={{ spoken: "north parkade", written: "North Parkade" }}
      />,
    );

    expect(
      screen.getByText("I mentioned “north parkade” today."),
    ).toBeVisible();
  });

  it("does not include Save or Apply controls", () => {
    render(
      <VocabularyDraftPreview
        value={{ spoken: "north parkade", written: "North Parkade" }}
      />,
    );

    expect(
      screen.queryByRole("button", { name: /save|apply/i }),
    ).not.toBeInTheDocument();
  });

  it("states that the example is synthetic and the list is not persisted or applied", () => {
    render(
      <VocabularyDraftPreview
        value={{ spoken: "north parkade", written: "North Parkade" }}
      />,
    );

    expect(
      screen.getByText(
        "This is a synthetic illustration of your draft. Entries in the development preview list are not persisted or applied to actual speech.",
      ),
    ).toBeVisible();
  });
});
