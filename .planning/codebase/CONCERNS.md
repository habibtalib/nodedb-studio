# Codebase Concerns

**Analysis Date:** 2026-06-13

## Critical: Mock-to-Real Data Gap

**The entire app runs on hardcoded mock data; the ConnectionService seam is far too narrow:**

- **Issue:** All UI screens render static mock collections, notifications, and stream events. The `ConnectionService` trait (`src/services/connection_service.rs`) exposes only three synchronous methods: `list_connections()`, `notifications()`, and `connect()`. This is insufficient for real client integration.
- **Files:** 
  - `src/services/connection_service.rs` (3 methods, sync only)
  - `src/data/mock.rs` (523 lines of hardcoded data)
  - `src/app.rs` (seeds state once from service; no refresh/polling)
  - All view files under `src/views/` expect mock data
- **Scope:** This is not a bug; it's the current skeleton architecture. However, completing the studio requires this to be addressed first.

### Per-View Mock Data Gaps

**Explorer (Document/Strict/Vector/Graph/Timeseries/KV/Spatial/FTS viewers):**
- Mock: 14 hardcoded collections in `mock::explorer_collections()`, each with a fake count string (e.g. "2.4M").
- Real need: Per-connection collection list (dynamically loaded), per-collection document sample(s), schema metadata, record counts (as numbers, not strings).
- Files: `src/views/explorer/view.rs`, `src/views/explorer/sidebar.rs`, `src/views/explorer/viewers/{document,strict,vector,graph,timeseries,kv,spatial,fts}.rs`
- Missing: Sample document retrieval, pagination, filtering, sorting, INSERT/UPDATE/DELETE operations.

**Query Console:**
- Mock: Hardcoded schema tree (events, users, orders collections only) and static result table.
- Real need: Schema introspection from server, SQL query execution, result streaming, multi-tab state.
- Files: `src/views/query.rs`
- Missing: Query execution, result set handling, error surfaces (syntax errors, timeouts, etc.).

**Streams (CDC, LISTEN/NOTIFY, Cron, MV):**
- Mock: Static CDC event rows (`mock::cdc_events()`), static notify channels/messages (`mock::notify_channels()`, `mock::notify_messages()`).
- Real need: Live CDC tail (subscription), live pub/sub tail (subscription), cron job list + execution log, materialized view list + stats.
- Files: `src/views/streams/cdc.rs`, `src/views/streams/notify.rs`, `src/views/streams/cron.rs`, `src/views/streams/mv.rs`
- Missing: Streaming subscriptions (long-lived async tasks), pagination, filtering.

**Admin (Cluster, Shards, Nodes, Raft, RBAC, RLS, Audit):**
- Mock: Renders UI chrome (tabs, headers) but no real data.
- Real need: Per-admin-tab queries to server: cluster topology, shard map, node status, Raft state, role/permission matrix, row-level security policies, audit log.
- Files: `src/views/admin/{cluster,shards,nodes,raft,rbac,rls,audit}.rs`
- Missing: Admin API calls, statistics, real-time status updates.

**Other views (Sync, Graph Explorer, Vector Space, Spatial View, Timeseries Dashboard, Designer, Studio Shell):**
- Mock: Layout/UI only; no real data or operations.
- Real need: Sync conflict resolution, graph query builder execution, vector search, spatial queries, time-series aggregation, schema design/DDL.

---

## Critical: The Seam Must Widen & Become Async

**The ConnectionService trait is the backend contract. It must grow substantially:**

- **Current state:** Three synchronous, data-only methods. No error handling. No async.
- **Why it's insufficient:** 
  - Real client needs async I/O (network calls, query execution).
  - Views need to load data on demand (Explorer collections, queries, streams).
  - AGENTS.md explicitly states async is introduced "at this boundary" (§6: use_resource/use_action), not sprinkled through views.
  - Current trait has no Result type; real operations can fail (network, auth, query syntax, etc.).

