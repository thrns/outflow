---
task_id: MVP-10
feature_ids: [F-02, F-03, F-04, F-11]
flows: [UF-02, UF-03, UF-04, UF-09]
acceptance: [AC-03, AC-04, AC-11]
owner: Codex / feature/mvp-shortcut-status-ui worktree
base_commit: b6f3e8a905880a2eabecd67f408dd98a481e871d
result_commit: 80566ad2ab212f92f09631f4e2df49a96bc6487b
platform: macOS 26.6.2 (25G83), arm64; Node v22.23.3; npm 10.9.9; Rust 1.99.1; in-app browser preview
implementation: Browser-only shortcut capability presentation, fixed development scenarios, accessibility coverage, docs and task progress note.
doc_updates: README.md; docs/TASKS.md; evidence/mvp/mvp-10-shortcut-status-ui.md
risks: Remote base branch head could not be independently checked because github.com DNS resolution failed. No native shortcut, microphone, OS permission, physical resource, or release gate was exercised. 200% browser zoom was NOT_RUN because the in-app browser exposed no working zoom control.
review: PENDING
---

# MVP-10 — Shortcut capability and recording-mode UI

## Outcome

Settings → Shortcuts now presents registration state, reliable matching press-and-release capability, and recording readiness as separate facts. The default browser preview says “Global keyboard shortcuts are not connected.” It shows registration and release capability as not checked, says no shortcut is registered by the preview, and says no recording mode is operational.

`ShortcutCapabilityPanel` explains push-to-talk and toggle, keeps push-to-talk unavailable unless release support is known, and describes toggle as only a possible fallback when release is unsupported. Native operational wording requires native registration and recording readiness; push-to-talk additionally requires supported release events. Conflict, authorization, unsupported and failure guidance are distinct. The optional status callback runs only after explicit activation and is disabled without a connected adapter.

`ShortcutScenarioPreview` exposes seven fixed simulated cases only in development builds. Scenario changes update local React state. No shortcut binding, keyboard listener, microphone, Tauri, clipboard, network, recording, persistence, or capability detection was added. No Rust files or dependencies changed. T-010 remains blocked; T-032, T-033, T-052, T-055 and AC-03, AC-04, AC-11 remain open.

## Files

- `src/app/ShortcutCapabilityPanel.tsx` — typed presentation contract, capability rows, mode explanations, recovery guidance and explicit status action.
- `src/app/ShortcutScenarioPreview.tsx` — development-only fixed scenarios.
- `src/app/App.tsx`, `src/styles/global.css` — Settings integration and responsive semantic-token styling.
- `src/app/ShortcutCapabilityPanel.test.tsx`, `src/app/ShortcutScenarioPreview.test.tsx`, `src/app/App.test.tsx` — component, scenario, production-mode and Settings regression coverage.
- `README.md`, `docs/TASKS.md` — run instructions and conservative progress note.

## Automated verification

| Command | Result |
|---|---|
| `npm ci` | PASS — installed locked frontend dependencies; no dependency files changed. |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run` | PASS — 18 test files, 258 tests. |
| `npm run build` | PASS — production build; JS 270.59 kB (81.15 kB gzip), CSS 27.24 kB (4.27 kB gzip). |
| Production bundle scenario-string scan (`rg` over `dist/assets/*.js`) | PASS — no shortcut scenario inspector strings found. |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS — 591 tests/doctests, zero failures. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS — 629 tests, 1 skipped. |
| `git diff --check` | PASS |

## Visual and interaction inspection

- Inspected the default and all six non-default scenarios at a 1280 × 720 CSS-pixel desktop viewport, including release supported, release unsupported, conflict, authorization unavailable, unsupported capability, and registration failure.
- Inspected the default and release-unsupported scenario at 320 × 900 CSS px. `document.documentElement.scrollWidth` and `document.body.scrollWidth` were both 320; no horizontal overflow was observed.
- Used the scenario selector with keyboard ArrowDown and Enter. Selection changed, focus remained on the selector, and the polite status text updated.
- 200% browser zoom: NOT_RUN. The in-app browser did not expose a working zoom control; no PASS is claimed.
- Native OS behavior, screen-reader behavior, physical hardware, aggregate RSS and installed release size: NOT RUN / NOT MEASURED.

## Repository and Git handoff

- Feature worktree: `/private/tmp/outflow-mvp-shortcut-status-ui`.
- Branch: `feature/mvp-shortcut-status-ui`.
- Implementation commit: `80566ad2ab212f92f09631f4e2df49a96bc6487b` (evidence is recorded in the following documentation commit).
- Base commit was verified locally as `b6f3e8a905880a2eabecd67f408dd98a481e871d`; the worktree was created directly from that commit. The original dirty checkout was preserved.
- Remote base branch verification: BLOCKED. `git ls-remote https://github.com/thrns/outflow.git refs/heads/feature/mvp-microphone-status-ui` failed with `Could not resolve host: github.com`.
- The requested `git push -u origin feature/mvp-shortcut-status-ui` succeeded and created the remote branch at `49279575229a48de51e89a45b6adb26a1085c705`.
- A subsequent `git ls-remote` for the old base branch and new feature branch again failed with `Could not resolve host: github.com`, so no independent post-push fetch was possible. The successful push output and updated local tracking ref report `4927957` as the pushed tip.
- Task checkboxes remain open. T-010 remains blocked.
