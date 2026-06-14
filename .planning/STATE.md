---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: unknown
stopped_at: Phase 2 context gathered
last_updated: "2026-06-14T12:37:44.377Z"
progress:
  total_phases: 6
  completed_phases: 1
  total_plans: 4
  completed_plans: 4
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

Phase: 2
Plan: Not started
| Field | Value |
|-------|-------|
| Current phase | 1 — Async Seam & Error Foundation |
| Current plan | 4 of 4 (all plans complete) |
| Phase status | Ready for verification (4/4 plans) |
| Milestone status | In Progress |

**Progress bar:**

```
[Phase 1] [Phase 2] [Phase 3] [Phase 4] [Phase 5] [Phase 6]
[ ▓▓▓▓▓ ] [  ---  ] [  ---  ] [  ---  ] [  ---  ] [  ---  ]
Phase 1: [██████████] 100% (4/4 plans)
```

---

## Phase Summary

| # | Phase | Status |
|---|-------|--------|
| 1 | Async Seam & Error Foundation | Ready for verification (4/4 plans) |
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
| Phases complete | 0 (Phase 1 ready for verification) |
| Plans complete | 4 |
| Requirements mapped | 29/29 |
| CI status | Green (build + clippy -D warnings + nextest 27/27 + fmt --check pass after 01-04) |

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P01 | 6 min | 2 tasks | 6 files |
| Phase 01 P02 | 8 min | 2 tasks | 8 files |
| Phase 01 P03 | 4 min | 2 tasks | 4 files |
| Phase 01 P04 | 12 min | 2 tasks | 5 files |

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
| Async trait shape locked (01-02) | `ConnectionService` is `#[async_trait(?Send)]`; all 3 methods return `Result<_, StudioError>`; `connect` returns `Result<ActiveConnection, StudioError>` (unknown/offline mock name → `NotConnected`); forward-compatible with CONN-03 |
| `NodeDbConnectionService` stub wraps `Option<NativeClient>` (01-02) | None this phase, returns `NotConnected` from every method (never panic/todo!); instantiated object-safe in app.rs; Phase 2 fills it via `ConnectionBuilder` |
| `AsyncState<T>` + `from_value` primitive (01-03) | Plain-Rust loading/empty/error mapping (None→Loading, Some(Err)→Error, Some(Ok(empty))→Empty, Some(Ok(data))→Loaded), unit-tested without a renderer; `IsEmpty` distinguishes empty vs data; no `_ =>` on the studio's own enum |
| `AsyncView` takes discrete props, not the generic enum (01-03) | Dioxus props need `Clone + PartialEq`; `StudioError`/`T` don't qualify, so the caller decodes `AsyncState` and passes `loading`/`empty`/`error`/`retriable` flags; AsyncView renders the three non-loaded states (Retry gated on `retriable`) and yields to caller markup for Loaded; scoped `#[allow(dead_code)]` until 01-04 consumes it |
| SEAM-04 render-path proof in the notifications popover (01-04) | Popover self-fetches via `use_resource(service.notifications().await)` (Rc cloned before the async block, no guard across `.await`); decodes the read inline into the four `AsyncState` states (approach A — mirrors `from_value` because `StudioError` is not `Clone`); Loading/Empty/Error via `AsyncView`, Loaded via the grouped list; Retry gated on `is_retriable()` → `Resource::restart()`; topbar badge + app.rs seeding untouched. Removed dead_code allows on `is_retriable()`/`AsyncView` (now consumed); kept the allow on `AsyncState`/`from_value` (test-only call site this phase) |

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

**To resume:** Phase 1 is COMPLETE (4/4 plans) and ready for the phase verifier. `StudioError`, the async `#[async_trait(?Send)]` `ConnectionService` seam, the `MockConnectionService` async impl + `NodeDbConnectionService` stub, the `AsyncState`/`AsyncView` loading/empty/error primitive, and the SEAM-04 render-path proof (notifications popover self-fetching via `use_resource`) are all in place. SEAM-01/02/03/04 done. Next: run the Phase 1 verifier, then transition to Phase 2 (Connect, Auth & Capabilities).

**Stopped at:** Phase 2 context gathered

**Baseline state:** Full UI skeleton on mock data behind the now-async `ConnectionService` seam; the notifications popover renders live via the seam (Loading/Empty/Loaded/Error + Retry). No real client wired yet (Phase 2); other views still seeded from mock.

**What "done" looks like for this milestone:** All 29 v1 requirements shipped; CI passes; app connects to a real NodeDB instance at `:6433`; SQL, collection management, and all five data browser engine types (document, KV, vector, graph, FTS) show live data.

---

*State initialized: 2026-06-13*
