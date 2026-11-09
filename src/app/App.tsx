import "../styles/global.css";
import { useEffect, useRef, useState } from "react";
import AppHeader, { type AppView } from "./AppHeader";
import HomePage from "./HomePage";
import WelcomePage from "./WelcomePage";
import PrivacyStepPage from "./PrivacyStepPage";
import PermissionsStepPage from "./PermissionsStepPage";
import ShortcutSetupStepPage from "./ShortcutSetupStepPage";
import TryDictationStepPage from "./TryDictationStepPage";
import TrayStatusStepPage from "./TrayStatusStepPage";
import { PageContainer } from "./PageContainer";
import { SettingsRow } from "./SettingsRow";
import { SettingsSwitch } from "./SettingsSwitch";
import { SettingsSection } from "./SettingsSection";
import SettingsNavigation, {
  SETTINGS_SECTIONS,
  type SettingsSectionId,
} from "./SettingsNavigation";
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
import {
  SETUP_STEP_DEFINITIONS,
  type SetupStepView,
} from "./SetupStepIndicator";

export default function App() {
  const [activeView, setActiveView] = useState<AppView>("home");
  const [settingsReturnView, setSettingsReturnView] =
    useState<SetupStepView | null>(null);
  const previousView = useRef(activeView);
  const homeHeading = useRef<HTMLHeadingElement | null>(null);
  const settingsHeading = useRef<HTMLHeadingElement | null>(null);
  const welcomeHeading = useRef<HTMLHeadingElement | null>(null);
  const privacyHeading = useRef<HTMLHeadingElement | null>(null);
  const permissionsHeading = useRef<HTMLHeadingElement | null>(null);
  const shortcutSetupHeading = useRef<HTMLHeadingElement | null>(null);
  const tryDictationHeading = useRef<HTMLHeadingElement | null>(null);
  const trayStatusHeading = useRef<HTMLHeadingElement | null>(null);
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
  const selectedIndex = SETTINGS_SECTIONS.findIndex(
    (section) => section.id === selectedSection,
  );
  const selectedDetails = SETTINGS_SECTIONS[selectedIndex];
  const settingsReturnStep = settingsReturnView
    ? SETUP_STEP_DEFINITIONS.find(
        (step) => step.view === settingsReturnView,
      )
    : undefined;

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
            : activeView === "privacy"
              ? privacyHeading.current
              : activeView === "permissions"
                ? permissionsHeading.current
                : activeView === "shortcut-setup"
                  ? shortcutSetupHeading.current
                  : activeView === "try-dictation"
                    ? tryDictationHeading.current
                    : trayStatusHeading.current;
    heading?.focus();
  }, [activeView]);

  function navigateTo(
    view: AppView,
    settingsSection?: SettingsSectionId,
  ) {
    if (view === activeView) {
      if (view === "settings") {
        setSettingsReturnView(null);
        setSelectedSection(settingsSection ?? "general");
        settingsHeading.current?.focus();
      }
      return;
    }

    setSettingsReturnView(null);
    if (view === "settings") {
      setSelectedSection(settingsSection ?? "general");
    }
    setActiveView(view);
  }

  function openSettingsFromSetup(
    view: SetupStepView,
    settingsSection: SettingsSectionId = "general",
  ) {
    setSettingsReturnView(view);
    setSelectedSection(settingsSection);
    setActiveView("settings");
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
              {settingsReturnStep ? (
                <button
                  className="app-navigation-button settings-return-button"
                  onClick={() => {
                    if (settingsReturnView) navigateTo(settingsReturnView);
                  }}
                  type="button"
                >
                  Return to setup: {settingsReturnStep.label}
                </button>
              ) : null}
              <p className="settings-description">
                Configure your offline dictation experience.
              </p>
            </div>
          </div>

          <div className="settings-layout">
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
        onOpenSettings={() => openSettingsFromSetup("welcome")}
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
        onOpenSettings={() => openSettingsFromSetup("privacy")}
      />
    );
  }

  if (activeView === "permissions") {
    return (
      <PermissionsStepPage
        headingRef={(heading) => {
          permissionsHeading.current = heading;
        }}
        onNavigate={navigateTo}
        onOpenSettings={() => openSettingsFromSetup("permissions")}
      />
    );
  }

  if (activeView === "shortcut-setup") {
    return (
      <ShortcutSetupStepPage
        headingRef={(heading) => {
          shortcutSetupHeading.current = heading;
        }}
        onNavigate={navigateTo}
        onOpenShortcuts={() =>
          openSettingsFromSetup("shortcut-setup", "shortcuts")
        }
      />
    );
  }

  if (activeView === "try-dictation") {
    return (
      <TryDictationStepPage
        headingRef={(heading) => {
          tryDictationHeading.current = heading;
        }}
        onNavigate={navigateTo}
      />
    );
  }

  if (activeView === "tray-status") {
    return (
      <TrayStatusStepPage
        headingRef={(heading) => {
          trayStatusHeading.current = heading;
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
