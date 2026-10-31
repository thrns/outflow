# AGENTS.md — Codex Engineering Contract for Offline Desktop Dictation

> **Status:** Normative repository-root instructions for OpenAI Codex and collaborating coding agents. **Date:** 2026-10-29. **Scope:** Entire repository unless a narrower, explicitly approved `AGENTS.md` adds compatible module-specific rules. **Owner:** Project maintainer. **Read this before editing code.**
>
> **Repository documents:** The project authorities are [`docs/PRD.md`](docs/PRD.md), [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md), [`docs/TECH_STACK.md`](docs/TECH_STACK.md), [`docs/TASKS.md`](docs/TASKS.md), [`docs/USER_FLOWS.md`](docs/USER_FLOWS.md), and [`docs/DESIGN.md`](docs/DESIGN.md). Read the relevant sections before starting work. The current repository's actual code, branch layout and build scripts must be inspected at execution time; they were not supplied with this authoring request. **Do not invent existing modules, commands, passing tests, installed component APIs, or completed tasks.**

## 0. Non-negotiable contract

1. **Inspect before editing:** Confirm Git root, worktree/branch/dirty state, nested agent instructions, relevant requirement/task IDs and actual code. Never claim unseen source exists.
2. **Product invariants:** Proprietary Rust/Tauri 2 desktop dictation on macOS, Windows and Linux. **100% local** inference/cleanup. One verified English streaming ASR model **bundled with installer**; no model downloads/chooser, accounts, cloud AI, telemetry, transcript/audio history, or generative LLM in MVP. Offline from first launch; updater is explicit opt-in.
3. **Release caps:** Physical **4 GB RAM CPU-only** hardware; <1,000,000,000 bytes installed size AND aggregate app-process peak RSS; **<950,000,000 bytes** release thresholds. Targets are unmeasured until proven. Benchmark quantized Moonshine Small then preapproved Tiny if needed; block release if no model meets size, speed **and** quality gates.
4. **Safe insertion:** Never insert into password/protected/elevated or focus-changed targets, bypass OS permissions, or claim unconditional Wayland support. Use in-memory Copy Result with **explicit Copy**, not silent clipboard overwrite.
5. **[`docs/DESIGN.md`](docs/DESIGN.md) is the sole UI/UX authority.** Read it before modifying UI; no conflicting components, tokens, layout, states or interaction without approval. Installed source determines actual available API props.
6. **Proof before completion:** Tests, lint/format/build as applicable, diff review, evidence, synchronized documentation and durable handoff. No fake completed tests/platforms.
7. **Collaboration:** Separate task-owned branches/worktrees, no overwriting collaborators; coordinator serializes shared files.
8. **Approval:** Proceed on scoped reversible changes. Ask for architecture, dependency, privilege, policy, destructive Git, external communication/PR/push or release operations.
9. **Existing GitHub repository; no GitHub Actions.** Never add `.github/workflows/*` or assume CI exists. Use local checks.

Preflight: `git rev-parse --show-toplevel`; `git status --short --branch`; `git worktree list`; search `AGENTS.md`, workspace manifests, lockfiles and toolchain. If no repository is mounted, do not fabricate paths or commit histories.

## 1. Documentation hierarchy and change control

| File | Canonical authority |
|---|---|
| `AGENTS.md` | Agent conduct, scope, safety, autonomy, validation and handoffs |
| [`docs/PRD.md`](docs/PRD.md) | Product scope, features **F-xx**, acceptance **AC-xx**, release gates |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Components **C-xx**, trust boundaries, APIs, state and OS integration |
| [`docs/TECH_STACK.md`](docs/TECH_STACK.md) | Approved stack/dependencies **TS-xx**, versions and licensing |
| [`docs/USER_FLOWS.md`](docs/USER_FLOWS.md) | User journeys **UF-xx**, behavior, failures/recovery |
| **[`docs/DESIGN.md`](docs/DESIGN.md)** | **Single UI/UX source of truth**: components, visual tokens, layout, interactions, responsiveness, accessibility |
| [`docs/TASKS.md`](docs/TASKS.md) | Dependencies, milestones, evidence, progress **T-xxx** |

