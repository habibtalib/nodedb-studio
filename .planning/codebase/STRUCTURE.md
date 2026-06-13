# Codebase Structure

**Analysis Date:** 2026-06-13

## Directory Layout

```
nodedb-studio-v1/
├── nodedb-studio/
│   ├── src/
│   │   ├── main.rs                   # Desktop window config, entry point
│   │   ├── app.rs                    # Root component, state machine, context providers
│   │   ├── routes.rs                 # Route enum, StudioLayout chrome
│   │   ├── components/               # Reusable UI pieces (rail, topbar, popovers, etc.)
│   │   ├── views/                    # Routed screens (one module per rail item)
│   │   ├── modals/                   # Modal bodies (new connection, preferences) + host
│   │   ├── state/                    # Live UI state as signal types (connection, ui, prefs, etc.)
│   │   ├── models/                   # Typed domain data (collection, notification)
│   │   ├── services/                 # Backend seam (ConnectionService trait, mock impl)
│   │   └── data/                     # Centralized mock data
│   ├── assets/
│   │   └── styles.css                # Single stylesheet
│   └── Cargo.toml
├── assets/styles.css                 # CSS ported from design (loaded as asset)
├── AGENTS.md                         # Source-of-truth guide (conventions, commands, boundaries)
├── Cargo.toml                        # Workspace root
└── .cargo/config.toml                # GITIGNORED: local NodeDB paths
```

## Directory Purposes

**src/:**
- Purpose: All Rust source code for the desktop app.
- Contains: Entry point, root component, views, state, services, components, data.
- Layout: Organized by concern (views, state, components, services), not by feature.

**src/components/:**
- Purpose: Reusable, mostly stateless UI components used across views.
- Contains: Rail (left nav), Topbar (top bar), Statusbar (bottom bar), Command Palette, Modal wrapper, Popovers (connection/database/notifications/avatar), sub-navigation components.
- Typical file size: 80–160 lines.
- Key files:
  - `rail.rs` — capability-gated navigation items
  - `topbar.rs` — connection chip, database selector, notifications bell, avatar
  - `popovers/` — dropdown menus (connection switch, database switch, notification list, avatar menu)
  - `command_palette.rs` — ⌘K action launcher

**src/views/:**
- Purpose: One module per routed screen (rail destination or admin sub-tab).
- Contains: Ported mockup layouts, no business logic, static mock data references.
- Modules:
  - `connection_manager.rs` — Disconnected: connection picker UI
  - `studio_shell.rs` — Connected: router host
  - `explorer/` — Collection browser with per-engine viewers (document, strict, temporal, graph-like)
  - `query.rs` — Query workspace (schema tree, editor placeholder, results)
  - `designer.rs` — Data model designer
  - `graph_explorer.rs` — Graph/relationship viewer (Graph capability required)
  - `vector_space.rs` — Vector search interface (Vector capability required)
  - `streams/` — Topics and Cron jobs (Streams capability required)
  - `sync.rs` — Replication/sync status (Sync capability required)
  - `timeseries_dashboard.rs` — Time-series metrics (Timeseries capability required)
  - `spatial_view.rs` — Geo/spatial viewer (Spatial capability required)
  - `fts_inspector.rs` — Full-text search inspector (Fts capability required)
  - `console.rs` — Database console/REPL
  - `admin/` — Admin panels (Cluster, Nodes, Raft, Shards, RBAC, RLS, Audit)
- File size limit: 500 lines non-test code (AGENTS.md). Explorer is split into view + sidebar + viewers.
- Signal access: Read `ActiveConnection`, modal state; may write popover state.

**src/views/explorer/:**
- Purpose: Collection browser with multiple viewer engines.
- Modules:
  - `view.rs` — Explorer component (main layout)
  - `sidebar.rs` — Collection list, database dropdown
  - `viewers/` — Per-engine viewers (document, strict, temporal, etc.)
