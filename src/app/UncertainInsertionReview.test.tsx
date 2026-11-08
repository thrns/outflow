import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { UncertainInsertionReview } from "./UncertainInsertionReview";

describe("UncertainInsertionReview", () => {
  it("sends one review acknowledgment even if the action is activated repeatedly", () => {
    const onReviewed = vi.fn();
    render(<UncertainInsertionReview onReviewed={onReviewed} />);

    const reviewButton = screen.getByRole("button", {
      name: "I have reviewed the destination",
    });
    fireEvent.click(reviewButton);
    fireEvent.click(reviewButton);

    expect(onReviewed).toHaveBeenCalledTimes(1);
  });
});