Authority is **by subject**, not a general numeric ranking: this file governs agents; `PRD` wins for product; `DESIGN` for UI; `ARCHITECTURE` for design boundaries; `TECH_STACK` for dependencies; `USER_FLOWS` for interaction; `TASKS` for ordering. Authorized direct maintainer/higher-priority instructions override repository guidance. For a cross-document contradiction, **stop the affected slice**, report exact sections/IDs and alternatives, seek decision, and update all affected docs together. Never silently make scope/design changes. Installed code establishes actual API availability; its current behavior does *not* supersede an approved product/design contract. The design's `[S]/[D]/[R]/[U]` provenance labels distinguish source, docs, recommendations and uncertainty.

IDs remain stable: **F-01–F-15**, **AC-01–AC-15**, **UF-01–UF-14**, **C-01–C-10**, **TS-01–TS-10**, and `T-xxx`. Navigate [`docs/PRD.md` §3.1](docs/PRD.md) for F→UF/C/TS/T/AC mapping; verify each linked section. Add/renumber IDs only by approved decision. Behavior changes require synchronized acceptance tests and relevant project documents (`PRD`, `ARCHITECTURE`, `TECH_STACK`, `USER_FLOWS`, `DESIGN` if UI, and `TASKS`) under coordinator ownership.

## 2. Runtime architecture and ownership

**Baseline:** single Tauri process with Rust inference worker thread (not a child process unless measured crash isolation and memory costs justify approved change). Tray/session core lives in background; on-demand settings webview **destroyed** on close. No always-on microphone or webview, cloud or runtime model download. Architecture crate paths below are **proposed**, not verified present in the existing repository; inspect the real workspace first.

| Component | Ownership/invariant |
|---|---|
| C-01 shell/UI | Tauri tray/settings/onboarding, sanitized typed command/events, idle webview lifecycle |
| C-02 session | Sole state machine, shortcuts, focus snapshot, idempotency and cancellation |
| C-03 audio | CPAL capture/format/resampling; **no blocking/allocation/I/O in callback** |
| C-04 buffer | Bounded SPSC queue, no-speech gating and documented overflow |
| C-05 ASR | Moonshine pinned C API, partial/final recognition, audited unsafe FFI |
| C-06 text | Deterministic punctuation/filler/vocabulary, golden tests |
| C-07 OS adapters | Focus/permissions/shortcut/insertion/copy fallback, capability checks |
| C-08 settings | Versioned atomic JSON, vocabulary settings only; no transcript storage |
| C-09 model | Bundled pinned model files, SHA validation, lifecycle; no model picker |
| C-10 diagnostics | Redacted events, opt-in signed full-app updater only |

[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) proposes `core`, `audio`, `speech-api`, `speech-moonshine`, `text`, `platform-api`, `platform-{macos,windows,linux}`, `settings`, `shell`, `release`. Keep pure `core` independent of UI/OS; `platform-*` implements `platform-api`; native unsafety stays inside `speech-moonshine`/OS facades. Stack: pinned Rust, Tauri 2, TypeScript/Vite, CPAL/rubato/ring buffer, candidate Moonshine Small Streaming quantized with Tiny fallback **only after real gate**, `serde`+JSON, `objc2`, `windows-rs`, `ashpd`/X11. Do not add SQLite, LLM, daemon, extra ASR backends, Python runtime or cloud service for MVP. [`docs/TECH_STACK.md`](docs/TECH_STACK.md) version figures are dated research—not permission for ad-hoc upgrades or invented hashes.

## 3. Codex startup, context and task selection