- Viewers are engine-specific: Document collections show as documents; Strict (time-series tables) as tables; Graph as node/edge inspector, etc.

**src/views/admin/:**
- Purpose: Admin sub-tabs (routed via `/admin/:tab`).
- Modules:
  - `view.rs` — Admin dispatcher (tabs, sidebar)
  - `cluster.rs` — Cluster topology, failover
  - `nodes.rs` — Node list, health, rebalance
  - `raft.rs` — Raft state, term, leader
  - `shards.rs` — Shard distribution, rebalance
  - `rbac.rs` — Users, roles, permissions
  - `rls.rs` — Row-level security rules
  - `audit.rs` — Audit log

**src/views/streams/:**
- Purpose: Streams feature (routed via `/streams/:tab`).
- Modules:
  - `view.rs` — Streams dispatcher (Topics and Cron tabs)
  - `landing.rs` — Topics listing/management
  - `cron.rs` — Cron jobs listing/management

**src/modals/:**
- Purpose: Modal bodies and the host that dispatches them.
- Contains:
  - `host.rs` — `ModalHost` component (reads `ModalKind` signal, renders the open modal or nothing)
  - `new_connection.rs` — New connection form
  - `preferences.rs` — Preferences panes (appearance, keyboard shortcuts, etc.)

**src/state/:**
- Purpose: Live UI state as Dioxus signal types. No logic; just type definitions and helper methods.
- Contains:
  - `connection.rs` — `ActiveConnection`, `Capabilities`, `Capability` enum
  - `connections_registry.rs` — `SavedConnection`, `ConnectionProfile`, `ConnStatus` enum
  - `ui.rs` — `Popover`, `ModalKind` enums (which overlay is open)
  - `notifications.rs` — Signal helpers for filtering/accessing notifications
  - `preferences.rs` — `Preferences` struct (theme, shortcuts, etc.)
- Pattern: All signals are provided at app root; views access them via `use_context()`.

**src/models/:**
- Purpose: Typed domain data (not live state; persistent domain objects).
- Contains:
  - `collection.rs` — `Collection` struct, `StorageMode` enum (Document, Strict, etc.)
  - `notification.rs` — `Notification` struct, `Severity` enum, `NotificationTarget` enum
- These are serialized via `serde` and displayed/manipulated by views.

**src/services/:**
- Purpose: Backend seam. All "outside world" interaction goes through trait methods.
- Contains:
  - `connection_service.rs` — `ConnectionService` trait with `list_connections()`, `notifications()`, `connect()`. `MockConnectionService` is the only impl today.
- Pattern: Trait methods are synchronous (mock). Real client impl wraps async calls in `use_resource`.

**src/data/:**
- Purpose: Centralized mock data (523 lines, all in one file to stay discoverable).
- Contains:
  - `mock.rs` — `connections()`, `notifications()`, per-admin-tab mock data, etc.
- Rule: No view or component hardcodes data. All hardcoded data lives here.
- Used by: `MockConnectionService` only.

**assets/styles.css:**
- Purpose: Single stylesheet, ported from design mockup.
- Loaded: As an asset in `src/app.rs` (`asset!("/assets/styles.css")`).
- Size: ~2000 lines of CSS.
- Pattern: No scoped styles, no CSS-in-JS. Design tokens (colors, spacing) are CSS variables.

**Cargo.toml (workspace root):**
- Purpose: Workspace definition, shared dependencies.
- Contains: `nodedb-studio` member crate.
- Pinned dependencies: Dioxus 0.7, `nodedb-client`, `nodedb-types` (via `.cargo/config.toml` patch).

**.cargo/config.toml (GITIGNORED):**
- Purpose: Local development configuration. Patches NodeDB crates to local paths.
- Never committed: Contains machine-specific paths.
- Template provided in AGENTS.md.

## Key File Locations

**Entry Points:**
- `src/main.rs` — Desktop window config, `LaunchBuilder::desktop().launch(App)`
- `src/app.rs` — Root `App()` component (Disconnected/Connected state machine)

