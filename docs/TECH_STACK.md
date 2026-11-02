# TECH_STACK.md — Research-Backed Runtime, Dependencies and Release Policy

> **Research verified:** 2026-10-29 · **Applies to:** MVP and strictly gated V1  
> **Scope:** [PRD.md](PRD.md) · **Architecture:** [ARCHITECTURE.md](ARCHITECTURE.md) · **Execution:** [TASKS.md](TASKS.md) · **Behavior:** [USER_FLOWS.md](USER_FLOWS.md)

## 0. Decision discipline

**Selected:** Rust + Tauri 2, React/TypeScript/Vite for the frontend, CPAL, quantized Moonshine Streaming English model bundled in each installer, deterministic Rust text cleanup, thin per-OS native adapters, versioned atomic JSON settings. React is selected for the Outflow foundation; keep its runtime and generated assets within the existing release size and memory gates. **No** LLM, Whisper/sherpa runtime, Python, Node.js, download-on-demand, SQLite, encryption server, inference service, or HTTP daemon in MVP. Development tooling can use Node.js/Python/CMake; the **installed release runtime must not depend on them**.

**Immutable contract:** one model per shipped version; no model picker/download UI; all speech local; no history; <950,000,000 bytes installed and aggregated process-tree peak RSS (<1 GB). Four GB physical RAM and CPU-only are required. These numeric limits are **release acceptance gates**, not observed performance. If Small fails resource gates, evaluate Tiny and require its quality to pass; if neither passes, release is blocked. See [PRD §0](PRD.md#0-binding-decisions-and-interpretation).

## 1. Version verification methodology

Versions are **as seen in primary project release listings/docs on 2026-10-29**, not guaranteed eternally latest. Actual production manifests must pin **exact release version** (Cargo `=x.y.z`/lockfile, npm lockfile) or verified immutable Git SHA, and a source-verified model asset revision/hash; `cargo update` is prohibited during a reproducible release. Some upstream GitHub release and ecosystem pages were slightly out of sync, so prefer artifact-specific GitHub release tags for exact ship candidates and let CI verify existence and compatibility.

| Package/tool | Verified version or family | Evidence / limitations |
|---|---|---|
| Rust stable toolchain | **1.99.1** (2026-10-15) | [official Rust release history](https://blog.rust-lang.org/releases/); use `rust-toolchain.toml` pin |
| Tauri core | **2.12.5** (2026-10-21) | [GitHub releases](https://github.com/tauri-apps/tauri/releases); **do not** use Tauri 3 alpha |
| Tauri CLI | **2.12.4** (2026-10-21) | [same upstream releases](https://github.com/tauri-apps/tauri/releases) |
| Tauri global-shortcut plugin | **2.4.1** (2026-10-10) | [plugin changelog](https://tauri.app/release/global-shortcut/all-versions/) — key-up support is platform-specific, not guaranteed by package presence |
| Tauri updater plugin | **2.13.3** (2026-10-10) | [updater changelog](https://tauri.app/release/updater/all-versions/), signed updates |
| CPAL | **0.18.2** (2026-08-16) | [docs.rs](https://docs.rs/crate/cpal/latest) |
| rubato | **5.0.1** | [docs.rs](https://docs.rs/crate/rubato/latest); 5.x API differs from 3.x/4.x |
| ringbuf | **0.5.2** | [docs.rs latest build](https://docs.rs/crate/ringbuf/latest/builds); API check before code generation |
| Tokio | **1.54.1** | [docs.rs](https://docs.rs/crate/tokio/latest/builds); not for real-time audio callback |
| Serde | **1.0.229** | [docs.rs](https://docs.rs/crate/serde/latest) |
| objc2 | **0.6.5** | [docs.rs](https://docs.rs/crate/objc2/latest); associated framework crates need compatible versions |
| windows-rs | **0.62.2** | [docs.rs](https://docs.rs/crate/windows/latest); select only required Windows features |
| ashpd | **0.13.13** | [docs.rs](https://docs.rs/crate/ashpd/latest); platform-portal APIs remain compositor-dependent |
| rusqlite | **0.41.0** | [docs.rs](https://docs.rs/crate/rusqlite/latest); **V1 alternative only**, not in MVP |
| React | **19.3.1** stable | [npm versions](https://www.npmjs.com/package/react?activeTab=versions) |
| TypeScript | **7.0.2** stable compiler + **6.0.3** compatibility API | [npm versions](https://www.npmjs.com/package/typescript?activeTab=versions); Microsoft documents the npm alias arrangement for API-consuming tools such as typescript-eslint |
| Vite | **8.3.5** stable | [npm versions](https://www.npmjs.com/package/vite?activeTab=versions); 8.3 supported line |
| Moonshine C API | **Header v3.0.0 (30000)** at reviewed docs | [C ABI docs](https://moonshine-voice.readthedocs.io/en/stable/api/c-api/); **not equivalent to a verified tagged binary release**; pin one tested commit and model asset revision in Task T-013 |

**Do not invent a Moonshine commit hash or model SHA-256.** The primary repo documents library features and current models, but the exact build artifact checksum is not provided by this comparison. Task T-013 must fetch the release candidate **at build time**, verify its commit and exact model file hashes, archive an internal immutable build manifest and verify license notices before any shipping binary is built. No runtime downloading.

## 2. Authoritative technology decisions

| Decision ID | Component(s) | Choice | Rationale | Alternatives/risks |
|---|---|---|---|---|
| TS-01 | C-01 | Tauri **2.12.5** + React **19.3.1** + TypeScript **7.0.2** compiler + TypeScript **6.0.3** API compatibility alias + Vite **8.3.5** | Mature packaging/tray and WebView reuse; React frontend compiles to static assets for the WebView; TypeScript ESLint tooling resolves the 6.0 API while `typecheck` invokes the 7.0 compiler directly | React adds runtime and bundle bytes; measure against release caps. Keep the aliases named and scripts explicit to avoid `tsc` executable ambiguity |
| TS-02 | C-02 | Rust **1.99.1**, Tokio **1.54.1**, explicit reducer/state machine | Strong ownership and testability; event-driven background | `std::thread` only acceptable for minimal first spike; Tokio must never run in audio callback |
| TS-03 | C-03 | CPAL **0.18.2**; rubato **5.0.1** | CoreAudio/WASAPI/Linux audio access, explicit rate conversion | Custom audio backend expensive; no Python sound stack |
| TS-04 | C-04 | ringbuf **0.5.2**, preallocated bounded SPSC; initial no external VAD | Predictable nonblocking capture; use model-based speech segmentation and explicit no-speech check | Silero VAD adds runtime/weights/CPU; add only on measured need |
| TS-05 | C-05,C-09 | **Moonshine Small Streaming English quantized** candidate, native C ABI + vendored ORT-dependent runtime | Streaming partials; commercial MIT model; compact deployment relative to larger ASR; C interoperability | Tiny Streaming fallback if Small violates hard caps; Whisper and sherpa-onnx evaluated but not shipped |
| TS-06 | C-06 | ICU4X `icu_casemap =2.4.0` + `icu_normalizer =2.4.0`, compiled Unicode 18.0.0 data, versioned canonical-caseless vocabulary matching, and ordered correction rules | Unicode D145 behavior is deterministic across hosts and uses bundled data without runtime providers | Compiled Unicode tables add transitive dependencies and binary bytes; measure with release builds. `llama.cpp`/Qwen remain excluded MVP |
| TS-07 | C-02,C-07 | Mac `objc2` + Accessibility/Quartz; Windows `windows` Win32/UIA; X11 XTest; Wayland `ashpd` portals | OS-native allowed insertion with safe fallback | `enigo` cross-OS abstractions insufficient to hide capability restrictions; manual keyboard hooks require maintenance |
| TS-08 | C-08 | `serde` + versioned JSON + atomic writes | Tiny local state; no SQL/database distribution burden | `rusqlite` bundled is feasible, but unnecessary for no-history MVP |
| TS-09 | C-09 | SHA-256 manifest generated in CI, signed app package, read-only bundled assets | Deterministic offline model; detect corrupt install | No CDN or model management server |
| TS-10 | C-10 | `tracing` (metadata only), Tauri opt-in signed full-app updater, cargo audits | Reliability without content telemetry | Automatic checks disabled by default, no hidden network |

### 2.0.1 Feature-to-stack trace (direct lookup)

This is deliberately redundant with [PRD §3.1](PRD.md#31-one-to-one-implementation-trace-matrix) to make stack changes auditable: F-01→TS-01,TS-09; F-02→TS-01,TS-07; F-03→TS-02,TS-03,TS-07; F-04→TS-02,TS-03; F-05→TS-03,TS-04; F-06→TS-05,TS-09; F-07→TS-06; F-08→TS-06,TS-08; F-09/F-10→TS-07; F-11→TS-01,TS-08; F-12→TS-01,TS-05,TS-08,TS-10; F-13→TS-09,TS-10; F-14→TS-05,TS-09; F-15→TS-06,TS-07,TS-08. The exact flow, acceptance and implementation tasks for each feature are in the PRD trace matrix.

### 2.1 Why Tauri rather than Slint, Iced, egui or Electron?

| Framework | Background cost | Developer experience | Native desktop integration | License | Outcome |
|---|---|---|---|---|---|
| Tauri 2 | Webview only when settings open; native Rust tray remains | HTML/CSS/TS mature | Strong cross-OS packaging/update plugins | MIT/Apache-2.0 | **Selected** |
| Slint | Compiled UI, potentially smaller footprint | Custom `.slint` DSL; less frontend web ecosystem | More native integration work; Rust support | GPL/commercial/royalty-free terms requiring review | **Contingency if Tauri fails memory** |
| Iced | Rust-native UI, GPU/backend-dependent | Elm-like patterns | More custom tray/distribution work | MIT | Valid alternative, higher integration effort |
| egui/eframe | Immediacy and flexible debugging | Very fast prototyping | More app-shell custom work | MIT/Apache-2.0 | For internal diagnostic tools |
| Electron | Chromium/Node packaged | Fast web development | Mature ecosystem | MIT plus bundled Chromium constraints | Not selected due footprint |

No framework memory numbers can be treated as measured totals. If Tauri settings-open and ASR coexistence exceeds RSS budget, benchmark a Slint prototype before changing core architecture. Tauri's reliance on WebView2/WKWebView/WebKitGTK includes OS/vendor dependency constraints. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

## 3. Speech recognition evaluation and hard gate

### 3.1 Primary candidate comparison

| ASR | Model/runtime footprint | Streaming | Accuracy evidence | Licensing | Decision |
|---|---|---|---|---|---|
| Moonshine **Small Streaming 123M** | Quantized `.ort` files; exact installed size and peak RSS **measure locally** | Yes | Vendor reports 7.84% 8-dataset English reference WER (**FP32** benchmark, not production quantized quality) | MIT streaming weights and core; transitive native notices required | **Primary MVP candidate** |
| Moonshine **Tiny Streaming 34M** | Smaller weights; measure exact size/CPU/RSS | Yes | Vendor reports 12.00% English reference WER, not equivalent to quantized production | MIT streaming weights | **Permitted fallback only if quality gate passes** |
| Moonshine Medium Streaming 245M | Higher weight/runtime pressure | Yes | 6.65% reference WER | MIT | Not preferred on 4 GB |
| whisper.cpp + Whisper tiny/base/small | Quantized GGML models; model-specific | Primarily windowed pseudo-streaming | Proven multilingual recognition family; latency/compute for continuous speech may be higher | MIT repo/model; verify individual assets | Architectural alternative, **not included MVP** |
| sherpa-onnx Zipformer/transducer | Depends on compatible graph | Yes on supported models | Depends entirely on selected model and evaluation | Runtime Apache-2.0; model licenses vary | Alternative benchmark candidate, not bundled |
| Parakeet 0.6B | Larger parameters and inference memory | Model/runtime-dependent | Competitive model cards, not comparable on same corpus | Model-specific (CC BY 4.0 for v3), attribution | Not selected for 4 GB default |
| Cloud ASR | Client small but needs Internet | Usually | Could improve quality | Variable | **Forbidden** |

Moonshine numbers are vendor-reported on separate test conditions; no common-device, same-corpus independent comparison has been performed here. Official [model catalog](https://github.com/moonshine-ai/moonshine/blob/main/docs/models/available-models.md), [quantization](https://moonshine-voice.readthedocs.io/en/stable/models/quantization/), [benchmark method](https://moonshine-voice.readthedocs.io/en/stable/using/benchmarks/), [commercial license exceptions](https://github.com/moonshine-ai/moonshine/blob/main/docs/license.md).

### 3.2 Mandatory Phase 0 ASR experiment

1. Pin the official Moonshine source commit, version of C header, and exact Small/Tiny quantized revision (not generated at runtime). Review per-file licenses, C/C++ third-party dependencies, and all .ort model components.
2. Compile release-mode static/dynamic binaries on Windows x64, macOS arm64/x64, Linux x86_64; build **with and without** unsupported CPU feature flags. Verify required CPU ISA on minimum device.
3. Bundle **only Small** in baseline installer and benchmark 100 short phrases, 50 longer phrases, 10-minute continuous session, and 30-minute capture; profile peak process tree RSS and installed bytes; verify CPU-only RTF.
4. Compare WER against same human labeled corpus using standard normalization; quantify improper proper noun, number and filler transformations separately.
5. If Small fails any hard cap, repeat with **only Tiny**; record accuracy regression. If Tiny fails quality requirement or still violates cap, stop—do not weaken privacy or hardware constraints.
6. Include `MODEL_DECISION.md` **in build evidence**, not as a sixth product blueprint: commit/hash, full file size, WER, p50/p95, RAM, build platform, license review, selected candidate, signoff.

**Commercial license nuance:** Moonshine states streaming STT models are MIT. Some *legacy nonstreaming non-English* models have a noncommercial Community license, so **never** assume V1 language assets are commercial merely because the repo is MIT; verify individual catalog entry and packaged transitive assets. The native runtime includes third-party libraries with independent notices. [License](https://github.com/moonshine-ai/moonshine/blob/main/docs/license.md).

### 3.3 Native API integration sequence

1. `moonshine_load_transcriber_from_files(path, MOONSHINE_MODEL_ARCH_SMALL_STREAMING, ..., MOONSHINE_HEADER_VERSION)`; 2. create/start stream; 3. feed mono `f32` samples with actual rate; 4. poll `moonshine_transcribe_stream` at 250–500 ms interval on ASR worker; 5. stop stream at user stop; 6. drain complete transcript; 7. free stream; 8. free transcriber at unload. The docs explicitly specify retained stream audio draining after `stop`, and result ownership. [Moonshine C API](https://moonshine-voice.readthedocs.io/en/stable/api/c-api/).

The header version currently documented is `MOONSHINE_HEADER_VERSION=30000` (3.0.0), **not a promise of ABI compatibility with arbitrary downloaded build**. Generate/import bindings only from the pinned checked-out header and verify runtime function symbols in CI. `moonshine_get_stt_dependencies` exists for developer downloads but must NOT be called by the consumer app.

## 4. Native audio and text insertion tradeoffs

**CPAL:** Provides platform-specific audio backend adapters. Windows minimum runtime can be constrained by WASAPI; for product certification explicitly target Windows 10/11, not the broadest OS a crate can theoretically build on. `cpal 0.18.2` supports ALSA/PipeWire/PulseAudio depending on features; choose one default backend and test distro runtime linkage. Native microphone capture itself is not “always listening”: open stream only during active dictation. [CPAL backends](https://docs.rs/crate/cpal/latest).

**Resampler:** `rubato 5.0.1`, using preallocated buffers; choose FFT or polynomial based on fixture accuracy and CPU benchmark. Input device may be stereo 48 kHz, model accepts explicit sample-rate parameter, but converting to mono stable rate is simpler and needs benchmarking.

**VAD:** Model-native segmentation first. Silero can help on silence/noise but its ONNX inference increases size and compute; add only if empty-input or endpointing tests fail. Unnecessary RNNoise denoiser excluded.

**Insertion decision table:**

| OS | Primary | Permissible fallback | Must not do |
|---|---|---|---|
| macOS | Supported AX editable text mutation where allowed | Quartz Unicode typing; then explicit copy | Bypass AX trust/sandbox; focus-stealing |
| Windows | `SendInput` Unicode keyboard to eligible same/lower integrity target, with UIA capability checks | Explicit copy | UIPI bypass, elevated helper |
| Linux X11 | X11 XTest simulated input on eligible target | Explicit copy | Root input daemon |
| Wayland | Compositor-supported portal RemoteDesktop with user grant; GlobalShortcuts portal | Tray toggle + explicit copy | Claim universally supported injection |

Windows `SendInput` is explicitly constrained by UIPI. [Microsoft docs](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-sendinput). Linux GlobalShortcuts emits activated/deactivated signals but compositor implementations vary. [GlobalShortcuts](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html), [RemoteDesktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html).

## 5. Suggested dependency manifests

These are **proposed exact pins for the currently verified public libraries**, NOT validated as a compiling unified dependency graph. Task T-022 creates a reproducible lockfile and resolves feature/ABI conflicts. Avoid adding unrelated crates simply because they are popular.

```toml
# Cargo.toml sketch — run cargo check and fix real feature bounds
[workspace]
members = ["crates/*", "src-tauri"]
resolver = "2"

[workspace.package]
edition = "2024"
rust-version = "1.99.1"

[workspace.dependencies]
serde = { version = "=1.0.229", features = ["derive"] }
tokio = { version = "=1.54.1", features = ["rt-multi-thread", "sync", "time"] }
cpal = "=0.18.2"
rubato = "=5.0.1"
ringbuf = "=0.5.2"

# platform specific in appropriate crate Cargo.toml:
# objc2 = "=0.6.5"      # target_os = "macos"
# windows = "=0.62.2"   # target_os = "windows", selective features
# ashpd = "=0.13.13"    # target_os = "linux", minimal features
# tauri = "=2.12.5"     # src-tauri only
# tauri-plugin-global-shortcut = "=2.4.1"
# tauri-plugin-updater = "=2.13.3" (explicit opt-in command only)
```

```json
{
  "private": true,
  "type": "module",
  "dependencies": {
    "react": "19.3.1",
    "react-dom": "19.3.1"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "6.1.2",
    "@typescript/native": "npm:typescript@7.0.2",
    "typescript": "npm:@typescript/typescript6@6.0.3",
    "typescript-eslint": "8.72.1",
    "vite": "8.3.5",
    "@tauri-apps/cli": "2.12.4"
  }
}
```

Vite 8 and Vitest 5 require Node.js **22.12.0 or newer** for this frontend setup; Node remains a **developer/build-only dependency**. Microsoft documents installing `@typescript/typescript6` under the `typescript` package name for API-consuming tools and TypeScript 7 under a separate npm alias. The build script calls the TypeScript 7 compiler by its explicit package path so npm's shared `tsc` shim cannot select the compatibility binary. Tauri JS API and individual plugin package versions must be aligned with actual Cargo dependencies and resolved from the official registry at lockfile generation. Do not add a component library or another UI framework without a separate decision.

### 5.1 Build and optimization profile

```toml
# Cargo.toml illustrative release profile; benchmark before enabling every knob
[profile.release]
opt-level = "z"
lto = "thin"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

**Tradeoff:** `opt-level="z"` reduces code size but can slow compute-heavy Rust paths; Moonshine native code has separate optimization. Benchmark `opt-level=3` and `s` for actual energy/CPU before pinning. Thin LTO may differ from fat LTO in binary size/speed. Keep debug symbols in private symbol server for crash analysis, never in user-visible transcript logs.

### 5.2 Reproducibility

- Commit `Cargo.lock`, npm lockfile, upstream Moonshine Git commit, model file SHA-256 manifest and transitive notices.
- Build native libraries from source in CI or vendor reviewed signed binaries with checksums; avoid fetching `HEAD` or `latest` dynamically during shipping build.
- Lock target compiler/linker and SDK per platform; code-sign after build. Each binary must match model architecture ABI and CPU capabilities.
- Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, tests, license audit, dependency audit, packaging checks.
- No model assets downloaded at installation/first run; CI may download/pin model from official CDN while building installer, with provenance record and immutable artifact store.

## 6. Platform support matrix

| Target | MVP certification | Notes |
|---|---|---|
| macOS 13+ arm64 | Tier 1 **subject to Phase 0 validation** | AppKit/AX and local audio; direct Developer ID distribution |
| macOS 13+ x64 | Tier 2 gated | Verify model CPU performance and Intel compatibility |
| Windows 10/11 x64 | Tier 1 | WASAPI, UIPI, WebView2 installed/runtime requirement, signing |
| Ubuntu-family X11 x64 | Tier 1 | WebKitGTK, tray, audio server dependencies; package-specific testing |
| Fedora-family X11 x64 | Tier 2 | Runtime library naming and packaging |
| GNOME/KDE Wayland x64 | Best-effort | Portal and compositor dependent shortcuts/injection; UI/Copy fallback required |
| Windows ARM64 / Linux ARM64 | Not MVP | Future if C ABI + model + installer and 4 GB cap pass |

**Do not advertise all OS versions** merely because Tauri or CPAL itself can compile. Supported matrix is narrower and must be established using real installer and interaction tests.

## 7. Security, supply chain, license, and standards

**Runtime permission strategy:** Tauri capability allowlist, no broad filesystem shell commands, no remote content, content security policy, allowlisted native commands, user-invoked update only. Remote host contacted for release metadata, not voice; validate pinned update key and full-app package signature. [Tauri permission guide](https://v2.tauri.app/learn/security/using-plugin-permissions/); [updater](https://v2.tauri.app/plugin/updater/).

**Release signing:** macOS Developer ID application + hardened runtime/notarization; Windows Authenticode or Artifact Signing as applicable; Linux verified package/checksums and distro packaging expectations. Re-sign as appropriate when embedding model binaries. Never disable TLS validation. [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution); [Microsoft signing](https://learn.microsoft.com/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control).

**Licensing bill of materials (SBOM):** store package name, source URL, version, license expression, license text hash, shipped file path, copied notices and proof that commercial redistribution is permitted. Distinguish Rust crate license from C++ native third-party licenses and model license. Tauri is MIT/Apache-2.0; Moonshine source/models MIT **except documented legacy models**, transitive dependency licenses separate. [Moonshine license](https://github.com/moonshine-ai/moonshine/blob/main/docs/license.md).

**Legal/security standards:** follow NIST SSDF 1.1 release practices; review applicability of GDPR, Canadian PIPEDA/BC PIPA, EU AI Act, EU Accessibility Act and WCAG2ICT. “Fully local” reduces egress but does not make the product automatically regulated-compliant. [NIST](https://csrc.nist.gov/pubs/sp/800/218/final); [WCAG 2.2](https://www.w3.org/TR/WCAG22/); [Canadian privacy](https://www.priv.gc.ca/en/privacy-topics/privacy-laws-in-canada/02_05_d_15).

## 8. Risks and explicit tradeoff decisions

| Risk | Severity | Mitigation / stop criterion |
|---|---|---|
| Small model peak RSS >950 MB | Critical | Benchmark Tiny; stop if Tiny fails quality or RAM |
| Quantized model quality below WER target | Critical | Frozen real-world fixture corpus; re-evaluate model/claims, no silent downgrade |
| Native ABI version mismatch | Critical | Build pinned source, bind matching headers, ASan/UBSan on FFI tests |
| Wayland unsupported global hold/insertion | High | Capability-based UI, portal tests and copy fallback; documented limitations |
| App Store sandbox prevents AX insertion | High | Direct signed Developer ID installers |
| Tauri WebView helper persists in idle | High | Destroy window; monitor process tree; switch UI only if profiler requires |
| Windows UIPI | Medium | Do not elevate; show CopyReady and explain |
| Unicode/IME and focus races | High | Session target identity, secure control detection, fuzz/integration tests |
| Privacy leak via diagnostics/clipboard | Critical | No content logs, explicit copy, filesystem/packet tests |
| Model CDN terms/license shift | High | Immutable provenance and license review at every build/update |
| App update temporarily needs extra disk | Medium | Preflight free space and rollback; documented staging exception |

## 9. Source index / verification links

1. [Tauri 2 releases](https://github.com/tauri-apps/tauri/releases), [global shortcut](https://tauri.app/release/global-shortcut/all-versions/), [updater](https://tauri.app/release/updater/all-versions/).
2. [Rust stable versions](https://blog.rust-lang.org/releases/), [CPAL](https://docs.rs/crate/cpal/latest), [rubato](https://docs.rs/crate/rubato/latest), [ringbuf](https://docs.rs/crate/ringbuf/latest/builds), [Tokio](https://docs.rs/crate/tokio/latest/builds).
3. [objc2](https://docs.rs/crate/objc2/latest), [windows](https://docs.rs/crate/windows/latest), [ashpd](https://docs.rs/crate/ashpd/latest), [Serde](https://docs.rs/crate/serde/latest), [rusqlite](https://docs.rs/crate/rusqlite/latest).
4. [TypeScript releases](https://www.npmjs.com/package/typescript?activeTab=versions), [Vite releases](https://www.npmjs.com/package/vite?activeTab=versions).
5. [Moonshine models](https://github.com/moonshine-ai/moonshine/blob/main/docs/models/available-models.md), [C ABI](https://moonshine-voice.readthedocs.io/en/stable/api/c-api/), [licenses](https://github.com/moonshine-ai/moonshine/blob/main/docs/license.md), [benchmarks](https://moonshine-voice.readthedocs.io/en/stable/using/benchmarks/).
6. [XDG desktop portals](https://flatpak.github.io/xdg-desktop-portal/docs/), [Microsoft SendInput](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-sendinput), [Apple AX trust](https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions).

**Decision changes:** Any change in TS-xx requires updating corresponding C-xx, F-xx, UF-xx, AC-xx and T-xxx references before implementation. See [TASKS §6](TASKS.md#6-change-control-and-agent-execution-rules).
