import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PageContainer } from "./PageContainer";

describe("PageContainer", () => {
  it("renders its children inside the shared layout wrapper", () => {
    render(
      <PageContainer>
        <p>Page content</p>
      </PageContainer>,
    );

    expect(screen.getByText("Page content").parentElement).toHaveClass(
      "page-container",
    );
  });
});