- **Expected expansion (not yet implemented):**
  - Query execution: `execute_query(sql: &str) -> Result<ResultSet, Error>`
  - Collection operations: `list_collections()`, `get_collection_schema(name: &str)`, `query_collection(name: &str, filters: ...) -> Result<Vec<Value>, Error>`
  - Per-engine CRUD: method per storage mode (get_document, insert_document, update_document, delete_document for Document mode; similar for others).
  - Streaming: `subscribe_cdc(...)`, `subscribe_notify(...)` returning long-lived async streams.
  - Admin/cluster: methods for admin stats, node status, shard map, Raft state, audit log, RBAC, RLS.
  - Capability negotiation: `capabilities()` to fetch from server instead of hardcoding.
  - Connection lifecycle: real `connect(uri, credentials) -> Result<ActiveConnection, Error>`, auth method selection (password/token/mTLS/OIDC), TLS negotiation.

- **Boundary flag:** AGENTS.md §7 states "changing the ConnectionService seam" requires **ask-first approval**. Before widening, coordinate on the async strategy and error types.

---

## Critical: Connection Lifecycle & Auth Gaps

**The connection flow is static mock; real auth/TLS/capability negotiation missing:**

- **Issue:** `new_connection.rs` (the form) renders fields for all auth methods (username/password, token, mTLS) but they are static placeholders. `ConnectionService::connect()` does not perform actual authentication, TLS negotiation, or capability discovery.

- **Files:**
  - `src/modals/new_connection.rs` (the form; no submit logic)
  - `src/services/connection_service.rs` (mock connect just looks up mock data)
  - `src/state/connections_registry.rs` (SavedConnection::open() builds a session from mock profile)

- **Gaps:**

  **Auth methods:** The form lists all three, but the real client must:
  - Validate credentials against NodeDB auth system
  - Support trust/password (basic auth), API key, and OIDC
  - Return proper errors (invalid credentials, account locked, etc.)
  - Store credentials securely (see Security section below)

  **Capability negotiation:** Currently hardcoded in mock:
    - `local-nodedb-dev`: graph, vector, streams, timeseries, spatial, fts (sync/cluster off)
    - `staging-cluster`: all 9 capabilities
    - `prod-replica-eu`: all 9 but readonly=true
  - Real flow must: call `server.capabilities()` after successful auth, extract server-advertised capabilities, account for user's role-based restrictions, store per-connection.

  **Error handling:** No error surfaces today:
    - Connection refused → app freezes (no timeout)
    - Auth failure → silent (mock always finds a profile)
    - Network timeout → no UI feedback
    - TLS mismatch → not handled
  - Real flow must: async try/catch at the seam, propagate user-facing errors (connectivity, auth, timeout), display in Connection Manager or a dismissable toast.

  **Loading/empty/error states:** Views don't have these:
    - No spinner while connecting
    - No error message if connection fails
    - No "no collections" fallback if list returns empty
    - Notification feed never shows errors
  - Must be added as part of state/UI layer alongside real async.

  **Ping & DB count:** Shown in Connection Manager (e.g. "8.4ms · 3 dbs") are mock strings. Real client must measure RTT and fetch live database count per connection.

---

## High: Version Pinning Risk

**nodedb-client and nodedb-types are unpublished crates with a manual patch setup:**

- **Issue:** `Cargo.toml` pins both to `0.3.0`, but they live in a separate `../nodedb` workspace (not on crates.io). The setup guide (AGENTS.md §2) requires a `.cargo/config.toml` patch file (gitignored):
  ```toml
  [patch.crates-io]
  nodedb-client = { path = "../nodedb/nodedb-client" }
  nodedb-types  = { path = "../nodedb/nodedb-types" }
  ```

- **Risk:**
  - If `../nodedb` crate versions drift from pinned `0.3.0`, builds fail with opaque version-mismatch errors.
  - New developer clones the repo, runs `cargo build`, hits missing path patch → unclear error message.
  - If NodeDB workspace is not at version `0.3.0`, the patch silently overrides the pin, masking version skew.
  - No CI check validates that pinned version matches local workspace version.

