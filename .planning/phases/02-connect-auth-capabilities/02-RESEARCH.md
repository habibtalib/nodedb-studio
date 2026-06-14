# Phase 2: Connect, Auth & Capabilities - Research

**Researched:** 2026-06-14
**Domain:** Real NodeDB native-client connect/auth wiring + capability/identity negotiation in a Dioxus 0.7 desktop seam
**Confidence:** HIGH (every API claim verified against `../nodedb` source on disk, version-matched 0.3.0)

## Summary

This phase fills the inert `NodeDbConnectionService` stub with a real `connect()` path. The studio constructs a `NativeClient` via `ConnectionBuilder` (or, for OIDC, via a directly-built `PoolConfig`), drives an actual round-trip to populate the lazily-negotiated handshake metadata, reads the server's `capabilities()` `u64` bitmask + `limits()` + `server_version()`, derives the studio's `Capabilities` struct from confirmed bits, and transitions the app into the connected `Studio` shell. Disconnect drops the client.

The research resolved every flagged target against source and found **three CONTEXT.md assumptions that need correcting**, all surfaced below: (1) **the `NativeClient` connection pool is lazy** — `capabilities()`/`server_version()`/`limits()` all return zero/empty defaults until the first real request runs (`pool.acquire()`), so connect MUST issue a round-trip (`ping()` or a `SELECT`) before reading them, or the shell gates everything off; (2) **identity (username, role, current database, databases list) is NOT exposed by the client API** — the auth handshake's `AuthResponse { username, tenant_id }` is parsed by the server but **discarded** by the client's `authenticate()`, and `negotiated_meta()` only carries proto/caps/version/limits — so identity must be derived from SQL (`SELECT current_user, current_database` + `SHOW DATABASES`) or fall back to the connect-form's typed username; (3) **there is no `readonly`, `vector`, or `cluster` capability bit** — `vector` is a core always-on engine, `cluster`/`readonly` are genuinely indeterminate from the native handshake and must lean to their safe defaults.

**Primary recommendation:** Build the `NativeClient` from the reshaped `SavedConnection` + in-session secret; force a handshake with `client.ping().await` immediately after build (this is what populates `negotiated_meta`); map `capabilities() -> u64` through the client's own `Capabilities::from_raw()` predicates into the studio `Capabilities` struct; derive identity via a single `SELECT current_user, current_database` + `SHOW DATABASES` (best-effort, fall back to the form username and `["default"]` on failure); wire OIDC by constructing `PoolConfig` directly (the builder has no OIDC setter but `PoolConfig`/`NativeClient::new` are public); map all connect failures through the existing Phase-1 `From<NodeDbError> for StudioError`.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Secrets live in-session only. The saved registry stores only non-secret connect params; the secret (password / api_key / OIDC token) is entered at connect time, held in memory for the live session, never written to disk this milestone. Reconnect re-prompts. (OS-keychain hardening deferred to v2.)
- **D-02:** The connection form uses a single "Auth method" dropdown that swaps the visible field set per mode: **trust** → username; **password** → username + password; **api_key** → token; **oidc_bearer** → token (+ optional provider hint). Replaces the stale mock picker (Username+password / Token / mTLS). **Fixes the wrong port default** (`2480` → native `6433`). mTLS is transport, not an auth method.
- **D-03:** Connect progress + failures surface **inline on the connecting card**: `Connecting…` → on failure an inline error message + a **Retry** affordance — reusing Phase 1 `AsyncState`/`AsyncView` and `StudioError::is_retriable()`. refused/timeout → `Connection`; bad credentials/unauthorized → `Auth`. The app never freezes and never enters the connected state on failure (CONN-03). Retry re-runs connect at the seam.
- **D-04:** **Conservative capability gating.** Gate features only on capabilities the server **confirms**. Map the `u64` bitmask (validate against `protocol/mod.rs`): `CAP_GRAPHRAG→graph`, `CAP_FTS→fts`, `CAP_SPATIAL→spatial`, `CAP_STREAMING→streams`, `CAP_TIMESERIES→timeseries`, `CAP_CRDT→sync`. If `capabilities()` can't be fetched or a bit is absent, treat the feature as **unavailable (hidden)** — never show a tab that errors on use.
- **D-05:** **`readonly` is derived from the session role/permissions** (no capability bit exists). Research confirms the exact signal; when indeterminate, **lean writable** (`readonly=false`) and flag for follow-up.
- **D-06:** Flags with no server bit — `vector`, `cluster` — resolved in research: `vector` expected to be a core engine (lean always-on once confirmed); `cluster` from server topology/limits if available, else off. Default off only when genuinely indeterminate (per D-04 conservative policy).
- **D-07:** Disconnect (⌘D) is immediate, no confirm — close/drop the `NativeClient`, clear the active-session signal, return to the connection manager, release the session (CONN-07).
- **D-08:** Command-palette and connection-popover "switch connection" actions **reuse the real connect path** (disconnect current + connect to chosen saved connection via the same seam). If the target needs a secret not held in session, route to the connect form/prompt rather than failing silently.
- **D-09:** A `SavedConnection` stores the real connect target: **host, port, auth-mode, username, default database, TLS settings, optional connect-timeout override** (no secret). The mock `ConnectionProfile` (pre-baked user/role/capabilities/databases) is **replaced** — identity and capabilities come from the live server after connect, not from the saved entry.

### Claude's Discretion

- Exact `ConnectionBuilder` wiring for OIDC (`AuthMethod::OidcBearer` direct construction vs builder extension) — pick whatever compiles cleanly and keeps the seam clean.
- The exact server-bit → `Capabilities` mapping, validated against `protocol/mod.rs` `CAP_*`.
- The client-side source of identity (user, role, current database, databases list) for the topbar chip.
- Connect-timeout default (client default is 5s) when no per-connection override is set.
- Whether `NodeDbConnectionService` becomes the **default** impl on a real connect (vs. `MockConnectionService` staying default until a connection is opened) — keep the mock working alongside per seam discipline.
- File/module layout (respect <500 LOC, `mod.rs` re-exports only).

### Deferred Ideas (OUT OF SCOPE)

