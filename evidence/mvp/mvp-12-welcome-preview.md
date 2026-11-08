# MVP-12 — Welcome setup introduction preview

```yaml
task_id: MVP-12 (browser preview milestone; no production T-xxx task changed)
feature_ids: [F-01, F-02]
flows: [UF-01]
acceptance: [AC-01, AC-02, AC-11]
owner: Codex
base_commit: d797f98affdb8ddfb69656b4f314c15893e2f8fa
result_commit: committed with this evidence; see feature branch HEAD in the final handoff
platform: macOS 26.6.2 arm64; Node v22.23.3; npm 10.9.9; Rust 1.99.1; Python 3.13.13
implementation: Separate browser-only Welcome view, transient App navigation, focus handoff, Home unmount and retained Copy Result lifecycle sequence; see paths below.
metrics: Resource/release metrics NOT_MEASURED. Browser layout verified at 320 CSS px with no horizontal overflow.
doc_updates: README.md; docs/TASKS.md (production checkboxes remain open)
risks: No Tauri shell or native dictation integration; T-010 remains blocked. Genuine 200% zoom and manual screen-reader checks NOT_RUN.
review: PENDING
```

## Implementation

- Added `src/app/WelcomePage.tsx` with the single Welcome heading, intended offline/privacy message, planned-MVP framing, native-app and microphone prerequisites, preview disclaimer, and Back to Home / Review permission readiness actions.
- Extended the existing App view state with a transient Welcome view. It uses the existing `PageContainer` and `AppHeader`; the primary navigation still contains only Home and Settings. Focus moves to the Welcome, Home, or Settings heading after each view change.
- Added Home → Getting Started → Preview setup. Welcome is rendered as its own view, so Home's volatile Copy Result and simulated acknowledgment unmount on entry. The App-root lifecycle counter remains mounted and issues a new lifecycle ID when a result is created after returning Home.
- Review permission readiness selects Settings → General and displays MVP-11's existing checklist. No setup state is written.
- Added responsive styles with existing neutral tokens, card radius, button patterns, focus rings, and reduced-motion button rules. No new dependency or frontend framework was added.
- Added seven integration tests in `src/app/App.test.tsx` for Home CTA navigation, Welcome copy/disclaimer/readiness, focus handoff, permission checklist and tabs, result/acknowledgment reset, distinct result IDs, and native/network/clipboard/keyboard/persistence API tripwires.
- Updated README setup and troubleshooting instructions and added this conservative MVP-12 note to `docs/TASKS.md`. No production task or acceptance checkbox was changed.

## Verification

Commands were run from `/private/tmp/outflow-mvp-welcome-preview` on macOS 26.6.2 arm64.

| Command | Result |
|---|---|
| `npm ci` | PASS |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run` | PASS — 19 files, 290 tests |
| `npm run build` | PASS |
| `npm run dev -- --host 127.0.0.1` | PASS — Vite started for browser inspection and was stopped after review |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS — no temporary-directory collision |
| `cargo test --workspace --locked -- --quiet` | PASS — 591 passed across unit, integration, and doc tests; 0 failed |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS — 629 passed, 1 skipped |
| `git diff --check` | PASS after evidence creation and final edits |

The Python skip is the existing real native ASR integration test. It requires a verified licensed fixture corpus and configured local runner, model, Moonshine source, ONNX Runtime, model assets, and fixture manifest; those eight `OUTFLOW_BENCHMARK_*` variables were absent. The focused test reports this as SKIP, not PASS.

## Browser and accessibility inspection

- Used the local browser preview at its default 1280×720 viewport. Home → Preview setup opened Welcome. The Welcome heading received focus, and the expected privacy statement, prerequisite explanation, disclaimer, and two actions were visible.
- Used keyboard navigation from the focused Home heading: Tab to the Scenario control, Tab to Preview setup, Return to open Welcome; Welcome focus moved to its heading; Tab selected Back to Home; Return returned to Home and focused the Home heading.
- Opened Review permission readiness and confirmed Settings received heading focus, General was selected, the existing Permission readiness checklist was visible and all six Settings tabs remained available.
- Set the browser viewport to 320×800 CSS px. Welcome text and stacked buttons reflowed without horizontal overflow. Read-only measurements were `innerWidth=320`, `documentElement.scrollWidth=320`, and `body.scrollWidth=320`. The temporary viewport override was reset afterward.
- Genuine 200% browser zoom: **NOT_RUN**. The in-app browser's zoom shortcut did not change its scale; desktop Brave browser access was not approved by the computer-use tool. The 320 CSS px check above used a viewport override, not zoom.
- Manual screen-reader, forced-colors/high-contrast, and reduced-motion preference checks: **NOT_RUN**. The view has no new animation and reuses the existing button motion and focus styles.

## Limitations and handoff

This is only the visual introduction step of UF-01. It does not implement privacy consent, permissions, shortcut assignment, microphone tests, first-launch detection, onboarding persistence/completion, Try Dictation, Tauri, or native platform integrations. It does not prove offline runtime behavior, model readiness, AC-01/AC-02/AC-11, or release size/performance. T-010 remains blocked; T-011, T-041, T-054, and all production acceptance criteria remain open.

Next executable action: review this feature branch and its evidence, then implement a later onboarding screen only when its existing requirements and prerequisites are ready.
