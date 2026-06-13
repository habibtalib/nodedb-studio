# Phase 1: Async Seam & Error Foundation - Context

**Gathered:** 2026-06-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Convert the single backend seam (`ConnectionService`) from sync to async, define the
studio's typed error model (mapped from `nodedb-client`'s `NodeDbError`), stand up a
`NodeDbConnectionService` stub that wraps `NativeClient`, and establish the reusable
loading/empty/error UI-state pattern — all while `MockConnectionService` keeps working
identically.

**No real data is wired in this phase.** Real connect/auth is Phase 2; SQL is Phase 3;
collections Phase 4; data browsers Phases 5–6. Phase 1 only builds the foundations every
later phase consumes: an async trait, a typed error, a UI-state primitive, and an
instantiable (but not-yet-functional) real-client struct.

Requirements covered: SEAM-01, SEAM-02, SEAM-03, SEAM-04.

</domain>

<decisions>
## Implementation Decisions

### Async mechanism
- **D-01:** Add the **`async-trait` 0.1** crate (workspace dependency) and make
  `ConnectionService` an `#[async_trait]` trait. This is the AGENTS.md "ask-first" dep
  gate — **approved**. Rationale: the seam is `Rc<dyn ConnectionService>`, so native
  `async fn`-in-trait is not dyn-compatible; `nodedb-client` itself uses `async_trait`;
  it keeps the trait and both impls clean with minimal churn. Hand-boxed
  `Pin<Box<dyn Future>>` returns were explicitly rejected (boilerplate, error-prone).
- **D-01a:** The service is held as `Rc<dyn ConnectionService>` (single UI thread, not
  `Arc`/`Send`). The executor must verify whether `#[async_trait(?Send)]` is required —
  `Rc` and `NativeClient`'s Send-ness drive this. Pick whichever lets the mock + stub
  compile under Dioxus desktop's tokio runtime without forcing `Send` on UI-thread types.

### Error model
- **D-02:** Define a **categorized** studio `thiserror` enum (not a thin passthrough),
  mapped from `NodeDbError`'s `ErrorCode`/`ErrorDetails`. Target variant set (refine in
  planning against the real `ErrorDetails` enum): **Connection** (transport/refused/
  timeout), **Auth** (bad credentials / unauthorized), **NotFound** (collection/document/
  key absent), **Conflict** (write conflict / constraint), **ReadOnly** (mutation on a
  read-only session), **Server** (server-returned error with message), and a **Setup**/
  transport variant for failures that never reach the server (e.g. client build, missing
  config). Rationale: lets views branch on category and show tailored messages instead of
  inspecting raw errors in the UI.
- **D-03:** Each variant **preserves the originating `NodeDbError` as `#[source]`** so
  `tracing` logs the full error chain, and the studio error **exposes `is_retriable()`**
  (derived from `NodeDbError::is_retriable()`) so views can offer a Retry affordance on
  transient failures. Message-only variants were rejected (lose the chain + retry signal).
- Hard constraints (carried from AGENTS.md): no `unwrap`/`expect`/`panic` in seam code,
  no `Result<T, String>`, propagate with `?`. This error type IS the seam's `Result` error.

### Loading / empty / error UI pattern
- **D-04:** Introduce a plain-Rust **`AsyncState<T>`** enum
  (`Loading` / `Empty` / `Loaded(T)` / `Error(StudioError)`) plus a **shared Dioxus
  component** that renders the three non-loaded states and yields to caller markup for
  `Loaded`. Keep the enum + any mapping logic in plain Rust so it is unit-testable without
  a renderer. Every later wiring phase reuses this — no per-view ad-hoc state matches.
- **D-05:** **Prove the pattern in Phase 1 against the async mock.** Route **one existing
  read** through the now-async seam with `use_resource` + the `AsyncState` component so the
  three states are exercised end-to-end in a real render path. Candidate read: the
  connection-manager connection list or the notifications feed (planner picks the
  lowest-risk one). Helper-only (no view change) was rejected — the pattern would ship
  unproven.
- Carried constraint: async runs **at the seam only** via `use_resource`/`use_action`;
  never hold a signal `.read()`/`.write()` guard across `.await`; never block the main
  thread.

### Stub service shape
- **D-06:** `NodeDbConnectionService` holds an **`Option<NativeClient>`** (`None` until
  Phase 2 connects). While the client is `None`, every data method returns a typed
  **`StudioError::NotConnected`** (no `panic!`, no `todo!()`). The struct must instantiate
  in `app.rs` (even if not the default impl) and compile against `NativeClient`. Rationale:
  gives Phase 2 a prepared slot to fill rather than a redesign. Real connect/auth wiring is
  explicitly **out of scope for Phase 1** (Phase 2 / CONN-01..07).
