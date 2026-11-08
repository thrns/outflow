# MVP-14 Permissions explanation step preview

**Status:** Browser preview implemented; automated checks pass. Manual visual walkthrough is `NOT_RUN` because no approved browser surface was available.

```yaml
milestone: MVP-14
features: [F-01, F-02, F-03, F-09, F-10, F-11, F-12]
flows: [UF-01, UF-02, UF-08, UF-09]
acceptance: [AC-01, AC-02, AC-03, AC-09, AC-10, AC-11, AC-12]
owner: Codex coordinator
branch: feature/mvp-permissions-step-preview
worktree: /private/tmp/outflow-mvp-permissions-step-preview
base_commit: 99092009ca8e85b611c9cb35d41c79497d517de4
implementation_commit: PENDING
platform: macOS 26.6.2 arm64; available RAM NOT_MEASURED
review: PENDING
```

## Implementation

- Added `PermissionsStepPage.tsx` with the required `Permissions for Outflow` heading, an explicit unconnected-preview disclaimer, and separate explanations for microphone, global shortcuts, and text insertion.
- Explained that microphone permission and device availability differ; push-to-talk needs reliable matching press/release events; insertion capability does not establish target eligibility; text-insertion permission is optional for a future Copy-only path while microphone permission remains necessary for recording.
- Reused `PermissionReadinessChecklist` without copying its types, scenarios, status labels, or recovery policies. Its default remains unverified and its Vite development fixtures remain local component state.
- Extended transient `AppView` navigation and destination-heading focus for Permissions. Privacy now offers **Continue to permissions** while preserving **Back to Welcome** and **Review permission readiness**. Permissions offers **Back to Privacy** and **View permission details in Settings**. Primary navigation remains Home and Settings.
- Kept Home conditionally unmounted throughout Welcome, Privacy, and Permissions. The App-root Copy Result lifecycle counter remains in memory; Home result and synthetic acknowledgment state are discarded. Setup and Settings checklist instances remain independent.
- No new dependencies, native APIs, permission prompts, microphone access, keyboard listeners, clipboard access, network calls, storage calls, saved setup progress, production task checkboxes, or acceptance checkboxes were added or changed.

## Changed files

- `src/app/PermissionsStepPage.tsx`
- `src/app/App.tsx`
- `src/app/AppHeader.tsx`
- `src/app/PrivacyStepPage.tsx`
- `src/app/App.test.tsx`
- `src/styles/global.css`
- `README.md`
- `docs/TASKS.md` (narrative handoff only; task and acceptance checkboxes unchanged)
- `evidence/mvp/mvp-14-permissions-step-preview.md`

No dependency manifest or lockfile changed.

## Automated verification

Environment: Node.js `v22.23.3`, npm `10.9.9`, Rust/Cargo `1.99.1`; macOS `26.6.2`, arm64. RAM was not measured; the system query was denied by the sandbox. No resource or release gate is inferred from these checks.

| Command | Result | Notes |
|---|---|---|
| `npm ci` | PASS | Installed the lockfile-pinned frontend packages. |
| `npm run typecheck` | PASS | TypeScript check completed without diagnostics. |
| `npm run lint` | PASS | ESLint completed without diagnostics. |
| `npm run test -- --run` | PASS (final) | 19 test files, 309 tests passed. The first run had two new assertion-query failures; the assertions were corrected and the final full run passed. |
| `npm run build` | PASS | Vite production build completed; 40 modules transformed. |
| Production selector scan (command below) | PASS | The generated production JavaScript omits the development selector label and control ID. |
| `npm run dev -- --host 127.0.0.1` | PASS (server start) | Vite started at `http://127.0.0.1:5173/`; the server was stopped after the browser-surface check. |
| `cargo fmt --all -- --check` | PASS | Workspace formatting check. |
| `cargo check --workspace --locked` | PASS | Workspace compiled. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | No warnings. |
| `cargo test --workspace --locked` | PASS | Workspace unit, integration, and doctests passed. A separate `cargo test --workspace --locked -- --list` enumerated 591 Rust test cases. No temporary-directory collision occurred, so serialized rerun was not needed. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py' -v` | PASS | 629 tests ran; 1 skipped. |
| `git diff --check` | PASS | Final pre-commit whitespace check; repeat after the evidence SHA is recorded. |

Production selector scan command:

```sh
if rg -q 'Developer scenarios \(simulated fixtures\)|permission-readiness-scenario-select' dist/assets/*.js; then rg -n 'Developer scenarios \(simulated fixtures\)|permission-readiness-scenario-select' dist/assets/*.js; exit 1; else printf 'Production bundle omits the developer selector label and control ID.\n'; fi
```

The App integration tests exercise Privacy → Permissions, destination heading focus, both Permissions actions, Settings General focus, the preserved direct Privacy → Settings route, primary navigation, checklist default and synthetic fixture behavior, separate Settings fixture state, Home result/acknowledgment discard, and distinct later result IDs. Existing frontend tests for Home, Welcome, Privacy, Settings, Microphone, Shortcuts, and the checklist passed in the full 309-test run. The checklist's existing capability tests verify that unknown release support does not imply working push-to-talk, target capability does not imply target eligibility, and Copy-only recovery does not enable a Copy action.

## Visual and native checks

| Check | Result | Notes |
|---|---|---|
| Home → Welcome → Privacy → Permissions → Back to Privacy in a browser | NOT_RUN | Automated React Testing Library navigation and focus assertions passed. No browser provider was available to CUA; Brave access was not approved and the in-app browser was unavailable. |
| Permissions → Settings → General in a browser; direct Privacy → Settings path | NOT_RUN | Automated route/action and heading-focus assertions passed. |
| Desktop visual inspection | NOT_RUN | Browser surface unavailable. |
| 320 CSS-pixel viewport and page-overflow measurement | NOT_RUN | Browser surface unavailable; no claim inferred from JSDOM tests. |
| Genuine 200% zoom/reflow | NOT_RUN | Browser surface unavailable. |
| Manual keyboard-only browser walkthrough and reduced-motion visual check | NOT_RUN | RTL verifies native button actions and focus handoff; actual browser keyboard/motion inspection was unavailable. |
| Screen-reader testing | NOT_RUN | No screen-reader test was performed; DOM semantics are not represented as assistive-technology validation. |
| Native permission, microphone, shortcut, target insertion, clipboard, network, and persistence behavior | NOT_RUN / out of scope | This is an unconnected browser presentation. No native capability provider exists on this screen. |

T-010 remains blocked. T-011, T-025, T-032, T-033, T-041, T-054, T-055 and the relevant acceptance/release gates remain open. No hardware, native, accessibility certification, installed-size, peak-RSS, latency, or recognition-quality result is claimed.
