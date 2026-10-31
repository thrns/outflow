# Outflow

Outflow is an offline desktop dictation application in development. This repository contains a browser-based React foundation screen and Settings preview, a Rust workspace foundation, and a native Rust settings schema v1 model with JSON serialization, validated decoding, explicit-path file loading and safe initialization, and an atomic JSON writer on Unix. The Tauri desktop shell, microphone and dictation flows, Windows owner-only ACL writing, and release packaging have not been implemented.

## Prerequisites

- Node.js **22.12.0 or newer**. The Vite 8 and Vitest 5 requirements set this minimum. The suggested version is recorded in `.node-version`.
- npm, included with Node.js.
- Rust **1.99.1** and Cargo. The root `rust-toolchain.toml` selects the documented toolchain and installs the `rustfmt` and `clippy` components when needed.

Vite documents its [Node.js support range](https://vite.dev/guide/) and Vitest documents its [Node.js requirement](https://vitest.dev/guide/).

## Rust workspace foundation

The Cargo workspace is separate from the React/Vite app. `outflow-core` remains dependency-free; `outflow-settings` uses pinned Serde and `serde_json` dependencies for schema v1 JSON:

```text
Cargo.toml
rust-toolchain.toml
crates/
├── core/
│   ├── Cargo.toml
│   ├── src/lib.rs
│   └── examples/foundation.rs
└── settings/
    ├── Cargo.toml
    ├── src/lib.rs
    ├── examples/defaults.rs
    ├── examples/json_roundtrip.rs
    ├── examples/version_compatibility.rs
    ├── examples/load_settings.rs
    └── examples/initialize_settings.rs
```

The `outflow-core` library currently provides a development-only status function. Run its example from the project root:

```sh
cargo run -p outflow-core --example foundation
```

Expected output:

```text
Outflow Rust foundation ready
```

Run the Rust workspace checks independently of Vite or a browser:

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

These are development checks only; the examples are not application runtime features. No Tauri windows, automatic settings recovery, microphone capture, or dictation behavior is part of this Rust foundation.

## Native Rust settings schema v1

The independent `outflow-settings` crate under `crates/settings/` defines local configuration data, JSON serialization, validation, an explicit-path read-only loader and initializer, and an explicit-path atomic writer. It uses `serde = 1.0.229` with derive support and `serde_json = 1.0.154`. It has no Tauri or operating-system integration dependency and performs no network operations. The loader uses standard-library filesystem APIs; the writer is currently implemented on Unix and these APIs do not connect to the React preview.

Schema version **1** is the only version supported by this model. Defaults are:

| Setting | Default |
|---|---|
| Schema version | `1` |
| Language | English (`en`) |
| Push-to-talk and toggle shortcuts | Unconfigured (`None`); no universal shortcut is selected |
| Input device | System default (`None`) |
| Cleanup mode | `Clean` |
| Remove fillers | `true` |
| Run at login | `false` |
| Update checks | User-initiated only (`true`) |
| Vocabulary | Empty |

The JSON schema v1 shape is:

```json
{
  "schema_version": 1,
  "language": "en",
  "recording": {
    "push_to_talk": null,
    "toggle": null,
    "input_device": "system-default"
  },
  "cleanup": {
    "mode": "clean",
    "remove_fillers": true
  },
  "startup": {
    "run_at_login": false
  },
  "updates": {
    "user_initiated_only": true
  },
  "vocabulary": []
}
```

Configured shortcuts are strings; an unconfigured shortcut is explicitly `null`. In the Rust API, `RecordingSettings::input_device == None` means the system default and maps to the JSON string `"system-default"`; a configured device key maps to its string value. Cleanup modes serialize as `"clean"` and `"verbatim"`. Field names are snake case, and Unicode vocabulary strings round-trip unchanged.

`Settings::validate()` rejects unsupported schema versions and languages, automatic update checking, duplicate or empty configured shortcuts, more than 500 vocabulary entries, empty source/replacement phrases, phrases over 128 Unicode scalar values, and duplicate source phrases ignoring case. `Settings::to_json()` validates before encoding. `Settings::from_json()` rejects malformed documents, unsupported schema versions, missing required fields (including omitted nullable shortcut keys), unknown fields, and invalid settings; it reuses `Settings::validate()` for schema, language, update-policy, shortcut, and vocabulary rules. Errors retain parser locations or safe validation details, never raw JSON or vocabulary/shortcut values.

Schema version **1** is the only supported version; unsupported versions are rejected. JSON encoding and decoding are in-memory operations. The separate atomic writer can save validated settings on Unix, and the explicit-path loader can read them; schema migration and frontend/Tauri integration are not implemented. Do not treat the schema codec or file operations as evidence that `T-017`, `T-018`, `AC-11`, or `AC-12` is complete; their broader prerequisites and definitions of done remain outstanding.

Run the non-sensitive defaults example and Rust tests with:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example defaults
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example json_roundtrip
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
```

The example prints:

```text
Outflow settings schema v1 ready
Language: English
Cleanup: Clean
Launch at Login: Off
```

These Rust defaults are available to native code; file loading and writing are available through separate explicit-path APIs below, with no frontend bridge. The React Launch at Login switch remains an independent preview-only boolean held in React memory; changing it does not mutate a Rust `Settings` value or alter operating-system startup behavior.

The JSON round-trip example constructs defaults, serializes and decodes them in memory, validates the result, and checks equality. It prints only:

```text
Outflow settings JSON round-trip passed
Schema version: 1
Validation: passed
```

### Settings schema version compatibility

Schema version **1** is the only supported version. Version **0** is rejected because the project has no historical v0 settings shape; it must not be guessed or converted. Versions greater than 1 are rejected until an explicit, tested migration path exists. Missing, `null`, negative, non-integer, string, malformed, or out-of-range version fields are rejected. The decoder never treats an unknown version as v1 and never substitutes defaults after an error. Errors do not include raw JSON or vocabulary text.

The loader and atomic writer accept schema v1 only. A version may be migrated only through an explicitly defined and tested path. Unknown newer versions must be rejected rather than downgraded; invalid or unsupported settings must not be overwritten. Any future migration must preserve user settings and vocabulary, or report failure without changing the original configuration. No v0-to-v1 migration is planned without evidence of a real historical v0 schema.

Run the compatibility example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example version_compatibility
```

The example constructs synthetic documents in memory, performs no file or network operations, and prints no configuration values. Expected output:

```text
Outflow settings version policy ready
Schema v1: supported
Schema v0: unsupported
Schema v2: unsupported
Missing version: rejected
```

### Settings file loading

`outflow_settings::load_settings(&path)` reads only the explicit path supplied by its caller and returns `Result<Option<Settings>, SettingsStorageError>`. It does not discover an Outflow configuration directory, create a file, apply settings, migrate versions, or change the source file. `Ok(None)` means the file is genuinely missing; a startup layer may choose `Settings::default()` in that case. Errors never fall back to defaults.

The loader rejects directories, symbolic links, files larger than **1 MiB (1,048,576 bytes)**, invalid UTF-8, malformed JSON, unsupported schema versions, and schema v1 values that fail validation. It reuses `Settings::from_json()` for decoding and validation. Reads are capped at one byte over the maximum so a file that grows after metadata inspection is rejected without being read fully into memory. Some schema v1 string fields have no separate byte-length limit, so unusually large but otherwise well-formed files are also rejected by this loader cap. Filesystem errors identify the safe operation and permission failures without including paths or operating-system error text. JSON errors retain only a line and column; errors do not include raw JSON, vocabulary, or shortcut values.

The loader checks path metadata before opening and checks the opened file and path again. The standard library does not provide a portable no-follow open operation for all supported targets, so a process that can concurrently replace the file may still race these path-based checks on some platforms. This is a documented limitation, not complete filesystem attack resistance. Real filesystem tests have run on **macOS 26.6.2 arm64 only**. Linux and Windows have not been tested. The existing atomic writer remains Unix-only; Windows writing still fails closed pending an approved owner-only ACL implementation.

Run the example from the repository root. It creates synthetic files in a temporary directory, prints no configuration values, and removes its fixtures on exit:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example load_settings
```

Expected output:

```text
Outflow settings load ready
Valid v1: loaded
Missing file: defaults available
Invalid file: rejected
```

Run loader and atomic-writer regression tests with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
```

### Safe settings initialization

`outflow_settings::initialize_settings(&path)` uses the explicit-path loader and returns one of two results:

- `SettingsInitialization::Loaded(settings)` when an existing schema v1 file is valid. The file is left unchanged.
- `SettingsInitialization::FirstRun(settings)` when the file is missing. The compiled-in defaults are validated and returned in memory; no file is created.

Malformed JSON, unsupported schema versions, invalid values, invalid UTF-8, oversized files, unsafe paths, permission denials, and other filesystem failures return the existing structured `SettingsStorageError`. Initialization never turns a loading error into defaults and never repairs, deletes, renames, backs up, or overwrites a file. Error text contains safe error categories and parser locations, not raw JSON or vocabulary/shortcut values.

Initialization does not save defaults automatically so first-run does not silently create persistent settings, and a damaged or newer configuration cannot be silently replaced. A future caller may choose when to persist settings through the existing `save_settings_atomic()` API. Current recovery is detection and reporting only: there is no user-directed reset, backup restore, schema migration, configuration-directory discovery, Tauri command, or frontend synchronization in this slice. Keep an invalid source file intact for future user-directed recovery.

Run the synthetic temporary-directory example and the Rust tests from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example initialize_settings
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
```

The example removes its temporary directory on exit and prints:

```text
Outflow settings initialization ready
Missing configuration: defaults initialized
Valid configuration: loaded
Invalid configuration: recovery required
Original invalid file: preserved
```

The verified run on macOS 26.6.2 arm64 passed all **63 Rust workspace tests** (61 settings and 2 core). The existing **16 frontend tests** also passed; no frontend source changed.

Troubleshooting:

- **First run returns defaults:** this is expected when the explicit path is missing; defaults remain in memory until an approved caller saves them.
- **Malformed JSON, unsupported version, or invalid settings:** initialization stops with a redacted error and leaves the original file unchanged. Preserve it for future user-directed recovery; no automatic repair or migration is available.
- **Invalid UTF-8 or file too large:** the loader rejects the file before decoding and does not truncate or replace it.
- **Permission or filesystem error:** initialization returns the loader's structured failure instead of defaults. Check that the supplied path is a readable regular file and that the parent is accessible. Error messages omit paths and operating-system error text.

These behaviors were exercised on macOS 26.6.2 arm64 only. Linux and Windows were not tested in this task; the existing writer remains Unix-only.

### Atomic settings file writing

`outflow-settings::save_settings_atomic(path, &settings)` validates the value, encodes it with the existing schema v1 JSON codec, and writes to a unique temporary file in the supplied destination directory. On Unix it creates that file with owner-only mode `0600`, writes and flushes the complete document, synchronizes the file, then atomically renames it over the destination and synchronizes the parent directory. It rejects symbolic-link and non-regular destinations. A failure before rename leaves an existing destination intact; the writer never deletes the old file first. If the rename succeeds but parent-directory synchronization fails, it returns `ReplacementDurabilityUncertain` because the new file is visible but crash durability could not be confirmed.

Windows is currently unsupported by this writer. The repository has no approved Windows ACL dependency or implementation, and the API returns an explicit unsupported-platform error instead of writing with inherited permissions. The local filesystem behavior was exercised on macOS arm64; Windows and Linux were not tested. POSIX mode `0600` was verified for the resulting file, but ACL behavior on arbitrary custom parent directories has not been audited.

The writer takes an explicit path and does not discover a configuration directory. It does not load, recover, migrate, watch, back up, or connect settings to Tauri or the frontend. The example below uses only synthetic defaults in a temporary test directory and removes that directory on exit:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example atomic_write
```

Expected output:

```text
Outflow atomic settings write passed
Settings validation: passed
JSON file: verified
```

Run the workspace checks from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
```

The storage tests cover schema v1 output and decoding, replacement, invalid and unsupported values, permission denial, simulated partial writes and file-sync failure, pre-replacement interruption, post-replacement durability uncertainty, temporary-file cleanup, missing and invalid parents, symbolic links, restrictive Unix mode, and redacted errors. They do not claim Windows ACL or Linux filesystem verification.

Run all requested local verification commands from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
git diff --check
```

## Installation

From the project root, install the dependencies:

```sh
npm install
```

For a clean, reproducible install from the npm lockfile, use:

```sh
npm ci
```

## Development server

Start the development server on the local loopback interface:

```sh
npm run dev -- --host 127.0.0.1
```

Open [http://localhost:5173](http://localhost:5173). Vite prints the exact local URL when the server is ready.

If port 5173 is already in use, Vite selects another port unless `--strictPort` is set. Open the URL printed in the terminal.

## Foundation screen and Settings preview

The foundation screen displays:

- **Outflow** as the application name
- **Foundation ready** as the page heading
- **Your application is set up and ready for development.** as supporting text
- A **Settings** button that opens the development preview

In Settings, use the six section tabs—General, Microphone, Shortcuts, Text & Vocabulary, Privacy, and About & Diagnostics—to change the displayed section. General is selected each time the preview opens. Use **Back** to return to the foundation screen. The tabs support Up/Down arrow keys, Home, and End, with visible keyboard focus.

General contains **Launch at Login**, which defaults to **Off**. Use the switch with a mouse or focus it and press Space. The visible On/Off state and accessible switch state update immediately. The preference is kept only in React memory: it remains selected while changing sections or closing and reopening Settings in the same mounted session, and resets to Off after a full page reload. The other five sections still show their title, description, and placeholder note.

The General section displays: **“Start Outflow automatically when you sign in to your computer.”** and **“Preview only — this setting does not change your computer's startup behavior yet.”** No operating-system login item is registered, and the preference is not written to a configuration file, localStorage, or sessionStorage. The section selection is not persisted. Microphone access, shortcut settings, diagnostics, native Tauri tray/window behavior, and all other dictation features remain unimplemented. The browser preview does not complete F-11, AC-11, T-051, or native startup integration in [`docs/TASKS.md`](docs/TASKS.md). The Rust schema contributes to T-017 but does not complete its migration behavior or its T-001 prerequisite.

The documented Base UI/Nova Switch and Field wrappers are not installed in this React/Vite scaffold (`@base-ui/react`, `components/ui/switch`, and `components/ui/field` are absent). To avoid adding a dependency, the preview uses a native button with `role="switch"`, `aria-checked`, label/description references, Space-key support, and the existing neutral and focus tokens. No component library was added.

The page uses semantic HTML, a neutral color palette, and a responsive centered layout. It adapts to the system's light or dark color preference.

## Testing and checks

The frontend checks remain separate from the Rust workspace and can be run without launching the Rust example:

Run the automated tests once:

```sh
npm run test -- --run
```

Run the tests in watch mode during development:

```sh
npm run test
```

Run the linter and TypeScript type check:

```sh
npm run lint
npm run typecheck
```

`npm run typecheck` invokes the TypeScript 7.0.2 compiler directly. This project also installs TypeScript 6.0.3 under the `typescript` package name because `typescript-eslint` currently uses its JavaScript API, which TypeScript 7 does not provide. The npm aliases follow [Microsoft's documented TypeScript 7 compatibility setup](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/): `@typescript/native` points to TypeScript 7, while `typescript` points to `@typescript/typescript6`. The explicit compiler path keeps app type checking on TypeScript 7 even if npm links a shared `tsc` executable to TypeScript 6.

## Production build

Create a production build:

```sh
npm run build
```

The command type-checks with TypeScript 7.0.2 before building the static app with Vite. Output is written to `dist/`. To serve that output locally, run:

```sh
npm run preview
```

## Initial Settings layout verification — 2026-10-30

| Command | Result |
|---|---|
| `npm run typecheck` | PASS; TypeScript 7.0.2 |
| `npm run test -- --run` | PASS; 1 test file, 11 tests, including all 4 foundation tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 production build |
| `git diff --check` | PASS; the repository has no commits and the source files are untracked in this checkout, so Git's normal diff check does not include them |

The development server was started with `npm run dev -- --host 127.0.0.1`. Port 5173 was already occupied and displayed the earlier foundation screen; the initial Settings layout preview ran at `http://localhost:5174/`.

The initial browser preview was visually inspected at **1440×900**, **768×800**, **390×844**, and **320×640**. Navigation sat beside the content at 1440px and 768px, then stacked above it at 390px and 320px. The 320px layout scrolled vertically to reveal the full panel and had no horizontal overflow. All six tabs were selected in the browser; each updated both the panel title/description and selected state. Light and dark system appearances were inspected. A keyboard-only pass opened Settings from the foundation, moved through tabs with arrows and Home/End, showed a visible focus ring, and activated Back; reopening returned to General. The browser accessibility tree exposed the navigation, tab list, selected tabs, panel, and headings. The system appearance was restored to its original Auto setting after the dark-theme check.

Text sizes use relative units and the content wraps, but direct 200%/400% browser-zoom testing was not completed. The narrow viewport checks exercise the same stacked layouts, but they do not replace a zoom test. A screen-reader session was also not run.

At that point, the checkout had no Git commits and all provided project files were untracked. `git diff --check` did not inspect those untracked files; the implementation files were reviewed directly.

## Launch at Login preview verification — 2026-10-30

The current preview was started with `npm run dev -- --host 127.0.0.1`. Ports 5173 and 5174 were occupied, so Vite served the latest source at `http://127.0.0.1:5175/`.

| Command | Result |
|---|---|
| `npm run typecheck` | PASS; TypeScript 7.0.2 |
| `npm run test -- --run` | PASS; 1 test file, 16 tests (the 11 existing cases remain) |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 production build |
| `git diff --check` | PASS; this checkout has no commits and the project files are untracked, so the command does not cover them; changed files were inspected directly |

Browser verification confirmed the setting defaults to Off; mouse and Space change it to On and back to Off; keyboard focus is visible; and the accessibility tree exposes a named switch with its checked value. The preview value stayed On after switching to Privacy and back, and after closing and reopening Settings; a page reload reset it to Off. The foundation screen and all other Settings sections remained available.

The current layout was inspected in light appearance at **1440×900**, **768×800**, **390×844**, and **320×640**, with no horizontal overflow. The navigation and panel sit side-by-side at 1440px and 768px and stack at 390px and 320px; narrow layouts scroll vertically to show the full panel. Dark appearance was inspected at 320px in macOS System Settings, and the system appearance was restored to its original **Auto** selection. The browser accessibility tree confirmed the label, description, switch role, state, and preview-only notice.

**Unverified:** 200% browser zoom could not be confirmed with the available in-app browser controls; no native screen-reader session or contrast-tool audit was run. The switch uses the documented behavior and existing semantic colors, but this preview is not an accessibility certification.

## Native settings schema v1 verification before JSON codec — 2026-10-30

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable rustc --version` | PASS; Rust 1.99.1 |
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example defaults` | PASS; printed the four lines shown above |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check -p outflow-settings --locked` | PASS; settings crate builds without external dependencies |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 17 tests total (15 settings, 2 original foundation tests) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 1 file, 16 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 |
| `git diff --check` | PASS; no tracked changes are available in this no-commit checkout, so the new files were also inspected directly |

The current preview ran at `http://127.0.0.1:5175/` because ports 5173 and 5174 were already in use. Visual verification confirmed the foundation screen, Settings navigation with all six sections, Launch at Login changing from Off to On and back to Off, the preview-only notice in the accessibility tree, and Back returning to the foundation screen. The development server was stopped after verification. No React source changed.

At this earlier checkpoint, the settings model and validation existed before JSON support was added. The subsequent JSON codec verification below records the current serialization status. Migration, file persistence, Tauri integration, and a React connection remain out of scope, and T-001 remains unchecked.

## Native settings JSON codec verification — 2026-10-30

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo generate-lockfile --offline` | PASS; locked the approved exact Serde and `serde_json` versions and their transitive dependencies |
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example json_roundtrip` | PASS; printed the expected three lines below |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 32 tests total: 30 settings tests and 2 core tests; all 17 pre-existing workspace tests remain passing |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS; an initial derivable-default warning was corrected before the final run |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; all 16 frontend tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 |
| `git diff --check` | PASS; this checkout has no commits and project files are untracked, so the changed files were reviewed directly as well |

Expected example output:

```text
Outflow settings JSON round-trip passed
Schema version: 1
Validation: passed
```

The codec uses `serde = 1.0.229` with derive support and `serde_json = 1.0.154`. It validates on encode and decode, rejects missing and unknown fields, unsupported schema versions and invalid values, and keeps error messages free of raw JSON and vocabulary. It performs no file or network operations. No React source changed. T-017 and AC-11 remain incomplete because schema migration, persistence, the T-001 prerequisite, and broader product acceptance work remain outstanding.

## Settings schema compatibility verification — 2026-10-30

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example version_compatibility` | PASS; printed the five expected policy lines below |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 33 tests total: 31 settings tests and 2 core tests. All 32 previously existing Rust tests remain passing. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; all 16 frontend tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 production build |
| `git status --short` | PASS; this checkout has no commits, so project files remain untracked; scoped source and new files were inspected directly |
| `git diff --check` | PASS; no tracked baseline is available in this checkout; a direct trailing-whitespace scan of the scoped files was also clean |

Example output:

```text
Outflow settings version policy ready
Schema v1: supported
Schema v0: unsupported
Schema v2: unsupported
Missing version: rejected
```

The example and its dependencies operate on synthetic in-memory JSON; source review confirms no file or network APIs are used. No configuration file was created, no network request was initiated by the example, and no React source changed. This is an in-memory compatibility check, not packet-capture evidence for the broader application. Current limitations remain: there is no historical v0 schema, migration implementation, settings persistence, Tauri integration, or frontend bridge. T-001 remains unchecked, and T-017 and AC-11 remain incomplete.

## Foundation verification (original task)

The foundation was verified on macOS with Node.js 22.23.3 and npm 10.9.9:

| Command | Result |
|---|---|
| `npm install` | PASS; dependencies installed, no reported vulnerabilities |
| `npm ci` | PASS; clean install from `package-lock.json`, no peer-dependency overrides |
| `npm run typecheck` | PASS; TypeScript 7.0.2 |
| `npm run test -- --run` | PASS; 1 test file, 4 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 production build |

The running page was visually inspected in the browser at **1440×900**, **390×844**, and **320×640**. The screen remained centered and readable at each size, with no visible horizontal overflow. Its accessibility tree exposed the application name, one level-one heading, and the supporting description.

## Troubleshooting

- **Rust or Cargo is missing:** install Rust 1.99.1 with rustup, then open a new shell and run `rustc --version` and `cargo --version`. The repository toolchain file also requests the required toolchain components.
- **The pinned Rust toolchain cannot be installed:** check network access to the configured Rust distribution server, then run `rustup toolchain install 1.99.1 --component rustfmt --component clippy` and retry the Cargo command.
- **Cargo reports a lockfile or workspace error:** run Cargo from the repository root, confirm `Cargo.toml`, `crates/core/Cargo.toml`, and `crates/settings/Cargo.toml` are present, then regenerate the lockfile with `cargo generate-lockfile` only when manifests have intentionally changed. Review any lockfile diff before keeping it.
- **Rust formatting or Clippy components are unavailable:** run `rustup component add rustfmt clippy --toolchain 1.99.1` and retry the relevant command.
- **`outflow-settings` rejects a settings value:** check the supported schema version, English-only language, user-initiated update policy, shortcut uniqueness, and vocabulary count/phrase limits in the validation error. JSON parse errors show only a line and column; validation and storage errors do not include phrase or shortcut contents. Schema versions other than v1 and unknown fields are rejected.
- **Settings loading reports malformed JSON or invalid UTF-8:** the file is left untouched. Check the file's encoding and JSON syntax, then compare its keys and values with the schema v1 example above. The loader does not repair the file or substitute defaults; preserve a copy before making any manual correction.
- **Settings loading reports an unsupported version or invalid values:** the loader does not migrate or overwrite the file. Version 1 is the only accepted schema. Correct the source using a known schema v1 document or preserve it for a future migration path; do not rename it to another version.
- **Settings loading reports a permission or filesystem error:** confirm the explicit file path exists, is a regular readable file, and is not a symbolic link. The loader does not print the path or operating-system error text. A missing path returns `Ok(None)`; a directory or unreadable existing path is an error.
- **Settings loading rejects a file over 1 MiB:** the loader enforces this resource bound before decoding. Some unusually large string values may exceed it even if they otherwise pass schema validation. Inspect the file size and content locally; loading will not truncate or modify it.
- **Atomic save reports permission or filesystem failure:** confirm the explicit parent directory exists and is writable, the destination is a regular file rather than a symbolic link, and the filesystem permits same-directory atomic rename and directory synchronization. An error before replacement preserves existing contents; `ReplacementDurabilityUncertain` means replacement already happened but directory durability could not be confirmed.
- **Atomic save reports unsupported platform:** current owner-only write guarantees are implemented only on Unix. Windows support requires an approved ACL implementation; the writer fails closed there.
- **Atomic writer example fails:** check that the system temporary directory is available and supports Unix mode bits and same-directory rename; the example prints no settings values or absolute paths and removes its test directory when it exits.
- **Node version warning or syntax errors:** run `node --version` and switch to Node.js 22.12.0 or newer.
- **Dependency or peer-resolution errors:** run `npm ci` from the project root. Keep both TypeScript aliases in `package.json`; do not use `--force` or `--legacy-peer-deps`.
- **Port 5173 is already in use:** Vite chooses an available port by default. Open the URL printed by Vite, or pass a free port explicitly, for example `npm run dev -- --host 127.0.0.1 --port 5174`.
- **Launch at Login returns to Off after a reload:** expected for this preview; the value is held in memory and is not persisted.
- **The computer does not start Outflow at login:** expected; this preview does not register an operating-system login item.
- **The browser cannot connect:** keep the development server running and open the local URL printed in its terminal output.
- **A stale or incomplete dependency install:** run `npm ci`, then rerun the failing command.

## Current implementation status

The starter screen and Settings layout/navigation preview are implemented, with one browser-only Launch at Login preference. The dependency-free `outflow-core` Rust foundation provides a smoke-test status message; `outflow-settings` provides schema v1 defaults, JSON serialization/deserialization, validation, explicit-path loading and safe initialization, and explicit-path atomic writes on Unix. Settings initialization and filesystem loading were tested on macOS 26.6.2 arm64 only; Linux and Windows were not tested. Migration, user-directed recovery, Windows ACL writing, and frontend persistence remain unimplemented. The React preference does not persist or affect computer startup, and it is not connected to the Rust model. The other Settings sections remain placeholders; no Home features, reusable component library, Tauri shell, microphone capture, or dictation behavior is included yet. T-001, T-017, and T-018 remain unchecked; T-017 is incomplete because migration behavior and its T-001 prerequisite remain outstanding, and T-018 is incomplete because the cross-platform writer requirements are not met. F-11, AC-11, AC-12, T-051, and native startup integration remain incomplete. The Rust crates are development foundations, not application runtime features.