- OS-keychain credential hardening — in-session secrets only for v1.
- Persisting secrets to disk — explicitly rejected (D-01).
- mTLS as an auth method — TLS is transport config (`ConnectionBuilder.tls()`), captured as a saved-connection setting, not an auth mode.
- Data-path wiring — SQL (Phase 3), collections (Phase 4), document/vector/graph/FTS/KV browsers (Phases 5–6).
- Streams / admin / sync / designer live wiring — v2.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CONN-01 | Open a real session from the connection manager using a saved connection's host:port + credentials | `ConnectionBuilder::new("host:port").username().password()/.api_key().database().tls().connect_timeout().build()` → `NativeClient`. Build is sync + cheap (lazy pool); the network round-trip happens on first request. (Target 1, 6, 7) |
| CONN-02 | Connect with each auth mode — trust, password, API key, OIDC bearer | Builder covers trust/password/api_key. **OIDC has no builder setter** — construct `AuthMethod::OidcBearer { token, provider }` and build a `PoolConfig` directly, then `NativeClient::new(config)`. (Target 5) |
| CONN-03 | A failed connection shows a clear error and does NOT enter the connected state | All failures are `NodeDbError` → existing `From<NodeDbError> for StudioError` (Phase 1). refused/timeout → `Connection`; bad creds → `Auth` (via `AuthorizationDenied`). `is_retriable()` gates Retry. (Target 6) |
| CONN-04 | On success, enter the connected `Studio` shell bound to the live session | Set `Signal<Option<ActiveConnection>>` after the await resolves; the root conditional in `app.rs` swaps to `Studio`. The `NativeClient` must be held somewhere live (service field / signal). (Architecture below) |
| CONN-05 | `Capabilities` derived from server's real `capabilities()`/`limits()` — rail, admin sub-tabs, views gate on actual caps | `NodeDb::capabilities() -> u64`; map via `nodedb_client::Capabilities::from_raw(bits)` predicates. **Must force a handshake first** (lazy pool). (Target 1, 4) |
| CONN-06 | Connection chip reflects real user, role, current database, databases list, server version | `server_version()` from handshake (real). user/role/db list **not on the client** — derive via SQL (`SELECT current_user, current_database`, `SHOW DATABASES`) best-effort; fall back to form username. (Target 2, 3) |
| CONN-07 | Disconnect returns to the connection manager, releasing the client session | Drop the `NativeClient` (its `Pool` closes connections on drop), clear the active signal. The existing ⌘D handler already clears `active`; add client release. (D-07) |
</phase_requirements>

---

## Critical Research Targets — Resolutions

### Target 1: Capability bitmask → `Capabilities` mapping (D-04) — **mapping validated, one correction**

**Source:** `../nodedb/nodedb-types/src/protocol/handshake.rs:27-45` (constants), `../nodedb/nodedb-types/src/protocol/mod.rs:14-19` (re-exports), `../nodedb/nodedb-client/src/capabilities.rs` (typed predicates).

The exact `CAP_*` constants that exist (all `pub const … : u64`):

