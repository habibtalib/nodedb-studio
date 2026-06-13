# Architecture

**Analysis Date:** 2026-06-13

## Pattern Overview

**Overall:** Two-state root state machine (Disconnected → Connected) with signal-based UI state management via Dioxus context. The backend seam is a trait-based service abstraction designed for swapping mock data with a real NodeDB client.

**Key Characteristics:**
- Root component (`App` in `src/app.rs`) is NOT routing-based. It mounts either `ConnectionManager` (disconnected) or `Studio` (connected) as a conditional, never both.
- Global UI state lives as fine-grained Dioxus signals provided via context, not globals.
- `ConnectionService` trait at `src/services/connection_service.rs` is THE backend boundary. Only `MockConnectionService` exists today; real client plugs in here.
- Per-connection identity: no global account. Each connection has its own user, role, and capabilities bitmask.
- Capability-driven rendering: `ActiveConnection.capabilities` (struct from `src/state/connection.rs`) gates rail items, admin sub-tabs, and routed views via `Capability` enum flags.
- Async/real-client integration MUST land at the `ConnectionService` seam using `use_resource` or `use_action`, NOT sprinkled through views.

## Layers

**App Root:**
- Purpose: Provide global state context, manage Disconnected/Connected states, load CSS.
- Location: `src/app.rs`
- Contains: Root `App()` component, service instantiation, context providers.
- Depends on: `ConnectionService` trait, `ActiveConnection` signal, modal state.
- Used by: Everything (entry point from desktop launch).
- State provided:
  - `Rc<dyn ConnectionService>` — the backend seam
  - `Signal<Option<ActiveConnection>>` — current session (None = disconnected)
  - `Signal<Vec<SavedConnection>>` — registry from service
  - `Signal<Vec<Notification>>` — feed from service
  - `Signal<Preferences>` — user prefs (persistent)
  - `Signal<Option<ModalKind>>` — which modal is open

**Disconnected State:**
- Purpose: Connection picker UI; let user select or create a connection.
- Location: `src/views/connection_manager.rs`
- Contains: Connection cards (grid), connection-open logic, recent-activity list.
- Depends on: `ConnectionService.connect()`, saved registry, modal state.
- Used by: Root state machine when `ActiveConnection` is `None`.
- Signal access: reads registry, writes `ActiveConnection` on connect.

**Connected State (Studio):**
- Purpose: Mounts the router and provides studio-scoped UI state (popover, command palette).
- Location: `src/views/studio_shell.rs`
- Contains: `Router` instantiation, studio-only signals (which popover is open, palette open).
- Depends on: Routing, persisted `ActiveConnection`.
- Used by: Root state machine when `ActiveConnection` is `Some(...)`.
- Signals provided to router: `Popover`, command-palette open flag.

**Studio Layout (Persistent Chrome):**
- Purpose: Wraps every routed view. Renders rail, topbar, statusbar, Outlet.
- Location: `src/routes.rs` (`StudioLayout` component).
- Contains: Persistent navigation, global keyboard handlers (⌘K palette, ⌘D disconnect, ⌘, prefs, Esc overlay close).
- Depends on: Current route, active connection, capability flags, popover/modal state.
- Used by: `Router`, via Dioxus `#[layout]` attribute.
- Capability fallback: If current route requires a capability the connection lacks (e.g., Graph), redirects to Explorer.

**Routing:**
- Purpose: Map Studio views to URLs; define which views exist and which capabilities they require.
- Location: `src/routes.rs` (`Route` enum and `required_cap()` method).
- Routes: `/`, `/query`, `/designer`, `/graph`, `/vector`, `/streams/:tab`, `/sync`, `/admin/:tab`, `/console`, `/timeseries`, `/spatial`, `/fts`.
- Per-route capability check: `Route::required_cap()` returns optional `Capability` for each route.

**Views (Studio Content):**
- Purpose: Each view is a single routed screen corresponding to one rail item (or sub-tab within Streams/Admin).
- Locations:
  - Core: `src/views/explorer/`, `src/views/query.rs`, `src/views/designer.rs`, `src/views/console.rs`
  - Feature-specific: `src/views/graph_explorer.rs`, `src/views/vector_space.rs`, `src/views/spatial_view.rs`, `src/views/streams/`, `src/views/sync.rs`, `src/views/timeseries_dashboard.rs`, `src/views/fts_inspector.rs`
  - Admin: `src/views/admin/` (sub-tabs: cluster, nodes, raft, shards, rbac, rls, audit)
- Contains: Layout, component composition, static mock data display.
- Depends on: Dioxus, components, mock data via `crate::data::mock`.
- Signal access: read `ActiveConnection`, modal state; write popover state.

