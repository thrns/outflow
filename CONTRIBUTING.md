# Contributing to Outflow

Outflow is proprietary software in development. Read [AGENTS.md](AGENTS.md) and the relevant project documents before changing behavior. Product scope lives in [docs/PRD.md](docs/PRD.md), Rust boundaries in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), approved dependencies in [docs/TECH_STACK.md](docs/TECH_STACK.md), task order in [docs/TASKS.md](docs/TASKS.md), and UI behavior in [docs/DESIGN.md](docs/DESIGN.md) and [docs/USER_FLOWS.md](docs/USER_FLOWS.md).

## Development prerequisites

- Rust **1.99.1** with Cargo, rustfmt, and Clippy. The root `rust-toolchain.toml` selects this toolchain and its two components.
- Node.js **22.12.0 or newer** and npm for the existing React/Vite frontend.

## Checks

Run Rust checks from the repository root:

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Run the frontend checks from the same root:

```sh
npm run typecheck
npm run test -- --run
npm run lint
npm run build
```

To print the Rust foundation smoke-test message, run:

```sh
cargo run -p outflow-core --example foundation
```

## Change boundaries

- Keep `outflow-core` independent of React, Tauri, operating-system APIs, and external services.
- Do not add Rust or npm runtime dependencies without the review required by `AGENTS.md` and `docs/TECH_STACK.md`.
- Do not add speech capture, settings persistence, a Tauri shell, telemetry, cloud services, or transcript/audio retention as part of the foundation.
- Do not add GitHub Actions workflows. Use the local commands above and record actual results in the task evidence file.
- Keep `Cargo.lock` and `package-lock.json` in sync with their manifests; do not update unrelated dependency versions.
- Do not mark a task complete until its documented definition of done and evidence requirements are met.
