# Roadmap: NodeDB Studio — Seam-to-Real Wiring Milestone

**Created:** 2026-06-13
**Granularity:** Standard (5-7 phases)
**Milestone goal:** Wire the existing Dioxus 0.7 UI shell's `ConnectionService` seam to the live `nodedb-client`, replacing mock data across the core data path (connect/auth, SQL editor, collection management, data browser per engine).

---

## Phases

- [ ] **Phase 1: Async Seam & Error Foundation** - Make `ConnectionService` async, define typed errors, keep MockConnectionService working alongside; all prerequisites for real I/O
- [ ] **Phase 2: Connect, Auth & Capabilities** - Wire the connection manager to the real `NativeClient`, support all auth modes, negotiate real `Capabilities` from the server, surface errors and identity
- [ ] **Phase 3: SQL Query Editor** - Execute real SQL via `execute_sql`, render `QueryResult` in the results grid, surface errors and history, non-blocking execution
- [ ] **Phase 4: Collection Management** - List/create/drop/undrop/purge real collections, inspect stats and engine types, list soft-deleted collections
- [ ] **Phase 5: Document & KV Browser** - Browse and mutate real documents and KV entries; establish loading/empty/error pattern for all engine viewers
- [ ] **Phase 6: Vector, Graph & FTS Browser** - Wire vector search/insert/delete, graph traversal, and full-text search; complete all browser engine views with consistent states

---

## Phase Details

### Phase 1: Async Seam & Error Foundation
**Goal**: The `ConnectionService` trait is async, all typed errors are defined, and both `MockConnectionService` and the stub `NodeDbConnectionService` compile and satisfy the trait
**Depends on**: Nothing (foundational phase)
**Requirements**: SEAM-01, SEAM-02, SEAM-03, SEAM-04
**Success Criteria** (what must be TRUE):
  1. `MockConnectionService` still satisfies the async `ConnectionService` trait and the app compiles, loads, and renders mock data identically to before
  2. A `NodeDbConnectionService` struct exists, implements the trait by wrapping `NativeClient`, and can be instantiated in `app.rs` (even if not yet the default)
  3. All errors from `nodedb-client`'s `NodeDbError` are mapped to a studio-owned `thiserror` error type — no `unwrap`, no `Result<T, String>` anywhere in the seam code
  4. Views that call the service render loading, empty, and error states (driven by `use_resource`/`use_action`); the main thread is never blocked on any service call
**Plans**: TBD

### Phase 2: Connect, Auth & Capabilities
**Goal**: Users can open a real session from the connection manager using any supported auth mode, the studio enters the connected shell bound to that live session, and all shell elements reflect real server capabilities and identity
**Depends on**: Phase 1
**Requirements**: CONN-01, CONN-02, CONN-03, CONN-04, CONN-05, CONN-06, CONN-07
**Success Criteria** (what must be TRUE):
  1. User selects a saved connection and clicks connect — a real TCP/MessagePack session to `:6433` is established and the studio transitions to the connected `Studio` shell
  2. Each auth mode (trust, password, API key, OIDC bearer) works end-to-end; the connection form correctly routes credentials to the matching `ConnectionBuilder` path
  3. A refused connection, bad credentials, or network timeout shows a clear error message inside the connection manager — the app never freezes and never enters the connected state
  4. The rail, admin sub-tabs, and capability-gated views (Graph, Vector, FTS, Streams, etc.) reflect the server's real `capabilities()`/`limits()` response, not hardcoded mock flags
  5. The connection chip (topbar) shows the real username, role, current database, databases list, and server version; disconnect (⌘D) returns to the connection manager and releases the client session
**Plans**: 3 plans
  - [x] 02-01-PLAN.md — Data foundation: SavedConnection reshape (D-09), capability-map + identity-parse tested helpers (Wave 1)
  - [x] 02-02-PLAN.md — Connect-and-probe seam: build client per auth mode, force handshake, derive caps/identity, disconnect (Wave 2)
  - [ ] 02-03-PLAN.md — UI wiring: form rework (D-02), inline connect card (D-03), shell transition, identity chip, quick-switch + live e2e checkpoint (Wave 3)
**UI hint**: yes

### Phase 3: SQL Query Editor
**Goal**: Users can execute arbitrary SQL against the connected database and see real results, errors, and execution history in the query editor
**Depends on**: Phase 2
**Requirements**: QURY-01, QURY-02, QURY-03, QURY-04, QURY-05
**Success Criteria** (what must be TRUE):
  1. User types a SELECT query and clicks Run — real rows from the live database appear in the results grid, with correct column names and typed `Value` cells
  2. A DML statement (INSERT/UPDATE/DELETE) shows `rows_affected` count in the results area, not an empty grid
  3. A malformed or failing query shows the server's error message inline in the query view without crashing or navigating away
  4. Executed queries accumulate in the session history list; clicking a history entry re-populates the editor
  5. While a query is in flight the editor remains interactive and a loading/running indicator is visible; the main thread is not blocked
