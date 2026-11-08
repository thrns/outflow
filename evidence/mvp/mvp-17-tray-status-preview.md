# MVP-17 — Tray and App Status preview

**Status:** Implemented as a browser explanation of the planned Tray Ready stage. It does not connect a native tray or complete production onboarding, any acceptance criterion, or an M3 task.

```yaml
task_id: MVP-17
feature_ids: [F-01, F-02, F-03, F-04, F-11, F-12]
flows: [UF-01, UF-02, UF-03, UF-04, UF-09]
acceptance: [] # Related acceptance criteria were reviewed; none is completed by this preview.
owner: Codex
branch: feature/mvp-tray-status-preview
worktree: /private/tmp/outflow-mvp-tray-status-preview
base_commit: 9e0aecd519774c9f0dceb202051fabe23d44937b
base_verification: GitHub feature/mvp-try-dictation-preview branch tip and local worktree both matched the requested SHA.
result_commit: included in the commit recorded in the task handoff
platform: macOS Darwin 25.6.0 arm64; Node.js v22.23.3; npm 10.9.9; Rust 1.99.1; Cargo 1.99.1; host model/RAM NOT_MEASURED (system query denied in sandbox).
review: PENDING
```

## Implementation

Added a transient `tray-status` App view after Try Dictation. The page uses the existing page container, AppHeader, typography, and semantic tokens. It has one focused `h1`, three `h2` sections, a text status that says system tray integration is not connected, and an informational list of the planned Start / Stop, Open Settings, Status / Permissions, and Quit functions. The list contains no working menu actions.

The page separately lists valid bundled offline speech-model assets and working recognition; microphone permission and input-device availability; native recording and session lifecycle integration; an implemented shortcut or other recording control; safe target handling or explicit Copy-only fallback; and platform integration and testing. It distinguishes opening Settings from being able to record, states that a registered shortcut alone does not prove recording readiness, and preserves the no-forced-insertion rule for unsupported or protected destinations.

The background section describes the future native requirement to keep the core and tray active while releasing resources owned by a closed Settings window. It says the browser tab does not preserve a native service and that this preview creates no process or tray icon and manages no microphone or native window lifecycle.

Navigation adds **Continue to tray overview**, **Back to Try Dictation**, and **Return to Home**. Destination headings receive focus. Home remains unmounted through setup, so volatile Copy Result and simulated acknowledgment state are discarded; its App-owned lifecycle ID counter remains mounted. Leaving Try Dictation unmounts the synthetic example, and returning opens with it hidden. Settings preference and vocabulary preview state stay App-owned. No setup completion, tray state, transcript, or session data is persisted.

## Automated verification

| Command | Result |
|---|---|
| `npm ci` | PASS — 204 locked packages installed; no package manifest or lockfile diff. |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run` | PASS — 21 files, 355 tests. |
| `npm run build` | PASS — Vite transformed 44 modules; JavaScript 296.08 kB and CSS 34.31 kB. |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS — 591 unit/integration tests and 11 doctests. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS — 629 tests; one existing skip. |
| `git diff --check` | PASS |

The normal Rust test command had no temporary-directory collision. A serialized rerun was **NOT_RUN** because its triggering failure did not occur.

Focused Vitest coverage checks the new route, page semantics and copy, informational versus interactive controls, readiness wording, safe insertion limits, native background lifecycle language, heading focus, sample reset, Home reset, primary navigation, Settings section selection, preview-only API boundaries, and absence of completion state. Existing tests continue to verify distinct Copy Result lifecycle IDs and retained Settings preference/vocabulary preview state across navigation.

## Browser verification

- `npm run dev -- --host 127.0.0.1` served the local preview at `http://127.0.0.1:5173/`.
- Followed **Home → Welcome → Privacy → Permissions → Shortcuts → Try Dictation → Tray and App Status**. The accessibility tree showed each destination heading focused; the Tray page had one `h1`, both named sections and lists, the not-connected statement, and only Home and Settings in primary navigation.
- From Try Dictation, revealed the fixed synthetic example, continued to Tray, returned to Try Dictation, and confirmed its example was hidden and its heading focused. Returned Home and confirmed the Home heading focused, the scenario was `unavailable`, and no Copy Result panel returned.
- Keyboard smoke on the Tray page: Tab moved to **Back to Try Dictation**, the next Tab moved to **Return to Home**, and Enter activated it and focused the Home heading.
- At desktop width, the full-page screenshot showed the page content and both actions within the existing centered layout. The browser viewport was 1280 CSS px wide with no horizontal document overflow.
- At 320 × 900 CSS px, `window.innerWidth`, `document.documentElement.clientWidth`, `document.documentElement.scrollWidth`, and `document.body.scrollWidth` were all 320. List items wrapped within a 264 px content width and both action buttons fit in 288 px; the full-page screenshot showed no horizontal overflow.
- The browser accessibility tree was inspected for heading order, labels, informational lists, and focus. The page adds no animation; the shared navigation controls remain covered by the existing `prefers-reduced-motion: reduce` CSS rule that removes transitions and active transforms.

### Manual checks

| Check | Result | Notes |
|---|---|---|
| Desktop layout and complete route | PASS | Inspected in the local browser preview. |
| 320 CSS px width and overflow | PASS | Measured viewport, document widths, list-item widths, and button widths. |
| Keyboard navigation and destination focus | PASS | Tab and Enter smoke; AX focus and active heading checked. |
| Genuine 200% browser zoom | NOT_RUN | The in-app browser did not expose a zoom control. Computer-use approval denied opening Brave Browser, so genuine zoom could not be inspected. |
| Reduced-motion preference override | NOT_RUN | The page adds no motion and shared CSS has the reduced-motion rule; the browser did not expose a preference override for an in-browser verification. |
| Spoken screen-reader output | NOT_RUN | The accessibility tree was inspected, but no VoiceOver/Narrator/Orca speech journey was available. |

## Remaining limitations and task status

No real tray, background process, native readiness probe, microphone, permission prompt, keyboard capture or shortcut registration, model recognition, text insertion, clipboard, telemetry, storage, or network operation was added or exercised. Release hardware, ASR quality/latency, install size, RSS, offline installer, platform integration, and accessibility certification remain unmeasured. **T-010 remains blocked.** T-011, T-021, T-025, T-032–T-034, T-041, T-048, T-050, T-051, T-054, T-055, related native work, and release gates remain open. No production task or acceptance checkbox was changed.