**Components (Reusable Chrome):**
- Purpose: Stateless or loosely-stateful UI pieces used across views.
- Locations:
  - `src/components/rail.rs` — left navigation (capability-gated items)
  - `src/components/topbar.rs` — top bar (connection/database/notifications popovers, avatar, status)
  - `src/components/statusbar.rs` — bottom bar (latency, server info)
  - `src/components/command_palette.rs` — ⌘K search/action launcher
  - `src/components/popovers/` — connection, database, notification, avatar dropdowns
  - `src/components/modal.rs` — modal wrapper (title, backdrop, close)
  - `src/components/snav.rs`, `src/components/subnav.rs` — navigation sub-components
- Contains: Reusable JSX, no business logic.
- Depends on: Component libraries, state signals.
- Used by: Views, layout, other components.

**Modals:**
- Purpose: Full-screen overlays (New Connection, Preferences).
- Locations:
  - `src/modals/host.rs` — `ModalHost` dispatcher (renders the open modal or nothing)
  - `src/modals/new_connection.rs` — form to create a connection
  - `src/modals/preferences.rs` — preferences panes (appearance, shortcuts, etc.)
- Contains: Form UI, modal bodies.
- Depends on: Modal state, preferences state, modal signal.
- Signal access: `ModalKind` (controlled at app root, reachable while disconnected or connected).

**State (Fine-Grained Signals):**
- Purpose: Live UI state shared across components via context.
- Locations:
  - `src/state/connection.rs` — `ActiveConnection` struct, `Capabilities` enum/struct
  - `src/state/connections_registry.rs` — `SavedConnection`, `ConnectionProfile`, `ConnStatus`
  - `src/state/ui.rs` — `Popover`, `ModalKind` enums (which overlay is open)
  - `src/state/notifications.rs` — notification filtering/signal helpers
  - `src/state/preferences.rs` — user preferences (theme, keyboard shortcuts, etc.)
- Contains: Type definitions and computed state helpers (e.g., `Capabilities::has()`).
- Depends on: `serde` for serialization, no UI logic.
- Used by: Every component that reads or writes state.

**Models (Domain Data):**
- Purpose: Typed data structures for collections, notifications, and other domain objects.
- Locations:
  - `src/models/collection.rs` — `Collection`, `StorageMode` enums
  - `src/models/notification.rs` — `Notification`, `Severity`, `NotificationTarget`
- Contains: `serde` derives, helper methods (e.g., `severity.css_class()`).
- Depends on: `serde`, no UI.
- Used by: Views, components, mock data.

**Services (Backend Seam):**
- Purpose: Trait boundary for swapping mock with real client.
- Location: `src/services/connection_service.rs`
- Contains:
  - `ConnectionService` trait with three methods:
    - `list_connections()` → `Vec<SavedConnection>` (registry)
    - `notifications()` → `Vec<Notification>` (full feed)
    - `connect(name: &str)` → `Option<ActiveConnection>` (open a session)
  - `MockConnectionService` — synchronous impl, reads `crate::data::mock`
- Depends on: State types, models.
- Used by: App root (instantiation), ConnectionManager (connect call).
- **Future:** Real impl wraps `nodedb-client` and uses `use_resource` to make async calls async-compatible.

**Data (Mock):**
- Purpose: Centralize ALL hardcoded test data in one module.
- Location: `src/data/mock.rs` (523 lines)
- Contains:
  - `connections()` — four `SavedConnection` entries (three online, one offline)
  - `notifications()` — mock notification feed with various severities
  - Mock data for admin tabs, explorer collections, query results, etc.
- Depends on: Models, state types.
- Used by: `MockConnectionService` only. No view should hardcode data.

## Data Flow

**Connection Lifecycle:**

1. App starts → `App()` instantiates `MockConnectionService`, seeds registry and notifications from service, provides signals.
2. UI shows `ConnectionManager` (disconnected state).
3. User clicks a connection card → `on_connect` handler calls `service.connect(name)`.
4. Service returns `Some(ActiveConnection)` → `active` signal is set to `Some(...)`.
5. Root state machine detects `active.is_some()` → unmounts `ConnectionManager`, mounts `Studio` + Router.
6. Router renders `StudioLayout` + one view.
7. All views read `active` signal to display user name, role, databases, capability gates.
8. User presses ⌘D → `active` set to `None` → `ConnectionManager` mounts again.

**State Management:**

- Global state (connection, registry, notifications, prefs, modals) as signals at app root → provided via context.
- Views read signals via `use_context()`, no local prop drilling.
- Views only write to signals they own (e.g., popover toggle) or are explicit targets (e.g., modal open).
- No global statics; all state flows through signals.

**Capability-Driven Rendering:**

