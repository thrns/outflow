# Frontend SettingsSwitch — Task 31

## Implementation

Extracted the existing native button switch into `src/app/SettingsSwitch.tsx` as a controlled reusable component. Its props are `checked`, `onCheckedChange`, `labelledBy`, and optional `describedBy`. The component retains `type="button"`, `role="switch"`, `aria-checked`, the supplied label/description associations, and the existing `settings-switch` / track / thumb CSS classes. Space toggles on keydown once per press, while native button Enter activation remains available. No installed switch primitive or disabled behavior existed in this implementation.

General Settings composes `SettingsRow` and `SettingsSwitch`; `App.tsx` retains the `run_at_login: false` preview state and passes its value and state update callback. No Rust settings or persistence behavior changed. The CSS already used reusable switch-specific class names, so its rules and appearance were left unchanged.

## Files changed for Task 31

- `src/app/SettingsSwitch.tsx` — controlled reusable switch.
- `src/app/SettingsSwitch.test.tsx` — eight focused component tests.
- `src/app/App.tsx` — Launch at Login now renders `SettingsSwitch` inside `SettingsRow`.
- `src/app/App.test.tsx` — fresh application mount starts Launch at Login OFF.
- `README.md` — component contract, composition, state ownership, commands, visual verification, and troubleshooting.
- `evidence/frontend-settings-switch-task-31.md` — this record.

The existing worktree also contained unrelated uncommitted files before Task 31 began. Those changes were preserved and are not attributed to this task.

## Automated verification

Environment: macOS development host, Apple Silicon arm64. No Rust, package-lock, or Cargo manifest changes were made for this task.

| Command | Result |
|---|---|
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 43 tests across 5 files (34 existing + 8 component + 1 fresh-mount integration test) |
| `npm run lint` | PASS |
| `npm run build` | PASS — Vite production build completed |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 72 tests (2 core + 70 settings) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 226 tests |
| `git diff --check` | PASS (final review) |

## Manual browser verification

Started the local Vite preview using `npm run dev -- --host 127.0.0.1 --port 5173` after the sandbox denied the initial local bind; stopped the server after verification.

- General Settings showed the Launch at Login row, label, description, Off state, and switch at its existing size and alignment.
- Mouse click changed Off → On. Space changed On → Off. Enter changed Off → On using native button activation.
- The keyboard focus ring was visible on the switch.
- The value remained On after switching Settings sections and after Home → Settings navigation.
- After a full browser reload, General Settings showed Off.
- All six Settings sections were navigable; Home and Settings navigation remained available.
- Checked the appearance at the available browser viewport (about 900 CSS pixels wide).
- **NOT RUN:** literal 320px viewport; the browser session exposed no viewport override. Literal 200% browser zoom was also **NOT RUN**.
- Screen-reader, high-contrast, and OS-level accessibility checks were **NOT RUN**.

## Limitations

Launch at Login remains a frontend-only in-memory preview and does not configure the operating system. Browser refresh was checked manually; the automated integration test verifies reset on a fresh App mount, not a real browser reload. No changes were made to mark AC-11 complete. The blocked Moonshine CMake configuration was not attempted.