1. Confirm applicable root/nested `AGENTS.md`/`AGENTS.override.md`; do not use nested guidance to weaken root safety/product invariants. Restart Codex after agent-instruction changes.
2. Read [`docs/PRD.md`](docs/PRD.md) §0/§3 and target F/AC; relevant sections in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) (C-xx), [`docs/TECH_STACK.md`](docs/TECH_STACK.md) (TS-xx), [`docs/USER_FLOWS.md`](docs/USER_FLOWS.md) (UF-xx and edge cases), and [`docs/TASKS.md`](docs/TASKS.md) (predecessors/DoD). For UI read [`docs/DESIGN.md`](docs/DESIGN.md) §1–5, §7–11 and exact component sections before coding. Search targeted ranges rather than repeatedly loading its full 2,000+ lines.
3. Inspect actual source, tests, manifests, CSS and local patterns. Create a concise briefing: IDs, owned files, assumptions, implementation plan, validation and blockers. Prefer one independent task or coherent 1–3 task batch.
4. Use bounded searches (`rg`/`git grep`), incremental tests and captured outputs; avoid rereading unchanged files or rerunning identical failures without diagnosis.
5. Before session end/compaction, leave a durable task evidence/handoff. Never rely on chat or hidden model context for future maintenance.

Codex discovers global/root/nested instructions on startup and has a **32 KiB default combined instructions limit**. Keep root rules concise; avoid duplicative nested files, or explicitly configure a larger limit with maintainer approval. Skills under `.agents/skills/` are optional for genuinely repeated procedures. Do not assume subagents/MCP/remote features are enabled; use one agent if unavailable.

## 4. Implementation lifecycle

**Observe → plan → implement → validate → handoff.** Check `T-xxx` prerequisites **with evidence**; inspect branch, existing work, nearby tests and interface owners; reproduce defects where possible. Implement minimal scoped change including expected failures/cancellation. Test before fixing regressions when practical. Never mix unrelated refactors or build speculative features.

Run formatting, static/type checks, focused tests and relevant integration/native smoke after final edits. Inspect `git diff --check`, changed files, secrets, network/logging, Cargo/JS lockfile diffs and mapped AC. Do not mark a hardware/OS gate passing from mocks. Store `evidence/T-xxx.md` with exact commands, results, environment, hashes, unresolved risks, documentation and reviewer state. **Only coordinator** updates centralized [`docs/TASKS.md`](docs/TASKS.md) `[x]` after accepting evidence/merged changes. Block on missing hardware, permissions, security approval or unknown model checksum; report reproducible next step rather than invent proof. Continue independent safe tasks if possible.

## 5. Strict Rust engineering rules

