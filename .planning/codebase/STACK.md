# Technology Stack

**Analysis Date:** 2026-06-13

## Languages

**Primary:**
- Rust (edition 2024, MSRV 1.96) - Desktop GUI application and all business logic

## Runtime

**Environment:**
- Desktop via Dioxus 0.7 desktop runtime

**Package Manager:**
- Cargo (Rust package manager)
- Lockfile: `Cargo.lock` (present)

## Frameworks

**Core:**
- Dioxus 0.7 (with `desktop` and `router` features) - Desktop GUI framework for the NodeDB Studio client

**UI Rendering:**
- Dioxus desktop - Native window management and rendering via `tao` (cross-platform windowing)

**Async Runtime:**
- Tokio 1 (with `rt-multi-thread` and `macros` features) - Multi-threaded async executor for I/O operations

## Key Dependencies

**Critical (NodeDB Backend):**
- `nodedb-client` 0.3.0 (locally patched via `.cargo/config.toml`) - Rust client library for NodeDB database communication
- `nodedb-types` 0.3.0 (locally patched via `.cargo/config.toml`) - Shared types and domain models from NodeDB

**Serialization:**
- `sonic_rs` 0.5 - Fast JSON serialization/deserialization for runtime JSON display (never `serde_json`)
- `serde` 1 (with `derive` feature) - Serialization framework for model derives only (not runtime parsing)

**Error Handling & Diagnostics:**
- `thiserror` 2.0 - Typed error definitions via derive macros
- `tracing` 0.1 - Structured logging and diagnostics framework

## Local Dependency Setup

**NodeDB Client Integration (Critical):**

The workspace depends on `nodedb-client` and `nodedb-types` from a separate NodeDB repository. These are NOT published to crates.io yet. Configure local path patching via `.cargo/config.toml` (gitignored):

```toml
[patch.crates-io]
nodedb-client = { path = "../nodedb/nodedb-client" }
nodedb-types  = { path = "../nodedb/nodedb-types" }
```

The version pinned in `Cargo.toml` (0.3.0) must match the local NodeDB workspace version.

**What nodedb-client provides:**
- `NativeClient` implementing the NodeDB wire protocol (MessagePack on port 6433)
- A typed trait-based API wrapping the network layer
- All domain types and value models

## Build & Test Tooling

**Build:**
- `cargo build` - Standard debug build
- `cargo build --release` - Optimized release binary (LTO: thin, strip: enabled)

**Development:**
- `dx serve` (via dioxus-cli, optional) - Hot-reload development mode

**Code Quality (CI Gates):**
- `cargo fmt --all` - Code formatting (CI checks with `--check`)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` - Lints (warnings block merge)

**Testing:**
- `cargo nextest run` - Test runner (preferred over `cargo test`)
- `cargo nextest run -E 'test(my_test_name)'` - Single test by name
- Install: `cargo install cargo-nextest --locked`

**Verification Commands (run locally before pushing):**
```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run
```

## Configuration

**Environment:**
- Desktop window size: 1440x900 (hardcoded in `main.rs`)
- Theme: Auto-detects OS dark/light preference

**Build Profiles:**
- `dev`: `debug = "line-tables-only"` for debug info with faster builds; package dependencies use `debug = false`
- `release`: Thin LTO + stripping for smaller binaries

**Rust Toolchain:**
- MSRV: 1.96 (pinned in workspace `Cargo.toml`)
- Run `rustup update stable` if toolchain is too old

## Workspace Structure

**Root Level:**
- `Cargo.toml` - Workspace manifest with workspace dependencies and profiles
- `Cargo.lock` - Dependency lockfile

**Member Crate:**
- `nodedb-studio/Cargo.toml` - The app crate (single binary member)
- `nodedb-studio/src/` - All source code for the desktop client

## Platform Requirements

**Development:**
- Recent stable Rust (1.96+)
- macOS, Windows, or Linux with X11/Wayland (via Dioxus desktop/tao)

**Production:**
- Standalone desktop application
- Targets: Windows (MSVC), macOS (Intel and Apple Silicon), Linux (x86_64)
- Requires local NodeDB instance running (default: localhost:6433 MessagePack port)

---

*Stack analysis: 2026-06-13*
