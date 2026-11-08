# MVP-09 — Microphone Capability and Permission Presentation

```yaml
task_id: MVP-09
feature_ids: [F-02, F-05, F-11, F-12]
flows: [UF-02, UF-09]
acceptance: [AC-02, AC-11, AC-12]
owner: Codex, feature/mvp-microphone-status-ui
base_commit: 387b6a6faaaf97e61a34bedcc23ee6edd1d3d615
result_commit: f4a0e8fc97515532b9bcea3cb93c24097dff30f3
platform: macOS 26.6.2, arm64; development host, not release hardware
implementation: Browser-only microphone status panel and development-only local scenario inspector; see src/app/MicrophoneCapabilityPanel.tsx and src/app/MicrophoneScenarioPreview.tsx
metrics: Release RAM, installed size, ASR quality, and native microphone measurements NOT_MEASURED
review: PENDING
```

## Result

Settings → Microphone now presents permission capability, input-device availability, native integration connection, and recovery guidance as separate facts. The default says **“Microphone status has not been checked.”** A simulated available permission and device never imply active recording or connected native integration. Retry and System Settings controls require connected callbacks and remain disabled in this browser preview.

The Vite development inspector offers fixed Unknown/default, available/device-present, denied, user-action-required, unsupported, no-device, and disconnected scenarios. Scenario changes are local React state only. A production build removes the scenario selector and fixture labels from its assets. The preview does not request browser or OS permission, enumerate devices, start recording, call Tauri, use clipboard APIs, or make network requests.

`README.md` documents setup, expected states, scenario inspection, and troubleshooting. `docs/TASKS.md` adds the MVP-09 implementation note only; no T-xxx checkbox or native/release acceptance state changed. T-010 remains blocked.

## Verification

| Command or inspection | Result |
|---|---|
| `npm ci` | PASS; 204 packages installed; package manifests and lockfile unchanged |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run` | PASS; 16 files, 231 tests |
| `npm run build` | PASS |
| Production asset string check for developer scenario labels | PASS; selector and fixture labels absent |
| `npm run dev -- --host 127.0.0.1` | PASS; opened local preview and stopped server after inspection |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS; 591 tests and doctests, 0 failures |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS; 629 tests, 1 skipped |
| `git diff --check` | PASS |
| Browser API tripwire tests across scenarios | PASS; no microphone, permissions, Tauri, clipboard, fetch, or XHR call |

## Visual and interaction inspection

- Desktop in-app browser preview at 1280 CSS px: inspected default, available/device-present, denied, user-action-required, unsupported, and disconnected-device states. Status text and recovery controls remained readable and distinct.
- Responsive inspection at exactly 320 CSS px: `window.innerWidth=320`, document `scrollWidth=320`; no horizontal overflow. Status items stacked into one column.
- Keyboard: native scenario selector changed values with ArrowDown/Return; focus remained on the selector. Settings ArrowDown changed Microphone to Shortcuts and moved focus to that tab; Shift+Tab returned to the selected tab. Existing Home/Settings heading-focus tests passed.
- Status semantics, labels, descriptions, disabled action explanations, and polite `role="status"` announcement are covered by component tests. VoiceOver/screen-reader execution: NOT_RUN.
- 200% browser zoom: NOT_RUN. The in-app browser did not respond to Meta+Plus or Meta+Shift+Equal; its reported width and scale stayed unchanged. The 320 CSS px viewport check was recorded separately and was not used as a zoom substitute.

## Limitations and next work

No native permission query, system-settings opener, device enumeration, device selection, recording, or audio capture is connected. Scenario values are presentation fixtures, not OS or hardware observations. `T-010` remains blocked; T-052, T-055, AC-02, AC-11, native microphone/platform integration, physical release resource checks, and all release gates remain open. Native permission/device and assistive-technology acceptance must be completed by their owning tasks on supported platforms.
