# NodeDB Studio

## What This Is

NodeDB Studio is a **desktop GUI client for NodeDB** — the query editor, data browser, and administration surface for NodeDB's multi-model engines (document, vector, graph, full-text, KV, plus timeseries/spatial/streams/cluster). Built in Rust with Dioxus 0.7 (desktop). The complete UI shell already exists and renders against hardcoded mock data behind a single `ConnectionService` seam; this milestone makes it real by wiring that seam to the live `nodedb-client`.

## Core Value

A user can connect to a real NodeDB instance, run SQL, and browse/inspect their actual data — the studio shows live database state, not mock data.

## Requirements

### Validated

<!-- Inferred from the existing skeleton on origin/main (Phases 1-6). These ship and are relied upon. -->

- ✓ Full desktop UI shell: rail, topbar, statusbar, command palette, keyboard nav, popovers, modal host — existing
- ✓ Connected/disconnected root state machine (`app.rs`): disconnected → `ConnectionManager`, connected → `Studio` — existing
- ✓ `ConnectionService` trait as the single backend seam, provided as `Rc<dyn ConnectionService>` via context — existing (mock impl only)
- ✓ Per-connection identity + capability-driven shell: `Capabilities` flags gate rail items, admin sub-tabs, and views — existing
- ✓ Screens for every engine/area rendering on mock data: explorer (document/vector/graph/fts/kv/spatial/timeseries/strict viewers), query console, graph explorer, streams (CDC/cron/MV/notify/topics), admin (cluster/raft/rbac/rls/shards/audit/nodes), connection manager, sync, designer — existing
- ✓ Saved-connection registry + new-connection / preferences modals — existing
- ✓ Conventions/CI established (AGENTS.md): no unwrap/panic, thiserror, sonic_rs, Dioxus 0.7 signal rules, fmt + clippy -D warnings + nextest gate — existing

### Active

<!-- This milestone: wire the seam to the real nodedb-client (core data path). Hypotheses until shipped. -->

- [ ] Evolve `ConnectionService` into a single **async** trait that grows per capability (connect/auth, query, collections, per-engine reads), satisfied by both the mock and a real impl
- [ ] Implement `NodeDbConnectionService` backed by `nodedb-client`'s `NativeClient` / `NodeDb` trait over MessagePack (:6433)
- [ ] Real connect/auth from the connection manager: trust / password / API key / OIDC; surface connect errors
- [ ] Negotiate `Capabilities` from the server's actual `capabilities()` / `limits()` after connect (drive the shell from real flags, not mock)
- [ ] SQL query editor wired to `execute_sql`: run query, render real `QueryResult` (columns/rows of `Value`) in the results grid, show errors, keep history
- [ ] Collection management against the real client: list / create / drop / undrop / purge, inspect engine type + `GraphStats`, list dropped collections
- [ ] Data browser on real data: documents (`document_get/put/delete`), vectors (`vector_search/insert/delete`), graph (`graph_traverse/stats`), full-text (`text_search`), KV
- [ ] Async introduced **at the seam** (`use_resource` / `use_action`), with loading / empty / error states in the wired views; never block the main thread
- [ ] Replace mock data in the wired views; keep `MockConnectionService` as a fallback/dev/test impl

### Out of Scope

<!-- Deferred to v2 / later milestones. Explicit to prevent scope creep. -->

- Web (WASM) target — codebase is desktop-only (`dioxus` `desktop`+`router`); WASM can't open raw TCP to :6433, so it needs the HTTP transport. Defer until core path is proven.
- pgwire (:6432) and HTTP/REST (:6480) transports — native client is the chosen path for v1.
- Wiring streams (CDC/cron/MV/notify/topics) to live data — v2.
- Wiring admin/cluster (cluster/raft/rbac/rls/shards/audit/nodes) to live data — v2; some depend on server features not yet on the client trait.
- Sync dashboard (CRDT/peer replication) and designer to live data — v2.
- Spatial + timeseries dashboards to live data — v2 (driven via SQL once the SQL path is solid).
- Bitemporal reads/writes UI (`*_as_of`, valid-time) — v2.
- Graph visualization layout/rendering beyond what the existing view provides — v2.
- Credential storage hardening (OS keychain) — start with in-session/registry handling; harden later.