1. `Capabilities` struct has boolean fields: `graph`, `vector`, `streams`, etc.
2. `Capability` enum matches each field (used in `Route::required_cap()`, `Notification.required_cap`).
3. `Capabilities::has(cap: Capability) -> bool` maps enum to field.
4. Rail renders each item only if `active.read().capabilities.has(item_cap)` is true.
5. Route fallback: if user navigates to `/graph` but connection lacks `Graph` capability, layout redirects to `/` (Explorer).
6. Notifications: bell badge filters out notifications with `required_cap` the connection doesn't have.

## Key Abstractions

**ConnectionService Trait:**
- Purpose: The single backend boundary. Abstracts "where connection data comes from."
- Examples: `MockConnectionService` (hardcoded), future `NodedbConnectionService` (real client).
- Pattern: Synchronous trait methods returning data (mock) or wrapped in `use_resource` (real client).
- Why: Lets future real-client impl plug in at `src/app.rs:31` without touching consumers.

**Capabilities:**
- Purpose: Per-connection feature set. Gated rendering at rail, routes, notifications.
- Pattern: `Capabilities` struct + `Capability` enum. All features are optional; absence is not an error.
- Examples: `staging-cluster` has `sync=true, cluster=true`; `prod-replica-eu` has `readonly=true`.

**ActiveConnection:**
- Purpose: Live session identity.
- Fields: `name`, `sub` (chip sub-line), `user`, `role`, `capabilities`, `databases`, `current_database`.
- Pattern: `Signal<Option<ActiveConnection>>` where `None` means disconnected.
- Method: `avatar_letter()` derives first char of username for UI avatar.

**SavedConnection:**
- Purpose: Entry in the connections registry (shown on Connection Manager).
- Fields: `name`, `meta`, `sub`, `status` (Online/ReadOnly/Offline), stats (ping, db_count), `profile` (credential details or `None` if offline).
- Pattern: Only connectable if `profile.is_some()`.

**Route + required_cap:**
- Purpose: Routing + capability guard.
- Pattern: `Route` enum variants map to view functions. `required_cap()` returns `Option<Capability>`.
- Layout logic: before rendering a route's view, check capability; if missing, fall back to Explorer.

## Entry Points

**Desktop Launch:**
- Location: `src/main.rs`
- Triggers: `cargo run -p nodedb-studio` or release binary execution.
- Responsibilities: Create window config (1440x900, titled "NodeDB Studio"), launch Dioxus desktop runtime with `App` component.

**App Root:**
- Location: `src/app.rs`
- Triggers: Dioxus runtime, runs once on startup.
- Responsibilities: Instantiate service, seed global state, mount state machine conditional (ConnectionManager or Studio).

**ConnectionManager:**
- Location: `src/views/connection_manager.rs`
- Triggers: Mounted by root state machine when `active.is_none()`.
- Responsibilities: Display saved connections, handle connection attempts, link to preferences.

**Studio Shell:**
- Location: `src/views/studio_shell.rs`
- Triggers: Mounted by root state machine when `active.is_some()`.
- Responsibilities: Provide studio-scoped signals, mount router.

**Router + StudioLayout:**
- Location: `src/routes.rs`
- Triggers: Inside `Studio`, routed by Dioxus `Router`.
- Responsibilities: Dispatch to view based on URL, render persistent chrome (rail, topbar, statusbar), apply capability fallback.

## Error Handling

**Strategy:** Return `Option` for fallible operations (connect, service calls). No `.unwrap()` or `.expect()` in non-test code.

**Patterns:**
- `service.connect(name)` returns `Option<ActiveConnection>`. If `None`, connection failed silently (already displayed as Offline in UI).
- Sync conflicts, errors, warnings shown as notifications (Notification with `Severity::Err` or `Severity::Warn`).
- Modal errors (e.g., new connection validation) shown as inline form feedback, not notifications.
- No panic paths in UI; graceful degradation.

## Cross-Cutting Concerns

**Logging:** No logging infrastructure yet (mock phase). When real client lands, log at service boundary and in connection flow.

**Validation:** Forms (new connection, preferences) validate inline on change or on submit. No global validation framework.

**Authentication:** Per-connection credentials stored in `ConnectionProfile` (part of `SavedConnection`). No session/token storage yet (mock phase). When NodeDB client lands, auth tokens live in the real client's state, not here.

**Keyboard Shortcuts:** Global handlers in `StudioLayout` (`src/routes.rs`):
- ⌘K / ⌘D: Open command palette, disconnect
- ⌘,: Open preferences
- Esc: Close all overlays (palette, popover, modal)

**CSS:** Single stylesheet `assets/styles.css` (ported from design mockup), loaded as an asset. No CSS-in-JS or scoped styles.

---

*Architecture analysis: 2026-06-13*
