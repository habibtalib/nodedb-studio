---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: unknown
stopped_at: Completed 01-03-PLAN.md
last_updated: "2026-06-14T01:11:49.353Z"
progress:
  total_phases: 6
  completed_phases: 0
  total_plans: 4
  completed_plans: 3
---

# Project State: NodeDB Studio

**Last updated:** 2026-06-14
**Milestone:** Seam-to-Real Wiring

---

## Project Reference

**Core value:** A user can connect to a real NodeDB instance, run SQL, and browse/inspect their actual data — the studio shows live database state, not mock data.

**Current focus:** Phase 01 — async-seam-error-foundation

---

## Current Position

Phase: 01 (async-seam-error-foundation) — EXECUTING
Plan: 4 of 4
| Field | Value |
|-------|-------|
| Current phase | 1 — Async Seam & Error Foundation |
| Current plan | 3 of 4 (Plans 1-2 complete) |
| Phase status | In Progress |
| Milestone status | In Progress |

**Progress bar:**

```
[Phase 1] [Phase 2] [Phase 3] [Phase 4] [Phase 5] [Phase 6]
[ ▓▓▓░░ ] [  ---  ] [  ---  ] [  ---  ] [  ---  ] [  ---  ]
Phase 1: [█████░░░░░] 50% (2/4 plans)
```

---

## Phase Summary

| # | Phase | Status |
|---|-------|--------|
| 1 | Async Seam & Error Foundation | In Progress (2/4 plans) |
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
| Plans complete | 2 |
| Requirements mapped | 29/29 |
| CI status | Green (build + clippy -D warnings + nextest 20/20 pass after 01-02) |

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P01 | 6 min | 2 tasks | 6 files |
| Phase 01 P02 | 8 min | 2 tasks | 8 files |

---
| Phase 01 P02 | 8min | 2 tasks | 8 files |
| Phase 01 P03 | 4 min | 2 tasks | 4 files |

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
| Async trait shape locked (01-02) | `ConnectionService` is `#[async_trait(?Send)]`; all 3 methods return `Result<_, StudioError>`; `connect` returns `Result<ActiveConnection, StudioError>` (unknown/offline mock name → `NotConnected`); forward-compatible with CONN-03 |
| `NodeDbConnectionService` stub wraps `Option<NativeClient>` (01-02) | None this phase, returns `NotConnected` from every method (never panic/todo!); instantiated object-safe in app.rs; Phase 2 fills it via `ConnectionBuilder` |
| `AsyncState<T>` + `from_value` primitive (01-03) | Plain-Rust loading/empty/error mapping (None→Loading, Some(Err)→Error, Some(Ok(empty))→Empty, Some(Ok(data))→Loaded), unit-tested without a renderer; `IsEmpty` distinguishes empty vs data; no `_ =>` on the studio's own enum |
| `AsyncView` takes discrete props, not the generic enum (01-03) | Dioxus props need `Clone + PartialEq`; `StudioError`/`T` don't qualify, so the caller decodes `AsyncState` and passes `loading`/`empty`/`error`/`retriable` flags; AsyncView renders the three non-loaded states (Retry gated on `retriable`) and yields to caller markup for Loaded; scoped `#[allow(dead_code)]` until 01-04 consumes it |

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

**To resume:** Phase 1, Plan 3 (`01-03-PLAN.md`). Plans 1-2 complete — `StudioError`, the async `#[async_trait(?Send)]` `ConnectionService` seam, `MockConnectionService` async impl, and the `NodeDbConnectionService` stub are all in place. SEAM-01/02/03 done; the async-state loading/empty/error pattern (and 01-04's Retry affordance) are next.

**Stopped at:** Completed 01-03-PLAN.md

**Baseline state:** Full UI skeleton on mock data, `MockConnectionService` sync only. No real async, no real client, no error surfaces in views.

**What "done" looks like for this milestone:** All 29 v1 requirements shipped; CI passes; app connects to a real NodeDB instance at `:6433`; SQL, collection management, and all five data browser engine types (document, KV, vector, graph, FTS) show live data.

---

*State initialized: 2026-06-13*