| Constant | Bit value | Studio `Capabilities` field (D-04 proposal) | Verdict |
|----------|-----------|---------------------------------------------|---------|
| `CAP_STREAMING` | `1 << 0` | `streams` | ✅ confirmed |
| `CAP_GRAPHRAG` | `1 << 1` | `graph` | ✅ confirmed (the graph bit IS `CAP_GRAPHRAG`; there is no `CAP_GRAPH`) |
| `CAP_FTS` | `1 << 2` | `fts` | ✅ confirmed |
| `CAP_CRDT` | `1 << 3` | `sync` | ✅ confirmed (CRDT = peer replication = the studio's `sync`) |
| `CAP_SPATIAL` | `1 << 4` | `spatial` | ✅ confirmed |
| `CAP_TIMESERIES` | `1 << 5` | `timeseries` | ✅ confirmed |
| `CAP_COLUMNAR` | `1 << 6` | *(no studio field)* | ⚠️ exists but unmapped — studio has no columnar capability; ignore |
| `CAP_MSGPACK` | `1 << 7` | *(transport)* | always set on native; not a feature gate |

**`capabilities()` returns `u64` — confirmed** (`NodeDb::capabilities(&self) -> u64`, `trait_def.rs:480`). The intended way to test bits is the client's own newtype: `nodedb_client::Capabilities::from_raw(bits)` with `.supports_graphrag()`, `.supports_fts()`, `.supports_spatial()`, `.supports_streaming()`, `.supports_timeseries()`, `.supports_crdt()`, `.supports_columnar()`, and a generic `.has(bit)` (`capabilities.rs:30-83`). **Recommend using this newtype** rather than hand-masking — it is re-exported as `nodedb_client::Capabilities`.

**Code-shaped recommendation:**

```rust
use nodedb_client::{Capabilities as ClientCaps, NodeDb};
use crate::state::connection::Capabilities; // studio's struct

fn derive_capabilities(client: &impl NodeDb) -> Capabilities {
    let caps = ClientCaps::from_raw(client.capabilities());
    Capabilities {
        graph:      caps.supports_graphrag(),   // CAP_GRAPHRAG
        fts:        caps.supports_fts(),         // CAP_FTS
        spatial:    caps.supports_spatial(),     // CAP_SPATIAL
        streams:    caps.supports_streaming(),   // CAP_STREAMING
        timeseries: caps.supports_timeseries(),  // CAP_TIMESERIES
        sync:       caps.supports_crdt(),        // CAP_CRDT
        vector:     true,                        // core engine, no bit — see Target 4
        cluster:    false,                       // no native bit — see Target 4
        readonly:   /* see Target 3 */ false,
    }
}
```

> ⚠️ **Critical correction (lazy pool):** `client.capabilities()` returns `0` until the first request runs. See Target 6's "force a handshake" note — call `client.ping().await` (or the identity `SELECT`) BEFORE `derive_capabilities`, or every flag is `false` and the entire shell collapses to Explorer-only.

**Conservative-gating note (D-04):** since `capabilities()` returns `0` on a never-used or failed-handshake client, the "absent bit → hidden" policy is automatically satisfied: a missing bit yields `false`. The only trap is reading capabilities before the handshake (all-zero false negative) — that is a correctness bug, not conservatism.

### Target 2: Identity source (D-05, Discretion) — **NOT available from the client; derive via SQL**

**This is the biggest CONTEXT.md divergence.** CONTEXT.md says "only `capabilities()`, `limits()`, `server_version()` are confirmed present" and asks research to find which methods expose username/role/current-database/databases-list. **Answer: none of them do.**

Evidence:
- The `NodeDb` trait's only connection-metadata accessors are `proto_version()`, `capabilities()`, `server_version()`, `limits()` (`trait_def.rs:466-497`). No `username`, `role`, `current_database`, or `databases`.
- `NegotiatedMeta` (`native/pool.rs:57-64`) carries only `proto_version`, `capabilities`, `server_version`, `limits`. No identity.
- The auth handshake DOES return identity: `AuthResponse { username: String, tenant_id: u64 }` (`protocol/auth.rs:42-45`), and the server populates it (`frames.rs:153 auth_ok(...)`). **But the client discards it** — `NativeConnection::authenticate()` (`native/connection/mod.rs:186-212`) only checks `resp.status == Error` and never reads `resp.auth`. The parsed `AuthResponse` is dropped on the floor.
- `grep` across the whole client for `fn username`/`fn role`/`fn current_database`/`fn databases`/`AuthResponse` usage confirms: the builder's `.username()` setter is the only `username` symbol; `AuthResponse` appears only in the types crate, never read by client code.

**Conclusion:** username/role/current-database/databases-list cannot be read from the `NativeClient` API as it stands. Three options, in order of recommendation:

1. **Derive via SQL (recommended).** The server's SQL surface supports identity functions and a database list:
   - `current_user`, `current_role`, `session_user`, `current_database` are recognized SQL functions (`../nodedb/nodedb-sql/src/resolver/expr/convert.rs:30-33,63`).
   - `SHOW DATABASES` is a parsed, dispatched statement (`../nodedb/nodedb-sql/src/ddl_ast/parse/database/dispatch.rs:43-45`; server lists via `catalog.list_databases()`).
   - So after connect, run one or two cheap queries through `client.execute_sql(...)`:
     ```rust
     // best-effort identity enrichment; failures fall back, never block connect
     let who = client.execute_sql("SELECT current_user, current_role, current_database", &[]).await;
     let dbs = client.execute_sql("SHOW DATABASES", &[]).await;
     ```
   - Parse the single row of `who` for user/role/current-db, and the `dbs` rows for the database list (column shape TBD — planner should verify the exact column names/positions against a live server or `SHOW DATABASES` execution path; treat as best-effort).
   - **This doubles as the "force a handshake" round-trip** required by Target 1/6 — one `SELECT` both populates `negotiated_meta` (for capabilities) and yields identity. Efficient.

2. **Fall back to the form-supplied username.** The user typed a username for trust/password modes (D-02). Use it as `ActiveConnection.user` when the SQL probe fails or returns nothing. For api_key/oidc modes there is no form username — fall back to a placeholder (e.g. the token's `provider` hint, or `"api_key"` / the connection name). `role` falls back to `""` (the topbar simply shows no role); `current_database` falls back to the saved default database (D-09) or `"default"`; `databases` falls back to `vec![current_database]`.

3. **(Not recommended) patch the client to expose `AuthResponse`.** Surfacing `username`/`tenant_id` from `authenticate()` would require editing `../nodedb` (the dependency), which is an ask-first cross-repo change and out of this milestone's scope. The SQL-derivation path needs no dependency change.

**`server_version()` is the one identity-ish field that IS real** (`dispatch.rs:41-46`, sourced from `HelloAckFrame.server_version`) — but, like capabilities, it's empty until the handshake round-trip runs.

**Recommended `ActiveConnection` population:**

| Field | Source | Fallback |
|-------|--------|----------|
| `name` | saved connection name | — |
| `user` | `SELECT current_user` | form username → connection name |
| `role` | `SELECT current_role` | `""` (topbar shows none) |
| `current_database` | `SELECT current_database` | saved default DB (D-09) → `"default"` |
| `databases` | `SHOW DATABASES` rows | `vec![current_database]` |
| `sub` | composed: `"{server} · {server_version}"` or `"{n} dbs"` | from saved `meta`/`server` |
| `capabilities` | `capabilities()` bitmask (Target 1) | all-false if probe fails |

### Target 3: `readonly` derivation (D-05) — **genuinely indeterminate from native handshake; lean writable**

There is no `CAP_READONLY` bit and no `readonly` field anywhere in the handshake or `NegotiatedMeta`. Possible signals, all weak from the client side:
- **Role string:** if the identity SQL probe (Target 2) returns a `current_role` containing a read-only marker (the mock used `"analyst (read-only)"`), the studio could heuristically set `readonly = role.contains("read-only")`. This is brittle and server-convention-dependent — there is no documented contract that roles encode read-only in the name.
- **Error-on-write:** the only authoritative signal is the `MirrorReadOnly` (NDB-1700) / `AppendOnlyViolation` errors which only surface *after* a write is attempted (these map to `StudioError::ReadOnly` in Phase 1). That is a runtime signal, not a connect-time one.
- **Server-side `GRANT … ON DATABASE` checks** exist (`nodedb/src/control/security/...`) but are not surfaced through any client read method.

**Resolution (per D-05):** at connect time `readonly` is **indeterminate** → **lean writable (`readonly = false`)**. Optionally apply the role-string heuristic as a soft hint if the identity probe returns a role. **Flag for follow-up:** a future phase could set `readonly` reactively when the first `StudioError::ReadOnly` is observed on a write attempt. Do not block legitimate writes by defaulting to read-only.

### Target 4: `vector` & `cluster` flags (D-06) — **vector = always-on; cluster = off (indeterminate)**

- **`vector`:** no `CAP_VECTOR` bit exists. Vector is a **core, always-on engine** — confirmed by the protocol surface: `Limits.max_vector_dim`/`max_top_k` exist unconditionally (`handshake.rs:51-52`), `OpCode::VectorSearch`/`VectorBatchInsert` are core opcodes (`protocol/mod.rs` test asserts `VectorBatchInsert as u8 == 0x70`), and `NodeDb::vector_search`/`vector_insert` are non-defaulted trait methods every backend must implement (`trait_def.rs:62-87`). **Set `vector = true`** (lean always-on, per D-06). Optionally treat a non-`None` `limits().max_vector_dim` as positive confirmation, but the safe default is on.
- **`cluster`:** no `CAP_CLUSTER` bit. The native handshake exposes no topology/node-count. `limits()` has no cluster dimension. The only cluster-adjacent signals are *runtime errors* (`NoLeader`/`NotLeader`/`NodeUnreachable`, surfaced via `NodeDbError::is_cluster()` — `types.rs:147-156`), not a connect-time flag. Cluster is **genuinely indeterminate** → **`cluster = false`** (per D-04/D-06 conservative policy). The admin cluster sub-tabs stay hidden this milestone, which matches REQUIREMENTS Out-of-Scope ("admin/cluster … to live data — v2").

### Target 5: OIDC builder wiring (D-02, Discretion) — **build `PoolConfig` directly; no builder change needed**

**Source:** `../nodedb/nodedb-client/src/native/builder.rs`, `native/pool.rs`, `native/mod.rs`, `native/client/mod.rs`.

`ConnectionBuilder` (`builder.rs:39-132`) exposes exactly: `new(addr)`, `.username()`, `.password()`, `.api_key()`, `.database()`, `.max_connections()`, `.connect_timeout()`, `.idle_timeout()`, `.tls(TlsConfig)`, `.build() -> NativeClient`. **Confirmed: no OIDC setter.** `build()` hard-codes the precedence `api_key → password → trust` and can NEVER produce an `OidcBearer` (`builder.rs:106-114`).

`AuthMethod::OidcBearer { token, provider: Option<String> }` exists (`protocol/auth.rs:24-31`), and `AuthMethod` is `#[non_exhaustive]`.

**Cleanest wiring (no dependency edit):** the pool module is public and `NativeClient::new(PoolConfig)` is public, so the studio can construct the config directly for OIDC:

```rust
use std::time::Duration;
use nodedb_client::native::pool::PoolConfig;        // pub mod pool; pub struct PoolConfig
use nodedb_client::native::connection::TlsConfig;    // re-exported from connection
use nodedb_client::NativeClient;                     // NativeClient::new is pub
use nodedb_types::protocol::AuthMethod;

let config = PoolConfig {
    addr: format!("{host}:{port}"),
    auth: AuthMethod::OidcBearer { token, provider }, // the variant the builder can't make
    database: default_db,                              // Option<String>
    max_size: PoolConfig::default().max_size,
    connect_timeout: timeout.unwrap_or(Duration::from_secs(5)),
    idle_timeout: PoolConfig::default().idle_timeout,
    tls,
};
let client = NativeClient::new(config);
```

For trust/password/api_key, **use the builder** (it's the documented happy path and produces the same `PoolConfig` internally). Recommended seam shape: one private helper `fn build_client(target: &ConnectTarget, secret: Secret) -> NativeClient` that branches — builder for the three builder-supported modes, direct `PoolConfig` for OIDC. This keeps OIDC's one-off in a single place and avoids touching `../nodedb`.

> Note: `PoolConfig.database` is `Option<String>`; `None` means the server default (`"default"`). `TlsConfig` is `{ enabled, ca_cert_path: Option<PathBuf>, server_name: Option<String>, danger_accept_invalid_certs }` (`native/connection/tls.rs:5-16`) — re-exported as `nodedb_client::native::connection::TlsConfig`.

### Target 6: Connect entrypoint & error mapping — **build is lazy; force a round-trip; reuse Phase-1 mapping**

**Entry signatures (verified):**
- `ConnectionBuilder::build(self) -> NativeClient` — **synchronous, infallible, no network**. It only assembles a `PoolConfig` and calls `NativeClient::new(config)` which creates an empty `Pool` (`builder.rs:103-131`, `core.rs:18-24`, `pool.rs:84-98`). **No connection is opened, no auth is performed, no error can occur at build time.**
- The network + auth happen lazily on the first `Pool::acquire()` (`pool.rs:114-183`): TCP connect (with `connect_timeout`), `perform_client_handshake()`, then `authenticate(auth, database)`. `acquire()` is triggered by any real op — `client.ping()`, `client.query(...)`, or any `NodeDb` method. All return `NodeDbResult<_>`.

**Therefore the connect path MUST issue a real request to surface connect/auth errors and to populate capabilities/version.** The thinnest trigger is `client.ping().await` (`core.rs:95-98` → `conn.ping()`); but since identity also needs a `SELECT` (Target 2), use the identity `SELECT current_user, current_database` as the single round-trip that (a) opens the socket, (b) runs the handshake → fills `negotiated_meta`, (c) authenticates, (d) returns identity. If it errors, connect failed.

**Failure-mode → `StudioError` mapping (all via the existing Phase-1 `From<NodeDbError>`):**

| Failure | `NodeDbError` produced (where) | → `StudioError` (Phase 1) | Retriable? |
|---------|-------------------------------|---------------------------|------------|
| TCP refused / host unreachable / connect timeout | `sync_connection_failed(...)` (`connection/mod.rs:48`, `pool.rs:117,154`) | `Connection` | ✅ (`SyncConnectionFailed` is retriable, `types.rs:67-80`) |
| Pool acquire timeout | `sync_connection_failed("pool acquire timeout")` (`pool.rs:117`) | `Connection` | ✅ |
| Protocol version mismatch | `handshake_failed(VersionMismatch, …)` (`connection/mod.rs:144`) | `Connection` | ❌ (`HandshakeFailed` not in retriable set) |
| Bad credentials / unauthorized | `authorization_denied(msg)` (`connection/mod.rs:207`) | `Auth` | ❌ (`AuthorizationDenied` not retriable) |
| HelloAck decode failure | `internal(...)` (`connection/mod.rs:148,171`) | `Server` | ❌ |
| msgpack encode/decode | `serialization(...)` (`connection/mod.rs:366,395`) | `Server` | ❌ |

`StudioError::is_retriable()` (`nodedb-studio/src/services/error.rs:36-47`) delegates to `NodeDbError::is_retriable()`. **Confirmed retriable set** (`types.rs:67-80`): `WriteConflict`, `DeadlineExceeded`, `NoLeader`, `NotLeader`, `MigrationInProgress`, `NodeUnreachable`, `SyncConnectionFailed`, `Bridge`, `MemoryExhausted`. **So for the D-03 Retry affordance: `Connection` errors are retriable (refused/timeout → show Retry); `Auth` errors are NOT retriable (bad creds → show error, no Retry).** This matches the intuitive UX: retry a flaky network, don't retry a wrong password. The four categories' retriability: `Connection` = mostly retriable (handshake-mismatch sub-case is not), `Auth` = never, `Setup` = never, `NotConnected` = never (studio-originated). No new error variants are needed — Phase 1's `StudioError` already covers every connect failure.

> **One concrete gotcha for the planner:** because the *whole* `NodeDbError` is moved into the `StudioError` variant and `is_retriable()` is delegated, a `HandshakeFailed` version-mismatch maps to `Connection` but is **not** retriable — the card will correctly show an error without a Retry button. Don't assume "category Connection ⇒ retriable".

### Target 7: Address / TLS construction — **`SavedConnection` reshape (D-09) maps 1:1 to the builder**

The connect target is `"host:port"` (e.g. `"localhost:6433"`). `DEFAULT_NATIVE_PORT = 6433` (`handshake.rs:16`) — the form's `2480` default (`new_connection.rs:22`) is wrong and must change to `6433` (D-02). The builder takes the joined string: `ConnectionBuilder::new(format!("{host}:{port}"))`.

The reshaped `SavedConnection` (D-09) maps cleanly:

| D-09 field | Builder / PoolConfig sink | Notes |
|------------|---------------------------|-------|
| host + port | `ConnectionBuilder::new("{host}:{port}")` / `PoolConfig.addr` | port defaults to `6433` |
| auth-mode | selects builder method / `AuthMethod` variant | trust/password/api_key/oidc_bearer |
| username | `.username(u)` | trust + password; ignored for api_key/oidc |
| default database | `.database(db)` / `PoolConfig.database: Option<String>` | `None` → server `"default"` |
| TLS settings | `.tls(TlsConfig { enabled, ca_cert_path, server_name, danger_accept_invalid_certs })` | transport, not auth |
| connect-timeout override | `.connect_timeout(Duration)` / `PoolConfig.connect_timeout` | **default 5s** (`pool.rs:45`); use 5s when no override (Discretion) |

The in-session secret (D-01) is the password / api_key token / OIDC token, supplied at connect time and fed into `.password()` / `.api_key()` / the `OidcBearer { token }` — never stored in `SavedConnection`.

---

## Runtime State Inventory

> This phase opens live sockets and holds an in-memory client; it does not rename or migrate stored data. Inventory of live runtime state created/held this phase:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no schema/keys/IDs written by this phase. Identity probe issues read-only `SELECT`/`SHOW`. | None |
| Live service config | A live `NativeClient` holds a TCP connection **pool** (`max_size` default 10, idle-timeout 5min) to `:6433`. On disconnect the `NativeClient`/`Pool` must be dropped so sockets close (CONN-07, D-07). | Drop client on disconnect; ensure no lingering `Rc`/signal clone keeps the pool alive after ⌘D. |
| OS-registered state | None — no daemons, scheduler entries, or OS registrations. | None |
| Secrets/env vars | In-session secret (password/api_key/OIDC token) held in memory for the live session only (D-01). **Must never be written to `SavedConnection`, disk, logs, or `tracing` output.** | Keep secret out of the saved registry and out of any `#[derive(Debug)]` that gets logged. |
| Build artifacts | None new. (`native` feature on `nodedb-client` was already enabled in Phase 1.) | None |

**Canonical question — after connect, what runtime state exists that grep won't find?** A live TCP pool inside the `NativeClient` and an in-memory secret. Both are released by dropping the client on disconnect. Verified: `Pool` has no explicit `close()`; connections drop with the pool (`PooledConnection::drop` returns to idle, the `Pool`/`PoolInner` drop frees them). Holding the client in a `Signal`/service field is fine as long as disconnect replaces it with `None`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `nodedb-client` (feature `native`) | `0.3.0` | `ConnectionBuilder`, `NativeClient`, `Capabilities`, `NodeDb` trait, re-exported `NodeDbError` | The seam's real backend. `native` feature already enabled (Phase 1). |
| `nodedb-types` | `0.3.0` | `AuthMethod`, `Limits`, `CAP_*` constants, `DEFAULT_NATIVE_PORT`, `Value`, `QueryResult` | Auth + capability + result types. |
| `async-trait` | `0.1` | Seam is `#[async_trait(?Send)]` (Phase 1) | The `connect()` method is async; mock + real both impl. |
| `dioxus` (`desktop`,`router`) | `0.7.9` | `use_action`/`spawn` for the one-shot connect; `AsyncState`/`AsyncView` for the inline card | Connect is a button-triggered one-shot → `spawn`/`use_action`, NOT `use_resource` (reactive read). |
| `thiserror` | `2.0` | `StudioError` (Phase 1, unchanged) | All connect failures already map. |
| `tokio` | `1` | Backs the async connect + `#[tokio::test]` | Already present. |

**No new dependencies required.** OIDC wiring uses already-public client APIs (`PoolConfig`, `NativeClient::new`). No `../nodedb` edits, no version bumps. (`AGENTS.md` ask-first dep gate is not triggered.)

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `sonic_rs` | `0.5` | Runtime JSON for display only | Not needed this phase (no JSON display); never `serde_json`. |

**Version verification:** All pinned at the versions Phase 1 verified (`nodedb-client`/`nodedb-types` `0.3.0` local workspace, `dioxus` `0.7.9`, `async-trait` `0.1.89`, `thiserror` `2.0.18`). No new packages to verify. HIGH.

---

## Architecture Patterns

### Where the live client lives (CONN-04/07)

The Phase-1 stub holds `client: Option<NativeClient>` (`nodedb_service.rs:18-23`). Two viable shapes:

- **(A) Service swap (recommended for clean seam discipline):** keep `MockConnectionService` as the default in `app.rs`; on a successful connect, replace the context-provided `Rc<dyn ConnectionService>` with an `Rc<NodeDbConnectionService>` whose `client` is `Some(NativeClient)`. Disconnect swaps back to the mock (or a fresh stub). This matches the Discretion note "whether `NodeDbConnectionService` becomes the default impl on a real connect — keep the mock working alongside." Caveat: the connect action itself can't easily live behind the *current* mock service; do the build + probe in the connect handler (at the seam), then provide the connected service.
- **(B) Interior-mutable client:** `NodeDbConnectionService` holds `RefCell<Option<NativeClient>>` (or a `Signal`), `connect()` builds + probes + stores the client, later phases read it. Simpler for "the service is always `NodeDbConnectionService`, mock only for tests." Watch the `?Send`/`Rc` interaction — `RefCell` is fine on the single UI thread.

Either honors seam discipline. The planner picks; **(B)** keeps `connect()` genuinely *at the seam* (the service method does the work), which reads cleaner against SEAM discipline and CONN-01. Whichever is chosen, the `NativeClient` must be reachable by Phase 3's `execute_sql`.

### Pattern: connect at the seam (one-shot `spawn`/`use_action`)

The existing `connection_manager.rs` already uses the right shape (`spawn` + `active.set()` after await, `connection_manager.rs:50-63`) but currently swallows the `Err` (`if let Ok(session) = …`). Phase 2 must:
1. Resolve the in-session secret (from the form/prompt — D-01).
2. Build the client (builder, or `PoolConfig` for OIDC — Target 5).
3. **Probe** (identity `SELECT` — the forcing round-trip, Target 2/6). On `Err`, map to `StudioError`, set the card's `AsyncState::Error(e)`, gate Retry on `e.is_retriable()` — **do not** set `active` (CONN-03).
4. On `Ok`, read `capabilities()` + `server_version()`, build `ActiveConnection`, store the client, then `active.set(Some(conn))` (CONN-04).

Use `AsyncState`/`AsyncView` (Phase 1) for the inline card lifecycle (D-03). The Retry button calls the same connect closure (a fresh `spawn`), not `Resource::restart()` (this is a `spawn`-driven one-shot, not a `use_resource`).

### Anti-patterns to avoid

- **Reading `capabilities()`/`server_version()` before any request** — returns `0`/`""` (lazy pool). Always probe first.
- **Entering the connected state on a build that hasn't been probed** — build never fails; only the probe reveals refused/auth errors. Set `active` only after a successful probe (CONN-03).
- **Holding a signal `.read()`/`.write()` guard across `.await`** — clone the `Rc`/copy values before the `async move`; `active.set()` only after the await (existing pattern is correct).
- **Logging the secret** — keep it out of `Debug`/`tracing`.
- **`_ =>` on the studio's own enums** — the only allowed catch-all is the existing foreign-`ErrorDetails` arm in `From<NodeDbError>` (already in place, Phase 1).
- **Storing identity/caps in `SavedConnection`** — they come from the live server now (D-09); the saved entry is connect-config only.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Auth method selection | Custom enum + wire encoding | `nodedb_types::protocol::AuthMethod` (`Trust`/`Password`/`ApiKey`/`OidcBearer`) | It's the wire type; the builder/pool consume it directly. |
| Capability bit masking | Manual `& (1<<n)` | `nodedb_client::Capabilities::from_raw(bits).supports_*()` | Named predicates, re-exported, unit-tested in the client. |
| Connect timeout / pool | Custom socket + retry loop | `ConnectionBuilder.connect_timeout()` + `Pool` (retries once on conn error, `core.rs:37-48`) | The pool handles timeout, health-check, single-retry. |
| Error categorization | Re-classify connect errors | Phase-1 `From<NodeDbError> for StudioError` + `is_retriable()` | Already maps every connect failure; reuse verbatim. |
| TLS client config | Hand-build rustls | `ConnectionBuilder.tls(TlsConfig{..})` | Builds the rustls config internally (`tls.rs`). |
| Address default | Hardcode strings | `nodedb_types::protocol::DEFAULT_NATIVE_PORT` (`6433`) | Single source of truth for the native port. |

**Key insight:** Phase 2 is almost entirely *composing existing client primitives*. The only genuinely new studio code is: the auth-mode form rework (D-02), the connect-and-probe handler, the capability/identity derivation, and the `SavedConnection`/`ConnectionProfile` reshape (D-09). No new error model, no new async machinery, no dependency changes.

---

## Common Pitfalls

### Pitfall 1: Capabilities/version read before the handshake (lazy pool)
**What goes wrong:** `client.capabilities()` returns `0`, `server_version()` returns `""`; the shell hides every capability tab and the chip shows no version — even though the server supports them.
**Why:** `ConnectionBuilder::build()` opens no socket; `negotiated_meta()` is `None` until the first `Pool::acquire()`. (`pool.rs:102-108,165-176`)
**How to avoid:** issue one real request (the identity `SELECT`, or `client.ping()`) and `.await` it BEFORE reading caps/version. Treat that request's `Err` as connect failure.
**Warning sign:** "everything works but no capability tabs show" / "version chip blank" on a server you know has features.

### Pitfall 2: Assuming `connect()`/`build()` can fail
**What goes wrong:** wiring error handling around `build()` and entering the connected state when it returns "Ok".
**Why:** `build()` is infallible and synchronous; the only failure surface is the probe round-trip.
**How to avoid:** put all error handling on the awaited probe, not the build. Never `active.set()` before a successful probe.

### Pitfall 3: Expecting identity from the client
**What goes wrong:** looking for `client.username()`/`client.current_database()` — they don't exist; the parsed `AuthResponse` is discarded by `authenticate()`.
**How to avoid:** derive identity via `SELECT current_user, current_database` + `SHOW DATABASES`, with form-username/default-db fallbacks (Target 2). Treat identity enrichment as best-effort — never fail connect because the identity probe returned nothing.

### Pitfall 4: OIDC via the builder
**What goes wrong:** trying `.oidc(token)` — there is no such method; `build()` can only produce trust/password/api_key.
**How to avoid:** construct `PoolConfig { auth: AuthMethod::OidcBearer{..}, .. }` and call `NativeClient::new(config)` (Target 5).

### Pitfall 5: Treating all `Connection` errors as retriable
**What goes wrong:** showing Retry on a protocol version mismatch (`HandshakeFailed` → `Connection` but NOT retriable).
**How to avoid:** gate Retry strictly on `StudioError::is_retriable()` (which delegates to `NodeDbError::is_retriable()`), not on the category name.

### Pitfall 6: Leaking the client/secret on disconnect
**What goes wrong:** ⌘D clears `active` but a stray `Rc<NativeClient>` clone keeps the pool (and its sockets) alive; or the secret survives in a signal.
**How to avoid:** ensure the single owner of the `NativeClient` is replaced with `None` on disconnect; don't clone the client into long-lived closures. Verify sockets close (CONN-07).

---

## Code Examples

### Building the client per auth mode (verified APIs)
```rust
// Source: ../nodedb/nodedb-client/src/native/builder.rs, native/pool.rs, native/connection/tls.rs
use std::time::Duration;
use nodedb_client::{ConnectionBuilder, NativeClient};
use nodedb_client::native::pool::PoolConfig;
use nodedb_client::native::connection::TlsConfig;
use nodedb_types::protocol::AuthMethod;

fn build_client(host: &str, port: u16, db: Option<String>, tls: TlsConfig,
                timeout: Option<Duration>, mode: AuthInput) -> NativeClient {
    let addr = format!("{host}:{port}");
    let to = timeout.unwrap_or(Duration::from_secs(5)); // builder/pool default is 5s
    match mode {
        AuthInput::Trust { username } =>
            base(&addr, db, tls, to).username(username).build(),
        AuthInput::Password { username, password } =>
            base(&addr, db, tls, to).username(username).password(password).build(),
        AuthInput::ApiKey { token } =>
            base(&addr, db, tls, to).api_key(token).build(),
        AuthInput::Oidc { token, provider } => {
            // builder has no OIDC setter -> direct PoolConfig
            let d = PoolConfig::default();
            NativeClient::new(PoolConfig {
                addr, auth: AuthMethod::OidcBearer { token, provider },
                database: db, max_size: d.max_size,
                connect_timeout: to, idle_timeout: d.idle_timeout, tls,
            })
        }
    }
}
fn base(addr: &str, db: Option<String>, tls: TlsConfig, to: Duration) -> ConnectionBuilder {
    let mut b = ConnectionBuilder::new(addr).tls(tls).connect_timeout(to);
    if let Some(d) = db { b = b.database(d); }
    b
}
```

### Connect-and-probe at the seam
```rust
// The probe is the forcing round-trip: it opens the socket, runs the handshake
// (populating capabilities/server_version), authenticates, and returns identity.
async fn connect_and_describe(client: &NativeClient, form_user: Option<&str>, default_db: &str)
    -> Result<ActiveConnection, StudioError>
{
    use nodedb_client::NodeDb;
    // 1. forcing round-trip + identity (best-effort columns)
    let who = client
        .execute_sql("SELECT current_user, current_role, current_database", &[])
        .await?;                                  // ? -> StudioError via From<NodeDbError>
    // 2. now caps/version are populated
    let caps = derive_capabilities(client);       // Target 1
    let version = client.server_version();        // real after the round-trip
    // 3. parse `who` row for user/role/db (fall back to form_user / default_db)
    // 4. SHOW DATABASES (best-effort; fall back to vec![current_db])
    let dbs = client.execute_sql("SHOW DATABASES", &[]).await.ok();
    // 5. assemble ActiveConnection { user, role, current_database, databases, capabilities, sub, name }
    # unimplemented!()
}
```

### Capability derivation test
```rust
#[test]
fn maps_graphrag_bit_to_graph_capability() {
    use nodedb_client::Capabilities as C;
    use nodedb_types::protocol::{CAP_GRAPHRAG, CAP_FTS};
    let caps = C::from_raw(CAP_GRAPHRAG | CAP_FTS);
    assert!(caps.supports_graphrag());  // -> studio graph = true
    assert!(caps.supports_fts());       // -> studio fts   = true
    assert!(!caps.supports_crdt());     // -> studio sync  = false
}
```

---

## State of the Art

| Old (mock) | New (real) | Impact |
|------------|------------|--------|
| `SavedConnection.profile: Option<ConnectionProfile>` pre-bakes user/role/caps/databases; `SavedConnection::open()` builds `ActiveConnection` locally (`connections_registry.rs:29-71`) | `SavedConnection` stores connect-config only (D-09); identity + caps come from the live server post-connect | `ConnectionProfile` removed/reshaped; `open()` replaced by the connect-and-probe path; `ConnStatus::ReadOnly`/`Offline` semantics revisited (offline = unreachable at connect time, not a pre-baked flag). |
| Form: port `2480`, picker "Username+password / Token / mTLS", "stored in OS keychain" copy (`new_connection.rs`) | Port `6433`; dropdown trust/password/api_key/oidc swapping fields; in-session secret copy | Form rework (D-02); mTLS demoted to a TLS transport toggle, not an auth mode. |
| `connect()` swallows `Err` (`connection_manager.rs:57`) | `connect()` surfaces `Err` as inline `AsyncState::Error` + Retry; never enters connected state on failure | CONN-03 satisfied. |

**Deprecated/corrected assumptions (vs CONTEXT.md):**
- "`capabilities()`/`limits()`/`server_version()` are confirmed present" — present, but **return defaults until the first request** (lazy pool). Must force a round-trip. (HIGH)
- "Research determines which client/trait methods provide user/role/db list" — **none do**; the parsed `AuthResponse` is discarded. Derive via SQL. (HIGH)
- "`readonly` derived from session role/permissions" — no client-side signal at connect; lean writable, flag for follow-up. (HIGH)

---

## Open Questions / Open Risks

1. **Exact column names/shape of `SELECT current_user, current_database` and `SHOW DATABASES`.**
   - Known: the functions/statement are recognized by the SQL layer (`resolver/expr/convert.rs`, `database/dispatch.rs`); `SHOW DATABASES` lists via `catalog.list_databases()`.
   - Unclear: the precise column labels/positions and whether `current_role` is supported identically (it's in the recognized-function list but untested via the native path here). Whether a multi-statement `SELECT` returns the columns the studio expects.
   - **Recommendation:** treat identity enrichment as best-effort with robust fallbacks (form username, default DB, `vec![current_db]`); a Wave-0 task should run these against a live server to pin column shapes, but connect must succeed even if the probe returns unexpected shapes. Do NOT block connect on identity parsing.

2. **`readonly` will be wrong for genuinely read-only sessions until the first write is rejected.**
   - Risk: a read-only credential shows a writable shell; writes fail at runtime (`StudioError::ReadOnly`). Acceptable for this phase (no data-path writes until Phase 5), but flag: a later phase should set `readonly` reactively on the first `ReadOnly` error, or add a server-side read-only probe if one becomes available.

3. **Identity for api_key / oidc modes has no form username.**
   - There is no human username in those modes and the client discards `AuthResponse`. The chip's `user`/`avatar_letter` fall back to a placeholder (provider hint, `"api_key"`, or the connection name). Cosmetic, but flag so the planner picks a deliberate fallback rather than an empty avatar.

4. **`SHOW DATABASES` may require privileges the connected role lacks.**
   - If it errors (auth/permission), fall back to `vec![current_database]`. Already covered by the best-effort policy, but note the database-chip list may be incomplete for low-privilege roles.

5. **Client ownership / `?Send` interaction (architecture choice B).**
   - Holding `NativeClient` in `RefCell<Option<…>>` inside an `Rc<NodeDbConnectionService>` is fine single-threaded, but the planner must ensure the connect closure doesn't capture a `.borrow_mut()` guard across `.await`. Keep the build/probe synchronous-then-store discipline.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `nodedb-client` `native` feature | `NativeClient`/`ConnectionBuilder` | ✓ (enabled in Phase 1) | 0.3.0 | none needed |
| `../nodedb` checkout | API verification + local patch | ✓ | workspace 0.3.0 | `.cargo/config.toml` patch present (`/Users/habib/Git/nodedb-studio-v1/.cargo/config.toml`) |
| `dioxus`/`dioxus-hooks` | `spawn`/`use_action`/`AsyncState` | ✓ | 0.7.9 | — |
| **Live NodeDB server :6433** | Manual end-to-end connect/auth/cap verification (CONN-01..06) | ✗ (not confirmed running in this env) | — | **No code fallback** — automated tests can cover build/mapping/error-mapping in plain Rust; the real connect/auth/identity path needs a live server (or a mock TCP server) for end-to-end proof. Flag for the planner: a Wave-0 task should stand up or point at a live instance, else CONN-01/02/05/06 success criteria are only unit-provable, not e2e-provable. |

**Missing dependency with no fallback (flag):**
- A reachable NodeDB server on `:6433` is needed to *prove* the connect/auth/capability/identity path end-to-end. Without it, the phase can be unit-tested (client construction, capability mapping, error mapping, identity-parse with fixture rows) but the live round-trip (handshake → caps populated, auth success/failure, `SHOW DATABASES`) can only be validated against a real or stubbed server. Plan a smoke step accordingly.

---

## Validation Architecture

> nyquist_validation not disabled in config → section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo-nextest` 0.9.x; `#[tokio::test]` for async seam methods; inline `#[cfg(test)] mod tests` |
| Config file | none (nextest defaults) |
| Quick run command | `cargo nextest run -p nodedb-studio` |
| Full suite command | `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CONN-01 | Builder assembles a `NativeClient` from host:port + trust/password/api_key | unit | `cargo nextest run -E 'test(builds_client_)'` | ❌ Wave 0 |
| CONN-02 | OIDC builds via direct `PoolConfig` with `AuthMethod::OidcBearer` | unit | `cargo nextest run -E 'test(oidc_builds_via_poolconfig)'` | ❌ Wave 0 |
| CONN-02 | Each auth mode connects end-to-end | manual/e2e (needs server) | manual `cargo run` against live :6433 | ❌ needs server |
| CONN-03 | Connect failure maps to `StudioError`, sets card error, never sets `active`; Retry gated on `is_retriable()` | unit + render | `cargo nextest run -E 'test(connect_error_)'` (+ `dioxus_ssr` optional) | ❌ Wave 0 |
| CONN-04 | Successful probe sets `active`; shell shows `Studio` | manual/e2e | manual `cargo run` | ❌ needs server |
| CONN-05 | `capabilities()` bitmask → studio `Capabilities` mapping (each bit) | unit | `cargo nextest run -E 'test(maps_)'` (caps mapping) | ❌ Wave 0 |
| CONN-06 | Identity-row parse → `ActiveConnection` fields with fallbacks | unit (fixture rows) | `cargo nextest run -E 'test(identity_parse_)'` | ❌ Wave 0 |
| CONN-07 | Disconnect clears `active` and drops the client | unit (signal) + manual | `cargo nextest run -E 'test(disconnect_releases)'` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo nextest run -p nodedb-studio`
- **Per wave merge:** full gate (`fmt --check && clippy -D warnings && nextest run`)
- **Phase gate:** full suite green + a manual `cargo run -p nodedb-studio` connecting to a live :6433 (or stub), exercising: success → shell with real caps/version/identity; bad password → `Auth` error, no Retry, stays disconnected; refused host → `Connection` error WITH Retry; ⌘D → back to manager.

### Wave 0 Gaps
- [ ] `services/nodedb_service.rs` tests — client build per auth mode (CONN-01/02), connect-error mapping (CONN-03)
- [ ] capability-mapping tests — `Capabilities::from_raw` → studio `Capabilities` per bit (CONN-05)
- [ ] identity-parse tests — fixture `QueryResult` rows → `ActiveConnection`, with fallbacks (CONN-06)
- [ ] `state/connections_registry.rs` reshape + tests — D-09 `SavedConnection` shape (remove/replace `ConnectionProfile`)
- [ ] `modals/new_connection.rs` rework — port `6433`, auth-mode dropdown, field-swap (D-02)
- [ ] Live/stub server for e2e (CONN-02/04/06) — stand up :6433 or a mock TCP handshake server
- [ ] (optional) `dioxus-ssr` for render-level card-state assertions — new dep, ask-first

---

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **No `.unwrap()`/`.expect()`/`panic!`/`todo!()` in non-test code.** Connect/probe propagate via `?` into `StudioError`. Identity-parse must use safe accessors (`Value::as_str()` etc.), never `unwrap`.
- **No `Result<T, String>`** — every fallible path returns `Result<_, StudioError>`.
- **`mod.rs` = only `pub mod`/`pub use`.** New connect/auth helper modules add `pub mod` lines; no logic in `mod.rs`.
- **Files < 500 LOC** — `nodedb_service.rs` will grow (build + probe + derive + identity-parse); split helpers (e.g. `auth.rs`, `capabilities_map.rs`, `identity.rs`) under `services/` if it nears the limit.
- **`sonic_rs` not `serde_json`** — not needed this phase, but never import `serde_json`. (`nodedb-client native` pulls `serde_json` internally — that's the dependency's concern, not studio code.)
- **`nodedb_types::Value`** for DB values — identity-probe rows are `Vec<Value>`; parse via `Value` accessors.
- **No `_ =>` on the studio's own exhaustive enums** — the capability/auth-mode/`ActiveConnection` derivations match all variants explicitly. The only allowed `_` is the existing foreign-`ErrorDetails` arm (Phase 1).
- **Dioxus 0.7:** connect is one-shot → `spawn`/`use_action` (NOT `use_resource`); never hold a signal guard across `.await`; `e.prevent_default()` on the form's `onsubmit`; stable keys on the connection list.
- **Seam discipline:** all async (build/probe/connect) lands at the `ConnectionService` seam; `MockConnectionService` keeps working alongside the real impl.
- **Ask-first:** no new deps and no `ConnectionService` *shape* change are required (the trait already has `connect()`); `dioxus-ssr` (optional render tests) would be ask-first. No `../nodedb` edits.
- **CI gate:** `cargo fmt --all` + `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo nextest run`.
- **Secret hygiene:** the in-session secret must never reach `SavedConnection`, disk, or `tracing`/`Debug` output (D-01).

---

## Sources

### Primary (HIGH confidence — read on disk 2026-06-14)
- `../nodedb/nodedb-types/src/protocol/handshake.rs` — `CAP_*` constants + bit values, `DEFAULT_NATIVE_PORT=6433`, `Limits`, `HelloAckFrame` (capabilities/server_version/limits)
- `../nodedb/nodedb-types/src/protocol/mod.rs` — `CAP_*` re-exports, `cap_bits_non_overlapping` test
- `../nodedb/nodedb-types/src/protocol/auth.rs` — `AuthMethod { Trust, Password, ApiKey, OidcBearer{token,provider} }` (`#[non_exhaustive]`), `AuthResponse { username, tenant_id }`
- `../nodedb/nodedb-types/src/protocol/frames.rs` — `NativeResponse.auth: Option<AuthResponse>`, `auth_ok()` (server populates identity)
- `../nodedb/nodedb-client/src/capabilities.rs` — `Capabilities::from_raw`/`supports_*`/`has`
- `../nodedb/nodedb-client/src/native/builder.rs` — `ConnectionBuilder` API; **no OIDC setter**; `build()` precedence api_key→password→trust; sync/infallible
- `../nodedb/nodedb-client/src/native/pool.rs` — `PoolConfig` (pub), `NegotiatedMeta` (no identity), lazy `acquire()` (connect+handshake+auth on first use), default `connect_timeout=5s`/`max_size=10`
- `../nodedb/nodedb-client/src/native/client/core.rs` — `NativeClient::new(PoolConfig)` (pub), `connect`, `ping`, `query`
- `../nodedb/nodedb-client/src/native/client/dispatch.rs` — `capabilities()`/`server_version()`/`limits()` read from `negotiated_meta()` (→ defaults when `None`)
- `../nodedb/nodedb-client/src/native/connection/mod.rs` — handshake/auth path; `authenticate()` **discards `AuthResponse`**; error ctors (`sync_connection_failed`/`handshake_failed`/`authorization_denied`)
- `../nodedb/nodedb-client/src/native/connection/tls.rs` — `TlsConfig` shape
- `../nodedb/nodedb-client/src/native/{mod.rs,client/mod.rs}` — `pub mod pool/client`, `pub use NativeClient`
- `../nodedb/nodedb-client/src/lib.rs` — `pub use Capabilities/NativeClient/ConnectionBuilder/NodeDbError`, `native` feature gate
- `../nodedb/nodedb-client/src/traits/core/trait_def.rs` — `NodeDb` connection-metadata accessors (only proto/caps/version/limits); `execute_sql` signature
- `../nodedb/nodedb-types/src/error/types.rs` — `NodeDbError` accessors, `is_retriable()` set (no `AuthorizationDenied`/`HandshakeFailed`), `is_cluster()`
- `../nodedb/nodedb-sql/src/resolver/expr/convert.rs` — `current_user`/`current_role`/`current_database`/`session_user` recognized
- `../nodedb/nodedb-sql/src/ddl_ast/parse/database/dispatch.rs` — `SHOW DATABASES` parsed/dispatched
- nodedb-studio: `services/error.rs`, `services/nodedb_service.rs`, `state/connection.rs`, `state/connections_registry.rs`, `views/connection_manager.rs`, `modals/new_connection.rs`, `components/topbar.rs`, `app.rs`

### Secondary (MEDIUM confidence)
- Exact `SHOW DATABASES` / `SELECT current_*` column shapes — inferred from parser/dispatch source; not executed against a live server (no :6433 confirmed in env). Treat identity-parse column mapping as needing a live-server Wave-0 check.

### Tertiary (LOW confidence)
- None — all load-bearing claims verified against source.

---

## Metadata

**Confidence breakdown:**
- Capability mapping (Target 1, 4): HIGH — `CAP_*` constants + `Capabilities` predicates read directly; lazy-pool caveat verified in `pool.rs`/`dispatch.rs`.
- Identity (Target 2, 3): HIGH on "not available from client" (verified `authenticate()` discards `AuthResponse`, no accessors); MEDIUM on exact SQL column shapes.
- OIDC wiring (Target 5): HIGH — builder API + public `PoolConfig`/`NativeClient::new` confirmed.
- Error mapping (Target 6): HIGH — error ctors + `is_retriable()` set read directly; reuses Phase-1 mapping verbatim.
- Address/TLS (Target 7): HIGH — builder + `TlsConfig` + `DEFAULT_NATIVE_PORT` confirmed.

**Research date:** 2026-06-14
**Valid until:** 2026-07-14 (stable — pinned 0.3.0 local workspace + Dioxus 0.7; re-verify if `../nodedb` bumps, especially if `authenticate()` starts surfacing `AuthResponse` or a `readonly`/identity accessor is added)