- **Safety and ownership:** Prefer idiomatic safe Rust; isolate `unsafe` in narrow audited modules (particularly Moonshine C FFI and native OS APIs). Every `unsafe` block has a `// SAFETY:` justification defining exact caller/handle/thread/lifetime conditions and the upstream contract. No global `#![allow(unsafe_code)]` escapes or broad lint suppression. Do not use `static mut` for session state.
- **Errors:** Model recoverable error variants (`thiserror` or existing equivalent) at boundaries; use `Result` and `?`. `unwrap`, `expect`, `panic!`, and `unreachable!` are normally forbidden on external/production input paths. An impossible invariant may use a documented `expect` only with a test and justification. Never silently swallow errors; avoid blanket `anyhow` in exported library APIs if typed errors are required.
- **Concurrency:** Keep explicit ownership and cancellation of streams/workers. `tokio` is for async orchestration, never inside the audio callback. No unbounded channels, blocking mutex/allocations, disk access, println or callback-side resampling/ASR. Use bounded pressure and a documented drop/overflow policy. Handle repeated rapid press/release, double-toggle, OS sleep, device removal, and shutdown idempotently.
- **Performance:** No background polling loop when an event callback exists; avoid keeping inference weights loaded unnecessarily; close settings webview when dismissed. Profile before adding caching/thread pools/extra processes. Prove performance with named hardware, process-tree RSS, installed bytes, p50/p95, CPU real-time factor and battery observations where task requires it.
- **Interfaces:** Use explicit typed units (`SampleRateHz`, `Duration`, `TargetIdentity`, etc. as appropriate), versioned serialized contracts, clear thread-safe trait boundaries. Do not introduce a needless abstraction for one call. Keep dependencies directional: pure core cannot import `tauri`, `objc2`, Windows/portal APIs or frontend types.
- **Platform code:** Select with `#[cfg(target_os = "...")]`; never hide platform constraints behind a false universal `supported=true`. Detect capabilities and provide meaningful error/fallback. Validate focus target identity **again immediately before insertion**. No root/elevated daemon, `sudo`, `ydotool` or OS trust bypass for input simulation.
- **Formatting:** `rustfmt` authoritative; Clippy warnings fail the relevant validation. Avoid blanket `clippy::all`/`restriction` suppressions; specific lint exceptions need scope and rationale. Document unsafe APIs, resource ownership, panics/errors, and threading requirements in rustdoc. Match nearby module naming/layout and avoid unsolicited public API breakage.
- **FFI:** Pin matched C header, library binary and model manifest. Test pointer/null/length, `f32` sample counts, UTF-8, native return-code translation, creation/destruction pairing, partial/final transcript ownership, cancellation and double-free safety. Validate upstream ABI at build time; do not call model download helpers at consumer runtime. Use sanitizers in isolated native builds when available.

## 6. UI/UX source-of-truth enforcement

**Required:** [`docs/DESIGN.md`](docs/DESIGN.md). Before any UI, CSS, frontend state, accessibility, onboarding, tray/menu or copy-result view change, consult its matching sections; a missing design document is a **UI blocker**. The supplied project design baseline is **shadcn/ui, Base UI primitive family, Nova style, neutral tokens, Tailwind CSS v4, and React/TypeScript example code**. [`docs/TECH_STACK.md`](docs/TECH_STACK.md) currently describes a *lightweight TypeScript/Vite* settings UI without an explicit React runtime choice. **Do not silently introduce React or assume shadcn's React examples already compile.** When the actual repository does not already contain the approved primitive/React setup, document the frontend integration conflict and request an explicit stack/size decision before installing React, Base UI, shadcn CLI outputs, or alternative component frameworks. No design change is authorized merely because a library is easier to install.

**Design governance:** Use existing `components.json`, chosen primitive base, generated/installed local `components/ui` source, tokens and CSS. A shadcn component is source-owned: inspect installed exports; Base UI `render` is not Radix `asChild`, and component lists/reference JSX are not guaranteed installed APIs. Preserve Nova geometry and semantic `--background`, `--foreground`, `--primary`, `--muted`, `--ring`, etc. Do not mix Base/Radix/Aria or change style, palettes, spacing scale, typography, radii, animation or icon family independently. Preserve dark/light state, focus rings, accessible names, screen-reader announcements and reduced motion. Avoid irrelevant dashboard/chart/chat/auth UI shown as illustrative examples in the design catalog; product navigation comes from [`docs/USER_FLOWS.md`](docs/USER_FLOWS.md) (tray, onboarding, settings, recording status, explicit Copy Result panel).

**UI work checklist:** Document user goal, precise UF-xx, view/state list and component mapping before coding. Implement and test default, focused, disabled, busy, success, permission-denied, empty, cancellation, error and copy-recovery where relevant. Keyboard navigation/shortcuts, label associations, modal focus/restore, color contrast, 200%/400% zoom, and reduced motion are essential. Account for small desktop windows, system text scaling, high contrast and accessibility tools (VoiceOver/Narrator/Orca). Before closing the task, typecheck/lint/build, use keyboard-only smoke, compare screenshots against [`docs/DESIGN.md`](docs/DESIGN.md), and record unsupported platforms/configurations. **Any proposed UI/UX deviation must identify the exact rule in `docs/DESIGN.md`, user impact, alternative, and maintainer approval; update the design document and affected tests together.**

