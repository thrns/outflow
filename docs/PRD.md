# PRD.md — Offline Rust Desktop Dictation

> **Status:** Implementation-ready specification; technical feasibility gates pending hardware tests  
> **Revision:** 1.0 | **Research cutoff:** 2026-10-29 | **Owner:** Product & Engineering  
> **Companion documents:** [ARCHITECTURE.md](ARCHITECTURE.md) · [TECH_STACK.md](TECH_STACK.md) · [USER_FLOWS.md](USER_FLOWS.md) · [TASKS.md](TASKS.md)

## 0. Binding decisions and interpretation

This document is the canonical product contract. When another document conflicts with this one, stop implementation and reconcile the conflict in all five files. The project is a **proprietary, commercially distributed, Rust-first, Tauri 2 desktop application** for macOS, Windows and Linux. MVP is **English-only**, with multilingual dictation considered for **V1** subject to bundled-size and peak-memory gates. All recognition and cleanup run on-device. No account, cloud inference, telemetry, recording retention, transcript history, model gallery, model chooser, in-app model download, or first-run model fetching. A verified **English streaming ASR model is included with every installer**; the user can install and dictate without any Internet connection. Models may change only with a signed **whole-application update**, never as a separately downloadable user option. Update checking is opt-in and not required for use.

**Ambiguity resolution:** “the entire app must use less than 1 GB” means BOTH **unpacked installed application footprint <1,000,000,000 bytes** (including bundled model and native libraries) and **aggregate peak RSS of every application-owned process <1,000,000,000 bytes** during supported 4 GB machine operation. A stricter **950,000,000-byte CI release threshold** leaves margin. Aggregate RSS intentionally double-counts some shared pages and is therefore conservative. Count process trees (UI/webview helper and optional inference worker), not unrelated operating-system processes. Exclude installer caches and transient OS update staging from installed footprint but report their temporary peak disk needs. Also report platform-native private footprint/PSS. These are **targets, not presently measured facts**. Never claim compliance until measured on reference hardware. Target 4 GB physical RAM, CPU-only; no discrete GPU requirement. See [ARCHITECTURE §8](ARCHITECTURE.md#8-performance-budget-and-benchmark-method).

**Model policy:** Phase 0 evaluates **quantized Moonshine Small Streaming English (123M parameters)** as the primary candidate; **Tiny Streaming English (34M)** is the preapproved fallback if Small fails memory, CPU or size gates. Only **one** approved model is bundled per release, with neither a switcher nor runtime download. A Tiny downgrade must also meet the recognition quality release bar; otherwise **do not ship**. The Small model is a candidate, **not verified under the cap**. See [TECH_STACK §3](TECH_STACK.md#3-speech-recognition-evaluation-and-hard-gate).

## 1. Vision, audience, positioning

**Vision:** Reliable, fast, keyboard-centric voice input for text fields across desktop applications, with a comprehensible privacy guarantee: audio and transcript text never leave the computer through the app.

**Primary personas:** (P1) writer/student drafting notes, email and chat; (P2) developer entering prose into an IDE, browser or issue tracker; (P3) privacy-conscious professional dictating routine text without cloud upload; (P4) resource-constrained 4 GB laptop user. **Excluded claims:** clinical transcription accuracy, guaranteed legal/medical compliance for customer content, flawless insertion into elevated/protected apps, and universal Wayland automation.

**Differentiation:** Immediate offline availability, bundled model, no account, no silent recording, resource caps, transparent fallback behavior, and deterministic text cleanup rather than a memory-heavy LLM. Not an autonomous voice agent. No cloud back end is needed for speech.

### 1.1 Jobs-to-be-done

- While composing text, hold a shortcut, speak, release, and receive appropriately punctuated text in the intended field without switching windows.
- Start longer dictation with a toggle shortcut, stop with the same shortcut, and insert text without a mouse.
- Teach the app a word or name and have it recognized or corrected consistently without sharing that vocabulary.
- Recover safely from missing permissions, lost microphone, unsupported target control, silence, or model corruption without losing control of the keyboard.
- Use the app on a plane, with DNS blocked, and on a 4 GB system with predictable memory usage.

## 2. Scope and release phases

| Phase | In scope | Explicitly out of scope |
|---|---|---|
| Phase 0 research spike | Audio, ASR, OS injection tests, objective benchmarks, license/asset provenance | Product release |
| MVP | English bundled model; push-to-talk + toggle; real-time audio; text finalization; deterministic punctuation/filler cleanup; custom vocabulary; tray; basic settings; onboarding; safe insertion and clipboard fallback; offline-only, no history | LLM rewrite, voice commands, cloud, multilingual, VAD-only hands-free, transcripts list |
| V1 | Additional **bundled** language models only if **combined** installer and peak RAM gates pass; language detection/selection in settings, per-app cleanup profiles, improved accessibility, app compatibility matrix | Downloadable language packs, optional model downloads, automatic cloud language selection |
| Future / separate product decision | Advanced command grammar, optional on-device rewriting **only if** 1 GB gate passes, custom hotword | Any feature that silently compromises strict local policy |

**Language-size conflict:** More languages typically require more model assets. For V1, a single multilingual model replacing English-only weights may be preferable; V1 is **blocked** if adequate multilingual coverage cannot fit both hard caps. V1 language promises must be adjusted instead of adding runtime downloads.

## 3. Core feature register and traceability

ID is immutable; all feature IDs must be present in [USER_FLOWS §3](USER_FLOWS.md#3-feature-to-flow-matrix), [ARCHITECTURE §3](ARCHITECTURE.md#3-component-responsibilities), [TECH_STACK §2](TECH_STACK.md#2-authoritative-technology-decisions), and [TASKS §4](TASKS.md#4-atomic-dependency-ordered-backlog). **AC-xxx** are formal acceptance criteria, **UF-xx** user flows, **C-xx** architecture components, **T-xxx** tasks.

| Feature | Priority | Expected user value | Flow | Component | Acceptance |
|---|---|---|---|---|---|
| F-01 Install/offline first run | MVP P0 | Immediately usable without Internet | UF-01 | C-01,C-09 | AC-01 |
| F-02 Permission onboarding | MVP P0 | Understand/resolve mic, accessibility, shortcut limits | UF-02 | C-01,C-07 | AC-02 |
| F-03 Push-to-talk | MVP P0 | Fast short dictation | UF-03 | C-02,C-03 | AC-03 |
| F-04 Toggle dictation | MVP P0 | Longer hands-free-from-keyboard recording | UF-04 | C-02,C-03 | AC-04 |
| F-05 Audio capture and VAD | MVP P0 | Reliable speech, no empty inserts | UF-03,UF-04 | C-03,C-04 | AC-05 |
| F-06 Bundled local streaming ASR | MVP P0 | Offline transcript within budget | UF-05 | C-05,C-09 | AC-06 |
| F-07 Punctuation/filler cleanup | MVP P0 | Readable output without expensive LLM | UF-05 | C-06 | AC-07 |
| F-08 Custom vocabulary | MVP P1 | Correct names/technical terms | UF-06 | C-06,C-08 | AC-08 |
| F-09 Safe system-wide insertion | MVP P0 | Text goes to correct target; never password fields | UF-07 | C-07 | AC-09 |
| F-10 Clipboard fallback | MVP P0 | Useful when insertion unsupported | UF-08 | C-07 | AC-10 |
| F-11 Tray, settings, accessibility | MVP P1 | Discoverable, keyboard-accessible app | UF-09 | C-01,C-08 | AC-11 |
| F-12 Local privacy & no retention | MVP P0 | Dictation never persisted or transmitted | UF-10 | C-01…C-09 | AC-12 |
| F-13 Diagnostics, recovery & updates | MVP P0/P1 | Predictable errors and safe signed releases | UF-11,UF-12 | C-01,C-09,C-10 | AC-13 |
| F-14 Multilingual bundled support | V1 gate | Additional languages fully offline | UF-13 | C-05,C-09 | AC-14 |
| F-15 Per-application profiles | V1 | Target-specific formatting preferences | UF-14 | C-06,C-07,C-08 | AC-15 |

### 3.1 One-to-one implementation trace matrix

This matrix is the **agent navigation index**. Each row connects a product feature to its real user interaction, owning implementation, specific stack choice, atomic first implementation task(s), and release test. For secondary task dependencies, follow the implementation task in [TASKS §4](TASKS.md#4-atomic-dependency-ordered-backlog).

| Requirement | User flow(s) | Architecture owner | Stack decision(s) | Key task(s) | Acceptance |
|---|---|---|---|---|---|
| F-01 | UF-01 | C-01,C-09 | TS-01,TS-09 | T-011,T-013,T-071 | AC-01 |
| F-02 | UF-02 | C-01,C-07 | TS-01,TS-07 | T-025,T-041 | AC-02 |
| F-03 | UF-03 | C-02,C-03 | TS-02,TS-03,TS-07 | T-031,T-032 | AC-03 |
| F-04 | UF-04 | C-02,C-03 | TS-02,TS-03 | T-033,T-034 | AC-04 |
| F-05 | UF-03,UF-04 | C-03,C-04 | TS-03,TS-04 | T-026,T-027,T-028 | AC-05 |
| F-06 | UF-05 | C-05,C-09 | TS-05,TS-09 | T-013,T-029,T-030 | AC-06 |
| F-07 | UF-05 | C-06 | TS-06 | T-035,T-036 | AC-07 |
| F-08 | UF-06 | C-06,C-08 | TS-06,TS-08 | T-038,T-039 | AC-08 |
| F-09 | UF-07 | C-07 | TS-07 | T-042,T-043,T-044,T-045 | AC-09 |
| F-10 | UF-08 | C-07 | TS-07 | T-046,T-047 | AC-10 |
| F-11 | UF-09 | C-01,C-08 | TS-01,TS-08 | T-051,T-052,T-053 | AC-11 |
| F-12 | UF-10 | C-01,C-03,C-05,C-08,C-10 | TS-01,TS-05,TS-08,TS-10 | T-061,T-062,T-063 | AC-12 |
| F-13 | UF-11,UF-12 | C-01,C-09,C-10 | TS-09,TS-10 | T-064,T-065,T-072 | AC-13 |
| F-14 | UF-13 | C-05,C-09 | TS-05,TS-09 | T-081,T-082,T-083 | AC-14 |
| F-15 | UF-14 | C-06,C-07,C-08 | TS-06,TS-07,TS-08 | T-084,T-085 | AC-15 |

## 4. Functional requirements

### F-01 — Installation and fully offline first run

- FR-01.1: Installers for each supported OS/CPU architecture contain the one pinned ASR model, its tokenizer/config and all redistributable runtime assets. No executable code/model weights are downloaded after installation.
- FR-01.2: First launch succeeds with all network interfaces disabled; no login, registration, license server call, cloud setup, or analytics prompt.
- FR-01.3: Validate model manifest checksum and file presence before activation. If damaged, show an actionable “Reinstall application” flow rather than reaching the Internet.
- FR-01.4: Bundle a plaintext Third-Party Notices file and privacy information.

### F-02 — Onboarding and permissions

- FR-02.1: Explain microphone capture, global shortcut and native insertion permissions **before** OS prompts; request only needed permissions.
- FR-02.2: Distinguish `available`, `denied`, `not_supported`, `needs_user_action`, `unknown` states per capability.
- FR-02.3: Offer Retry, Open System Settings (when OS supports), and Copy-only fallback when permissions cannot be granted.
- FR-02.4: On Wayland, detect actual portal capabilities; display a plain-language limitation, not an incorrect “supported” claim.

### F-03 — Push-to-talk

- FR-03.1: Hold a dedicated configurable shortcut to start capture; release finalizes; **do not insert partially transcribed text**.
- FR-03.2: Shortcut repeat events must not start duplicates. Start a session on key-down only; stop on corresponding key-up, window/focus changes notwithstanding.
- FR-03.3: If OS API lacks release events, present push-to-talk as unsupported and offer toggle. Do not synthesize key-up from a timeout.
- FR-03.4: While model is cold-loading, show loading state; a second press cannot create concurrent sessions.

### F-04 — Toggle dictation

- FR-04.1: First shortcut begins recording; second stops and finalizes; Escape via tray/settings cancels without insertion.
- FR-04.2: Shortcut collision with another application must be detected and displayed with an alternate assignment path.
- FR-04.3: At most one active session; two rapid events are debounced via state machine, not timing-only heuristics.

### F-05 — Capture/audio/VAD

- FR-05.1: Support selectable input device and current system default; device sampling may be 44.1/48 kHz, mono/stereo, integer/float. Convert to model-compatible mono floats, sample rate per model metadata.
- FR-05.2: No inference, filesystem I/O, locks with unbounded waits, or heap allocation in audio callback. Use bounded/preallocated SPSC buffer.
- FR-05.3: Speech detection gates empty inserts; audio samples are released after completion/cancel. Do not keep “background listening” audio.
- FR-05.4: Handle unplug, device loss, input stream errors, audio overflow with explicit recoverable state and no bogus text insert.

### F-06 — Local ASR

- FR-06.1: One bundled, pinned quantized English model; built-in runtime never fetches weights nor contacts model provider.
- FR-06.2: Stream incremental speech to ASR; partial results may appear in an app-owned preview, but **never** enter target app before finalization.
- FR-06.3: Support final transcript drain and completion; clear model streaming state between sessions.
- FR-06.4: Avoid always-on inference when idle; deterministic warm retention and unloading policy with hard resource guard.
- FR-06.5: If model initialization fails, display error without falling back to remote or downloading another model.

### F-07 — Deterministic text cleanup

- FR-07.1: Punctuation/spacing/capitalization normalization with **no inference LLM**; preserve original proper nouns/numbers/abbreviations where heuristics are ambiguous.
- FR-07.2: Remove **only explicit, well-supported isolated filler tokens** such as “um”/“uh” when in Clean mode. Verbatim mode disables filler deletion. Never silently erase a repeated word if it might be meaningful.
- FR-07.3: Order: canonicalize whitespace -> recognize dictation commands limited to “new line” and “new paragraph” (opt-in MVP) -> vocabulary post-corrections -> safe punctuation/spacing -> optional filler removal. Keep deterministic, idempotent pipeline; explicit rule order documented in C-06.
- FR-07.4: Settings expose `Verbatim` and `Clean` (default). All transformations run offline.

### F-08 — Custom vocabulary

- FR-08.1: Add/edit/delete exact words or phrases with a case-insensitive **boundary-aware** correction map; prevent duplicate/conflicting entries. Examples: `north parkade` → `North Parkade`, `rust` → `Rust`.
- FR-08.2: Before inference, optionally supply keyterms via supported Moonshine context API; post-processing map is authoritative if model ignores hints.
- FR-08.3: Persist **only** explicit vocabulary/settings, not transcript-derived data. Keep file permission user-only, with local export/delete in settings.
- FR-08.4: Limit MVP to 500 terms, each source/target <=128 Unicode scalar values; reject empty or pathological input.

### F-09 — Safe insertion

- FR-09.1: Snapshot intended target identity at session start (`process`, `window`, focused editable control when available, session instance) and validate target at commit. Never insert into a new target silently.
- FR-09.2: Reject password/secure text fields, elevated/protected targets, unsupported controls, and ambiguous focus targets. Do not inspect or retain existing target text unnecessarily.
- FR-09.3: Platform adapters: macOS accessibility/Quartz where allowed, Windows UIA/SendInput where allowed, Linux X11; Wayland only with explicit granted portal/session capability.
- FR-09.4: Commit at most once per session, with in-process idempotency key. Do not retry after uncertain partial success; show copy option instead.
- FR-09.5: Append text at cursor or replace only the actively selected range where supported. Do not delete unrelated user data.

### F-10 — Clipboard fallback

- FR-10.1: When safe insertion unavailable, **show transcript in a small app-owned result panel** with an explicit Copy button; do not auto-paste by default.
- FR-10.2: Warn that clipboard managers/OS synchronization may retain copies. Do not promise clipboard deletion. Avoid overwriting clipboard until user acts.
- FR-10.3: If copy fails, retain result only in volatile memory until panel closed or app exits; provide Select All.

### F-11 — Tray/UI/settings/accessibility

- FR-11.1: Tray menu: Start/Stop, Open Settings, Status/permissions, Quit. Recording state distinguishable with text and icon, not color alone.
- FR-11.2: Destroy settings webview on close and keep tray/core running; prevent foreground focus stealing during target dictation.
- FR-11.3: Keyboard-only navigation, focus indicators, screen-reader labels, text scaling and reduced motion. WCAG 2.2 AA and native screen-reader testing are targets.
- FR-11.4: Settings: shortcut assignment, input device, cleanup mode, custom vocabulary, start-at-login opt-in, opt-in update checker, privacy and troubleshooting. **No model picker/download/update UI**.

### F-12 — Privacy

- FR-12.1: Network egress is zero during ordinary dictation and first run. App may connect **only** after explicit user invocation of a signed full-app update check (the update payload includes model), and only to allowlisted release host(s).
- FR-12.2: No transcript history, recording files, session audio persistence, crash dumps with dictated content, logging transcript text, OS clipboard access except user-requested copy, or usage tracking.
- FR-12.3: Clear memory buffers best-effort on cancellation/finish; recognize that memory allocators and OS swap may keep copies—do not claim cryptographic erasure.
- FR-12.4: No microphone access when idle, permission denied, or app exited.

### F-13 — Diagnostics/recovery/release

- FR-13.1: Show stable error codes with remediation instructions; diagnostics export contains versions, OS/build, device capabilities, timing and aggregate metrics **but not transcript, audio or vocabulary values**.
- FR-13.2: App never retries prohibited insertion methods or elevates privileges to bypass OS protections.
- FR-13.3: App and update packages are signed; user-initiated update is optional, verifies signature, and has recovery path. Security patches may be announced but cannot silently force a model download.
- FR-13.4: Restore valid last-known settings after interruption; on corrupted settings use safe defaults and request user review.

### F-14 — V1 languages (gated)

- FR-14.1: Explicit set of supported languages per release; models **bundled at build time only**.
- FR-14.2: V1 requires the *combined* application to remain below disk/memory release thresholds with acceptable quality per language; if not feasible, ship fewer languages or defer.
- FR-14.3: No implicit remote translation, no on-demand language pack.

### F-15 — Per-app formatting profiles (V1)

- FR-15.1: Optional locally stored per-application format preferences keyed by app identifier, not document contents.
- FR-15.2: Fallback to global profile for unknown/hidden app; never snoop content to choose profile.

## 5. Formal measurable acceptance criteria

**Test matrix M0:** macOS Apple Silicon 4 GB if available or nearest available low-memory Apple Silicon with memory-constrained validation; Intel/AMD Windows 4 GB; Linux X11 4 GB; GNOME/KDE Wayland documented best-effort. A **physical 4 GB machine** is the certification standard for supported 4 GB platforms; cgroup/process limits can supplement but cannot replace representative hardware. Benchmark reference recordings: clean quiet-office, noise, accents, proper nouns, short/long phrases, silence. Collect p50/p95 from ≥100 attempts per supported hardware/environment for timing claims and ≥10 min continuous dictation; record OS/build/driver/model SHA. Exact baseline details in [ARCHITECTURE §8](ARCHITECTURE.md#8-performance-budget-and-benchmark-method).

| ID | Pass condition / evidence | Related feature |
|---|---|---|
| AC-01 | Offline clean install + first dictation succeed with network disabled; bundled models intact; unpacked installed size <950,000,000 bytes on each release target. | F-01 |
| AC-02 | Every permission denied/revoked/not-supported case offers actionable state; no capture until microphone permission. | F-02 |
| AC-03 | 100 push-to-talk cycles; one start per press, one finalize per release, zero duplicate insertion; unsupported release semantics trigger toggle instruction. | F-03 |
| AC-04 | 100 toggle cycles, including rapid presses; no concurrent recordings, Escape cancel never inserts text. | F-04 |
| AC-05 | 30 min continuous capture without ring-buffer underrun under reference load; unplug/replug yields recoverable error; no insert for silence in 50/50 trials. | F-05 |
| AC-06 | No network calls; startup model checksum valid; ASR RTF <1.0 CPU-only on supported minimum hardware for 95th percentile reference utterances; p95 stop-to-final-ready <1,500 ms; end-to-end p95 ≤2,000 ms when direct insertion available; benchmark clean-speech WER ≤12% on frozen diverse test set (see quality caveat). | F-06 |
| AC-07 | Golden corpus of ≥200 transformations: ≥99% exact expected output, zero altered numeric tokens/proper nouns in protected fixture corpus; tests demonstrate Verbatim vs Clean distinction. | F-07 |
| AC-08 | 500 vocabulary entries saved/reloaded offline; boundary preservation in 100% of constructed tests; source/target edits/delete are idempotent. | F-08 |
| AC-09 | On certified targets, 100 insertions with no duplicates; password/secure and focus-change tests never write; unsupported Wayland gives fallback not silent failure. | F-09 |
| AC-10 | Unsupported field leads to result panel; clipboard remains unchanged until Copy; copied text equals final transcript byte-for-byte. | F-10 |
| AC-11 | Keyboard-only onboarding/settings/navigation; screen-reader announcements; closed webview is not retained in process memory; privacy control discoverable. | F-11 |
| AC-12 | Packet-capture + mock DNS + deny-all network verify zero egress during install, first run, dictation, and normal UI; no transcript/audio files in app directories/logs; no microphone stream in idle. | F-12 |
| AC-13 | 100 injected faults (permission/device/malformed model/focus/IPC) do not crash primary app or lose settings; signed artifacts and update signature rejection tested. | F-13 |
| AC-14 | Every advertised language passes AC-01, AC-06, AC-09, AC-12 plus per-language published benchmark gates; no model download path. | F-14 |
| AC-15 | Per-app rules work in test targets with no document scanning and no cross-app leakage. | F-15 |

### 5.1 Non-functional requirements (NFR)

| ID | Target | Validation |
|---|---|---|
| NFR-01 Memory | **<950 MB aggregate peak RSS** throughout cold start, first recording, 10-minute session, settings open, finalization, cleanup and error recovery | Automated per-OS process-tree profiler, peak over 10 repeated runs |
| NFR-02 Disk | **<950 MB unpacked installed** incl model, runtime and notices | Directory tree byte count after production installer |
| NFR-03 System | Works on **4 GB installed RAM** CPU-only without OS out-of-memory and with system still interactive | Native reference laptop tests |
| NFR-04 Background | Median idle core CPU <1% of one logical CPU after warm-up; mic stream closed | Profiler after 10 min idle |
| NFR-05 Latency | ASR stop-to-ready p95 <1.5 s, supported target stop-to-insertion p95 <2.0 s | Timestamped trace, ≥100 sessions |
| NFR-06 Energy | Idle energy use comparable to inert tray app; target <3% additional battery drain over 8 h idle, calibrated vs baseline; no false promise before hardware test | Battery controlled A/B test |
| NFR-07 Reliability | ≥99.5% successful completion of valid requests over 1,000 reference sessions in certified editable targets | Automated endurance suite |
| NFR-08 Security | No unreviewed high/critical dependency advisory, signed notarized builds, no telemetry | CI evidence + security review |
| NFR-09 Accessibility | Keyboard reachable, screen-reader names, 200% UI text scaling, reduced-motion support, WCAG 2.2 AA UI review | Accessibility checklist + assistive tech smoke |
| NFR-10 Privacy | Audio/transcripts absent from persistent state, diagnostics, network; no microphone when idle | Network + filesystem + memory lifecycle review |
| NFR-11 Maintainability | Strict API contracts, cargo fmt/clippy/test, versioned settings, clean boundaries | CI and architecture review |

**Accuracy caveat:** The vendor's reported English Small Streaming average 7.84% WER is for a defined reference evaluation with floating-point weights. This is not the expected WER for customer dictation; measure the exact bundled quantized checkpoint on real speech before claiming AC-06. If the ≥12% target is not met, modify product scope or model only via documented change control, not by quietly relaxing the test.

## 6. Experience and navigation

- First launch: Welcome → privacy facts → microphone → insertion permission → shortcut test → offline test dictation → Done (UF-01/02).
- Tray menu: Recording status, Dictate (toggle), Settings, Troubleshooting, Quit (UF-09).
- Settings tabs: General, Microphone, Shortcuts, Text & Vocabulary, Privacy, About & Diagnostics; no AI/models section.
- Floating indicator (optional low-resource window): `Preparing`, `Listening`, `Finalizing`, `Inserted`, `Needs action`; must not take keyboard focus or screen-reader focus unexpectedly.
- Result panel: opens only after safe insertion failure, shows text + Copy + Dismiss; transient in-memory content deleted on dismissal.

## 7. Failure and recovery policy

| Failure | Expected behavior | Never do |
|---|---|---|
| Mic denied | Stop, explain and link to permission settings | Continue recording |
| Model absent/corrupt | Disable Dictate, reinstall guidance, log code only | Download weights |
| 4 GB system memory pressure | Stop/abort current session safely with visible error; release model on next idle | OOM or silently fall back to cloud |
| Empty/no speech | Show "No speech detected" briefly, insert nothing | Invent text |
| Uncertain focus | Show result panel for copy | Paste into newly focused app |
| Password/secure field | Refuse insertion, offer copy with warning | Inspect password or inject |
| Elevated Windows window | Copy fallback with explanation | Privilege escalation |
| Wayland portal missing | Copy result, declare limited integration | Use `sudo`/input daemon to bypass |
| Process crash mid-dictation | Restart safely; no automatic retry insertion | Double-insert after restart |
| Shortcut collision | Ask user to rebind | Hijack other app's shortcut |
| Disk full during settings write | Atomic rollback, clear message | Truncate valid settings |
| Update interrupted | Keep old signed app, retry later | Load partial model bundle |

## 8. Privacy, security, accessibility, compliance

**Threat model summary:** sensitive audio/transcripts; malicious/compromised local processes; native library vulnerabilities; user-initiated clipboard exposure; compromised installer/update; unintended network use; OS injection crossing protected boundaries. Mitigate with least-privilege permissions, integrity checking, FFI safety, signed releases, minimal logs, bounded buffers, and local-only operation. The operating system and third-party target applications may still store dictated text; privacy promises apply to **this application**, not the destination app.

**Data classification:** raw audio and transcript = ephemeral sensitive volatile data; vocabulary and shortcuts = persistent user data; model manifest/build info = nonpersonal system metadata; diagnostic counters = ephemeral until explicitly exported. Default retention for raw audio/transcripts **zero** beyond active session; settings kept until uninstall/reset. OS swap/memory dump might contain residual data; disclose limitation.

**Accessibility standard:** use WCAG 2.2 AA as a practical UI target plus OS native accessibility conventions; perform screen-reader testing with VoiceOver, NVDA/Narrator, and Orca where feasible. **Regulatory caveat:** GDPR, PIPEDA/BC PIPA, EU AI Act, EU Accessibility Act and US laws are applicability-dependent; offline execution is not a legal exemption. Consult qualified legal review before commercial launch.

**Commercial distribution:** direct Mac signed+notarized DMG, Windows signed MSI/NSIS, Linux signed package/checksums; Tauri updater optional and opt-in. License attribution for Moonshine MIT and all bundled transitive C/C++ code; OSS notices packaged. No App Store parity commitment due to sandbox restrictions.

## 9. Analytics without telemetry

Success metrics measured only in explicit local testing or consenting beta studies: task completion, WER, p95 latency, app memory/disk, microphone failure rate, supported-app insertion success, battery delta. **No automatic user tracking**. Error report export requires explicit user selection and manual inspection for sensitive information.

## 10. Decisions, assumptions and release gates

| ID | Decision / unresolved verification | Blocking point |
|---|---|---|
| D-01 | Tauri 2; settings webview destroyed on close | Phase 0 baseline footprint |
| D-02 | Small English quantized streaming as first candidate; Tiny fallback **only if** quality threshold passes | Phase 0 recognition benchmark |
| D-03 | Bundled model; no runtime model download or chooser | Every release |
| D-04 | Deterministic cleanup, no LLM in MVP | Product scope |
| D-05 | Cap both installed size and aggregate peak RSS at 950 MB release threshold | Every platform release |
| D-06 | Wayland best effort only, safe manual copy fallback | Documentation/support matrix |
| D-07 | No history/analytics/background mic | Privacy test |
| D-08 | Sign complete app update incl model; opt-in update | Release security |
| D-09 | Advanced permission-dependent insertion is never guaranteed for all fields | Compatibility test |
| D-10 | If hard caps and accuracy cannot coexist on 4 GB, **no release** until engineering/project scope decision | Launch go/no-go |

## 11. Sources and evidence

1. [Tauri stable 2.x releases (2026-10-21)](https://github.com/tauri-apps/tauri/releases); [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
2. [Moonshine streaming model catalog, architectures, WER and licensing](https://github.com/moonshine-ai/moonshine/blob/main/docs/models/available-models.md); [Moonshine C API](https://moonshine-voice.readthedocs.io/en/stable/api/c-api/); [license and model exceptions](https://github.com/moonshine-ai/moonshine/blob/main/docs/license.md).
3. [Moonshine official benchmark procedure](https://moonshine-voice.readthedocs.io/en/stable/using/benchmarks/); [quantization method](https://moonshine-voice.readthedocs.io/en/stable/models/quantization/).
4. [Apple accessibility trust](https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions); [Microsoft SendInput privilege isolation](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-sendinput).
5. [XDG GlobalShortcuts](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html); [XDG RemoteDesktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html).
6. [W3C WCAG 2.2](https://www.w3.org/TR/WCAG22/); [NIST SSDF SP 800-218](https://csrc.nist.gov/pubs/sp/800/218/final); [Canadian privacy-law jurisdiction](https://www.priv.gc.ca/en/privacy-topics/privacy-laws-in-canada/02_05_d_15).

**Implement next:** [TASKS.md](TASKS.md) phase 0; do not bypass hardware go/no-go gates.
