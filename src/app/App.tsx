import "../styles/global.css";
import { useRef, useState, type KeyboardEvent } from "react";

const settingsSections = [
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
    description: "Choose how dictation starts and stops.",
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

type SettingsSectionId = (typeof settingsSections)[number]["id"];

export default function App() {
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [selectedSection, setSelectedSection] =
    useState<SettingsSectionId>("general");
  const [startup, setStartup] = useState({ run_at_login: false });
  const sectionTabs = useRef<Array<HTMLButtonElement | null>>([]);
  const selectedIndex = settingsSections.findIndex(
    (section) => section.id === selectedSection,
  );
  const selectedDetails = settingsSections[selectedIndex];

  function openSettings() {
    setSelectedSection("general");
    setIsSettingsOpen(true);
  }

  function toggleRunAtLogin() {
    setStartup((current) => ({
      ...current,
      run_at_login: !current.run_at_login,
    }));
  }

  function handleRunAtLoginKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.key !== " ") return;

    // Prevent the native keyup click so holding Space changes the switch once.
    event.preventDefault();
    if (!event.repeat) toggleRunAtLogin();
  }

  function handleTabKeyDown(
    event: KeyboardEvent<HTMLButtonElement>,
    currentIndex: number,
  ) {
    let nextIndex: number | undefined;

    switch (event.key) {
      case "ArrowDown":
        nextIndex = (currentIndex + 1) % settingsSections.length;
        break;
      case "ArrowUp":
        nextIndex =
          (currentIndex - 1 + settingsSections.length) % settingsSections.length;
        break;
      case "Home":
        nextIndex = 0;
        break;
      case "End":
        nextIndex = settingsSections.length - 1;
        break;
      default:
        return;
    }

    event.preventDefault();
    const nextSection = settingsSections[nextIndex];
    setSelectedSection(nextSection.id);
    sectionTabs.current[nextIndex]?.focus();
  }

  if (isSettingsOpen) {
    return (
      <main className="settings-page">
        <div className="settings-shell">
          <header className="settings-header">
            <div className="settings-topline">
              <button
                className="back-button"
                type="button"
                onClick={() => setIsSettingsOpen(false)}
              >
                <span aria-hidden="true" className="back-arrow">
                  ←
                </span>
                Back
              </button>
              <p className="settings-app-name">Outflow</p>
            </div>
            <div className="settings-heading-group">
              <h1 id="settings-heading">Settings</h1>
              <p className="settings-description">
                Configure your offline dictation experience.
              </p>
            </div>
          </header>

          <div className="settings-layout">
            <nav
              aria-label="Settings sections"
              className="settings-navigation"
            >
              <p className="settings-nav-label">Sections</p>
              <div
                aria-orientation="vertical"
                className="settings-tab-list"
                role="tablist"
              >
                {settingsSections.map((section, index) => {
                  const isSelected = section.id === selectedSection;

                  return (
                    <button
                      aria-controls="settings-panel"
                      aria-selected={isSelected}
                      className="settings-tab"
                      id={`settings-tab-${section.id}`}
                      key={section.id}
                      onClick={() => setSelectedSection(section.id)}
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

            <section
              aria-labelledby={`settings-tab-${selectedDetails.id}`}
              className="settings-panel"
              id="settings-panel"
              role="tabpanel"
              tabIndex={0}
            >
              <div className="settings-panel-heading">
                <h2>{selectedDetails.title}</h2>
                <p>{selectedDetails.description}</p>
              </div>
              {selectedSection === "general" ? (
                <div className="settings-preferences">
                  <div
                    aria-labelledby="launch-at-login-label"
                    className="settings-field"
                    role="group"
                  >
                    <div className="settings-field-copy">
                      <p
                        className="settings-field-label"
                        id="launch-at-login-label"
                      >
                        Launch at Login
                      </p>
                      <p
                        className="settings-field-description"
                        id="launch-at-login-description"
                      >
                        Start Outflow automatically when you sign in to your
                        computer.
                      </p>
                    </div>
                    <div className="settings-field-control">
                      <span className="settings-switch-state">
                        {startup.run_at_login ? "On" : "Off"}
                      </span>
                      <button
                        aria-checked={startup.run_at_login}
                        aria-describedby="launch-at-login-description launch-at-login-preview-note"
                        aria-labelledby="launch-at-login-label"
                        className="settings-switch"
                        onClick={toggleRunAtLogin}
                        onKeyDown={handleRunAtLoginKeyDown}
                        role="switch"
                        type="button"
                      >
                        <span
                          aria-hidden="true"
                          className="settings-switch-track"
                        >
                          <span className="settings-switch-thumb" />
                        </span>
                      </button>
                    </div>
                  </div>
                  <p
                    className="settings-preview-note"
                    id="launch-at-login-preview-note"
                    role="note"
                  >
                    Preview only — this setting does not change your computer&apos;s
                    startup behavior yet.
                  </p>
                </div>
              ) : (
                <p className="settings-placeholder">
                  Settings controls will be added in a later step.
                </p>
              )}
            </section>
          </div>
        </div>
      </main>
    );
  }

  return (
    <main className="foundation-page">
      <section className="foundation-card" aria-labelledby="foundation-heading">
        <p className="application-name">Outflow</p>
        <h1 id="foundation-heading">Foundation ready</h1>
        <p className="foundation-description">
          Your application is set up and ready for development.
        </p>
        <button className="primary-button" onClick={openSettings} type="button">
          Settings
        </button>
      </section>
    </main>
  );
}