- `MockConnectionService` must keep satisfying the same async trait and render mock data
  identically to before (Phase 1 success criterion #1).

### Claude's Discretion
- Exact `async-trait` patch version within `0.1.*`, and `?Send` vs `Send` choice (D-01a),
  subject to compiling under the existing tokio + Dioxus desktop runtime.
- Final variant names/shape of the categorized error enum, validated against the actual
  `nodedb_types::error::details::ErrorDetails` variants during planning.
- Which single read view demonstrates `AsyncState` in Phase 1 (connection list vs
  notifications) — planner picks the lowest-blast-radius option.
- File/module layout for the new error type and `AsyncState` (respecting <500 LOC,
  `mod.rs` = re-exports only).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 1 goal + the 4 success criteria (source of truth for "done")
- `.planning/REQUIREMENTS.md` — SEAM-01, SEAM-02, SEAM-03, SEAM-04 wording
- `.planning/PROJECT.md` — milestone framing, constraints, key decisions (seam-async, keep mock)
- `.planning/STATE.md` — accumulated decisions + the 3 pre-Phase-1 setup todos

### The seam being modified (studio)
- `nodedb-studio/src/services/connection_service.rs` — the trait + `MockConnectionService` to convert
- `nodedb-studio/src/app.rs` — provides `Rc<dyn ConnectionService>` via context (line ~30); where the stub instantiates
- `nodedb-studio/src/views/connection_manager.rs` — sync `connect()`/`list_connections()` consumer (candidate for AsyncState proof)
- `nodedb-studio/src/components/command_palette.rs`, `nodedb-studio/src/components/popovers/connection_popover.rs` — other sync `connect()` call sites that must move to async
- `nodedb-studio/src/data/mock.rs` — mock data backing the mock service
- `nodedb-studio/src/state/connection.rs` — `ActiveConnection` + `Capabilities`

### Client surface to wrap / map from (../nodedb)
- `../nodedb/nodedb-client/src/lib.rs` — re-exports `NodeDbError`, `NodeDbResult`; `NativeClient` + `ConnectionBuilder` entry
- `../nodedb/nodedb-client/src/traits/core/trait_def.rs` — the `NodeDb` async trait method surface (uses `async_trait`)
- `../nodedb/nodedb-types/src/error/types.rs` — `NodeDbError` struct, `is_retriable()`, `is_client_error()`, `Display`, `source()`
- `../nodedb/nodedb-types/src/error/details.rs` — `ErrorDetails` enum (drives the categorized-error mapping)
- `../nodedb/nodedb-types/src/error/code.rs` — `ErrorCode`

### Studio conventions (hard rules)
- `AGENTS.md` — error handling, async/Dioxus rules, dep ask-first gate, CI gate
- `.planning/codebase/CONVENTIONS.md`, `.planning/codebase/INTEGRATIONS.md` — seam + error conventions
- `.planning/codebase/TESTING.md` — test expectations (nextest, `dioxus_ssr` render checks)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ConnectionService` trait (`services/connection_service.rs`): 3 sync methods —
  `list_connections() -> Vec<SavedConnection>`, `notifications() -> Vec<Notification>`,
  `connect(name) -> Option<ActiveConnection>`. These become `async`.
- `MockConnectionService`: zero-sized `#[derive(Default)] struct`, reads `crate::data::mock`.
  Bodies stay effectively instant after going async.
- Context wiring in `app.rs`: `let service: Rc<dyn ConnectionService> = Rc::new(MockConnectionService);`
  then `use_context_provider(|| service.clone())`. The stub plugs in at this exact line.
- `tokio` (`rt-multi-thread`, `macros`), `thiserror 2.0`, `tracing 0.1` already in the
  workspace — no new deps beyond `async-trait`.

### Established Patterns
- Single seam, dyn-dispatched behind `Rc`, provided via Dioxus context (never globals).
- Views read the service from context: `use_context::<Rc<dyn ConnectionService>>()`.
  Current call sites use the **return value synchronously** — they must migrate to
  `use_resource`/`use_action` when the methods go async.
- No thiserror types exist in the studio yet — this phase introduces the first one.
- `nodedb-client`'s `NodeDb` trait is itself `async_trait`-based, so the studio's choice
  mirrors the dependency it wraps.

### Integration Points
- `app.rs:~30` — service instantiation / context provision (stub goes here).
- `connection_manager.rs:~18,53`, `command_palette.rs:~18,70,79`,
  `connection_popover.rs:~19,58` — every current `.connect(...)`/service read; all must
  move to the async pattern.

### Setup Prerequisite (flag for planner — NOT a design choice)
- `.cargo/config.toml` (gitignored) **does not exist yet**. Without a
  `[patch.crates-io]` block pointing `nodedb-client`/`nodedb-types` at `../nodedb`, the
  crates won't resolve and `NodeDbConnectionService` cannot compile (Phase 1 criterion #2).
  `../nodedb` is present on disk. STATE.md lists three pre-Phase-1 todos:
  (1) verify `.cargo/config.toml` patch paths resolve, (2) confirm local nodedb workspace
  is at `0.3.0`, (3) run the CI gate on baseline before any changes. **Do these first.**

</code_context>

<specifics>
## Specific Ideas

- Categorized error variants requested: Connection, Auth, NotFound, Conflict, ReadOnly,
  Server, Setup — validate/refine against `ErrorDetails` during planning.
- `AsyncState<T>` enum named explicitly: `Loading | Empty | Loaded(T) | Error(StudioError)`.
- Retry affordance should be drivable from `is_retriable()` on the studio error.

</specifics>

<deferred>
## Deferred Ideas

- Real connect/auth, capability negotiation, identity chip — Phase 2 (CONN-01..07).
- Making `NodeDbConnectionService` the default/active impl on a real connection — Phase 2.
- Wiring any data-path method (SQL, collections, document/vector/graph/FTS/KV) — Phases 3–6.
- Simulated latency in the mock to exercise loading states — not needed; deferred unless a
  later phase wants it for demos/tests.

None of the discussion strayed outside the phase scope.

</deferred>

---

*Phase: 01-async-seam-error-foundation*
*Context gathered: 2026-06-14*
