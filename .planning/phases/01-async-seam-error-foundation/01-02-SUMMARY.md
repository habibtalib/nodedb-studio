---
phase: 01-async-seam-error-foundation
plan: 02
subsystem: infra
tags: [rust, dioxus, async-trait, nodedb-client, use_resource, connection-service]

# Dependency graph
requires:
  - phase: 01-01
    provides: "StudioError (the seam's Result error), nodedb-client native feature (NativeClient importable), async-trait dependency"
provides:
  - "ConnectionService is now an #[async_trait(?Send)] trait; all 3 methods return Result<_, StudioError>"
  - "MockConnectionService async impl returning identical mock data wrapped in Ok (offline/dev/test path preserved)"
  - "NodeDbConnectionService stub wrapping Option<NativeClient>, returning StudioError::NotConnected for every method, instantiated (object-safe) in app.rs"
  - "All 3 connect() call sites (connection_manager, command_palette x2, connection_popover) migrated to spawn + .await with no signal guard held across await"
  - "app.rs seeds registry + notifications via use_resource at the async seam (mock = instant)"
affects: [01-03-asyncstate-pattern, 01-04, 02-connect, 02-auth, 02-capabilities, query, collections, browser]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Async seam: #[async_trait(?Send)] trait + Result<_, StudioError> on every method (single-threaded Dioxus runtime, no Send bound)"
    - "Inert real-client stub: wraps Option<NativeClient> = None, returns NotConnected (never panic/todo!) until Phase 2 fills it via ConnectionBuilder"
    - "Call-site async migration: clone the Rc before the async block, copy/.peek() signal values before awaiting, call .set() only AFTER the await — never hold a .read()/.write() guard across .await"
    - "app.rs registry/notifications seeded EMPTY then filled via use_resource after the async call resolves"

key-files:
  created:
    - nodedb-studio/src/services/nodedb_service.rs
  modified:
    - nodedb-studio/src/services/connection_service.rs
    - nodedb-studio/src/services/mod.rs
    - nodedb-studio/src/services/error.rs
    - nodedb-studio/src/app.rs
    - nodedb-studio/src/views/connection_manager.rs
    - nodedb-studio/src/components/command_palette.rs
    - nodedb-studio/src/components/popovers/connection_popover.rs

key-decisions:
  - "Async trait shape locked: all 3 methods return Result<_, StudioError>; connect returns Result<ActiveConnection, StudioError> (forward-compatible with CONN-03; unknown/offline mock name maps to StudioError::NotConnected)"
  - "NodeDbConnectionService wraps Option<NativeClient> = None this phase; never calls the client (Phase 2 / CONN-01..07 fills it via ConnectionBuilder)"
  - "Removed the enum-level #[allow(dead_code)] from StudioError now that the seam consumes it; kept the scoped #[allow(dead_code)] on is_retriable() since it is unused until plan 01-04 wires the Retry affordance"

patterns-established:
  - "Async ConnectionService seam: the canonical async boundary every later plan calls via use_resource/use_action/spawn"
  - "Guard-safe async handler: Rc cloned before await, .set() after await, no signal guard across .await"

requirements-completed: [SEAM-01, SEAM-02]

# Metrics
duration: 8min
completed: 2026-06-14
---

# Phase 1 Plan 2: Async Seam & NodeDbConnectionService Stub Summary

**ConnectionService converted to an `#[async_trait(?Send)]` trait returning `Result<_, StudioError>` — Mock async impl preserves identical mock data, an inert `NodeDbConnectionService` stub wraps `Option<NativeClient>`, and all 3 `connect()` call sites + app.rs seeding migrated to the guard-safe `spawn`/`use_resource` async pattern.**

## Performance

- **Duration:** ~8 min
- **Completed:** 2026-06-14
- **Tasks:** 2 (committed as one intermingled feature unit — the trait going async breaks all sync call sites atomically)
- **Files modified/created:** 8 (1 created, 7 modified)

