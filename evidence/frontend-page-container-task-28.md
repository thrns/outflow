# Frontend Task 28 — Shared page container evidence

```yaml
task_id: Frontend Task 28 (not repository T-028)
feature_ids: [F-11]
flows: [UF-09]
acceptance: [AC-11]
owner: Codex on the existing working tree
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: uncommitted
platform: macOS 26.6.2, Darwin 25.6.0, arm64; installed RAM NOT_MEASURED
toolchain: Node.js v22.23.3, npm 10.9.9, rustc 1.99.1 (stable), Python 3.13.13
implementation: Added a neutral PageContainer used by Home and Settings; consolidated duplicated 56rem page width, responsive gutters, and vertical spacing while retaining the Settings navigation/panel breakpoint.
commands:
  - {command: 'npm run dev -- --host 127.0.0.1', result: PASS, notes: 'Vite served http://127.0.0.1:5174/ because port 5173 was already in use.'}
  - {command: 'npm run typecheck', result: PASS, notes: 'TypeScript check completed.'}
  - {command: 'npm run test -- --run', result: PASS, notes: '22 frontend tests passed (20 existing plus 2 focused container tests).'}
  - {command: 'npm run lint', result: PASS, notes: 'ESLint completed.'}
  - {command: 'npm run build', result: PASS, notes: 'Typecheck and Vite production build completed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'Formatting check completed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Workspace check completed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '72 Rust tests passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'Clippy completed without warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build completed.'}
  - {command: 'python3 -m unittest discover -s scripts/tests -v', result: PASS, notes: '226 Python tests passed.'}
  - {command: 'git diff --check', result: PASS, notes: 'Whitespace check completed after documentation update.'}
metrics: 'At 320x850: page container width 320px, 16px gutters, document/body width 320px. At 900x900: page container width 900px, 32px gutters, document/body width 900px. Settings used one column at 320px and 176px + 628px columns with a 32px gap at 900px. Literal 200% zoom NOT RUN.'
doc_updates: 'README.md frontend Task 28 instructions; this evidence file. docs/TASKS.md unchanged; do not mark AC-11 complete.'
risks: 'Browser preview only. Native VoiceOver, high-contrast/forced-colors, a separate reduced-motion preference run, and literal 200% browser zoom were NOT RUN. Hardware/resource and native Tauri gates were not part of this UI refactor.'
review: PENDING
```

## Implementation

- Added `src/app/PageContainer.tsx`, a small `ReactNode` wrapper with no variants or extra semantics.
- Home and Settings render the same container. It includes the shared header, consistent with the prior shell structure. `App.tsx` retains navigation, heading focus, and Launch at Login state.
- Moved the identical 56rem centered shell, responsive horizontal gutter, and vertical page spacing declarations into `.page-container` in `src/styles/global.css`. Existing 16px–32px page gutters and inner content width are preserved. Settings retains its existing one-column mobile layout and navigation/panel grid from 48rem upward.
- Added one direct rendering test and one integration test for both pages using one shared container inside exactly one `main`. Existing tests continue to cover both navigation directions, heading focus, Launch at Login retention, and the six Settings sections.
- Preserved the initially dirty working tree and layered changes into previously modified shared files. No dependencies, lockfiles, Rust/native code, or `docs/TASKS.md` were changed. No files were staged or committed.

## Browser verification

- Inspected Home and Settings in the local Vite preview at 320×850 and 900×900. Screenshots were captured and visually inspected; text, cards, navigation, and Settings controls remained readable, with no clipping or visible overflow.
- `document.documentElement.scrollWidth` and `document.body.scrollWidth` matched the viewport at both widths. Each active page exposed exactly one `main` landmark and one `.page-container`.
- At 320px, Settings sections and panel stacked. At 900px, the existing navigation-plus-panel layout remained side by side.
- Opened all six Settings sections and confirmed their matching headings. Keyboard Enter navigation worked Home→Settings→Home, focus moved to each destination heading, and the computed heading outline was visible. Launch at Login was toggled On with Space and remained On after navigating Home and back to Settings.
- Browser console error log was empty.
- Literal 200% zoom was **NOT RUN** because the preview exposed no zoom capability. A responsive viewport test was not counted as a zoom test.

## Scope and status

- AC-11 remains unverified for the native application; this browser refactor does not establish screen-reader announcements, Tauri webview lifecycle, or native settings behavior.
- Native VoiceOver testing, forced-colors/high-contrast, a separately emulated reduced-motion preference, and literal 200% browser zoom remain **NOT RUN**.
- This requested frontend Task 28 is distinct from repository task T-028, which concerns the bounded SPSC audio buffer. The repository task ledger was not changed.
- Existing Rust and Python checks passed but do not prove hardware, platform, audio, model, packaging, or release gates.
- No Moonshine CMake configuration was attempted.