- **Files:** `Cargo.toml` (version pins), `.cargo/config.toml` (gitignored; local only), `AGENTS.md` §2 (setup instructions).

- **Mitigation:** Add a pre-build script or CI step that validates:
  - `.cargo/config.toml` exists and patch paths resolve
  - Local nodedb crate versions match pinned versions in Cargo.toml
  - Clear error message if patch is missing (dev experience).

---

## High: Credential Storage for Saved Connections (Security)

**Credentials are rendered in the form but not persisted securely; production needs OS keychain:**

- **Current state:**
  - Form fields: username/password, token, mTLS cert paths — all static in `new_connection.rs`.
  - Saving: Not implemented. `SavedConnection` struct carries no credential data, just `ConnectionProfile` (user, role, capabilities).
  - AGENTS.md: A comment in the form hints at OS keychain ("Credentials are stored in your OS keychain"), but no actual integration.

- **Issue:** When real connections are saved, credentials must not be stored as plaintext in config files. Instead:
  - Prompt user to select auth method (password/token/mTLS).
  - Store only public/non-secret data in SQLite/config (connection name, host, port, username for reference).
  - Store secrets in OS keychain:
    - macOS: Keychain Services (use `keyring` crate or platform SDK)
    - Linux: Secret Service (via `keyring` crate)
    - Windows: Credential Manager (via `keyring` or `windows-sys`)
  - On load, retrieve secrets from keychain; if not found, prompt user to re-authenticate.

- **Files:** `src/modals/new_connection.rs` (form), `src/state/connections_registry.rs` (SavedConnection model).

- **Implementation path:** Introduce a credential store abstraction, integrate platform keychain at app startup, update form submit handler to save via keychain.

---

## High: TLS & Secure Transport

**TLS/SSL is not configured anywhere; default to secure in production:**

- **Issue:**
  - Form has no explicit "use TLS" toggle or certificate validation checkbox.
  - Client connection string (host/port) does not indicate TLS intent.
  - No certificate pinning or custom CA support.

- **Expected flow (to design):**
  - Form should offer "Secure (TLS)" toggle and optional CA cert file picker for self-signed servers.
  - Connection string passed to real client must include TLS flags.
  - Certificate validation failures should surface as user-friendly errors, not panics.

- **Files:** `src/modals/new_connection.rs`, `src/services/connection_service.rs` (the real client impl will handle this).

---

## Medium: Test Coverage Gaps

**No integration tests; only inline unit tests in mock.rs:**

- **Current state:**
  - `src/data/mock.rs` has 2 unit tests (MockDoc serialization order).
  - No `tests/` directory.
  - AGENTS.md specifies integration test location (`tests/`, shared helpers in `tests/common/mod.rs`), but none exist yet.

- **What's not tested:**
  - ConnectionService trait behavior (connection lifecycle, error paths).
  - State signal updates (notifications, active connection, preferences).
  - View rendering with different capability sets (capability-gating logic).
  - Modal open/close state transitions.
  - Notification filtering (capability-based hiding).

- **Priority gaps:**
  - ConnectionService error scenarios (auth failure, network timeout, capability mismatch).
  - Explorer view should load collections, not render hardcoded list.
  - Query execution and result handling (once real client lands).

- **Recommendation:** Start with `tests/connection_service.rs` testing the mock impl, then add tests per view/state module as real client integration proceeds.

---

## Medium: Error Handling & Result Types Not Introduced

**No `Result<T, E>` in ConnectionService yet; mock only returns `Option`:**

