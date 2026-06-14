# Requirements: NodeDB Studio

**Defined:** 2026-06-13
**Core Value:** A user can connect to a real NodeDB instance, run SQL, and browse/inspect their actual data — the studio shows live database state, not mock data.

## v1 Requirements

Wire the existing UI skeleton's `ConnectionService` seam to the live `nodedb-client` (native, :6433), covering the core data path. UI already exists; these requirements are about real data behind it.

### Seam (async backend boundary)

- [x] **SEAM-01**: `ConnectionService` is an async trait (via `async_trait`) whose methods cover the core data path; the existing `MockConnectionService` still satisfies it for offline/dev/test
- [x] **SEAM-02**: A `NodeDbConnectionService` implements the trait by wrapping `nodedb-client`'s `NativeClient` / `NodeDb` trait, and is the impl provided in `app.rs` when a real connection is opened
- [x] **SEAM-03**: All client errors surface as the studio's typed `thiserror` error (mapped from `NodeDbError`), never `unwrap`/`panic`/`Result<T, String>`
- [x] **SEAM-04**: Async work runs at the seam via `use_resource`/`use_action` (never blocking the main thread, never holding a signal guard across `.await`); wired views render loading, empty, and error states

### Connection & auth

- [ ] **CONN-01**: User can open a real session from the connection manager using a saved connection's host:port and credentials
- [ ] **CONN-02**: User can connect with each auth mode the client supports — trust, password, API key, and OIDC bearer
- [ ] **CONN-03**: A failed connection (bad host, refused, bad credentials) shows a clear error in the connection manager and does not enter the connected state
- [ ] **CONN-04**: On successful connect, the studio enters the connected `Studio` shell bound to that live session
- [x] **CONN-05**: The active connection's `Capabilities` are derived from the server's real `capabilities()`/`limits()`, so the rail items, admin sub-tabs, and views gate on actual server capabilities
- [x] **CONN-06**: The connection chip / identity (user, role, current database, databases list, server version) reflects the real session, not mock values
- [ ] **CONN-07**: User can disconnect and return to the connection manager, releasing the client session

### SQL query editor

- [ ] **QURY-01**: User can type SQL in the query editor and execute it against the connected database via the client's `execute_sql`
- [ ] **QURY-02**: Results render in the grid from the real `QueryResult` (column names + typed `Value` rows); `rows_affected` is shown for writes
- [ ] **QURY-03**: A query error (syntax, runtime) is shown inline with the server's message, without crashing the view
- [ ] **QURY-04**: Query history records executed statements for the session and lets the user re-run a previous query
- [ ] **QURY-05**: Query execution is non-blocking — the editor stays responsive and shows a running/loading indicator while the query is in flight

### Collection management

- [ ] **COLL-01**: User can see the real list of collections for the current database, each with its engine type
- [ ] **COLL-02**: User can create a collection (with engine type) against the live client
- [ ] **COLL-03**: User can drop a collection (soft delete) and purge a collection (hard delete) via the client
- [ ] **COLL-04**: User can view soft-deleted collections (`list_dropped_collections`) and undrop one within the retention window
- [ ] **COLL-05**: User can inspect a collection's stats — including `GraphStats` (node/edge/label counts) for graph collections

### Data browser (per engine)

- [ ] **BROW-01**: User can browse documents in a document collection and view a selected document's fields (`document_get`)
- [ ] **BROW-02**: User can create/update and delete a document (`document_put` / `document_delete`)
- [ ] **BROW-03**: User can run a vector similarity search in a vector collection (`vector_search`, top-k) and see real `SearchResult`s (id, distance, metadata)
- [ ] **BROW-04**: User can insert and delete a vector with optional metadata (`vector_insert` / `vector_delete`)
- [ ] **BROW-05**: User can traverse a graph from a start node (`graph_traverse`, depth) and view the returned `SubGraph` (nodes + edges)
- [ ] **BROW-06**: User can run a full-text search over a field (`text_search`, BM25) and see ranked real results
- [ ] **BROW-07**: User can browse KV entries from the live database
- [ ] **BROW-08**: Each browser view distinguishes loading, empty, and error states on real data, and the viewer shown matches the collection's engine type

