# CLAUDE.md

@AGENTS.md

<!-- GSD:project-start source:PROJECT.md -->
## Project

**NodeDB Studio**

NodeDB Studio is a **desktop GUI client for NodeDB** — the query editor, data browser, and administration surface for NodeDB's multi-model engines (document, vector, graph, full-text, KV, plus timeseries/spatial/streams/cluster). Built in Rust with Dioxus 0.7 (desktop). The complete UI shell already exists and renders against hardcoded mock data behind a single `ConnectionService` seam; this milestone makes it real by wiring that seam to the live `nodedb-client`.

**Core Value:** A user can connect to a real NodeDB instance, run SQL, and browse/inspect their actual data — the studio shows live database state, not mock data.

### Constraints

- **Tech stack**: Rust edition 2024, MSRV 1.96, Dioxus 0.7 (`desktop` + `router`). No new deps or version bumps without asking (AGENTS.md).
- **Transport**: native `nodedb-client` over MessagePack (:6433). Desktop-only — raw TCP, so no browser target this milestone.
- **Conventions** (AGENTS.md, hard rules): no `.unwrap()`/`.expect()`/`panic!` in non-test code — typed `thiserror` errors, propagate with `?`, never `Result<T, String>`; `mod.rs` is re-exports only; files < 500 LOC; `sonic_rs` (never `serde_json`); `nodedb_types::Value` for DB values; no `_ =>` on exhaustive domain enums.
- **Dioxus 0.7**: never hold a `.read()`/`.write()` guard across `.await`; `.peek()` in handlers; stable list keys; never block the main thread (async IO via `spawn`/`use_resource`/`use_action`); keep logic in plain Rust for testability.
- **Seam discipline**: async lands at the `ConnectionService` boundary, not scattered through views. Mock impl must keep working alongside the real one.
- **CI gate**: `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo nextest run` must pass.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust (edition 2024, MSRV 1.96) - Desktop GUI application and all business logic
## Runtime
- Desktop via Dioxus 0.7 desktop runtime
- Cargo (Rust package manager)
- Lockfile: `Cargo.lock` (present)
## Frameworks
- Dioxus 0.7 (with `desktop` and `router` features) - Desktop GUI framework for the NodeDB Studio client
- Dioxus desktop - Native window management and rendering via `tao` (cross-platform windowing)
- Tokio 1 (with `rt-multi-thread` and `macros` features) - Multi-threaded async executor for I/O operations
## Key Dependencies
- `nodedb-client` 0.3.0 (locally patched via `.cargo/config.toml`) - Rust client library for NodeDB database communication
- `nodedb-types` 0.3.0 (locally patched via `.cargo/config.toml`) - Shared types and domain models from NodeDB
- `sonic_rs` 0.5 - Fast JSON serialization/deserialization for runtime JSON display (never `serde_json`)
- `serde` 1 (with `derive` feature) - Serialization framework for model derives only (not runtime parsing)
- `thiserror` 2.0 - Typed error definitions via derive macros
- `tracing` 0.1 - Structured logging and diagnostics framework
## Local Dependency Setup
- `NativeClient` implementing the NodeDB wire protocol (MessagePack on port 6433)
- A typed trait-based API wrapping the network layer
- All domain types and value models
## Build & Test Tooling
- `cargo build` - Standard debug build
- `cargo build --release` - Optimized release binary (LTO: thin, strip: enabled)
- `dx serve` (via dioxus-cli, optional) - Hot-reload development mode
- `cargo fmt --all` - Code formatting (CI checks with `--check`)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` - Lints (warnings block merge)
- `cargo nextest run` - Test runner (preferred over `cargo test`)
- `cargo nextest run -E 'test(my_test_name)'` - Single test by name
- Install: `cargo install cargo-nextest --locked`
## Configuration
- Desktop window size: 1440x900 (hardcoded in `main.rs`)
- Theme: Auto-detects OS dark/light preference
- `dev`: `debug = "line-tables-only"` for debug info with faster builds; package dependencies use `debug = false`
- `release`: Thin LTO + stripping for smaller binaries
- MSRV: 1.96 (pinned in workspace `Cargo.toml`)
- Run `rustup update stable` if toolchain is too old
## Workspace Structure
- `Cargo.toml` - Workspace manifest with workspace dependencies and profiles
- `Cargo.lock` - Dependency lockfile
- `nodedb-studio/Cargo.toml` - The app crate (single binary member)
- `nodedb-studio/src/` - All source code for the desktop client
## Platform Requirements
- Recent stable Rust (1.96+)
- macOS, Windows, or Linux with X11/Wayland (via Dioxus desktop/tao)
- Standalone desktop application
- Targets: Windows (MSVC), macOS (Intel and Apple Silicon), Linux (x86_64)
- Requires local NodeDB instance running (default: localhost:6433 MessagePack port)
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Overview
## Rust Core Conventions
### Error Handling
- **No `.unwrap()` / `.expect()` / `panic!` in non-test code**
- Use typed `thiserror` errors instead
- Propagate errors with `?` operator
- Never use `Result<T, String>` — use proper error types
- `.unwrap()` calls present only in:
### Module Structure
- `mod.rs` files contain **ONLY** `pub mod` and `pub use` declarations
- No logic, no type definitions in `mod.rs`
- All logic lives in sibling files (e.g., `view.rs`, `host.rs`)
- `src/components/mod.rs` — 11 lines, only pub mod declarations
- `src/state/mod.rs` — 10 lines, only pub mod declarations
- `src/models/mod.rs` — 6 lines, only pub mod declarations
- `src/modals/mod.rs` — 7 lines, includes pub use re-exports
- `src/views/mod.rs` — 23 lines, only pub mod declarations
- `src/data/mod.rs` — only pub use declarations
### File Size
- Files stay under 500 lines of non-test code
- Split by concern first
- Largest files: `src/data/mock.rs` (~524 lines, but mostly test data), `src/modals/preferences.rs` (205 lines)
- Modal components are split by concern (host, preferences, new_connection as separate files)
- Views are split by feature (streams/ has cdc.rs, cron.rs, landing.rs, mv.rs, notify.rs, topics.rs, view.rs)
### Type Naming
- Types: `UpperCamelCase`
- Functions/modules: `snake_case`
- No `get_` prefix on accessor functions
- Types: `Notification`, `ActiveConnection`, `Capabilities`, `Preference`, `StorageMode`, `Theme`, `ModalKind`, `Popover`, `Collection`
- Functions: `connections()`, `notifications()`, `visible()`, `unread_count()`, `avatar_letter()`, `icon_letter()`, `label()`, `key()`, `css_class()`
- No `get_` prefixes observed (e.g., `Severity::css_class()` not `get_css_class()`)
### Data Types
- Use `nodedb_types::Value` for values coming from/going to the database
- Use `sonic_rs` for runtime JSON (display only), **NEVER** `serde_json`
- Serialize to JSON only at the view boundary for display
- `data/mock.rs` line 241-242: Comment explicitly states this pattern: "viewers serialize them to JSON via `sonic_rs` purely for display"
- `views/streams/cdc.rs:16`: `sonic_rs::to_string(&ev.payload)` for display
- `views/streams/notify.rs:18`: `sonic_rs::to_string(&m.payload)` for display
- `data/mock.rs` tests use `sonic_rs::to_string()` (lines 508, 517)
- No `serde_json` imports found in codebase
### Exhaustive Pattern Matching
- Do **not** write `_ =>` catch-alls on exhaustive domain enums
- Let the compiler flag every site that needs updating
- `models/notification.rs`: `Severity` enum match at line 21 — all 3 variants explicitly matched, no catch-all
- `models/collection.rs`: `StorageMode` enum matches at lines 27–68 — all 8 variants explicitly matched, no catch-alls
- `modals/preferences.rs`: `ModalKind` match at lines 18–23 — exhaustive, no catch-all
- `modals/preferences.rs`: String match on pane at lines 47–54 — has default `_` because it's not a closed enum
## Dioxus 0.7 Conventions
### Components
- Components are `PascalCase` `#[component]` functions returning `Element`
- Never hold a `.read()` / `.write()` guard across an `.await`
- Prefer `.peek()` in event handlers and when reading + writing the same signal
- List items must have stable keys (never array index)
#[component]
#[component]
#[component]
### State Management
- Use `use_signal` (signals are `Copy`)
- Subscribe with `.read()`
- Use `.peek()` in event handlers and when reading + writing the same signal
- Provide context via `use_context_provider()`, never global statics
- Never hold a `.read()` / `.write()` guard across `.await`
### Formatting & Attributes
- Use inline format strings in attributes/text (`"{value}"`)
- Avoid redundant closures over existing handlers
- Forms submit by default in 0.7 — call `e.prevent_default()` in `onsubmit`
### Threading & Async
- Never block the main thread
- CPU work → `std::thread::spawn`
- Async IO → `spawn` / `use_resource` / `use_action`
- Long-lived tasks → `spawn_forever`
- Write results back into a signal
### List Keys
- Lists need stable keys (`key: "{item.id}"`)
- Never the array index
### Business Logic Testability
- Keep business/state logic in plain Rust (`state/`, `services/`, `data/`)
- Logic must be testable without a renderer
## Naming Conventions Summary
| Category | Convention | Examples |
|----------|-----------|----------|
| Types | `UpperCamelCase` | `Notification`, `StorageMode`, `Preference`, `ActiveConnection` |
| Functions | `snake_case` | `connections()`, `unread_count()`, `visible()` |
| Modules | `snake_case` | `components`, `modals`, `views`, `state`, `data` |
| Components | `PascalCase` `#[component]` | `Topbar()`, `Modal()`, `Preferences()` |
| Enum variants | `PascalCase` | `Notification::Info`, `StorageMode::Document` |
| Module files | `snake_case.rs` | `modal.rs`, `preferences.rs`, `topbar.rs` |
| Module roots | `mod.rs` | Only pub mod/use, no logic |
## Import Organization
## Comments & Documentation
- Module-level doc comments on all public modules
- Inline comments explain *why*, not *what*
- Clear intent in type/function names reduces need for comments
## Architecture-Level Patterns
### Backend Seam (Trait-Based)
#[derive(Debug, Clone, Copy, Default)]
### Capability-Driven Rendering
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Root component (`App` in `src/app.rs`) is NOT routing-based. It mounts either `ConnectionManager` (disconnected) or `Studio` (connected) as a conditional, never both.
- Global UI state lives as fine-grained Dioxus signals provided via context, not globals.
- `ConnectionService` trait at `src/services/connection_service.rs` is THE backend boundary. Only `MockConnectionService` exists today; real client plugs in here.
- Per-connection identity: no global account. Each connection has its own user, role, and capabilities bitmask.
- Capability-driven rendering: `ActiveConnection.capabilities` (struct from `src/state/connection.rs`) gates rail items, admin sub-tabs, and routed views via `Capability` enum flags.
- Async/real-client integration MUST land at the `ConnectionService` seam using `use_resource` or `use_action`, NOT sprinkled through views.
## Layers
- Purpose: Provide global state context, manage Disconnected/Connected states, load CSS.
- Location: `src/app.rs`
- Contains: Root `App()` component, service instantiation, context providers.
- Depends on: `ConnectionService` trait, `ActiveConnection` signal, modal state.
- Used by: Everything (entry point from desktop launch).
- State provided:
- Purpose: Connection picker UI; let user select or create a connection.
- Location: `src/views/connection_manager.rs`
- Contains: Connection cards (grid), connection-open logic, recent-activity list.
- Depends on: `ConnectionService.connect()`, saved registry, modal state.
- Used by: Root state machine when `ActiveConnection` is `None`.
- Signal access: reads registry, writes `ActiveConnection` on connect.
- Purpose: Mounts the router and provides studio-scoped UI state (popover, command palette).
- Location: `src/views/studio_shell.rs`
- Contains: `Router` instantiation, studio-only signals (which popover is open, palette open).
- Depends on: Routing, persisted `ActiveConnection`.
- Used by: Root state machine when `ActiveConnection` is `Some(...)`.
- Signals provided to router: `Popover`, command-palette open flag.
- Purpose: Wraps every routed view. Renders rail, topbar, statusbar, Outlet.
- Location: `src/routes.rs` (`StudioLayout` component).
- Contains: Persistent navigation, global keyboard handlers (⌘K palette, ⌘D disconnect, ⌘, prefs, Esc overlay close).
- Depends on: Current route, active connection, capability flags, popover/modal state.
- Used by: `Router`, via Dioxus `#[layout]` attribute.
- Capability fallback: If current route requires a capability the connection lacks (e.g., Graph), redirects to Explorer.
- Purpose: Map Studio views to URLs; define which views exist and which capabilities they require.
- Location: `src/routes.rs` (`Route` enum and `required_cap()` method).
- Routes: `/`, `/query`, `/designer`, `/graph`, `/vector`, `/streams/:tab`, `/sync`, `/admin/:tab`, `/console`, `/timeseries`, `/spatial`, `/fts`.
- Per-route capability check: `Route::required_cap()` returns optional `Capability` for each route.
- Purpose: Each view is a single routed screen corresponding to one rail item (or sub-tab within Streams/Admin).
- Locations:
- Contains: Layout, component composition, static mock data display.
- Depends on: Dioxus, components, mock data via `crate::data::mock`.
- Signal access: read `ActiveConnection`, modal state; write popover state.
- Purpose: Stateless or loosely-stateful UI pieces used across views.
- Locations:
- Contains: Reusable JSX, no business logic.
- Depends on: Component libraries, state signals.
- Used by: Views, layout, other components.
- Purpose: Full-screen overlays (New Connection, Preferences).
- Locations:
- Contains: Form UI, modal bodies.
- Depends on: Modal state, preferences state, modal signal.
- Signal access: `ModalKind` (controlled at app root, reachable while disconnected or connected).
- Purpose: Live UI state shared across components via context.
- Locations:
- Contains: Type definitions and computed state helpers (e.g., `Capabilities::has()`).
- Depends on: `serde` for serialization, no UI logic.
- Used by: Every component that reads or writes state.
- Purpose: Typed data structures for collections, notifications, and other domain objects.
- Locations:
- Contains: `serde` derives, helper methods (e.g., `severity.css_class()`).
- Depends on: `serde`, no UI.
- Used by: Views, components, mock data.
- Purpose: Trait boundary for swapping mock with real client.
- Location: `src/services/connection_service.rs`
- Contains:
- Depends on: State types, models.
- Used by: App root (instantiation), ConnectionManager (connect call).
- **Future:** Real impl wraps `nodedb-client` and uses `use_resource` to make async calls async-compatible.
- Purpose: Centralize ALL hardcoded test data in one module.
- Location: `src/data/mock.rs` (523 lines)
- Contains:
- Depends on: Models, state types.
- Used by: `MockConnectionService` only. No view should hardcode data.
## Data Flow
- Global state (connection, registry, notifications, prefs, modals) as signals at app root → provided via context.
- Views read signals via `use_context()`, no local prop drilling.
- Views only write to signals they own (e.g., popover toggle) or are explicit targets (e.g., modal open).
- No global statics; all state flows through signals.
## Key Abstractions
- Purpose: The single backend boundary. Abstracts "where connection data comes from."
- Examples: `MockConnectionService` (hardcoded), future `NodedbConnectionService` (real client).
- Pattern: Synchronous trait methods returning data (mock) or wrapped in `use_resource` (real client).
- Why: Lets future real-client impl plug in at `src/app.rs:31` without touching consumers.
- Purpose: Per-connection feature set. Gated rendering at rail, routes, notifications.
- Pattern: `Capabilities` struct + `Capability` enum. All features are optional; absence is not an error.
- Examples: `staging-cluster` has `sync=true, cluster=true`; `prod-replica-eu` has `readonly=true`.
- Purpose: Live session identity.
- Fields: `name`, `sub` (chip sub-line), `user`, `role`, `capabilities`, `databases`, `current_database`.
- Pattern: `Signal<Option<ActiveConnection>>` where `None` means disconnected.
- Method: `avatar_letter()` derives first char of username for UI avatar.
- Purpose: Entry in the connections registry (shown on Connection Manager).
- Fields: `name`, `meta`, `sub`, `status` (Online/ReadOnly/Offline), stats (ping, db_count), `profile` (credential details or `None` if offline).
- Pattern: Only connectable if `profile.is_some()`.
- Purpose: Routing + capability guard.
- Pattern: `Route` enum variants map to view functions. `required_cap()` returns `Option<Capability>`.
- Layout logic: before rendering a route's view, check capability; if missing, fall back to Explorer.
## Entry Points
- Location: `src/main.rs`
- Triggers: `cargo run -p nodedb-studio` or release binary execution.
- Responsibilities: Create window config (1440x900, titled "NodeDB Studio"), launch Dioxus desktop runtime with `App` component.
- Location: `src/app.rs`
- Triggers: Dioxus runtime, runs once on startup.
- Responsibilities: Instantiate service, seed global state, mount state machine conditional (ConnectionManager or Studio).
- Location: `src/views/connection_manager.rs`
- Triggers: Mounted by root state machine when `active.is_none()`.
- Responsibilities: Display saved connections, handle connection attempts, link to preferences.
- Location: `src/views/studio_shell.rs`
- Triggers: Mounted by root state machine when `active.is_some()`.
- Responsibilities: Provide studio-scoped signals, mount router.
- Location: `src/routes.rs`
- Triggers: Inside `Studio`, routed by Dioxus `Router`.
- Responsibilities: Dispatch to view based on URL, render persistent chrome (rail, topbar, statusbar), apply capability fallback.
## Error Handling
- `service.connect(name)` returns `Option<ActiveConnection>`. If `None`, connection failed silently (already displayed as Offline in UI).
- Sync conflicts, errors, warnings shown as notifications (Notification with `Severity::Err` or `Severity::Warn`).
- Modal errors (e.g., new connection validation) shown as inline form feedback, not notifications.
- No panic paths in UI; graceful degradation.
## Cross-Cutting Concerns
- ⌘K / ⌘D: Open command palette, disconnect
- ⌘,: Open preferences
- Esc: Close all overlays (palette, popover, modal)
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