**Configuration:**
- `Cargo.toml` — Workspace, dependencies, MSRV
- `.cargo/config.toml` — Local NodeDB paths (GITIGNORED)
- `AGENTS.md` — Commands, conventions, boundaries (source of truth)

**State Management:**
- `src/state/connection.rs` — `ActiveConnection`, `Capabilities`, `Capability` enum
- `src/state/ui.rs` — `Popover`, `ModalKind` (overlays)
- `src/app.rs` — Context providers (all signals seeded and provided here)

**Backend Seam:**
- `src/services/connection_service.rs` — `ConnectionService` trait, `MockConnectionService` impl

**Mock Data:**
- `src/data/mock.rs` — All hardcoded test data (connections, notifications, collections, etc.)

**Routing & Chrome:**
- `src/routes.rs` — `Route` enum, `StudioLayout`, capability fallback
- `src/components/rail.rs` — Left navigation (capability-gated)
- `src/components/topbar.rs` — Top bar (connection, database, notifications, avatar)

**Core Views:**
- `src/views/connection_manager.rs` — Disconnected: connection picker
- `src/views/explorer/` — Explorer (main collection browser)
- `src/views/query.rs` — Query workspace
- `src/views/admin/` — Admin tabs
- `src/views/streams/` — Streams (topics, cron)

**Modal System:**
- `src/modals/host.rs` — `ModalHost` dispatcher
- `src/modals/new_connection.rs` — New connection form
- `src/modals/preferences.rs` — Preferences panes

## Naming Conventions

**Files:**
- `mod.rs` — Module root; contains ONLY `pub mod` declarations and re-exports. NO logic, NO type definitions.
- `view.rs` — Main component for a view module (e.g., `explorer/view.rs` exports `Explorer()`).
- `*.rs` — Sibling files for sub-components or helper logic (e.g., `rail.rs`, `topbar.rs`).
- No `lib.rs` or `main.rs` exports; `mod.rs` is the aggregator.

**Components:**
- `PascalCase` function names (Dioxus convention): `Rail()`, `Topbar()`, `Explorer()`, `ConnectionCard()`.
- Attribute names in JSX: lowercase kebab-case (`class`, `onclick`, `onchange`).

**Functions & Variables:**
- `snake_case` in Rust (non-component functions): `connect()`, `avatar_letter()`, `has()`.
- Event handlers: `on_*` (e.g., `on_connect`, `onclick`).
- Signal names: descriptive, plural for collections (e.g., `registry`, `notifications`).

**Enums & Types:**
- `UpperCamelCase`: `Route`, `Capability`, `Severity`, `ConnStatus`, `ModalKind`.
- Enum variants: `UpperCamelCase`: `Capability::Graph`, `ConnStatus::Online`, `Severity::Err`.

**Directories:**
- `snake_case`: `src/components/`, `src/views/`, `src/state/`, `src/services/`, `src/models/`, `src/data/`.

## Where to Add New Code

**New Feature (e.g., Indexes Manager):**
1. Create a new view module: `src/views/indexes/mod.rs` with re-exports and `src/views/indexes/view.rs` with the main component.
2. Add a new `Route` variant in `src/routes.rs` (e.g., `#[route("/indexes")] Indexes {}`).
3. Optional: Add a `Capability` if the feature is gated (e.g., `Capability::Indexes`).
4. Add the rail item in `src/components/rail.rs` with capability check.
5. Add mock data (collections, indexes) to `src/data/mock.rs`.
6. Write tests as `#[cfg(test)] mod tests { }` inline in the view module.

**New Component (e.g., DetailsPanel):**
1. Create `src/components/details_panel.rs` with a single `#[component] fn DetailsPanel() -> Element` function.
2. Re-export in `src/components/mod.rs`: `pub mod details_panel;`.
3. Use in views or other components via `use crate::components::details_panel::DetailsPanel;`.
4. Keep under 500 lines; split if larger.