## v2 Requirements

Deferred. Acknowledged but not in this roadmap.

### Streams

- **STRM-01**: Wire CDC change streams to live data
- **STRM-02**: Wire NOTIFY/LISTEN, durable topics, materialized views, cron scheduler to live data

### Administration & cluster

- **ADMN-01**: Wire cluster / raft / nodes / shards status to live data
- **ADMN-02**: Wire RBAC / RLS / audit views to live data

### Sync & advanced engines

- **SYNC-01**: Wire CRDT sync dashboard (peer replication, deltas, conflicts) to live data
- **ADV-01**: Wire spatial + timeseries dashboards to live data (via SQL)
- **ADV-02**: Bitemporal reads/writes UI (`*_as_of`, valid-time)
- **ADV-03**: Designer wired to real schema

## Out of Scope

| Feature | Reason |
|---------|--------|
| Web (WASM) target | Codebase is desktop-only; WASM can't open raw TCP to :6433. Needs HTTP transport — defer until core path proven |
| pgwire (:6432) / HTTP/REST (:6480) transports | Native client chosen for v1; full `NodeDb` trait with least glue |
| OS keychain credential storage | Start with in-session/registry handling; harden after core path works |
| Graph visualization layout engine | Beyond existing view; rendering/layout is its own effort |
| New UI screens or redesign | UI skeleton is complete and approved; this milestone is backend wiring only |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SEAM-01 | Phase 1 — Async Seam & Error Foundation | Complete |
| SEAM-02 | Phase 1 — Async Seam & Error Foundation | Complete |
| SEAM-03 | Phase 1 — Async Seam & Error Foundation | Complete |
| SEAM-04 | Phase 1 — Async Seam & Error Foundation | Complete |
| CONN-01 | Phase 2 — Connect, Auth & Capabilities | Pending |
| CONN-02 | Phase 2 — Connect, Auth & Capabilities | Pending |
| CONN-03 | Phase 2 — Connect, Auth & Capabilities | Pending |
| CONN-04 | Phase 2 — Connect, Auth & Capabilities | Pending |
| CONN-05 | Phase 2 — Connect, Auth & Capabilities | Complete |
| CONN-06 | Phase 2 — Connect, Auth & Capabilities | Complete |
| CONN-07 | Phase 2 — Connect, Auth & Capabilities | Pending |
| QURY-01 | Phase 3 — SQL Query Editor | Pending |
| QURY-02 | Phase 3 — SQL Query Editor | Pending |
| QURY-03 | Phase 3 — SQL Query Editor | Pending |
| QURY-04 | Phase 3 — SQL Query Editor | Pending |
| QURY-05 | Phase 3 — SQL Query Editor | Pending |
| COLL-01 | Phase 4 — Collection Management | Pending |
| COLL-02 | Phase 4 — Collection Management | Pending |
| COLL-03 | Phase 4 — Collection Management | Pending |
| COLL-04 | Phase 4 — Collection Management | Pending |
| COLL-05 | Phase 4 — Collection Management | Pending |
| BROW-01 | Phase 5 — Document & KV Browser | Pending |
| BROW-02 | Phase 5 — Document & KV Browser | Pending |
| BROW-07 | Phase 5 — Document & KV Browser | Pending |
| BROW-08 | Phase 5 — Document & KV Browser | Pending |
| BROW-03 | Phase 6 — Vector, Graph & FTS Browser | Pending |
| BROW-04 | Phase 6 — Vector, Graph & FTS Browser | Pending |
| BROW-05 | Phase 6 — Vector, Graph & FTS Browser | Pending |
| BROW-06 | Phase 6 — Vector, Graph & FTS Browser | Pending |

**Coverage:**
- v1 requirements: 29 total
- Mapped to phases: 29
- Unmapped: 0

---
*Requirements defined: 2026-06-13*
*Last updated: 2026-06-13 — traceability table populated after roadmap creation*