**Tauri boundary:** UI may call explicitly allowlisted Tauri commands and consume sanitized typed events only. It must not access microphone raw buffers or native injection APIs through general-purpose JS bridges. Avoid persistent webview timers when settings are closed; never add cloud fonts/assets/CDN fetches that break offline-first operation. Frontend bundle and accessibility controls count toward size and resource gates.

## 7. Privacy, security and commercial constraints

- Open microphone only during user-initiated dictation; stop/release on cancel, error, sleep, device loss or permission removal. **Never persist, log, upload, index or crash-report audio, partial/final transcripts, or clipboard content.** Only settings and user vocabulary may persist, as versioned atomic JSON.
- No speech/telemetry/cloud endpoints, background update polling, CDN fonts/assets or model downloads. Build-time dependency research/retrieval is separate from consumer runtime and must not use user content. Test normal operation with network blocked; whole-app update is explicit opt-in and signed.
- Validate focus and permissions, protect secure/elevated controls, and surface restricted OS/Wayland capabilities. No privilege escalation, trusted-input bypass or automatic clipboard overwrite. Test clipboard failure and fallback.
- No committed keys, tokens, signing certificates, recordings, private fixtures or logs containing content. Bound parsers/IPC, validate asset hashes and file paths, prevent unsafe filesystem traversal.
- Pin dependencies/model assets, audit native transitive licenses, maintain required notices/SBOM, and verify release signatures. Never claim legal certification based solely on offline behavior; obtain human legal/privacy/accessibility sign-off where required.

## 8. Quality gates, commands and evidence

References to “CI” in earlier project documents mean reproducible **local validation/build evidence** here; they do **not** authorize GitHub Actions. Run **only commands appropriate to the actual repository**, in the agent-owned worktree. Inspect `Cargo.toml`, scripts, toolchain and lockfiles first. Recommended baseline once available:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
# Only if installed/configured: cargo audit; cargo deny check
# Frontend: run existing package.json typecheck/lint/test/build scripts.
# E.g., do not invent `npm run typecheck` if the script does not exist.
git diff --check
```

Do not use `--all-features` when mutually exclusive OS/backend features make that invalid. Build/test on native OS/CPU combinations for platform-specific claims; Linux X11 and Wayland are separate targets. OS integration smoke tests cannot be “passed” in a mock-only environment. Use licensed, non-personal test speech fixtures with a redacted fixture manifest. Never commit human dictation samples or screenshots containing user content.

**Change-specific minimum proof:**

| Change | Required verification beyond format/static checks |
|---|---|
| Session or shortcuts | State transition/model tests; key-up/cancel/idempotence; OS hotkey permission test |
| Audio/ring/resampler | No callback allocation/blocking; overflow, device change, wrong format, silence; leak/endurance |
| ASR/FFI/model | ABI ownership tests, real English corpus WER, quantized-model checksums, latency/RSS/CPU |
| Cleanup/vocabulary | Deterministic golden text, Unicode/punctuation, names, code/numbers, migration/export/delete |
| Injection/clipboard | Target/focus/protected-field matrix on each OS; no unsafe retry; explicit copy and clipboard failure |
| UI | Design conformity + typecheck/build, keyboard, focus, screen reader and visual regression |
| Packaging/update | Completely offline installer/first run, bundled model, signature/rollback, asset licenses |
| Privacy/security | No connections in core use, no transcript/audio logs, malformed input and permissions tests |
| Documentation | Correct local links and cross-IDs, no false completion claim; accurately updated decisions |

**Release-only gates (never pretend a local unit test meets them):** All MVP `AC-01…AC-13`; physical 4 GB system; aggregate process-tree peak RSS sampled under [`docs/TASKS.md`](docs/TASKS.md) protocol **<950 MB**, installed bytes **<950 MB**, signed/notarized installers, measured recognition quality and latency, commercial notices, accessibility checks, offline use and exhaustive platform/focus recovery. Existing references include [`docs/TASKS.md` §5](docs/TASKS.md) and [`docs/ARCHITECTURE.md` §8](docs/ARCHITECTURE.md). Distinguish `NOT RUN`, `PASS`, `FAIL`, `BLOCKED`, and `N/A (reason)`; never mark `SKIPPED` as PASS.

**Task evidence template** (use `evidence/T-xxx.md`, append not overwrite peer-owned evidence):

```yaml
task_id: T-XXX
feature_ids: [F-XX]
flows: [UF-XX]
acceptance: [AC-XX]
owner: <agent/worktree>
base_commit: <sha>
result_commit: <sha-or-uncommitted>
platform: <OS/version/arch/hardware/RAM>
implementation: <summary and key paths>
commands:
  - {command: '<exact command>', result: PASS|FAIL|BLOCKED|NOT_RUN, notes: '<detail>'}
