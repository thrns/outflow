import { useRef, type KeyboardEvent } from "react";

export const SETTINGS_SECTIONS = [
  {
    id: "general",
    title: "General",
    description: "Manage general application preferences.",
  },
  {
    id: "microphone",
    title: "Microphone",
    description: "Configure microphone access and input selection.",
  },
  {
    id: "shortcuts",
    title: "Shortcuts",
    description: "Review global shortcut availability and recording-mode requirements.",
  },
  {
    id: "text-vocabulary",
    title: "Text & Vocabulary",
    description: "Set text cleanup preferences and custom terms.",
  },
  {
    id: "privacy",
    title: "Privacy",
    description: "Review how Outflow handles audio, transcripts, and diagnostics.",
  },
  {
    id: "about-diagnostics",
    title: "About & Diagnostics",
    description: "Find application details and troubleshooting information.",
  },
] as const;

export type SettingsSectionId = (typeof SETTINGS_SECTIONS)[number]["id"];

type SettingsNavigationProps = {
  selectedSection: SettingsSectionId;
  onSelectSection: (section: SettingsSectionId) => void;
};

export default function SettingsNavigation({
  selectedSection,
  onSelectSection,
}: SettingsNavigationProps) {
  const sectionTabs = useRef<Array<HTMLButtonElement | null>>([]);

  function handleTabKeyDown(
    event: KeyboardEvent<HTMLButtonElement>,
    currentIndex: number,
  ) {
    let nextIndex: number | undefined;

    switch (event.key) {
      case "ArrowDown":
        nextIndex = (currentIndex + 1) % SETTINGS_SECTIONS.length;
        break;
      case "ArrowUp":
        nextIndex =
          (currentIndex - 1 + SETTINGS_SECTIONS.length) %
          SETTINGS_SECTIONS.length;
        break;
      case "Home":
        nextIndex = 0;
        break;
      case "End":
        nextIndex = SETTINGS_SECTIONS.length - 1;
        break;
      default:
        return;
    }

    event.preventDefault();
    const nextSection = SETTINGS_SECTIONS[nextIndex];
    onSelectSection(nextSection.id);
    sectionTabs.current[nextIndex]?.focus();
  }

  return (
    <nav aria-label="Settings sections" className="settings-navigation">
      <p className="settings-nav-label">Sections</p>
      <div
        aria-orientation="vertical"
        className="settings-tab-list"
        role="tablist"
      >
        {SETTINGS_SECTIONS.map((section, index) => {
          const isSelected = section.id === selectedSection;

          return (
            <button
              aria-controls="settings-panel"
              aria-selected={isSelected}
              className="settings-tab"
              id={`settings-tab-${section.id}`}
              key={section.id}
              onClick={() => onSelectSection(section.id)}
              onKeyDown={(event) => handleTabKeyDown(event, index)}
              ref={(element) => {
                sectionTabs.current[index] = element;
              }}
              role="tab"
              tabIndex={isSelected ? 0 : -1}
              type="button"
            >
              {section.title}
            </button>
          );
        })}
      </div>
    </nav>
  );
}