**Plans**: TBD
**UI hint**: yes

### Phase 4: Collection Management
**Goal**: Users can see, create, and manage real collections for the connected database, including inspecting engine type and stats, and recovering soft-deleted collections
**Depends on**: Phase 2
**Requirements**: COLL-01, COLL-02, COLL-03, COLL-04, COLL-05
**Success Criteria** (what must be TRUE):
  1. The collection list (Explorer sidebar and any collection panel) shows the actual collections from the server, each labeled with its real engine type — not the 14 hardcoded mock entries
  2. User creates a collection via the UI (name + engine type) and it immediately appears in the refreshed list from the server
  3. User can soft-drop a collection and purge (hard-delete) it via distinct UI actions; after each action the list reflects the real server state
  4. The dropped-collections panel lists real soft-deleted collections via `list_dropped_collections`; user can undrop one and it returns to the active list
  5. Selecting a collection shows its real stats — including node count, edge count, and label counts for graph collections (`GraphStats`)
**Plans**: TBD
**UI hint**: yes

### Phase 5: Document & KV Browser
**Goal**: Users can browse, view, create, update, and delete real documents in document collections, and browse real KV entries; the engine viewer pattern for loading/empty/error states is established
**Depends on**: Phase 4
**Requirements**: BROW-01, BROW-02, BROW-07, BROW-08
**Success Criteria** (what must be TRUE):
  1. Selecting a document collection in the Explorer loads a real document list from the server; clicking a document shows its actual fields via `document_get`
  2. User can create/update a document (`document_put`) and the document list refreshes to show the change; user can delete a document (`document_delete`) and it disappears from the list
  3. Selecting a KV collection loads real key-value entries from the live database
  4. All wired engine viewers (document, KV) display a spinner while loading, a "no records" message when the collection is empty, and a clear error message when the data fetch fails — no view crashes or shows stale mock data
**Plans**: TBD
**UI hint**: yes

### Phase 6: Vector, Graph & FTS Browser
**Goal**: Users can run vector similarity searches, traverse graph relationships, and execute full-text queries against real data; all six engine browser views are fully wired with consistent state handling
**Depends on**: Phase 5
**Requirements**: BROW-03, BROW-04, BROW-05, BROW-06
**Success Criteria** (what must be TRUE):
  1. User enters a query vector and top-k in a vector collection — real `SearchResult`s (id, distance, metadata) are returned and displayed; user can insert a vector with optional metadata and delete one by id
  2. User enters a start node and depth in a graph collection — `graph_traverse` returns a real `SubGraph`; nodes and edges are rendered in the existing graph viewer
  3. User types a search query in an FTS collection — `text_search` returns BM25-ranked real results showing document id and score
  4. All engine viewers (document, KV, vector, graph, FTS) consistently show loading, empty, and error states; the viewer rendered in the Explorer always matches the selected collection's engine type
**Plans**: TBD
**UI hint**: yes

---

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Async Seam & Error Foundation | 4/4 | Ready for verification | 2026-06-14 |
| 2. Connect, Auth & Capabilities | 0/3 | Planned | - |
| 3. SQL Query Editor | 0/? | Not started | - |
| 4. Collection Management | 0/? | Not started | - |
| 5. Document & KV Browser | 0/? | Not started | - |
| 6. Vector, Graph & FTS Browser | 0/? | Not started | - |

---

## Coverage Map

| Requirement | Phase |
|-------------|-------|
| SEAM-01 | Phase 1 |
| SEAM-02 | Phase 1 |
| SEAM-03 | Phase 1 |
| SEAM-04 | Phase 1 |
| CONN-01 | Phase 2 |
| CONN-02 | Phase 2 |
| CONN-03 | Phase 2 |
| CONN-04 | Phase 2 |
| CONN-05 | Phase 2 |
| CONN-06 | Phase 2 |
| CONN-07 | Phase 2 |
| QURY-01 | Phase 3 |
| QURY-02 | Phase 3 |
| QURY-03 | Phase 3 |
| QURY-04 | Phase 3 |
| QURY-05 | Phase 3 |
| COLL-01 | Phase 4 |
| COLL-02 | Phase 4 |
| COLL-03 | Phase 4 |
| COLL-04 | Phase 4 |
| COLL-05 | Phase 4 |
| BROW-01 | Phase 5 |
| BROW-02 | Phase 5 |
| BROW-07 | Phase 5 |
| BROW-08 | Phase 5 |
| BROW-03 | Phase 6 |
| BROW-04 | Phase 6 |
| BROW-05 | Phase 6 |
| BROW-06 | Phase 6 |

**Mapped: 29/29**

---

*Roadmap created: 2026-06-13*
*Last updated: 2026-06-13 after initial creation*
