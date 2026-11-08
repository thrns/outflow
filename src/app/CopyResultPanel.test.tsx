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
  text:
    "Synthetic preview result: Outflow keeps this sample in memory.\n" +
    "Unicode sample: café, naïve, 東京, and 🪶.\n" +
    "Long Unicode line: " +
    "長".repeat(80) +
    "\n" +
    "Second line remains selectable.",
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
    expect(copyButton).toHaveTextContent("Copy pending…");
    expect(screen.getByLabelText("Copy Result actions")).toHaveAttribute(
      "aria-busy",
      "true",
    );
    expect(screen.getByText("Copy request pending.")).toHaveAttribute(
      "aria-live",
      "polite",
    );
  });

  it("keeps a failed request's exact multiline Unicode text and assigns a new retry ID", () => {
    const onCopyRequest = vi.fn();
    const { rerender } = render(
      <StatefulCopyPanel onCopyRequest={onCopyRequest} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    expect(onCopyRequest).toHaveBeenNthCalledWith(1, {
      requestId: "test-session:1",
    });
    rerender(
      <StatefulCopyPanel
        acknowledgment={{ requestId: "test-session:1", outcome: "failed" }}
        onCopyRequest={onCopyRequest}
      />,
    );

    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveValue(syntheticResult.text);
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    expect(onCopyRequest).toHaveBeenNthCalledWith(2, {
      requestId: "test-session:2",
    });

    rerender(
      <StatefulCopyPanel
        acknowledgment={{ requestId: "test-session:2", outcome: "succeeded" }}
        onCopyRequest={onCopyRequest}
      />,
    );
    expect(screen.getByRole("status")).toHaveTextContent("Result cleared");
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
    expect(
      screen.getByText(
        "Copy failed. Your result is still available in this panel.",
      ),
    ).toHaveAttribute("aria-live", "polite");
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

  it("does not let a stale success acknowledgment from a previous result clear the new lifecycle", () => {
    const onCopyRequest = vi.fn();
    const onResultCopied = vi.fn();
    const nextResult = {
      text: "New lifecycle result: Résumé\nSecond line.",
      reason: "changed-target" as const,
    };
    const { rerender } = render(
      <CopyResultPanel
        key="copy-result-1"
        onCopyRequest={onCopyRequest}
        onDismiss={vi.fn()}
        onResultCopied={onResultCopied}
        requestScope="copy-result-1"
        result={syntheticResult}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    expect(onCopyRequest).toHaveBeenCalledWith({
      requestId: "copy-result-1:1",
    });

    rerender(
      <CopyResultPanel
        copyAcknowledgment={{
          requestId: "copy-result-1:1",
          outcome: "succeeded",
        }}
        key="copy-result-2"
        onCopyRequest={onCopyRequest}
        onDismiss={vi.fn()}
        onResultCopied={onResultCopied}
        requestScope="copy-result-2"
        result={nextResult}
      />,
    );

    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveValue(nextResult.text);
    expect(onResultCopied).not.toHaveBeenCalled();
    expect(screen.queryByText("Copy completed.")).not.toBeInTheDocument();
  });

  it("does not let a stale failure acknowledgment alter feedback for a pending newer lifecycle", () => {
    const onCopyRequest = vi.fn();
    const onResultCopied = vi.fn();
    const nextResult = {
      text: "Second lifecycle text: 東京\nStill available.",
      reason: "changed-target" as const,
    };
    const { rerender } = render(
      <CopyResultPanel
        key="copy-result-1"
        onCopyRequest={onCopyRequest}
        onDismiss={vi.fn()}
        onResultCopied={onResultCopied}
        requestScope="copy-result-1"
        result={syntheticResult}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));

    rerender(
      <CopyResultPanel
        key="copy-result-2"
        onCopyRequest={onCopyRequest}
        onDismiss={vi.fn()}
        onResultCopied={onResultCopied}
        requestScope="copy-result-2"
        result={nextResult}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    expect(onCopyRequest).toHaveBeenNthCalledWith(2, {
      requestId: "copy-result-2:1",
    });

    rerender(
      <CopyResultPanel
        copyAcknowledgment={{
          requestId: "copy-result-1:1",
          outcome: "failed",
        }}
        key="copy-result-2"
        onCopyRequest={onCopyRequest}
        onDismiss={vi.fn()}
        onResultCopied={onResultCopied}
        requestScope="copy-result-2"
        result={nextResult}
      />,
    );

    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveValue(nextResult.text);
    expect(screen.getByRole("button", { name: "Copy pending…" })).toBeDisabled();
    expect(
      screen.queryByText("Copy failed. Your result is still available in this panel."),
    ).not.toBeInTheDocument();
    expect(onResultCopied).not.toHaveBeenCalled();
  });

  it("invalidates an in-flight request when Dismiss closes its lifecycle", () => {
    const onCopyRequest = vi.fn();
    const onDismiss = vi.fn();
    const onResultCopied = vi.fn();
    const { rerender } = render(
      <CopyResultPanel
        key="copy-result-1"
        onCopyRequest={onCopyRequest}
        onDismiss={onDismiss}
        onResultCopied={onResultCopied}
        requestScope="copy-result-1"
        result={syntheticResult}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(onDismiss).toHaveBeenCalledTimes(1);

    rerender(
      <CopyResultPanel
        copyAcknowledgment={{
          requestId: "copy-result-1:1",
          outcome: "succeeded",
        }}
        key="copy-result-2"
        onCopyRequest={onCopyRequest}
        onDismiss={vi.fn()}
        onResultCopied={onResultCopied}
        requestScope="copy-result-2"
        result={{ ...syntheticResult, text: "Reopened result." }}
      />,
    );

    expect(
      screen.getByLabelText("Finalized text (kept in memory until dismissed)"),
    ).toHaveValue("Reopened result.");
    expect(onResultCopied).not.toHaveBeenCalled();
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
    const onCopyRequest = vi.fn();
    render(
      <CopyResultFixture
        onCopyRequest={onCopyRequest}
        result={{ ...syntheticResult, reason: "protected-target" }}
      />,
    );

    const copyButton = screen.getByRole("button", { name: "Copy" });
    expect(copyButton).toBeDisabled();
    expect(
      screen.getByText(/protected destination may be retained outside Outflow/),
    ).toBeVisible();

    fireEvent.click(
      screen.getByRole("checkbox", {
        name: /protected destination may be retained outside Outflow/,
      }),
    );
    expect(copyButton).toBeEnabled();
    fireEvent.click(copyButton);
    expect(onCopyRequest).toHaveBeenCalledWith({
      requestId: "test-session:1",
    });
  });


});