**New State Signal (e.g., SidebarOpen):**
1. Define the type in `src/state/ui.rs` (if ephemeral/studio-scoped) or a new state module (if persistent).
2. Provide it in `src/app.rs` (if global) or `src/views/studio_shell.rs` (if studio-scoped) via `use_context_provider()`.
3. Access in components via `use_context::<Signal<Type>>()`.

**New Modal (e.g., DeleteConnection):**
1. Create `src/modals/delete_connection.rs` with the form.
2. Add a variant to `ModalKind` enum in `src/state/ui.rs` (e.g., `ModalKind::DeleteConnection`).
3. Add a case in `ModalHost` in `src/modals/host.rs` to render it.
4. Trigger by setting `modal.set(Some(ModalKind::DeleteConnection))` from any view/component.

**New Admin Sub-Tab (e.g., Backups):**
1. Create `src/views/admin/backups.rs` with the component.
2. Re-export in `src/views/admin/mod.rs`.
3. Update `Admin` dispatcher in `src/views/admin/view.rs` to match on `"backups"` tab and render the component.

**Utilities & Helpers:**
- Shared helper functions: `src/state/` (state logic) or a new `src/utils/` module if many.
- Per-domain helpers: keep in the module where they're used (e.g., collection helpers in `src/models/collection.rs`).

## Special Directories

**assets/:**
- Purpose: Static assets (CSS, images, icons).
- Generated: No (hand-written CSS).
- Committed: Yes.
- Loaded: Via `asset!()` macro in Dioxus.

**src/components/popovers/:**
- Purpose: Dropdown menus (one file per popover).
- Generated: No.
- Committed: Yes.
- Pattern: Each popover is a component that reads/writes `Popover` signal via context.

**src/views/explorer/viewers/:**
- Purpose: Engine-specific collection viewers (Document, Strict, Temporal, Graph-like).
- Generated: No.
- Committed: Yes.
- Pattern: Each viewer is a component that renders a collection in engine-specific layout. Swapped by Explorer based on collection storage mode.

**src/views/admin/:**
- Purpose: Sub-tabs within the Admin view (not routed separately).
- Generated: No.
- Committed: Yes.
- Pattern: Admin dispatcher mounts one sub-tab component based on `:tab` param.

**target/:**
- Purpose: Build artifacts (ignored by git).
- Generated: Yes (`cargo build`).
- Committed: No.

**.cargo/config.toml:**
- Purpose: Local development config (machine-specific paths).
- Generated: No (hand-created per instructions in AGENTS.md).
- Committed: No (GITIGNORED).

## File Organization Principles

**Rule: mod.rs Exports Only**
- Every module root (`mod.rs`) contains ONLY:
  - `pub mod` declarations (one per sub-module)
  - `pub use` re-exports (for convenience)
- NO logic, NO type definitions in `mod.rs`.
- Example: `src/components/mod.rs` lists `pub mod rail; pub mod topbar;` etc.

**Rule: One View per Rail Item**
- Each routed screen (rail destination) has its own module under `src/views/`.
- Sub-modules inside a view (e.g., `explorer/sidebar.rs`) are organized by concern.
- Views may split into multiple files if they exceed 500 lines; prefer `view.rs` + helper files.

**Rule: 500-Line Limit**
- No Rust file exceeds 500 lines of non-test code.
- Views, components, and service impls that approach the limit are split by concern.
- Example: `data/mock.rs` is 523 lines; it's an exception because it's pure data and consolidation matters for discoverability.

**Rule: No Global Statics**
- All state flows through Dioxus signals provided via context.
- No `lazy_static`, `once_cell`, or global `static` variables.

**Rule: Test Organization**
- Unit tests inline: `#[cfg(test)] mod tests { }` at the end of the file that defines the type.
- Integration tests: `tests/` directory at workspace root (future; none yet).
- No separate test files for components unless integration-level.

---

*Structure analysis: 2026-06-13*