metrics: <measured values or explicitly NOT_MEASURED>
doc_updates: <files/sections>
risks: <known gaps and mitigations>
review: PENDING|ACCEPTED|CHANGES_REQUESTED
```

Do not put raw code dumps, sensitive text, unrestricted log files or fabricated SHA values in evidence. Attach concise, reproducible outputs and location of any locally secured performance trace. Baseline regressions should be captured separately from failures introduced by the current diff.

## 9. Parallel Codex agents, task ownership and Git worktrees

**Default policy:** With one agent, that agent is also coordinator; with multiple, appoint one coordinator. One focused branch and worktree per independent change; no agent writes another agent's working directory. The maintainer/coordinator owns integration, shared documentation and task status. Use parallel agents for **disjoint files behind frozen contracts** (e.g., macOS vs Windows vs Linux adapters after `platform-api` is agreed, or independent research/QA); serial execution for shared trait design, global Cargo manifests/lockfiles, UI tokens/components, release config, task ledger and documentation reconciliation.

**Coordinator assignment record** (chat or approved issue/task note): `task ID(s); purpose; prerequisites; exact owned paths; shared read-only paths; base SHA; worktree/branch; interface version; test commands; handoff evidence path; merge order`. A worker must reject an assignment if file ownership overlaps a live worker; coordinator resolves it before coding. Never invent a shared lock system that pretends to provide atomic cross-worktree locks. The coordinator is the actual serialization point.

**Example** (only after inspecting actual default branch and obtaining coordinator assignment):

```bash
# From the repository root; adapt branch names and parent directory.
git worktree list
git status --short
git worktree add -b agent/T-043-macos ../dictation-T-043 <approved-base-branch>
cd ../dictation-T-043
git status --short --branch
# Implement only assigned files; write evidence/T-043.md.
```

Git worktrees share repository history but have distinct working directories/indexes; do not `git checkout` the **same local branch** simultaneously. Do not use `--force` to overrule Git worktree protections. Never remove, prune, reset or relocate another agent's worktree without explicit owner/maintainer approval. Avoid shared build outputs, ports and caches where they can corrupt concurrent runs; use per-worktree `CARGO_TARGET_DIR` only if necessary and document the tradeoff.

**Shared-file ownership:** `AGENTS.md`, [`docs/PRD.md`](docs/PRD.md), [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md), [`docs/TECH_STACK.md`](docs/TECH_STACK.md), [`docs/USER_FLOWS.md`](docs/USER_FLOWS.md), [`docs/DESIGN.md`](docs/DESIGN.md), [`docs/TASKS.md`](docs/TASKS.md), root lockfiles, workspace manifests, `components.json`, global themes and release/signing configuration are **coordinator-owned** by default. Workers propose patches or note required edits in their handoff; the coordinator applies synchronized changes after integration. For shared trait or schema changes: one owner implements/reviews contract first; all workers rebase/refresh on the accepted interface, and dependent tasks rerun contract tests. Don't quietly copy a stale interface across branches.

**Integration order:** (1) worker completes focused proof and a reviewed diff; (2) coordinator verifies touched-file ownership, immutable IDs, security and tests; (3) merge one accepted change into an integration branch or maintainer-reviewed PR, never direct to `main`; (4) resolve conflicts semantically against documents, not by indiscriminately choosing ours/theirs; (5) rerun workspace and contract tests after each shared-interface merge; (6) coordinator records T-xxx evidence/status; (7) cleanup worktree only with owner approval and after checking unpushed/uncommitted changes.

**PR/commit discipline:** Stage explicit paths, use clear conventional-ish subject (`feat(audio): …`, `fix(platform): …`, `docs: …`); preserve authorship and history. `git add .` is discouraged when parallel generated content may be present. Avoid amending someone else's commit, force pushing, large mechanical rebases or automatically publishing remote changes. You may prepare local commits in your assigned branch when asked to implement; ask before `git push`, opening a public PR, merging, tagging or release. No GitHub Actions workflow file is created or modified absent an **explicit new owner request**.

## 10. Detailed code comments and long-term retention

Write code for a future engineer or AI agent who has **never seen this session**. Explain **why** hard constraints, safety decisions and edge cases exist. Do not describe syntax obvious from identifiers.

- **Crate/module docs (`//!`)**: Purpose; owning C-xx/TS-xx; import/export and thread boundaries; lifecycle; platform restrictions; safety/resource invariants; links to architecture or ADR. Keep current as behavior evolves.
- **Public APIs (`///`)**: Contract and units; side effects; invariant; ownership; `# Errors`, `# Panics`, `# Safety` when relevant; minimal truthful examples (prefer compilable rustdoc examples). Document what happens on cancel, permissions lost, duplicate events and unsupported targets.
- **Non-obvious inline comments (`//`)**: Explain rationale for resampling/segmentation thresholds, buffer capacity and overflow, focus checks, OS-specific fallbacks, deterministic replacements, memory/time bounds, unsafe preconditions. Include upstream issue/doc link or approved ADR when material. Avoid narrating `i += 1` or restating names.
- **FFI comments**: Every `unsafe` call site documents pointer validity, buffer lifetime/length, calling thread, native ownership, resulting resource cleanup, and assumptions about the *pinned* header/runtime. A reviewer must be able to audit soundness without reverse engineering native code.
- **Security/perf comments**: Link safety invariant to test/evidence: e.g., why explicit Copy rather than automatic paste; why stream stops on shortcut cancellation; why no arbitrary background timer; whether memory cost was measured or still hypothesized.
- **TypeScript/React** (if approved and actually in repo): TypeDoc/TSDoc for non-obvious public hooks and command/event contracts; accessible control intent and async cancellation; never comments used to excuse broken semantics.
- **TODO policy:** Format `TODO(T-xxx, owner, condition): reason; exact exit criterion` when unresolved design or hardware work is legitimate. No context-free `TODO: fix later`, fictitious deadlines, or silent regression. A TODO cannot replace AC-xx proof or justify shipping blocked behavior.