## Accomplishments
- Converted `ConnectionService` to `#[async_trait(?Send)]` with all 3 methods (`list_connections`, `notifications`, `connect`) returning `Result<_, StudioError>` — SEAM-01.
- `MockConnectionService` now satisfies the async trait, returning the exact same mock data wrapped in `Ok` (offline/dev/test path unchanged); `connect` on an unknown/offline name maps to `StudioError::NotConnected`.
- Stood up `NodeDbConnectionService`, an inert stub wrapping `Option<NativeClient>` (None this phase) that returns `StudioError::NotConnected` from every method — never a panic or `todo!()` — and is instantiated object-safe behind `Rc<dyn ConnectionService>` in app.rs — SEAM-02.
- Migrated all 3 `connect()` call sites (connection_manager `on_connect`, command_palette's two switch handlers, connection_popover switch item) to `spawn` + `.await`, setting the active signal only after the await resolves.
- Rewired app.rs to seed registry + notifications EMPTY then fill them via `use_resource` after the async seam resolves (mock is instant), so no signal guard is held across `.await` and the main thread never blocks.
- App compiles and renders mock data identically; clippy clean under `-D warnings`; full suite green at 20/20 tests (incl. new async/stub/mock tests and the 2 pre-existing mock regression tests).

## Task Commits

1. **Task 1 + Task 2 (atomic): async trait conversion, Mock async impl, NodeDbConnectionService stub, 3 call-site migrations, app.rs use_resource seeding** - `60f6ad1` (feat)

**Plan metadata:** docs commit (this SUMMARY + STATE/ROADMAP/REQUIREMENTS) follows.

_Note: the plan's two tasks were already complete and intermingled in the working tree (previous executor cut off before committing); they were committed together as the 01-02 feature unit since the trait going async breaks the sync call sites atomically._

## Files Created/Modified
- `nodedb-studio/src/services/connection_service.rs` - `#[async_trait(?Send)]` trait (3 `async fn` returning `Result<_, StudioError>`); `MockConnectionService` async impl returning identical mock data; async unit tests.
- `nodedb-studio/src/services/nodedb_service.rs` (new) - `NodeDbConnectionService` stub wrapping `Option<NativeClient>`, returning `StudioError::NotConnected`; `#[tokio::test]` stub tests incl. `Rc<dyn ConnectionService>` object-safety coercion.
- `nodedb-studio/src/services/mod.rs` - registered `pub mod nodedb_service;` (re-exports only).
- `nodedb-studio/src/services/error.rs` - removed the enum-level `#[allow(dead_code)]` (now consumed by the seam); retained the scoped `#[allow(dead_code)]` on `is_retriable()` until 01-04.
- `nodedb-studio/src/app.rs` - instantiates `NodeDbConnectionService::default()` (object-safe coercion proof); seeds registry/notifications via `use_resource` after the async call resolves.
- `nodedb-studio/src/views/connection_manager.rs` - `on_connect` handler now `spawn`s and `await`s `service.connect(&name)`, setting `active` after the await.
- `nodedb-studio/src/components/command_palette.rs` - both switch handlers (`staging-cluster`, `prod-replica-eu`) now `spawn` + `.await`; `open.set(false)` stays synchronous.
- `nodedb-studio/src/components/popovers/connection_popover.rs` - switch-item handler now `spawn`s + `.await`s `connect`; `popover.set(None)` stays synchronous; no new `.read()` inside the spawn.

## Decisions Made
- **Async trait shape locked for the phase:** all 3 methods return `Result<_, StudioError>`; `connect` returns `Result<ActiveConnection, StudioError>` (forward-compatible with CONN-03; unknown/offline mock name maps to `StudioError::NotConnected`).
- **Stub wraps `Option<NativeClient>` = None this phase** and never calls the client; Phase 2 (CONN-01..07) fills it via `ConnectionBuilder` — explicitly out of scope here.
- **Dead-code allow scope adjusted:** removed the enum-level `#[allow(dead_code)]` from `StudioError` now that the async seam references it; kept the scoped `#[allow(dead_code)]` on `is_retriable()` because it stays unused until plan 01-04 wires the Retry affordance.

## Deviations from Plan
None - plan executed exactly as written. (The implementation was completed by a prior executor before a session limit; this finalization pass re-confirmed all gates, committed the work, and produced tracking artifacts. No code was modified.)

## Issues Encountered
None. All three CI gates were re-confirmed green before committing:
- `cargo build -p nodedb-studio` — clean
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — exit 0
- `cargo nextest run -p nodedb-studio` — 20 tests run, 20 passed, 0 skipped

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The async `ConnectionService` seam is live; `MockConnectionService` and the `NodeDbConnectionService` stub both satisfy it object-safe behind `Rc<dyn ConnectionService>`.
- SEAM-01 and SEAM-02 complete. Plan 01-03 (async-state loading/empty/error pattern) and 01-04 (Retry affordance, which un-gates `is_retriable()`) are unblocked.
- No blockers. Wave 2 complete.

## Known Stubs
- `NodeDbConnectionService` (nodedb_service.rs) is an INTENTIONAL inert stub for this phase: wraps `Option<NativeClient>` = None and returns `StudioError::NotConnected` from every method. This is the explicit scope of SEAM-02 (instantiable real-client stub); Phase 2 (CONN-01..07) wires it to a live `NativeClient` via `ConnectionBuilder`. App still runs entirely on `MockConnectionService` this phase, so the stub does not block the phase goal.
- `StudioError::is_retriable()` retains a scoped `#[allow(dead_code)]` — genuinely unused until plan 01-04 wires the Retry affordance. Intentional and self-documenting.

## Self-Check: PASSED

All created/modified files exist on disk; the feature commit (`60f6ad1`) exists in history.

---
*Phase: 01-async-seam-error-foundation*
*Completed: 2026-06-14*
