---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: in_progress
last_updated: "2026-06-13T18:08:09.226Z"
progress:
  total_phases: 6
  completed_phases: 0
  total_plans: 4
  completed_plans: 1
---

# Project State: NodeDB Studio

**Last updated:** 2026-06-13
**Milestone:** Seam-to-Real Wiring

---

## Project Reference

**Core value:** A user can connect to a real NodeDB instance, run SQL, and browse/inspect their actual data — the studio shows live database state, not mock data.

**Current focus:** Phase 01 — async-seam-error-foundation

---

## Current Position

Phase: 01 (async-seam-error-foundation) — EXECUTING
Plan: 2 of 4
| Field | Value |
|-------|-------|
| Current phase | 1 — Async Seam & Error Foundation |
| Current plan | 2 of 4 (Plan 1 complete) |
| Phase status | In Progress |
| Milestone status | In Progress |

**Progress bar:**

```
[Phase 1] [Phase 2] [Phase 3] [Phase 4] [Phase 5] [Phase 6]
[ ▓▓░░░ ] [  ---  ] [  ---  ] [  ---  ] [  ---  ] [  ---  ]
Phase 1: [███░░░░░░░] 25% (1/4 plans)
```

---

## Phase Summary

| # | Phase | Status |
|---|-------|--------|
| 1 | Async Seam & Error Foundation | In Progress (1/4 plans) |
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
| Plans complete | 1 |
| Requirements mapped | 29/29 |
| CI status | Green (fmt + clippy + nextest pass after 01-01) |

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P01 | 6 min | 2 tasks | 6 files |

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
| StudioError categorized + delegated `is_retriable()` (01-01) | 8 variants (Connection/Auth/NotFound/Conflict/ReadOnly/Setup/Server/NotConnected); `#[source]` preserves the `NodeDbError` cause chain; the only `_ =>` arm is the foreign `#[non_exhaustive]` `ErrorDetails` catch-all |
| Activated `.cargo/config.toml` patch (01-01) | Local `../nodedb` checkout is the build source for reproducibility; `native` feature still enabled via the studio dep |

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

- [x] Verify `.cargo/config.toml` patch paths resolve before starting Phase 1 (created + active; build green from local `../nodedb`)
- [x] Confirm `nodedb-client`/`nodedb-types` local workspace is at 0.3.0 (confirmed; native feature enabled)
- [x] Run CI gate (`fmt + clippy + nextest`) — clippy + nextest pass after 01-01

---

## Session Continuity

**To resume:** Phase 1, Plan 2 (`01-02-PLAN.md`). Plan 1 complete — `StudioError`, `native` feature, and `async-trait` are in place; the async `ConnectionService` seam + `NodeDbConnectionService` stub are next.

**Stopped at:** Completed 01-01-PLAN.md

**Baseline state:** Full UI skeleton on mock data, `MockConnectionService` sync only. No real async, no real client, no error surfaces in views.

**What "done" looks like for this milestone:** All 29 v1 requirements shipped; CI passes; app connects to a real NodeDB instance at `:6433`; SQL, collection management, and all five data browser engine types (document, KV, vector, graph, FTS) show live data.

---

*State initialized: 2026-06-13*
