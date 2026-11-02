import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import PrivacyOverview from "./PrivacyOverview";

describe("PrivacyOverview", () => {
  it("renders the informational overview", () => {
    render(<PrivacyOverview />);

    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Privacy implementation in progress",
      }),
    ).toBeVisible();
    for (const title of [
      "Offline processing",
      "What Outflow is designed not to retain",
      "What may be stored",
      "Clipboard, operating system, and updates",
    ]) {
      expect(
        screen.getByRole("heading", { level: 3, name: title }),
      ).toBeVisible();
    }
  });

  it("describes offline processing as an intended requirement", () => {
    render(<PrivacyOverview />);

    const section = screen.getByRole("region", {
      name: "Offline processing",
    });
    expect(section).toHaveTextContent(
      /designed to run speech recognition and text cleanup locally/i,
    );
    expect(section).toHaveTextContent(
      /intended to work without an Internet connection/i,
    );
    expect(section).toHaveTextContent(/an account, or cloud inference/i);
    expect(section).toHaveTextContent(
      /model is intended to ship with the installer/i,
    );
    expect(section).toHaveTextContent(
      /not verified behavior in this frontend preview/i,
    );
  });

  it("explains that transcript history and persistent caches are prohibited", () => {
    render(<PrivacyOverview />);

    const section = screen.getByRole("region", {
      name: "What Outflow is designed not to retain",
    });
    expect(section).toHaveTextContent(
      "Transcript history or persistent transcript caches.",
    );
    expect(section).toHaveTextContent(
      /requirements prohibit retaining or tracking/i,
    );
  });

  it("explains that saved recordings are prohibited", () => {
    render(<PrivacyOverview />);

    const section = screen.getByRole("region", {
      name: "What Outflow is designed not to retain",
    });
    expect(section).toHaveTextContent("Saved audio recordings.");
    expect(section).toHaveTextContent(
      /paths have not yet been fully implemented and privacy-tested/i,
    );
  });

  it("distinguishes local settings and vocabulary from speech content", () => {
    render(<PrivacyOverview />);

    const section = screen.getByRole("region", { name: "What may be stored" });
    expect(section).toHaveTextContent(
      /Application settings may be stored locally/i,
    );
    expect(section).toHaveTextContent(
      /vocabulary entries may eventually be stored locally/i,
    );
    expect(section).toHaveTextContent(
      /Recording audio and transcript history must not be stored/i,
    );
  });

  it("says the current vocabulary draft is unsaved and preview values reset on refresh", () => {
    render(<PrivacyOverview />);

    const section = screen.getByRole("region", { name: "What may be stored" });
    expect(section).toHaveTextContent(/vocabulary draft is not persisted/i);
    expect(section).toHaveTextContent(
      /Refreshing the page resets these preview values/i,
    );
    expect(section).toHaveTextContent(
      /Launch at Login and Clean\/Verbatim are in-memory settings/i,
    );
  });

  it("says clipboard copying is intended to require an explicit user action", () => {
    render(<PrivacyOverview />);

    const section = screen.getByRole("region", {
      name: "Clipboard, operating system, and updates",
    });
    expect(section).toHaveTextContent(/only after an explicit user action/i);
    expect(section).toHaveTextContent(
      /Native clipboard behavior .* not implemented in this preview/i,
    );
  });

  it("discloses operating-system retention and memory-erasure limitations", () => {
    render(<PrivacyOverview />);

    const section = screen.getByRole("region", {
      name: "Clipboard, operating system, and updates",
    });
    expect(section).toHaveTextContent(
      /destination application, operating system swap, and clipboard managers may retain content/i,
    );
    expect(section).toHaveTextContent(
      /cannot guarantee cryptographic erasure of data from system memory/i,
    );
  });

  it("describes updates as explicit and optional without requiring them for dictation", () => {
    render(<PrivacyOverview />);

    const section = screen.getByRole("region", {
      name: "Clipboard, operating system, and updates",
    });
    expect(section).toHaveTextContent(
      /optional, explicit, user-initiated check for signed full-application updates/i,
    );
    expect(section).toHaveTextContent(
      /Normal dictation must not require update checks/i,
    );
    expect(section).toHaveTextContent(/runtime model downloads/i);
    expect(section).toHaveTextContent(/signed updates are not implemented in this preview/i);
  });

  it("communicates that native privacy work is incomplete and recording is absent from this preview", () => {
    render(<PrivacyOverview />);

    const status = screen.getByRole("heading", {
      level: 3,
      name: "Privacy implementation in progress",
    }).parentElement;
    expect(status).not.toBeNull();
    expect(status).toHaveTextContent(/requirements/i);
    expect(status).toHaveTextContent(
      /have not yet been fully implemented and verified/i,
    );
    expect(status).toHaveTextContent(/does not implement microphone recording/i);
  });

  it("does not display a privacy certification or absolute guarantee", () => {
    render(<PrivacyOverview />);

    expect(
      screen.queryByText(
        /privacy certified|100% secure|network traffic verified zero|no data can ever leave your device|all recordings are securely erased|privacy audit passed/i,
      ),
    ).not.toBeInTheDocument();
  });

  it("adds no microphone, clipboard, update, or diagnostics action", () => {
    const { container } = render(<PrivacyOverview />);

    expect(
      container.querySelectorAll(
        "button, a, input, select, textarea, [role='button'], [tabindex]",
      ),
    ).toHaveLength(0);
  });

  it("keeps static privacy information out of alert and live-region semantics", () => {
    const { container } = render(<PrivacyOverview />);

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(container.querySelector("[role='alert'], [aria-live]")).toBeNull();
    expect(
      within(container).getByRole("heading", {
        level: 3,
        name: "Privacy implementation in progress",
      }),
    ).toBeVisible();
  });
});
