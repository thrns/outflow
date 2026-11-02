import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import AboutDiagnosticsOverview from "./AboutDiagnosticsOverview";

describe("AboutDiagnosticsOverview", () => {
  it("renders the About, Troubleshooting, Diagnostics, and Updates sections", () => {
    render(<AboutDiagnosticsOverview />);

    for (const heading of [
      "About",
      "Troubleshooting",
      "Diagnostics",
      "Application updates",
    ]) {
      expect(
        screen.getByRole("heading", { level: 3, name: heading }),
      ).toBeVisible();
    }
  });

  it("identifies Outflow and describes the interface as a development preview", () => {
    render(<AboutDiagnosticsOverview />);

    const about = screen.getByRole("region", { name: "About" });
    expect(within(about).getByText("Outflow")).toBeVisible();
    expect(about).toHaveTextContent(
      "Outflow is an offline-first desktop dictation application currently under development.",
    );
    expect(about).toHaveTextContent("This interface is a development preview.");
  });

  it("does not present package metadata as a native release version", () => {
    render(<AboutDiagnosticsOverview />);

    const about = screen.getByRole("region", { name: "About" });
    expect(about).not.toHaveTextContent(/version\s+0\.1\.0|build\s+\d/i);
    expect(about).not.toHaveTextContent(/model version|native runtime version/i);
    expect(about).toHaveTextContent(
      "Native desktop release metadata is not available here.",
    );
  });

  it("lists the planned troubleshooting topics and labels them as undetected", () => {
    render(<AboutDiagnosticsOverview />);

    const troubleshooting = screen.getByRole("region", {
      name: "Troubleshooting",
    });
    for (const topic of [
      /Microphone permission is denied/i,
      /input device is disconnected/i,
      /speech model cannot load or fails an integrity check/i,
      /resources or memory limits/i,
      /recording buffer overflows/i,
      /Text insertion is uncertain or unsupported/i,
    ]) {
      expect(troubleshooting).toHaveTextContent(topic);
    }
    expect(troubleshooting).toHaveTextContent(
      /understandable error codes and recovery guidance/i,
    );
    expect(troubleshooting).toHaveTextContent(
      /not problems detected in this development preview/i,
    );
  });

  it("describes the intended privacy-safe diagnostic export fields", () => {
    render(<AboutDiagnosticsOverview />);

    const diagnostics = screen.getByRole("region", { name: "Diagnostics" });
    for (const field of [
      /Application and runtime version information/i,
      /Operating system and build information/i,
      /Device capability results/i,
      /Anonymized error codes/i,
      /Timing and aggregate performance metrics/i,
      /Model integrity-check results/i,
    ]) {
      expect(diagnostics).toHaveTextContent(field);
    }
  });

  it("states that intended diagnostics exclude audio, transcripts, vocabulary, and clipboard", () => {
    render(<AboutDiagnosticsOverview />);

    const diagnostics = screen.getByRole("region", { name: "Diagnostics" });
    for (const excludedValue of [
      "Recorded audio.",
      "Dictated transcripts.",
      "Custom vocabulary values.",
      "Clipboard content.",
    ]) {
      expect(diagnostics).toHaveTextContent(excludedValue);
    }
  });

  it("explicitly says diagnostic collection and export are unavailable", () => {
    render(<AboutDiagnosticsOverview />);

    const status = screen.getByRole("heading", {
      level: 3,
      name: "Diagnostics export unavailable",
    }).parentElement;
    expect(status).toHaveTextContent(
      "Diagnostic collection and export are not connected in this development preview.",
    );
    expect(status).toHaveTextContent(
      "No diagnostic report can be generated from this page.",
    );
  });

  it("describes update checks as explicit and potentially connecting to the release host", () => {
    render(<AboutDiagnosticsOverview />);

    const updates = screen.getByRole("region", {
      name: "Application updates",
    });
    expect(updates).toHaveTextContent(
      /requires an explicit user action and may connect to the vendor's release host/i,
    );
  });

  it("describes signed full-application updates with the bundled model and recovery", () => {
    render(<AboutDiagnosticsOverview />);

    const updates = screen.getByRole("region", {
      name: "Application updates",
    });
    expect(updates).toHaveTextContent(/verified, signed full-application packages/i);
    expect(updates).toHaveTextContent(/include the bundled speech model/i);
    expect(updates).toHaveTextContent(/safe failure or recovery path/i);
    expect(updates).toHaveTextContent(
      /ordinary offline dictation must not require an update check/i,
    );
  });

  it("explicitly says update checking is unavailable in this preview", () => {
    render(<AboutDiagnosticsOverview />);

    expect(
      screen.getByText(
        "Update checking is not implemented in this development preview.",
      ),
    ).toBeVisible();
    expect(screen.queryByText(/Up to date|Update available/i)).not.toBeInTheDocument();
  });

  it("does not add functional diagnostic or update controls", () => {
    const { container } = render(<AboutDiagnosticsOverview />);

    expect(
      container.querySelectorAll(
        "button, a, input, select, textarea, [role='button'], [tabindex]",
      ),
    ).toHaveLength(0);
  });

  it("keeps static information out of alert and live-region semantics", () => {
    const { container } = render(<AboutDiagnosticsOverview />);

    expect(container.querySelector("[role='alert'], [aria-live]")).toBeNull();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
