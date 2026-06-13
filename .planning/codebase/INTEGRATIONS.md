# External Integrations

**Analysis Date:** 2026-06-13

## APIs & External Services

**NodeDB Backend:**
- NodeDB (local or remote instance) - The primary database being visualized
  - SDK/Client: `nodedb-client` 0.3.0 (Rust client library)
  - Wire Protocol: MessagePack over TCP (port 6433)
  - Auth: Per-connection username/password credentials (stored in saved connections)

## Data Storage

**Databases:**
- NodeDB (remote or local instance)
  - Connection: Managed via `SavedConnection` / `ConnectionProfile` in `src/state/connections_registry.rs`
  - Client: `nodedb-client::NativeClient` wraps the MessagePack protocol
  - Data Exchange: Server returns `nodedb_types::Value` documents; Studio serializes to JSON via `sonic_rs` for display only
  - Multi-modal: Single NodeDB instance exposes eight internal storage modes (Document, Strict, Vector, Graph, Timeseries, KV, Spatial, FTS)

**File Storage:**
- Local filesystem only - No cloud/external file storage
- Preferences stored locally (theme, sidebar state, etc.)

**Caching:**
- None currently implemented

## Authentication & Identity

**Auth Model:**
- Per-connection credentials (username + password)
- Role-based access control (RBAC) determined by the NodeDB server
- Saved connections registered in `src/state/connections_registry.rs`
- Connection profiles include user identity, role string, and capability flags

**Capabilities (Feature Gating):**
The `Capabilities` struct in `src/state/connection.rs` gates UI features based on what the connected NodeDB instance and user role support:

```rust
pub struct Capabilities {
    pub graph: bool,           // Graph storage mode enabled
    pub vector: bool,          // Vector storage mode enabled
    pub streams: bool,         // Streams (CDC/pub-sub) enabled
    pub timeseries: bool,      // Timeseries storage mode enabled
    pub spatial: bool,         // Spatial storage mode enabled
    pub fts: bool,            // Full-text search enabled
    pub sync: bool,           // Peer replication enabled
    pub cluster: bool,        // Multi-node cluster mode
    pub readonly: bool,       // Current user is read-only
}
```

Rail items and Admin sub-tabs render conditionally based on these flags. Notifications also filter by capability (see `NotificationTarget` in `src/models/notification.rs`).

## The ConnectionService Seam

**Purpose:**
All external I/O goes through the `ConnectionService` trait (`src/services/connection_service.rs`). This is the **single point of integration** for the NodeDB client.

**Trait Definition:**

```rust
pub trait ConnectionService {
    /// List all saved connections (from registry).
    fn list_connections(&self) -> Vec<SavedConnection>;

    /// All notifications in the feed.
    fn notifications(&self) -> Vec<Notification>;

    /// Open a session by saved-connection name.
    /// Returns `None` if the name is unknown or the connection is offline.
    fn connect(&self, name: &str) -> Option<ActiveConnection>;
}
```

**Current Implementation:**
- `MockConnectionService` - Reads hardcoded data from `src/data/mock.rs`
- Synchronous only (no async I/O yet)
- Returns fixed test data for three saved connections (two online, one offline)

**How the Real Client Will Plug In:**

When `nodedb-client` integration is active, a second implementor wraps it:

```rust
pub struct NodeDBConnectionService {
    // Will hold nodedb_client::NativeClient or similar
}

impl ConnectionService for NodeDBConnectionService {
    fn list_connections(&self) -> Vec<SavedConnection> {
        // Query NodeDB for saved connections from persistent storage
    }

    fn notifications(&self) -> Vec<Notification> {
        // Stream notifications from NodeDB (CDC events, cluster alerts, etc.)
    }

    fn connect(&self, name: &str) -> Option<ActiveConnection> {
        // Instantiate nodedb_client::NativeClient with saved credentials
        // Return user/role/capabilities from the server
    }
}
```

**Async Boundary:**
Async I/O is **introduced at this seam only**, not scattered through views. When real integration lands, methods will return `Future`s, and views will use Dioxus `use_resource` / `use_action` to drive them.

## Data Flow: Connection to Display

**When a user connects:**

1. User selects a saved connection in `ConnectionManager` (UI at `src/views/connection_manager.rs`)
2. `connect(name)` is called on the injected `ConnectionService` (provided via context in `src/app.rs`)
3. Mock implementation returns a pre-built `ActiveConnection` with user identity and capabilities
4. Real implementation will instantiate `nodedb_client::NativeClient`, authenticate, fetch user/role/capabilities from the server
5. Studio renders the `Studio` component with the connected session in scope

**Example (from `src/data/mock.rs`):**
```rust
SavedConnection {
    name: "local-nodedb-dev".into(),
    meta: "nodedb · localhost:2480".into(),
    status: ConnStatus::Online,
    profile: Some(ConnectionProfile {
        user: "root".into(),
        role: "admin".into(),
        capabilities: Capabilities {
            graph: true,
            vector: true,
            streams: true,
            timeseries: true,
            spatial: true,
            fts: true,
            sync: false,
            cluster: false,
            readonly: false,
        },
        databases: vec![...],
        default_database: "analytics".into(),
    }),
}
```

## Notifications & Events

**Current (Mock) Flow:**
- `notifications()` returns a static list from `src/data/mock.rs`
- Notifications model: `src/models/notification.rs`
- Notifications filter by capability (e.g., cluster alerts only show if `cluster: true`)
- Notification feed displayed in the Notifications popover

**Future (Real Integration):**
- Will stream change events (CDC, LISTEN/NOTIFY) from NodeDB
- May publish cluster alerts, replication lag warnings, etc. via WebSocket or long-polling
- Each notification carries a `required_cap: Option<Capability>` for gating

## Environment Configuration

**Required env vars:**
- None (all configuration is in-app via Connection Manager or prefs modal)

**Secrets location:**
- None currently (credentials are entered in UI, stored locally per connection)
- Future: May use system keychain (macOS Keychain, Windows Credential Manager, etc.)

## Monitoring & Observability

**Error Tracking:**
- None (internal error handling only)
- Errors use `thiserror` typed definitions (`src/services/`, `src/state/`)
- Propagated to UI as user-facing messages or modal alerts

**Logs:**
- `tracing` crate configured but not yet integrated (setup available for structured logging)
- No external log aggregation

## Known Limitations (Current Skeleton)

**Mock Data Only:**
- `nodedb-client` and `nodedb-types` are imported as dependencies but **not yet used**
- All data is hardcoded in `src/data/mock.rs`
- No network I/O to NodeDB; no real authentication

**No Async Integration:**
- `ConnectionService` methods are synchronous
- Real implementation will use Dioxus `use_resource` / `use_action` for async fetching

**No Streaming:**
- CDC (change data capture), pub-sub, and LISTEN/NOTIFY are mocked as static documents
- Real implementation will stream live events from NodeDB

**No Persistence:**
- Saved connections are lost when the app exits (future: serialize to local config)
- Notifications are static (future: persisted or streamed from server)

---

*Integration audit: 2026-06-13*
