# Project State: NodeDB Studio

**Last updated:** 2026-06-13
**Milestone:** Seam-to-Real Wiring

---

## Project Reference

**Core value:** A user can connect to a real NodeDB instance, run SQL, and browse/inspect their actual data — the studio shows live database state, not mock data.

**Current focus:** Phase 1 — Async Seam & Error Foundation

---

## Current Position

| Field | Value |
|-------|-------|
| Current phase | 1 — Async Seam & Error Foundation |
| Current plan | None yet (not started) |
| Phase status | Not started |
| Milestone status | Not started |

**Progress bar:**
```
[Phase 1] [Phase 2] [Phase 3] [Phase 4] [Phase 5] [Phase 6]
[  ---  ] [  ---  ] [  ---  ] [  ---  ] [  ---  ] [  ---  ]
```

---

## Phase Summary

| # | Phase | Status |
|---|-------|--------|
| 1 | Async Seam & Error Foundation | Not started |
| 2 | Connect, Auth & Capabilities | Not started |
| 3 | SQL Query Editor | Not started |
| 4 | Collection Management | Not started |
| 5 | Document & KV Browser | Not started |
| 6 | Vector, Graph & FTS Browser | Not started |

---

## Performance Metrics

| Metric | Value |
|--------|-------|
| Phases total | 6 |
| Phases complete | 0 |
| Plans complete | 0 |
| Requirements mapped | 29/29 |
| CI status | Unknown (pre-implementation) |

---

## Accumulated Context

### Key Decisions

| Decision | Rationale |
|----------|-----------|
| SEAM before CONN | Cannot wire real auth/connect without an async-capable trait and typed error surface |
| QURY before COLL | SQL editor is the thinnest vertical slice that proves the seam pattern end-to-end (single method `execute_sql`, no collection lifecycle complexity) |
| COLL before BROW | Data browser needs a real collection list to navigate; COLL-01 is a hard dependency for every browser view |
| BROW-08 assigned to Phase 5 | The loading/empty/error pattern is established when first real browser views land; Phase 6 inherits the pattern for the remaining engines |
| Document + KV in Phase 5, Vector + Graph + FTS in Phase 6 | Documents and KV are the simplest CRUD engines; splitting keeps each phase under scope control and delivers value incrementally |
| `MockConnectionService` kept throughout | Offline/dev/test capability; it satisfies the same async trait after Phase 1 |

### Constraints to Remember

- Async lands AT the seam only (`use_resource`/`use_action`); never in views
- Never hold a signal `.read()`/`.write()` guard across `.await`
- `sonic_rs` always, never `serde_json`
- `nodedb_types::Value` for all DB values
- No `unwrap`/`expect`/`panic` in non-test code — `thiserror` + `?`
- No `_ =>` on exhaustive domain enums
- Files < 500 LOC; `mod.rs` is re-exports only
- CI gate: `cargo fmt --all` + `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo nextest run`
- `nodedb-client`/`nodedb-types` 0.3.0 via `.cargo/config.toml` `[patch.crates-io]` (gitignored)

### Blockers

None at this time. Phase 1 is unblocked.

### Todos

- [ ] Verify `.cargo/config.toml` patch paths resolve before starting Phase 1
- [ ] Confirm `nodedb-client`/`nodedb-types` local workspace is at 0.3.0
- [ ] Run CI gate (`fmt + clippy + nextest`) on baseline before any changes

---

## Session Continuity

**To resume:** Start at Phase 1, Plan 1. Run `/gsd:plan-phase 1` to generate the execution plan.

**Baseline state:** Full UI skeleton on mock data, `MockConnectionService` sync only. No real async, no real client, no error surfaces in views.

**What "done" looks like for this milestone:** All 29 v1 requirements shipped; CI passes; app connects to a real NodeDB instance at `:6433`; SQL, collection management, and all five data browser engine types (document, KV, vector, graph, FTS) show live data.

---

*State initialized: 2026-06-13*