- **Issue:** AGENTS.md forbids `.unwrap()` in non-test code, but the trait signature is `Option`-based today. When real client is plugged in, all call sites must handle errors properly.
- **Files:** `src/services/connection_service.rs` (connect returns `Option<ActiveConnection>`), all view/state files that will eventually call the service.
- **Expected:** Error enum per domain (ConnectionError, QueryError, etc.) using `thiserror`, Result wrapping throughout.
- **Current workarounds:** Views use `.unwrap_or_default()` on JSON serialization (e.g., `sonic_rs::to_string(...).unwrap_or_default()` in `streams/cdc.rs:16`, `streams/notify.rs:18`), which silently drops errors; must be surfaced once data is dynamic.

---

## Medium: Fragile Areas — Hardcoded Assumptions

**Several places assume mock data structure; will break when real data lands:**

- **Explorer sidebar, viewers:** Iterate over `mock::explorer_collections()` directly. Switching to dynamic collection list requires no code change to the iteration, but schema/sample queries do.
  - Files: `src/views/explorer/sidebar.rs`, `src/views/explorer/view.rs`, each viewer.

- **Notification filtering:** `state/notifications.rs` filters by capability flag. When notifications come from server (not mock), this filtering still works, but notification sources (sync conflicts, cron failures, etc.) depend on real server events.
  - Files: `src/state/notifications.rs`.

- **Capabilities:** Hardcoded per mock connection (e.g., `staging-cluster` has all 9, `prod-replica-eu` has readonly=true). Real flow must pull from `server.capabilities()` after auth.
  - Files: `src/data/mock.rs`, `src/state/connection.rs`.

- **Collection storage modes:** Eight modes are enum variants in `src/models/collection.rs`. Viewers are hardcoded to each mode. If a new storage mode is added to NodeDB, the enum must be updated, and a new viewer component added.
  - Files: `src/models/collection.rs`, `src/views/explorer/view.rs` (match statement), `src/views/explorer/viewers/mod.rs`.

---

## Low: Documentation of Model Types

**Some struct fields carry "pre-rendered" data for the skeleton:**

- **Collection.count:** Stored as a `String` (e.g. "2.4M") to match the mockup. Real client will want numeric count; UI can format it.
  - File: `src/models/collection.rs` (line 78 comment acknowledges this).

- **Notification.when:** Relative timestamp as a pre-rendered string (e.g. "2m ago"). Real server will send absolute timestamps; viewer must compute relative strings.
  - File: `src/models/notification.rs`.

- **SavedConnection.ping:** Optional pre-formatted string (e.g. "8.4ms"). Real flow will measure RTT; viewer formats it.
  - File: `src/state/connections_registry.rs`.

- **Impact:** Low; these are display hints and will be replaced/computed as real data lands. Not blocking, but good to note when refactoring models.

---

## Low: No Real Error Surface

**Connection failures, query errors, and async failures are not surfaced to the user:**

- **Current:** Mock never fails; no UI for error toasts, modal dialogs, or inline error messages.
- **Expected (part of async seam):** Add error toast component, wire failed `use_resource` futures to display errors, handle network timeouts gracefully.
- **Effort:** Low; Dioxus patterns established in AGENTS.md; wait for async seam to be defined.

---

## Summary of Blocking Issues

| Issue | Severity | Blocker | Effort |
|-------|----------|---------|--------|
| Mock-to-real seam narrowness | Critical | Yes, entire app on mock | L (seam design) then XL (impl) |
| ConnectionService sync-only | Critical | Yes, blocks async | M (trait redesign) |
| Version pinning risk (nodedb deps) | High | Possible, CI gap | S (add CI check) |
| Credential storage insecure | High | Yes, for saved connections | M (platform keychain) |
| TLS not configured | High | Yes, for prod deployments | M (form + client impl) |
| No test coverage | Medium | No, but risky at scale | M (write tests) |
| No error types in seam | Medium | Yes, once real client lands | S (define error enum) |
| Auth/connect lifecycle incomplete | Medium | Yes, form has no logic | M (implement auth flow) |
| No loading/error states in views | Medium | Yes, async will need these | S (add UI states) |

---

*Concerns audit: 2026-06-13*
