import "../styles/global.css";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import AppHeader, { type AppView } from "./AppHeader";
import HomePage from "./HomePage";
import WelcomePage from "./WelcomePage";
import PrivacyStepPage from "./PrivacyStepPage";
import { PageContainer } from "./PageContainer";
import { SettingsRow } from "./SettingsRow";
import { SettingsSwitch } from "./SettingsSwitch";
import { SettingsSection } from "./SettingsSection";
import { MicrophoneCapabilityPanel } from "./MicrophoneCapabilityPanel";
import { MicrophoneScenarioPreview } from "./MicrophoneScenarioPreview";
import { ShortcutCapabilityPanel } from "./ShortcutCapabilityPanel";
import { ShortcutScenarioPreview } from "./ShortcutScenarioPreview";
import { PermissionReadinessChecklist } from "./PermissionReadinessChecklist";
import TextCleanupModeSelector, {
  type CleanupMode,
} from "./TextCleanupModeSelector";
import VocabularyEntryForm, {
  MAX_VOCABULARY_PREVIEW_ENTRIES,
  type VocabularyDraft,
} from "./VocabularyEntryForm";
import VocabularyDraftPreview from "./VocabularyDraftPreview";
import VocabularyPreviewList, {
  type VocabularyPreviewEntry,
} from "./VocabularyPreviewList";
import PrivacyOverview from "./PrivacyOverview";
import AboutDiagnosticsOverview from "./AboutDiagnosticsOverview";

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

type SettingsSectionId = (typeof settingsSections)[number]["id"];