**Durable record layout** (create only when needed; never flood the repository with redundant chat transcripts):

- `evidence/T-xxx.md`: task-specific hypothesis, test proof, changes and verification (canonical completion record).
- `docs/adr/NNNN-topic.md` or existing ADR location: consequential, approved design/architecture/security/stack changes. Status `Proposed | Accepted | Superseded`, context, options, decision, consequences, links to F/AC/UF/C/TS/T, owner/date, migration and rollback.
- `docs/handoffs/T-xxx-<scope>.md` (optional for long-lived/unmerged work): concise current tree/branch/SHA, implemented files and tests, exact open blockers, assumptions, next safe operation. If `evidence/T-xxx.md` adequately covers all this, don't duplicate it.
- `docs/implementation-notes/<component>.md` **only** for complex algorithms/FFI/lifecycle behavior that cannot be understood from crate docs and tests. Link rather than repeat architectural chapters.

**Mandatory handoff at session end or context compaction:** `goal + IDs; branch/worktree/SHA; exact changed files; behavior implemented; tests executed with results; evidence path; open failures and platform gaps; relevant decisions/links; next executable action and any approval required`. Make handoffs independently usable; no claims about hidden prior conversation. If the change is unfinished, mark status **IN PROGRESS/BLOCKED**, never mark completed, and avoid orphaned uncommitted state when a durable worktree/commit is available and authorized.

