# Frontend Task 38 — Privacy Settings overview

**Result:** Implemented and verified as a static frontend overview. This does not complete the repository's privacy, security, or accessibility gates.

```yaml
task_id: frontend-task-38
feature_ids: [F-12, F-13]
flows: [UF-09, UF-10]
acceptance: [AC-11, AC-12]
owner: Codex coordinator
branch: codex/task23-moonshine-build-scope
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: UNCOMMITTED (same HEAD; no commit created)
platform: macOS local frontend preview in Codex in-app browser; no release hardware gate assessed
review: PENDING
```

## Files created or modified for this task

- Created `src/app/PrivacyOverview.tsx` and `src/app/PrivacyOverview.test.tsx`.
- Modified `src/app/App.tsx` and `src/app/App.test.tsx` to render and cover the existing Privacy tab. Existing navigation, panel heading, state ownership, and focus behavior remain in place.
- Modified `src/styles/global.css` with responsive, neutral informational-content spacing and wrapping using existing tokens and surfaces.
- Updated `README.md` to describe the Privacy preview and its use and verification.
- Created this evidence file.

Other files already had uncommitted changes before this task; they were preserved. The repository task `T-038` (vocabulary CRUD) and `docs/TASKS.md` were not changed.

## Requirements represented

- `docs/PRD.md` F-12 / FR-12.1–FR-12.4: intended local/offline operation, no dictated-content retention or tracking, best-effort memory clearing with swap/allocator limitations, and no idle microphone access.
- `docs/PRD.md` F-13 / FR-13.3: intended explicit, optional, signed full-application updates.
- `docs/USER_FLOWS.md` UF-09 and UF-10: discoverable Settings navigation, keyboard behavior, privacy disclosure, offline-use requirements, and destination-app / swap / clipboard-manager limitations.
- `docs/ARCHITECTURE.md` §0 A-INV-01, A-INV-03, and A-INV-08: local processing and no ordinary network traffic, no transcript/audio persistence, and intended offline-first operation.
- `docs/TASKS.md` T-061, T-062, T-063, and T-077 remain incomplete, as do AC-11 and AC-12. The page states requirements and current preview limits; it does not claim the native behaviors were audited or verified.

## Implementation summary

`PrivacyOverview` has no state, effects, backend calls, network requests, storage, timers, or interactive controls. It reuses `SettingsStatusMessage` and presents four semantic sections: Offline processing; What Outflow is designed not to retain; What may be stored; and Clipboard, operating system, and updates. Copy distinguishes intended requirements from current implementation, notes the frontend preview has no microphone recording or speech recognition, and discloses OS memory and clipboard retention boundaries. No certification or absolute privacy guarantee is shown.

The overview accurately describes the current preview: Launch at Login and Clean/Verbatim are in-memory values, the vocabulary draft is unsaved, and refreshing resets them. It does not add storage, collection, networking, monitoring, native integration, clipboard operations, update checks, or diagnostics export.

## Automated verification

Frontend baseline before edits: **109 tests passed**. After changes: **122 tests passed across 9 files** (13 new PrivacyOverview tests).

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 122 passed |
| `npm run lint` | PASS |
| `npm run build` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests (2 core, 70 settings) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |

The workspace contains only `crates/core` and `crates/settings`; the Python configure-runner tests use mocked CMake calls. The blocked Moonshine CMake configuration was not attempted.

## Visual and accessibility verification

- Started `npm run dev -- --host 127.0.0.1`; Vite used port 5175 because 5173 and 5174 were occupied.
- Visually inspected Settings → Privacy at 320px and 900px. `document.documentElement.scrollWidth` matched each viewport width (320 and 900); no horizontal overflow was observed. The 900px view showed the two-column Settings layout, readable content, and neutral status styling.
- Confirmed all four disclosure sections and all six Settings tabs were present.
- Used the section keyboard navigation (End, then ArrowUp); Privacy was selected and held focus. Verified Home and Settings destination headings receive focus on page navigation.
- Confirmed Launch at Login remained `true`, Verbatim remained selected, and both vocabulary draft fields remained unchanged after visiting Privacy. Browser console error entries: none.
- Screen-reader testing: **NOT RUN**.
- Literal 200% zoom: **NOT RUN**. Viewport resizing was kept separate from zoom testing.

## Remaining privacy verification

Native microphone lifecycle, offline speech recognition, network isolation, audio/text retention, transcript-safe logging and crash handling, clipboard restrictions, signed updater behavior, OS memory erasure, release hardware limits, and full application accessibility remain unimplemented or unverified in this preview. This task introduces no data collection, persistence, or network behavior and does not satisfy T-061, T-062, T-063, T-077, AC-11, or AC-12.