export default function App() {
  const [activeView, setActiveView] = useState<AppView>("home");
  const previousView = useRef(activeView);
  const homeHeading = useRef<HTMLHeadingElement | null>(null);
  const settingsHeading = useRef<HTMLHeadingElement | null>(null);
  const welcomeHeading = useRef<HTMLHeadingElement | null>(null);
  const privacyHeading = useRef<HTMLHeadingElement | null>(null);
  const [selectedSection, setSelectedSection] =
    useState<SettingsSectionId>("general");
  const [startup, setStartup] = useState({ run_at_login: false });
  const [cleanupMode, setCleanupMode] = useState<CleanupMode>("clean");
  const [vocabularyDraft, setVocabularyDraft] = useState<VocabularyDraft>({
    spoken: "",
    written: "",
  });
  // Keep preview entries above the conditional views so navigation preserves them;
  // a fresh App mount or page reload starts with an empty list.
  const [vocabularyEntries, setVocabularyEntries] = useState<
    VocabularyPreviewEntry[]
  >([]);
  const nextVocabularyEntryId = useRef(0);
  // Settings and transient setup pages unmount Home; keep result IDs unique across routes.
  // A native bridge must still validate the Rust coordinator's authoritative IDs.
  const nextCopyResultLifecycleId = useRef(0);
  const sectionTabs = useRef<Array<HTMLButtonElement | null>>([]);
  const selectedIndex = settingsSections.findIndex(
    (section) => section.id === selectedSection,
  );
  const selectedDetails = settingsSections[selectedIndex];

  useEffect(() => {
    if (previousView.current === activeView) return;

    previousView.current = activeView;
    const heading =
      activeView === "home"
        ? homeHeading.current
        : activeView === "settings"
          ? settingsHeading.current
          : activeView === "welcome"
            ? welcomeHeading.current
            : privacyHeading.current;
    heading?.focus();
  }, [activeView]);

  function navigateTo(view: AppView) {
    if (view === activeView) return;

    if (view === "settings") setSelectedSection("general");
    setActiveView(view);
  }

  function addVocabularyPreviewEntry(value: VocabularyDraft) {
    if (vocabularyEntries.length >= MAX_VOCABULARY_PREVIEW_ENTRIES) return;

    const entry = { id: nextVocabularyEntryId.current, ...value };
    nextVocabularyEntryId.current += 1;
    setVocabularyEntries((current) =>
      current.length >= MAX_VOCABULARY_PREVIEW_ENTRIES
        ? current
        : [...current, entry],
    );
    setVocabularyDraft({ spoken: "", written: "" });
  }

  function allocateCopyResultLifecycleId() {
    const nextId = nextCopyResultLifecycleId.current + 1;
    if (!Number.isSafeInteger(nextId)) {
      throw new RangeError("Copy Result lifecycle identity is exhausted.");
    }
    nextCopyResultLifecycleId.current = nextId;
    return nextId;
  }

  function saveVocabularyPreviewEntry(id: number, value: VocabularyDraft) {
    setVocabularyEntries((current) =>
      current.map((entry) => (entry.id === id ? { ...entry, ...value } : entry)),
    );
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

  if (activeView === "settings") {
    return (
      <main>
        <PageContainer>
          <div className="settings-header">
            <AppHeader currentPage="settings" onNavigate={navigateTo} />
            <div className="settings-heading-group">
              <h1 id="settings-heading" ref={settingsHeading} tabIndex={-1}>
                Settings
              </h1>
              <p className="settings-description">
                Configure your offline dictation experience.
              </p>
            </div>
          </div>

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

            <SettingsSection
              id="settings-panel"
              labelledBy={`settings-tab-${selectedSection}`}
              title={selectedDetails.title}
              description={selectedDetails.description}
            >
              {selectedSection === "general" ? (
                <div className="settings-preferences">
                  <SettingsRow
                    title="Launch at Login"
                    titleId="launch-at-login-label"
                    description="Start Outflow automatically when you sign in to your computer."
                    descriptionId="launch-at-login-description"
                  >
                    <div className="settings-field-control">
                      <span className="settings-switch-state">
                        {startup.run_at_login ? "On" : "Off"}
                      </span>
                      <SettingsSwitch
                        checked={startup.run_at_login}
                        onCheckedChange={(checked) =>
                          setStartup((current) => ({
                            ...current,
                            run_at_login: checked,
                          }))
                        }
                        labelledBy="launch-at-login-label"
                        describedBy="launch-at-login-description launch-at-login-preview-note"
                      />
                    </div>
                  </SettingsRow>
                  <p
                    className="settings-preview-note"
                    id="launch-at-login-preview-note"
                    role="note"
                  >
                    Preview only — this setting does not change your computer&apos;s
                    startup behavior yet.
                  </p>
                  <PermissionReadinessChecklist />
                </div>
              ) : selectedSection === "microphone" ? (
                import.meta.env.DEV ? (
                  <MicrophoneScenarioPreview />
                ) : (
                  <MicrophoneCapabilityPanel />
                )
              ) : selectedSection === "shortcuts" ? (
                import.meta.env.DEV ? (
                  <ShortcutScenarioPreview />
                ) : (
                  <ShortcutCapabilityPanel />
                )
              ) : selectedSection === "text-vocabulary" ? (
                <div className="text-vocabulary-settings">
                  <TextCleanupModeSelector
                    value={cleanupMode}
                    onChange={setCleanupMode}
                  />
                  <section
                    aria-labelledby="custom-vocabulary-heading"
                    className="custom-vocabulary-preview"
                  >
                    <h3 id="custom-vocabulary-heading">Custom vocabulary</h3>
                    <p className="vocabulary-development-note" role="note">
                      Development preview only. Entries stay in memory until
                      this page reloads and do not change actual speech.
                    </p>
                    <VocabularyEntryForm
                      onClearDraft={() =>
                        setVocabularyDraft({ spoken: "", written: "" })
                      }
                      onAddToPreviewList={addVocabularyPreviewEntry}
                      onChange={setVocabularyDraft}
                      isAtLimit={
                        vocabularyEntries.length >=
                        MAX_VOCABULARY_PREVIEW_ENTRIES
                      }
                      value={vocabularyDraft}
                    />
                    <VocabularyPreviewList
                      entries={vocabularyEntries}
                      onSave={saveVocabularyPreviewEntry}
                      onRemove={(id) =>
                        setVocabularyEntries((current) =>
                          current.filter((entry) => entry.id !== id),
                        )
                      }
                    />
                    <VocabularyDraftPreview value={vocabularyDraft} />
                  </section>
                </div>
              ) : selectedSection === "privacy" ? (
                <PrivacyOverview />
              ) : selectedSection === "about-diagnostics" ? (
                <AboutDiagnosticsOverview />
              ) : (
                <p className="settings-placeholder">
                  Settings controls will be added in a later step.
                </p>
              )}
            </SettingsSection>
          </div>
        </PageContainer>
      </main>
    );
  }

  if (activeView === "welcome") {
    return (
      <WelcomePage
        headingRef={(heading) => {
          welcomeHeading.current = heading;
        }}
        onNavigate={navigateTo}
      />
    );
  }

  if (activeView === "privacy") {
    return (
      <PrivacyStepPage
        headingRef={(heading) => {
          privacyHeading.current = heading;
        }}
        onNavigate={navigateTo}
      />
    );
  }

  return (
    <HomePage
      allocateResultLifecycleId={allocateCopyResultLifecycleId}
      headingRef={(heading) => {
        homeHeading.current = heading;
      }}
      onNavigate={navigateTo}
    />
  );
}
