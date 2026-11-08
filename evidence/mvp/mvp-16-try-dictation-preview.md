# MVP-16 — Try Dictation preview

**Status:** Implemented as a browser presentation preview only. This does not complete production onboarding, native dictation, any acceptance criterion, or an M3 task.

```yaml
task_id: MVP-16
feature_ids: [F-02, F-03, F-04, F-05, F-06, F-07, F-09, F-10, F-11, F-12]
flows: [UF-01, UF-03, UF-04, UF-05, UF-07, UF-08]
acceptance: [] # No AC is completed by this browser-only slice.
owner: Codex
branch: feature/mvp-try-dictation-preview
base_commit: fc3a65276b5e32168f5b0608ed376c02eff9948a
implementation_commit: f0fdaf41a6f36b8ff95d990e43783479a9d61eee
platform: Local browser preview; host hardware/resource gates not measured.
review: PENDING
```

## Implementation

Added a transient `try-dictation` App view after Shortcut setup. The page explains the intended Start, Speak, Stop, and Review stages, protected-target and focus-change insertion blocks, and the no-automatic-retry rule for uncertain insertion. It has one focused page heading, Back to Shortcuts, and Return to Home; Home and Settings remain the only primary destinations.

In Vite development mode, `SyntheticDictationExample` reveals a local constant, “So I met North Parkade at three.”, only after an explicit button action. Its read-only textarea remains selectable. A polite live region announces show/clear without announcing the sample. Component-local state unmounts on navigation. This path does not use session state, `CopyResultPanel`, clipboard, microphone, model, insertion, Tauri, storage, or network APIs.

Production uses a compile-time `import.meta.env.DEV` branch. It retains the explanation and renders: “Native dictation is not connected. This example is available only in the development preview.” The build removes the sample component and its developer controls.

Home remains unmounted throughout setup. Its volatile Copy Result and simulated acknowledgment are discarded on leaving Home, while the App-owned lifecycle counter remains mounted and continues to allocate distinct IDs. Returning Home creates a new Home view in its unavailable default. No preference, vocabulary, setup progress, sample selection, speech, or transcript is persisted.

## Automated verification

| Command | Result |
|---|---|
| `npm ci` | PASS — locked dependencies installed; no manifest or lockfile changes. |
| `npm run typecheck` | PASS |
| `npm run lint` | PASS |
| `npm run test -- --run` | PASS — 20 files, 342 tests. |
| `npm run build` | PASS — production Vite bundle generated; main JavaScript asset 291.29 kB. |
| Production JavaScript scan for fixture, synthetic label, show/clear labels, demonstration heading, and announcement | PASS — all six strings absent from `dist/assets/*.js`. |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked` | PASS — 591 unit/integration tests and 11 doctests; no temporary-directory collision. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS — 629 tests, one existing skip. |
| `git diff --check` | PASS |

The first frontend validation attempt caught a syntax typo in the new test and an existing shortcut-page assertion that incorrectly rejected the newly required navigation button. Both were corrected before the final passing frontend run.

## Browser verification

- `npm run dev -- --host 127.0.0.1` served the development build at `http://127.0.0.1:5173/`.
- Followed Home → Welcome → Privacy → Permissions → Shortcuts → Try Dictation. Each destination heading received focus.
- Show revealed the exact constant in the labeled read-only field; Clear removed it. Back to Shortcuts → Continue to Try Dictation reopened with no sample. Return to Home focused Home and restored the unavailable default.
- At 1280 × 720 CSS px, `document.documentElement.scrollWidth` and `document.body.scrollWidth` were both 1280. At 320 × 1000 CSS px, both were 320; the example and safety copy wrapped, and actions stacked without horizontal overflow.
- Keyboard smoke used Enter to navigate and activate Show, Tab moved from Show to Clear and then Back to Shortcuts, Enter navigated back, and the Shortcut heading received focus.
- The production preview (`npm run preview -- --host 127.0.0.1`) rendered the required not-connected message, omitted the synthetic controls and sample, and focused the Try Dictation heading.

### Manual checks not run

| Check | Result | Reason |
|---|---|---|
| Genuine 200% browser zoom | NOT_RUN | The in-app browser zoom shortcut did not change browser scale; native Brave inspection was denied by computer-use approval. |
| Screen-reader speech output | NOT_RUN | The browser accessibility tree was inspected, but no VoiceOver/Narrator/Orca speech journey was run. |
| Reduced-motion preference toggle | NOT_RUN | The in-app browser did not expose a system reduced-motion override. The new page adds no animation. |

## Remaining limitations and task status

No native microphone, permissions, shortcuts, ASR, cleanup, target checks, insertion, clipboard, offline-network, platform, accessibility certification, 4 GB hardware, installed-size, or RSS gate was exercised. T-010 remains **BLOCKED**. T-011, T-032, T-033, T-034, T-041, T-043–T-048, T-054, T-055, related native tasks, and release criteria remain open. No production task or acceptance checkbox was changed.