## 11. Autonomy, approvals and prohibited actions

**Proceed autonomously:** scoped reversible fixes, tests, comments, private refactors preserving API/behavior and approved dependency graph. Resolve minor reversible ambiguity with the smallest assumption, document it, validate.

**Ask first:** any new/updated runtime dependency or lockfile; major architecture/interface/ASR model/ABI change; changed audio/privacy/network/retention policy; weakening the 4 GB/950 MB or quality gates; UI departure from `docs/DESIGN.md`; elevated OS permission or clipboard strategy; shared-file edits during concurrent work; destructive Git/file operations; pushing, publishing PRs, merges, releases, signing or deploying. Include exact affected IDs, reason, alternatives, migration, risks and verification plan. Do not interpret silence as approval.

**Never:** hardcode/expose secrets, leak audio/transcripts, record user clipboard, bypass OS trust/UIPI/Wayland restrictions, fetch speech models at runtime, add cloud fallback/telemetry/history, invent passing tests/metrics, lower security to silence checks, fabricate signatures or SHA hashes, add GitHub Actions for this request, commit personal speech fixtures or delete peer work. Block only the affected task and continue independent safe work.

## 12. Reviewer checklist and agent handoff

**Review:** requirements/IDs and design match; diff scoped; no unapproved dependencies; no transcript/log/network/history; FFI soundness; protected-field/focus tests; keyboard/a11y; proper native evidence; measured or explicitly **unmeasured** resource gates; license notices; current comments/ADRs; no fake PASS. Block unsafe releases.

**Handoff (mandatory):** outcome + F/T IDs, owned branch/worktree/SHA and changed files; exact commands/results and tested OS/hardware; evidence paths; known failing/untested behavior; doc/ADR and [`docs/TASKS.md`](docs/TASKS.md) status; next executable action and approvals needed. Never claim unobserved success.

## 13. Evidence for agent practices (October 2026)

[OpenAI Codex AGENTS discovery and 32 KiB budget](https://learn.chatgpt.com/docs/agent-configuration/agents-md); [Codex customization](https://learn.chatgpt.com/docs/customization/overview); [Codex development workflow](https://developers.openai.com/cookbook/examples/codex/iterating-development-workflows-with-codex); [multi-agent delegation](https://developers.openai.com/api/docs/guides/agents-api/multi-agent); [Git worktrees](https://git-scm.com/docs/git-worktree); [Rust API docs](https://rust-lang.github.io/api-guidelines/documentation.html); [Rust Clippy](https://rust-lang.github.io/rust-clippy/master/index.html). Real-world patterns inspected: [uv AGENTS.md](https://github.com/astral-sh/uv/blob/main/AGENTS.md), [Next.js AGENTS.md](https://github.com/vercel/next.js/blob/canary/AGENTS.md). Cross-agent compatibility: [Copilot custom instructions](https://docs.github.com/en/copilot/reference/custom-instructions-support); [VS Code custom instructions](https://code.visualstudio.com/docs/agent-customization/custom-instructions). Codex is the intended primary agent; no Claude/Copilot-specific files are requested.

**Maintenance:** Update this root contract only for recurring failures or approved policy changes, remove obsolete rules, verify links and restart Codex. Never weaken privacy, focus safety or performance gates through nested rules.
