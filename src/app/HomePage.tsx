import { useState, type ChangeEvent } from "react";
import AppHeader, { type AppPage } from "./AppHeader";
import { CopyResultPanel, type CopyResult } from "./CopyResultPanel";
import { PageContainer } from "./PageContainer";
import { SessionStatus, type SessionPresentation } from "./SessionStatus";
import { UncertainInsertionReview } from "./UncertainInsertionReview";

type HomeScenarioId =
  | "unavailable"
  | "preparing"
  | "recording"
  | "finalizing"
  | "inserted"
  | "no-speech"
  | "changed-target"
  | "protected-target"
  | "uncertain"
  | "copy-failure"
  | "recoverable-error";

type HomeScenario = {
  id: HomeScenarioId;
  presentation: SessionPresentation;
  result: CopyResult | null;
};

const scenarioOptions: ReadonlyArray<{ id: HomeScenarioId; label: string }> = [
  { id: "unavailable", label: "Unavailable / default" },
  { id: "preparing", label: "Preparing" },
  { id: "recording", label: "Recording" },
  { id: "finalizing", label: "Finalizing" },
  { id: "inserted", label: "Inserted" },
  { id: "no-speech", label: "No speech" },
  { id: "changed-target", label: "Changed target" },
  { id: "protected-target", label: "Protected target" },
  { id: "uncertain", label: "Uncertain insertion" },
  { id: "copy-failure", label: "Copy failure" },
  { id: "recoverable-error", label: "Recoverable error" },
];

const syntheticResultText =
  "Synthetic preview result: Outflow keeps this sample in memory.";

function createScenario(id: HomeScenarioId): HomeScenario {
  switch (id) {
    case "preparing":
      return { id, presentation: { state: "preparing" }, result: null };
    case "recording":
      return { id, presentation: { state: "recording" }, result: null };
    case "finalizing":
      return { id, presentation: { state: "finalizing" }, result: null };
    case "inserted":
      return { id, presentation: { state: "inserted" }, result: null };
    case "no-speech":
      return { id, presentation: { state: "no-speech" }, result: null };
    case "changed-target":
      return {
        id,
        presentation: { state: "copy-ready" },
        result: { text: syntheticResultText, reason: "changed-target" },
      };
    case "protected-target":
      return {
        id,
        presentation: { state: "copy-ready" },
        result: { text: syntheticResultText, reason: "protected-target" },
      };
    case "uncertain":
      return {
        id,
        presentation: { state: "uncertain" },
        result: { text: syntheticResultText, reason: "uncertain-reviewed" },
      };
    case "copy-failure":
      return {
        id,
        presentation: { state: "copy-ready" },
        result: { text: syntheticResultText, reason: "unsupported-control" },
      };
    case "recoverable-error":
      return {
        id,
        presentation: { state: "error", reason: "audio-device" },
        result: null,
      };
    case "unavailable":
      return { id, presentation: { state: "unavailable" }, result: null };
  }
}

type HomePageProps = {
  onNavigate: (page: AppPage) => void;
  headingRef: (heading: HTMLHeadingElement | null) => void;
};

export default function HomePage({ onNavigate, headingRef }: HomePageProps) {
  const [scenario, setScenario] = useState<HomeScenario>(() =>
    createScenario("unavailable"),
  );
  const [simulatedCopyFailure, setSimulatedCopyFailure] = useState(false);
  const isSyntheticScenario =
    import.meta.env.DEV && scenario.id !== "unavailable";

  function handleScenarioChange(event: ChangeEvent<HTMLSelectElement>) {
    const selected = event.currentTarget.value as HomeScenarioId;
    setSimulatedCopyFailure(false);
    setScenario(createScenario(selected));
  }

  function handleManualReview() {
    setScenario((current) => {
      if (current.presentation.state !== "uncertain" || !current.result) {
        return current;
      }

      return {
        ...current,
        presentation: { state: "copy-ready" },
        result: { ...current.result, reason: "uncertain-reviewed" },
      };
    });
  }

  function clearResult() {
    setSimulatedCopyFailure(false);
    setScenario(createScenario("unavailable"));
  }

  function handleSimulatedCopyFailure() {
    setSimulatedCopyFailure(true);
  }

  return (
    <main>
      <PageContainer>
        <AppHeader currentPage="home" onNavigate={onNavigate} />
        <div className="home-heading-group">
          <h1 id="home-heading" ref={headingRef} tabIndex={-1}>
            Home
          </h1>
          <p className="home-description">
            Outflow is designed for offline desktop dictation.
          </p>
        </div>

        <div className="home-content">
          <SessionStatus
            presentation={scenario.presentation}
            synthetic={isSyntheticScenario}
          />

          {import.meta.env.DEV && (
            <section
              aria-labelledby="home-scenario-heading"
              className="home-scenario-card"
            >
              <div>
                <p className="home-status-label">Development-only preview</p>
                <h2 id="home-scenario-heading">Inspect synthetic scenarios</h2>
                <p className="home-scenario-description">
                  Every state and result is synthetic. These controls do not
                  request microphone access, start a model, register shortcuts,
                  insert text, or access the system clipboard. Switching
                  scenarios discards the previous in-memory result.
                </p>
              </div>
              <label className="home-scenario-label" htmlFor="home-scenario">
                Scenario
              </label>
              <select
                className="home-scenario-select"
                id="home-scenario"
                onChange={handleScenarioChange}
                value={scenario.id}
              >
                {scenarioOptions.map((option) => (
                  <option key={option.id} value={option.id}>
                    {option.label}
                  </option>
                ))}
              </select>
            </section>
          )}

          {scenario.presentation.state === "uncertain" &&
            scenario.result && (
              <UncertainInsertionReview
                key={scenario.id}
                onReviewed={handleManualReview}
              />
            )}

          {scenario.presentation.state === "copy-ready" &&
            scenario.result && (
              <CopyResultPanel
                copyAcknowledgment={null}
                key={scenario.id}
                onDismiss={clearResult}
                onResultCopied={clearResult}
                requestScope={scenario.id}
                result={scenario.result}
              />
            )}

          {import.meta.env.DEV && scenario.id === "copy-failure" && (
            <section
              aria-label="Simulated Copy demonstration"
              className="home-status-card copy-result-simulation"
            >
              <p className="home-status-label">Development-only action</p>
              <h2>Copy feedback preview</h2>
              <p>
                This action never accesses or changes the system clipboard.
              </p>
              <button
                className="copy-result-action"
                onClick={handleSimulatedCopyFailure}
                type="button"
              >
                Simulate Copy action
              </button>
              {simulatedCopyFailure && (
                <p aria-atomic="true" aria-live="polite" role="status">
                  Simulated Copy failed. No clipboard was accessed; your result
                  is still available.
                </p>
              )}
            </section>
          )}

          <section
            aria-labelledby="getting-started-heading"
            className="home-getting-started-card"
          >
            <div>
              <p className="home-section-label">Getting started</p>
              <h2 id="getting-started-heading">Dictation that stays local</h2>
              <p className="home-getting-started-description">
                The planned desktop app will let you start dictation with a
                shortcut, process speech offline, then use finished text in a
                safe target or copy it when insertion is unavailable. Recording
                and native dictation are not connected in this preview.
              </p>
            </div>
            <button
              className="home-open-settings-button"
              onClick={() => onNavigate("settings")}
              type="button"
            >
              Open Settings
            </button>
          </section>
        </div>
      </PageContainer>
    </main>
  );
}
