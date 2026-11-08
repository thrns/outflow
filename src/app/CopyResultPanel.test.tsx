import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import {
  CopyResultPanel,
  type CopyAcknowledgment,
  type CopyRequest,
  type CopyResult,
} from "./CopyResultPanel";

const syntheticResult: CopyResult = {
  text: "Synthetic preview result: Outflow keeps this sample in memory.",
  reason: "changed-target",
};

type StatefulCopyPanelProps = {
  onCopyRequest: (request: CopyRequest) => void;
  acknowledgment?: CopyAcknowledgment | null;
};

function StatefulCopyPanel({
  onCopyRequest,
  acknowledgment = null,
}: StatefulCopyPanelProps) {
  const [result, setResult] = useState<CopyResult | null>(syntheticResult);

  if (!result) return <p role="status">Result cleared</p>;

  return (
    <CopyResultPanel
      copyAcknowledgment={acknowledgment}
      onCopyRequest={onCopyRequest}
      onDismiss={() => setResult(null)}
      onResultCopied={() => setResult(null)}
      requestScope="test-session"
      result={result}
    />
  );
}

function CopyResultFixture({
  onCopyRequest,
  result = syntheticResult,
}: {
  onCopyRequest?: (request: CopyRequest) => void;
  result?: CopyResult;
}) {
  return (
    <CopyResultPanel
      onCopyRequest={onCopyRequest}
      onDismiss={() => undefined}
      onResultCopied={() => undefined}
      requestScope="test-session"
      result={result}
    />
  );
}

describe("CopyResultPanel", () => {
  it("shows selectable final text, fallback reason, clipboard warning, and actions", () => {
    render(<CopyResultFixture />);

    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveValue(syntheticResult.text);
    expect(
      screen.getByText(
        "The destination changed while dictation was in progress. No text was inserted.",
      ),
    ).toBeVisible();
    expect(
      screen.getByText(
        "Your operating system's clipboard managers or synchronization services may retain copied text.",
      ),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "Copy" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Select All" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Dismiss" })).toBeEnabled();
  });

  it("keeps Copy unavailable when no clipboard adapter callback is provided", () => {
    render(<CopyResultFixture />);

    expect(screen.getByRole("button", { name: "Copy" })).toBeDisabled();
    expect(
      screen.getByText(
        "Copy is unavailable because this browser preview has no clipboard adapter connected.",
      ),
    ).toBeVisible();
  });

  it("calls an explicitly provided Copy callback once for an accepted action", () => {
    const onCopyRequest = vi.fn();
    render(<CopyResultFixture onCopyRequest={onCopyRequest} />);

    const copyButton = screen.getByRole("button", { name: "Copy" });
    fireEvent.click(copyButton);
    fireEvent.click(copyButton);

    expect(onCopyRequest).toHaveBeenCalledTimes(1);
    expect(onCopyRequest).toHaveBeenCalledWith({
      requestId: "test-session:1",
    });
    expect(copyButton).toBeDisabled();
  });

  it("retains the result when a matching Copy acknowledgment fails", () => {
    const onCopyRequest = vi.fn();
    const { rerender } = render(
      <StatefulCopyPanel onCopyRequest={onCopyRequest} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));

    rerender(
      <StatefulCopyPanel
        acknowledgment={{
          requestId: "test-session:1",
          outcome: "failed",
        }}
        onCopyRequest={onCopyRequest}
      />,
    );

    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveValue(syntheticResult.text);
    expect(
      screen.getByText(
        "Copy failed. Your result is still available in this panel.",
      ),
    ).toBeVisible();
  });

  it("clears the owning result after a matching successful Copy acknowledgment", () => {
    const onCopyRequest = vi.fn();
    const { rerender } = render(
      <StatefulCopyPanel onCopyRequest={onCopyRequest} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));

    rerender(
      <StatefulCopyPanel
        acknowledgment={{
          requestId: "test-session:1",
          outcome: "succeeded",
        }}
        onCopyRequest={onCopyRequest}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("Result cleared");
    expect(
      screen.queryByLabelText("Finalized text (kept in memory until dismissed)"),
    ).not.toBeInTheDocument();
  });

  it("ignores acknowledgments that do not match the pending request", () => {
    const onCopyRequest = vi.fn();
    const { rerender } = render(
      <StatefulCopyPanel onCopyRequest={onCopyRequest} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));

    rerender(
      <StatefulCopyPanel
        acknowledgment={{
          requestId: "another-session:1",
          outcome: "succeeded",
        }}
        onCopyRequest={onCopyRequest}
      />,
    );

    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveValue(syntheticResult.text);
    expect(screen.getByRole("button", { name: "Copy pending…" })).toBeDisabled();
  });

  it("clears the result when Dismiss is activated", () => {
    render(<StatefulCopyPanel onCopyRequest={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));

    expect(screen.getByRole("status")).toHaveTextContent("Result cleared");
  });

  it("selects the exact result text without making a Copy request", () => {
    const onCopyRequest = vi.fn();
    render(<CopyResultFixture onCopyRequest={onCopyRequest} />);

    const field = screen.getByLabelText(
      "Finalized text (kept in memory until dismissed)",
    ) as HTMLTextAreaElement;
    fireEvent.click(screen.getByRole("button", { name: "Select All" }));

    expect(field.selectionStart).toBe(0);
    expect(field.selectionEnd).toBe(syntheticResult.text.length);
    expect(onCopyRequest).not.toHaveBeenCalled();
    expect(
      screen.getByText("The full result is selected. Nothing was copied."),
    ).toBeVisible();
  });

  it("requires explicit acknowledgment before Copy for a protected destination", () => {
    render(
      <CopyResultFixture
        onCopyRequest={vi.fn()}
        result={{ ...syntheticResult, reason: "protected-target" }}
      />,
    );

    expect(screen.getByRole("button", { name: "Copy" })).toBeDisabled();
    expect(
      screen.getByText(/protected destination may be retained outside Outflow/),
    ).toBeVisible();
  });


});