## Context

- **Brownfield.** Origin/main already contains the full UI skeleton (origin Phases 1-6, ~14k LOC, 85 files) on hardcoded mock data. The codebase map lives in `.planning/codebase/`. This milestone is **backend wiring**, not UI construction.
- **The seam is the whole game.** `services/connection_service.rs` defines `ConnectionService` (today: `list_connections`, `notifications`, `connect` — all sync). `app.rs:30` provides `Rc<dyn ConnectionService>` via context; views read context signals seeded from the service. The trait is far narrower than the views need and must grow + go async. AGENTS.md flags changing this seam as an **ask-first** boundary (approved for this milestone).
- **NodeDB client surface** (mapped from `../nodedb`): `NativeClient` via `ConnectionBuilder("host:6433").username().password()/.api_key().database().tls().build()`; unified `NodeDb` async trait with `execute_sql`, `document_*`, `vector_*`, `graph_*`, `text_search`, collection lifecycle (`undrop_collection`, `drop_collection_purge`, `list_dropped_collections`), `capabilities()`/`limits()`/`server_version()`. Types in `nodedb-types`: `Value`, `Document`, `QueryResult`, `SearchResult`, `SubGraph`, `NodeId`/`EdgeId`, `MetadataFilter`, `Capabilities`, `NodeDbError`/`NodeDbResult`.
- **Local dependency.** `nodedb-client`/`nodedb-types` are pinned to `0.3.0` but unpublished; require a gitignored `.cargo/config.toml` `[patch.crates-io]` pointing at `../nodedb`. Studio Cargo version must match the local NodeDB workspace version.
- **Known issue.** App is 100% mock today; no real error/loading surfaces exist yet in views; only ~2 tests (in `data/mock.rs`).

## Constraints

- **Tech stack**: Rust edition 2024, MSRV 1.96, Dioxus 0.7 (`desktop` + `router`). No new deps or version bumps without asking (AGENTS.md).
- **Transport**: native `nodedb-client` over MessagePack (:6433). Desktop-only — raw TCP, so no browser target this milestone.
- **Conventions** (AGENTS.md, hard rules): no `.unwrap()`/`.expect()`/`panic!` in non-test code — typed `thiserror` errors, propagate with `?`, never `Result<T, String>`; `mod.rs` is re-exports only; files < 500 LOC; `sonic_rs` (never `serde_json`); `nodedb_types::Value` for DB values; no `_ =>` on exhaustive domain enums.
- **Dioxus 0.7**: never hold a `.read()`/`.write()` guard across `.await`; `.peek()` in handlers; stable list keys; never block the main thread (async IO via `spawn`/`use_resource`/`use_action`); keep logic in plain Rust for testability.
- **Seam discipline**: async lands at the `ConnectionService` boundary, not scattered through views. Mock impl must keep working alongside the real one.
- **CI gate**: `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo nextest run` must pass.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Desktop-only + native `nodedb-client` (:6433) for v1 | Codebase already chose it; native client exposes the full `NodeDb` trait with least glue; WASM can't do raw TCP | — Pending |
| v1 = core data path (connect/auth + capabilities, SQL editor, collection mgmt, data browser); defer streams/admin/sync to v2 | Gets the daily-driver working end-to-end; some v2 areas aren't on the client trait yet | — Pending |
| Keep a single `ConnectionService` trait, make it async, grow it per capability | Matches the existing one-seam context-provider pattern in `app.rs`; mock + real both satisfy it; minimal churn to consumers | — Pending |
| Keep `MockConnectionService` as a parallel impl | Enables tests + offline dev; the seam is explicitly designed for swappable impls | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-06-13 after initialization (replanned against origin/main skeleton)*
