import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SettingsNavigation, {
  SETTINGS_SECTIONS,
  type SettingsSectionId,
} from "./SettingsNavigation";
import { SettingsSection } from "./SettingsSection";

function ControlledSettings({
  initialSelectedSection = "general",
}: {
  initialSelectedSection?: SettingsSectionId;
}) {
  const [selectedSection, setSelectedSection] = useState(
    initialSelectedSection,
  );
  const selectedDetails =
    SETTINGS_SECTIONS.find((section) => section.id === selectedSection) ??
    SETTINGS_SECTIONS[0];

  return (
    <>
      <SettingsNavigation
        selectedSection={selectedSection}
        onSelectSection={setSelectedSection}
      />
      <SettingsSection
        id="settings-panel"
        labelledBy={`settings-tab-${selectedSection}`}
        title={selectedDetails.title}
        description={selectedDetails.description}
      >
        <p>Content for {selectedDetails.id}</p>
      </SettingsSection>
    </>
  );
}

describe("SettingsNavigation", () => {
  it("renders the six tabs in order with General selected and roving tab stops", () => {
    render(<ControlledSettings />);

    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((tab) => tab.textContent)).toEqual(
      SETTINGS_SECTIONS.map((section) => section.title),
    );
    expect(
      screen.getByRole("tab", { name: "General" }),
    ).toHaveAttribute("aria-selected", "true");
    expect(tabs.filter((tab) => tab.getAttribute("aria-selected") === "true"))
      .toHaveLength(1);
    expect(tabs.map((tab) => tab.tabIndex)).toEqual([0, -1, -1, -1, -1, -1]);
  });

  it("calls the controlled selection callback with each existing section ID", () => {
    const onSelectSection = vi.fn();
    render(
      <SettingsNavigation
        selectedSection="general"
        onSelectSection={onSelectSection}
      />,
    );

    SETTINGS_SECTIONS.forEach((section, index) => {
      fireEvent.click(screen.getByRole("tab", { name: section.title }));
      expect(onSelectSection).toHaveBeenNthCalledWith(index + 1, section.id);
    });
  });

  it("keeps the selected panel associated with its tab and updates its content", () => {
    render(<ControlledSettings />);

    fireEvent.click(screen.getByRole("tab", { name: "Privacy" }));

    const privacyTab = screen.getByRole("tab", { name: "Privacy" });
    const panel = screen.getByRole("tabpanel", { name: "Privacy" });
    expect(privacyTab).toHaveAttribute("aria-selected", "true");
    expect(privacyTab).toHaveAttribute("aria-controls", "settings-panel");
    expect(panel).toHaveAttribute("id", "settings-panel");
    expect(panel).toHaveAttribute("aria-labelledby", privacyTab.id);
    expect(panel).toHaveTextContent("Content for privacy");
    expect(
      screen
        .getAllByRole("tab")
        .filter((tab) => tab.getAttribute("aria-selected") === "true"),
    ).toHaveLength(1);
    expect(
      screen.getAllByRole("tab").filter((tab) => tab.tabIndex === 0),
    ).toEqual([privacyTab]);
  });

  it("advances with ArrowDown and wraps from the last tab to the first", () => {
    render(<ControlledSettings />);

    const generalTab = screen.getByRole("tab", { name: "General" });
    generalTab.focus();
    fireEvent.keyDown(generalTab, { key: "ArrowDown" });

    const microphoneTab = screen.getByRole("tab", { name: "Microphone" });
    expect(microphoneTab).toHaveFocus();
    expect(microphoneTab).toHaveAttribute("aria-selected", "true");

    const lastTab = screen.getByRole("tab", { name: "About & Diagnostics" });
    fireEvent.click(lastTab);
    fireEvent.keyDown(lastTab, { key: "ArrowDown" });

    expect(generalTab).toHaveFocus();
    expect(generalTab).toHaveAttribute("aria-selected", "true");
  });

  it("reverses with ArrowUp and wraps from the first tab to the last", () => {
    render(<ControlledSettings />);

    const generalTab = screen.getByRole("tab", { name: "General" });
    generalTab.focus();
    fireEvent.keyDown(generalTab, { key: "ArrowUp" });

    const lastTab = screen.getByRole("tab", { name: "About & Diagnostics" });
    expect(lastTab).toHaveFocus();
    expect(lastTab).toHaveAttribute("aria-selected", "true");

    const privacyTab = screen.getByRole("tab", { name: "Privacy" });
    fireEvent.keyDown(lastTab, { key: "ArrowUp" });
    expect(privacyTab).toHaveFocus();
    expect(privacyTab).toHaveAttribute("aria-selected", "true");
  });

  it("uses Home and End to select and focus the first and last tabs", () => {
    render(<ControlledSettings initialSelectedSection="privacy" />);

    const privacyTab = screen.getByRole("tab", { name: "Privacy" });
    privacyTab.focus();
    fireEvent.keyDown(privacyTab, { key: "Home" });

    const generalTab = screen.getByRole("tab", { name: "General" });
    expect(generalTab).toHaveFocus();
    expect(generalTab).toHaveAttribute("aria-selected", "true");

    fireEvent.keyDown(generalTab, { key: "End" });
    const lastTab = screen.getByRole("tab", { name: "About & Diagnostics" });
    expect(lastTab).toHaveFocus();
    expect(lastTab).toHaveAttribute("aria-selected", "true");
  });

  it("leaves selection and focus alone for unsupported keys", () => {
    const onSelectSection = vi.fn();
    render(
      <SettingsNavigation
        selectedSection="general"
        onSelectSection={onSelectSection}
      />,
    );

    const generalTab = screen.getByRole("tab", { name: "General" });
    generalTab.focus();
    fireEvent.keyDown(generalTab, { key: "a" });

    expect(onSelectSection).not.toHaveBeenCalled();
    expect(generalTab).toHaveAttribute("aria-selected", "true");
    expect(generalTab).toHaveFocus();
  });

  it("retains the vertical tablist and selected tabpanel ARIA relationships", () => {
    render(<ControlledSettings initialSelectedSection="shortcuts" />);

    expect(screen.getByRole("navigation", { name: "Settings sections" }))
      .toBeVisible();
    expect(screen.getByRole("tablist")).toHaveAttribute(
      "aria-orientation",
      "vertical",
    );
    const shortcutsTab = screen.getByRole("tab", { name: "Shortcuts" });
    const panel = screen.getByRole("tabpanel", { name: "Shortcuts" });
    expect(shortcutsTab).toHaveAttribute("aria-controls", panel.id);
    expect(panel).toHaveAttribute("aria-labelledby", shortcutsTab.id);
  });
});
