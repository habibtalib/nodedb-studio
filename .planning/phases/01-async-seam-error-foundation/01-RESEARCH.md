# Phase 1: Async Seam & Error Foundation - Research

**Researched:** 2026-06-14
**Domain:** Rust async-trait seam design + Dioxus 0.7 desktop async-at-the-boundary + `thiserror` error mapping from `nodedb-client`
**Confidence:** HIGH (all critical targets settled against actual source in `../nodedb` and verified Dioxus 0.7 docs/signatures)

## Summary

Phase 1 converts the studio's single `ConnectionService` seam from sync to async, introduces the studio's first `thiserror` type mapped from `nodedb-client`'s `NodeDbError`, stands up an instantiable-but-inert `NodeDbConnectionService` stub wrapping `NativeClient`, and proves a reusable `AsyncState<T>` loading/empty/error pattern by routing one existing read through `use_resource`. No real data is wired.

The research settled every flagged open question against the actual NodeDB source on disk (version-matched 0.3.0) and the verified Dioxus 0.7.9 API surface. The two findings that most affect the plan are: (1) **`#[async_trait(?Send)]` is the correct and safe choice** — Dioxus 0.7's runtime is single-threaded and `use_resource`'s future bound is `Future + 'static` with **no `Send` requirement**, so the seam can capture the `Rc<dyn ConnectionService>` directly; and (2) **`NativeClient` is behind `nodedb-client`'s `native` feature, which the studio's workspace dependency does NOT currently enable** — this Cargo.toml change (not just `.cargo/config.toml`) is the real prerequisite for criterion #2.

**Primary recommendation:** Add `async-trait = "0.1"` to the workspace, declare `ConnectionService` as `#[async_trait(?Send)]`, enable `nodedb-client`'s `native` feature in the workspace dependency, define a categorized `StudioError` (8 variants) via a `From<NodeDbError>` impl with a catch-all `Server` arm (required because `ErrorDetails` is `#[non_exhaustive]`), add a plain-Rust `AsyncState<T>` + a shared Dioxus component, and prove the pattern on the **notifications feed** (lowest blast radius — read-only, no `active.set()` side effect).

---

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Add the **`async-trait` 0.1** crate (workspace dependency) and make `ConnectionService` an `#[async_trait]` trait. Ask-first dep gate — **approved**. Hand-boxed `Pin<Box<dyn Future>>` returns explicitly rejected.
- **D-01a:** Service held as `Rc<dyn ConnectionService>` (single UI thread, not `Arc`/`Send`). Executor must verify whether `#[async_trait(?Send)]` is required. *(Settled below: yes, use `?Send`.)*
- **D-02:** Define a **categorized** studio `thiserror` enum (not a thin passthrough), mapped from `NodeDbError`'s `ErrorCode`/`ErrorDetails`. Target variants: Connection, Auth, NotFound, Conflict, ReadOnly, Server, Setup (+ NotConnected from D-06). Refine against the real `ErrorDetails`. *(Refined mapping table below.)*
- **D-03:** Each variant **preserves the originating `NodeDbError` as `#[source]`**; the studio error **exposes `is_retriable()`** (derived from `NodeDbError::is_retriable()`). Message-only variants rejected.
- **D-04:** Introduce a plain-Rust **`AsyncState<T>`** enum (`Loading` / `Empty` / `Loaded(T)` / `Error(StudioError)`) plus a **shared Dioxus component** that renders the three non-loaded states and yields to caller markup for `Loaded`. Enum + mapping logic stay in plain Rust (unit-testable).
- **D-05:** **Prove the pattern in Phase 1 against the async mock.** Route **one existing read** through the async seam with `use_resource` + the `AsyncState` component. Candidate: connection list or notifications feed (planner picks lowest-risk). Helper-only rejected.
- **D-06:** `NodeDbConnectionService` holds an **`Option<NativeClient>`** (`None` until Phase 2). While `None`, every data method returns `StudioError::NotConnected` (no `panic!`/`todo!()`). Must instantiate in `app.rs`. Real connect/auth out of scope.
- Hard constraints (AGENTS.md): no `unwrap`/`expect`/`panic` in seam code, no `Result<T, String>`, propagate with `?`. `MockConnectionService` must keep working identically.

### Claude's Discretion

- Exact `async-trait` patch within `0.1.*`, and `?Send` vs `Send` (subject to compiling under tokio + Dioxus desktop).
- Final variant names/shape of the categorized error enum, validated against actual `ErrorDetails`.
- Which single read view demonstrates `AsyncState` (connection list vs notifications) — lowest blast radius.
- File/module layout for the error type and `AsyncState` (respect <500 LOC, `mod.rs` = re-exports only).

### Deferred Ideas (OUT OF SCOPE)

- Real connect/auth, capability negotiation, identity chip — Phase 2 (CONN-01..07).
- Making `NodeDbConnectionService` the default/active impl — Phase 2.
- Wiring any data-path method (SQL, collections, document/vector/graph/FTS/KV) — Phases 3–6.
- Simulated latency in the mock — deferred unless a later phase needs it.

---

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SEAM-01 | `ConnectionService` is an async trait (via `async_trait`); `MockConnectionService` still satisfies it | `#[async_trait(?Send)]` confirmed compatible (`NodeDb` itself uses `async_trait`; Dioxus single-threaded). Mock bodies become `async fn` returning the same data instantly. |
| SEAM-02 | A `NodeDbConnectionService` wraps `NativeClient`/`NodeDb` and is the impl provided in `app.rs` | `NativeClient` confirmed at `nodedb-client/src/native/client/core.rs` (`NativeClient { pool: Pool }`), reachable **only with the `native` feature** — requires Cargo.toml change. `ConnectionBuilder` is the construction entry (Phase 2). Phase 1: `Option<NativeClient>` = `None`. |
| SEAM-03 | All client errors surface as the studio's typed `thiserror` error mapped from `NodeDbError`; never `unwrap`/`panic`/`Result<T,String>` | Full `ErrorDetails`/`ErrorCode`/`NodeDbError` surface read; concrete mapping table produced below. `NodeDbError: std::error::Error + Clone`, so `#[source]` + `is_retriable()` passthrough both work. |
| SEAM-04 | Async runs at the seam via `use_resource`/`use_action`; never blocks main thread / holds guard across `.await`; wired views render loading/empty/error | `use_resource` signature + `Resource<T>` API (`read`/`value`/`state`/`restart`/`finished`/`pending`) verified for 0.7.9; `restart()` drives the Retry affordance. |

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `async-trait` | `0.1` (resolves `0.1.89`) | Makes `ConnectionService` dyn-compatible as an async trait behind `Rc<dyn …>` | Native `async fn`-in-trait is not dyn-safe; `nodedb-client`'s `NodeDb` trait uses the identical pattern. Already present transitively in `Cargo.lock` (0.1.89). |
| `dioxus` (`desktop`, `router`) | `0.7.9` | UI + the `use_resource`/`use_action`/`spawn` async-at-seam primitives | Already the project framework. Single-threaded runtime → `!Send` futures allowed. |
| `nodedb-client` | `0.3.0` | `NativeClient`, `NodeDb` trait, re-exported `NodeDbError`/`NodeDbResult` | The seam's real backend. **Must enable `native` feature** for `NativeClient`. |
| `nodedb-types` | `0.3.0` | `NodeDbError`, `ErrorCode`, `ErrorDetails`, `Value`, result types | Source of the error model the studio maps from. |
| `thiserror` | `2.0` (resolves `2.0.18`) | Derive the studio's `StudioError` enum | Already in workspace. The studio's first `thiserror` type. |
| `tracing` | `0.1` | Log the full `NodeDbError` source chain | Already in workspace; `#[source]` preservation feeds it. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio` (`rt-multi-thread`, `macros`) | `1` | Backs Dioxus desktop's async + `#[tokio::test]` for async unit tests | Already present. Use `#[tokio::test]` to drive async trait methods in tests. |
| `dioxus-ssr` | `0.7` (**NOT in lock**) | `render_element(...)` render-level assertions (per AGENTS.md) | **Optional Wave 0 add** — only if render-level tests are wanted; not required for Phase 1's plain-Rust validation. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `async-trait` | Native `async fn` in traits (stable since Rust 1.75) | Rejected: not dyn-compatible — the seam is `Rc<dyn ConnectionService>`, which native async-fn-in-trait does not support. |
| `async-trait` | Hand-rolled `fn …() -> Pin<Box<dyn Future + '_>>` | Rejected in D-01 (boilerplate, error-prone). |
| `#[async_trait(?Send)]` | plain `#[async_trait]` (Send futures) | Both compile (`NativeClient` is `Send + Sync`), but `?Send` is the correct intent for a `Rc`-held, single-thread UI seam and avoids gratuitous `Send` bounds leaking onto future captures. See D-01a resolution. |
| Categorized `StudioError` | Thin `StudioError(NodeDbError)` newtype | Rejected in D-02 — views need to branch on category for tailored messages. |

**Installation (workspace `Cargo.toml`):**

```toml
[workspace.dependencies]
# add:
async-trait = "0.1"
# change (enable native client transport):
nodedb-client = { version = "0.3.0", features = ["native"] }
```

**App crate `nodedb-studio/Cargo.toml`** — add `async-trait = { workspace = true }` to `[dependencies]`. `nodedb-client = { workspace = true }` already inherits the feature.

**Version verification (performed):**
- `async-trait` → `0.1.89` already resolved in `Cargo.lock` (pulled via `nodedb-client`). HIGH.
- `dioxus` / `dioxus-hooks` → `0.7.9` in `Cargo.lock`. HIGH.
- `thiserror` → `2.0.18`, `tokio` 1.x — present. HIGH.
- `nodedb-client` / `nodedb-types` local workspace → **`0.3.0` confirmed** (`../nodedb/Cargo.toml` `version = "0.3.0"`), matches the pin. HIGH.

---

## Critical Research Targets — Resolutions

### 1. The real `ErrorDetails` enum → categorized `StudioError` mapping (D-02)

Source read: `../nodedb/nodedb-types/src/error/{details.rs,types.rs,code.rs}`.

**Hard fact #1:** `ErrorDetails` is `#[non_exhaustive]` (`details.rs:14`). The studio's `From<NodeDbError>` mapper **MUST include a catch-all `_` arm**. This does NOT violate the project's "no `_ =>` on exhaustive domain enums" rule — that rule applies to *the studio's own exhaustive domain enums*; `ErrorDetails` is a foreign, explicitly non-exhaustive enum where a catch-all is mandatory. Document this rationale in a code comment so reviewers don't flag it.

**Hard fact #2:** `NodeDbError` is `#[derive(Debug, Clone, ...)]`, implements `std::error::Error` (with `source()`), and `Display` (`"[NDB-XXXX] message"`). So it can be stored as `#[source]` and is cheap to clone. `is_retriable()` and `is_client_error()` are public methods.

**Hard fact #3:** `NodeDbError`'s fields are `pub(super)` — the studio **cannot pattern-match the struct directly**. It must call the public accessors: `.details()` (returns `&ErrorDetails`), `.code()`, `.message()`, `.is_retriable()`. Map on `*err.details()`.

**Refined variant set.** The proposed 7 (+NotConnected) categories are sound; the mapping below assigns real `ErrorDetails` variants to each. The proposal had **no unrepresented category that needs cutting**, but the real enum has many more cases than the proposal anticipated — they fold into `Server` (server-returned) vs `Setup` (never reached the server). Recommended final set: **`Connection`, `Auth`, `NotFound`, `Conflict`, `ReadOnly`, `Setup`, `Server`, `NotConnected`** (8 variants).

| StudioError variant | Maps from `ErrorDetails` (and codes) | Notes |
|---------------------|--------------------------------------|-------|
| **`Connection`** (transport never completed a request, retriable transport) | `HandshakeFailed` (2100), `SyncConnectionFailed` (3000), `NodeUnreachable` (6003), `NoLeader` (6000), `NotLeader` (6001), `MigrationInProgress` (6002), `ServerOverload` (1403), `MemoryExhausted` (7000) | These overlap with `is_retriable()`. Drives the Retry affordance. (Plus `From<io::Error>` on `NodeDbError` produces `Storage` — see `Server`/`Connection` judgement note.) |
| **`Auth`** | `AuthorizationDenied` (2000), `AuthExpired` (2001) | Tenant-quota auth cases (`TenantVectorDimExceeded`/`TenantGraphDepthExceeded`) are quota, not auth → `Server`. |
| **`NotFound`** | `CollectionNotFound` (1100), `DocumentNotFound` (1101), `CollectionDraining` (1102), `CollectionDeactivated` (1103) | Matches `NodeDbError::is_not_found()` for the first two; draining/deactivated are "effectively gone." |
| **`Conflict`** | `WriteConflict` (1001), `ConstraintViolation` (1000), `PrevalidationRejected` (1003), `BalanceViolation` (1011), `StateTransitionViolation` (1013), `TransitionCheckViolation` (1014), `TypeGuardViolation` (1024), `InsufficientBalance` (1022), `Overflow` (1021), `TypeMismatch` (1020) | Write-path rejections. `WriteConflict` is also retriable (covered by `is_retriable()` passthrough). |
| **`ReadOnly`** | `MirrorReadOnly` (1700), `AppendOnlyViolation` (1010), `PeriodLocked` (1012), `LegalHoldActive` (1016), `RetentionViolation` (1015), `MirrorNotPromoted` (1702) | Mutation rejected because the target is read-only / locked. |
| **`Setup`** (failure before/instead of reaching the server: config, client build, bad request) | `Config` (5000), `BadRequest` (5001), `SqlNotEnabled` (1202) | "Fix your config/request" class. Distinct from `Server` (which carries a server message). |
| **`Server`** (server-returned error with a message — the catch-all) | everything else: `Storage`/`SegmentCorrupted`/`ColdStorage`/`Wal`, `Serialization`/`Codec`, `Internal`/`Bridge`/`Dispatch`, `PlanError`, `FanOutExceeded`, `DeadlineExceeded`, `Encryption`, `Array`, `Cluster`, quota (`QuotaOvercommit`/`QuotaExceeded`/`TenantVectorDimExceeded`/`TenantGraphDepthExceeded`/`RateExceeded`), all clone/mirror/move-tenant DDL cases, `SyncDeltaRejected`/`ShapeSubscriptionFailed`, **and the `#[non_exhaustive]` `_` arm** | Carries `NodeDbError::message()`. The default destination. |
| **`NotConnected`** (D-06) | *(none — studio-originated)* | Returned by `NodeDbConnectionService` when its `Option<NativeClient>` is `None`. Not mapped from `NodeDbError`; constructed directly. |

**Recommended shape** (illustrative — names are Claude's discretion):

```rust
// src/services/error.rs  (or src/error.rs) — keep < 500 LOC
use nodedb_client::NodeDbError;
use nodedb_types::error::ErrorDetails;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StudioError {
    #[error("connection error: {0}")]
    Connection(#[source] NodeDbError),
    #[error("authentication error: {0}")]
    Auth(#[source] NodeDbError),
    #[error("not found: {0}")]
    NotFound(#[source] NodeDbError),
    #[error("conflict: {0}")]
    Conflict(#[source] NodeDbError),
    #[error("read-only: {0}")]
    ReadOnly(#[source] NodeDbError),
    #[error("setup error: {0}")]
    Setup(#[source] NodeDbError),
    #[error("server error: {0}")]
    Server(#[source] NodeDbError),
    #[error("not connected to a database")]
    NotConnected,
}

impl StudioError {
    /// Retry affordance signal. Delegates to the wrapped NodeDbError where present.
    pub fn is_retriable(&self) -> bool {
        match self {
            StudioError::NotConnected => false,
            StudioError::Connection(e)
            | StudioError::Auth(e)
            | StudioError::NotFound(e)
            | StudioError::Conflict(e)
            | StudioError::ReadOnly(e)
            | StudioError::Setup(e)
            | StudioError::Server(e) => e.is_retriable(),
        }
    }
}

impl From<NodeDbError> for StudioError {
    fn from(e: NodeDbError) -> Self {
        use ErrorDetails as D;
        match e.details() {
            D::HandshakeFailed { .. } | D::SyncConnectionFailed | D::NodeUnreachable
            | D::NoLeader | D::NotLeader { .. } | D::MigrationInProgress
            | D::ServerOverload | D::MemoryExhausted { .. } => StudioError::Connection(e),

            D::AuthorizationDenied { .. } | D::AuthExpired => StudioError::Auth(e),

            D::CollectionNotFound { .. } | D::DocumentNotFound { .. }
            | D::CollectionDraining { .. } | D::CollectionDeactivated { .. } => StudioError::NotFound(e),

            D::WriteConflict { .. } | D::ConstraintViolation { .. } | D::PrevalidationRejected { .. }
            | D::BalanceViolation { .. } | D::StateTransitionViolation { .. }
            | D::TransitionCheckViolation { .. } | D::TypeGuardViolation { .. }
            | D::InsufficientBalance { .. } | D::Overflow { .. } | D::TypeMismatch { .. } => StudioError::Conflict(e),

            D::MirrorReadOnly { .. } | D::AppendOnlyViolation { .. } | D::PeriodLocked { .. }
            | D::LegalHoldActive { .. } | D::RetentionViolation { .. }
            | D::MirrorNotPromoted { .. } => StudioError::ReadOnly(e),

            D::Config | D::BadRequest | D::SqlNotEnabled => StudioError::Setup(e),

            // Foreign enum is #[non_exhaustive]: this `_` arm is REQUIRED by the
            // compiler and is NOT a violation of the studio's no-`_` rule (that
            // rule covers the studio's own exhaustive domain enums).
            _ => StudioError::Server(e),
        }
    }
}
```

> **Note on the wrapped value:** `#[source]` requires the field type to impl `std::error::Error`. `NodeDbError` does (verified). Because the variants store the *whole* `NodeDbError` (which already carries code + message + cause chain), `tracing` gets the full chain via `source()` (D-03 satisfied). `e.is_retriable()` is delegated, so `WriteConflict`/`DeadlineExceeded`/cluster/etc. retriability survives categorization.

> **Pattern-match caveat:** the `From` matches on `e.details()` (a `&ErrorDetails`) but then needs to move `e` into the variant. Because `e.details()` borrows `e`, write the match to compute the *category* first (or match on `e.code()` which is `Copy`), then `e` is free to move. Simplest: `match e.code() { ErrorCode::COLLECTION_NOT_FOUND => … }` using the `ErrorCode` constants — `ErrorCode` is `Copy`, removing the borrow problem entirely. The planner should prefer matching on `e.code()` constants (no borrow of `e`) over `e.details()` to sidestep the borrow-then-move issue. Both are valid; code-based is cleaner.

### 2. `async-trait` `?Send` vs `Send` (D-01a) — **resolved: use `#[async_trait(?Send)]`**

Evidence:
- **Dioxus runtime is single-threaded.** Official 0.7 docs: *"The Dioxus runtime is single threaded which means futures can use `!Send` types, but they need to be careful to never block the thread."* (`dioxuslabs.com/learn/0.7/essentials/basics/async/`).
- **`use_resource` imposes no `Send` bound:** signature is `pub fn use_resource<T, F>(future: impl FnMut() -> F + 'static) -> Resource<T> where T: 'static, F: Future<Output = T> + 'static` — `F: Future + 'static`, no `Send`. (`docs.rs/dioxus-hooks`.)
- The seam is `Rc<dyn ConnectionService>` (`Rc` is `!Send`). With `#[async_trait]` (Send variant), the *returned future* must be `Send`, which would force every captured value (including the `Rc` and any `!Send` internals) to be `Send` — defeating the `Rc` choice. `#[async_trait(?Send)]` drops that bound.
- `NativeClient` **is** `Send + Sync` (its `NodeDb` impl has supertrait `NodeDbMarker: Send + Sync` on native — `nodedb-client/src/traits/core/marker.rs`). So `Send` *would* technically compile too. But `?Send` is the correct intent for a UI-thread `Rc` seam and is strictly more permissive. **Use `?Send`.**
- Apply `#[async_trait(?Send)]` to the trait **and every impl** (`MockConnectionService`, `NodeDbConnectionService`) — `async_trait` requires the attribute on both. This mirrors `nodedb-client`'s own cfg-swap (`#[cfg_attr(target_arch="wasm32", async_trait(?Send))]`).

**Gotcha:** inside the stub, calling `NativeClient`'s `NodeDb` methods returns `Send` futures (fine to `.await` inside a `?Send` future). No conflict.

### 3. The Dioxus 0.7 `use_resource` async-at-seam pattern (D-04/D-05, SEAM-04)

Verified API (`dioxus-hooks` 0.7.9):
- `use_resource(move || async move { … }) -> Resource<T>`. The closure reruns when read signals it touches change (reactive).
- `Resource<T>` methods: `.read()`/`.value()` → `Option<T>` (None while first run pending), `.state()` → `ReadSignal<UseResourceState>` (`Pending` | `Paused` | `Stopped` | `Ready`), `.finished()`, `.pending()`, **`.restart()`** (drives Retry), `.cancel()`, `.clear()`.
- Errors: idiomatic pattern is `Resource<Result<T, E>>` → match `Some(Ok(_))` / `Some(Err(_))` / `None`.

**Recommended Phase 1 wiring (prove on the notifications feed — lowest blast radius):**

```rust
// In a view (e.g. notification_popover.rs or a small wrapper):
let service = use_context::<Rc<dyn ConnectionService>>();
let feed = use_resource(move || {
    let service = service.clone();        // Rc clone OUTSIDE the async block
    async move { service.notifications().await } // -> Result<Vec<Notification>, StudioError>
});

// Map Resource state -> AsyncState<T> in plain Rust (testable mapping fn):
let state = AsyncState::from_resource(&feed);   // see below
rsx! { AsyncView::<Vec<Notification>> { state, /* loaded render via children/closure */ } }
```

`AsyncState<T>` (plain Rust, unit-testable — keep mapping logic out of the component):

```rust
pub enum AsyncState<T> {
    Loading,
    Empty,
    Loaded(T),
    Error(StudioError),
}

impl<T: IsEmpty> AsyncState<T> {
    /// Pure mapping from (Option<Result<T, StudioError>>) — unit test this directly.
    pub fn from_value(v: Option<Result<T, StudioError>>) -> Self {
        match v {
            None => AsyncState::Loading,
            Some(Err(e)) => AsyncState::Error(e),
            Some(Ok(t)) if t.is_empty() => AsyncState::Empty,
            Some(Ok(t)) => AsyncState::Loaded(t),
        }
    }
}
```

**Rules verified / gotchas to encode in the plan:**
- **Never hold a signal guard across `.await`** (AGENTS.md + Dioxus): clone the `Rc` *before* the `async move` block; read any needed signal value with `.peek()`/copy *before* awaiting; never `.read()`/`.write()` inside the awaited body. `Rc<dyn ConnectionService>` is `Clone`, so `let service = service.clone();` outside the async block is the safe move.
- **`use_resource` restart caveat:** historically (issue #2784) async hooks did not auto-restart on *prop* changes — they restart on *signal reads inside the closure* changing. For Phase 1 the read has no dynamic deps, so a manual `feed.restart()` is what the Retry button calls. Confirmed `restart()` exists.
- **`Resource::read()` returns `Option`** — `None` = first run not finished = your `Loading`. Distinguish empty-result (`Loaded` with empty Vec → `Empty`) from pending (`None`).
- Mutation-style calls (e.g. future `connect()`) use **`use_action`** / `spawn` (one-shot, button-triggered) rather than `use_resource` (reactive read). Phase 1 only needs `use_resource` for the proof; the `connect()` call sites (connection_manager, command_palette, connection_popover) move to a `spawn`-based handler that awaits `service.connect(name).await` then `active.set(...)` (set the signal AFTER the await completes, never across it).

**Why notifications over connection list:** the notifications feed is a pure read with no `active.set()` side effect; the connection list read is entangled with the `on_connect` handler that also mutates `active`. Proving `AsyncState` on the read-only feed isolates the pattern from the connect mutation (which itself must move to `spawn`). Both still must migrate, but the *AsyncState demo* is cleanest on notifications. (Final pick is planner's discretion per D-05.)

### 4. `async-trait 0.1` workspace dependency (D-01)

- Latest `0.1.*` resolves to **`0.1.89`** — already present in `Cargo.lock` (transitive via `nodedb-client`). HIGH.
- Add to workspace `[workspace.dependencies]`: `async-trait = "0.1"`. Add to `nodedb-studio/Cargo.toml [dependencies]`: `async-trait = { workspace = true }`.
- No version bump risk: it's the same version `nodedb-client` already uses, so no duplicate-version compile.

### 5. Setup prerequisite — `.cargo/config.toml`, `native` feature, baseline gate

Verified on disk (2026-06-14):

| Check | Result | Confidence |
|-------|--------|------------|
| `../nodedb` present | YES (`/Users/habib/Git/nodedb`, full workspace) | HIGH |
| `../nodedb` workspace version == `0.3.0` | YES (`../nodedb/Cargo.toml` → `version = "0.3.0"`) | HIGH |
| `.cargo/config.toml` exists | **NO** — absent in project, home, and search path; `.gitignore` line 101 = `.cargo/` | HIGH |
| Crates resolve WITHOUT the patch | **YES, currently** — `cargo metadata` reports `nodedb-client`/`nodedb-types` 0.3.0 from `registry+crates.io`; `cargo build -p nodedb-studio` **succeeds** (53s) and `cargo nextest run` passes (2 tests) | HIGH (observed) |
| **`NativeClient` reachable today** | **NO** — `nodedb-client` `default = []`; `NativeClient`/`ConnectionBuilder` are `#[cfg(feature = "native")]` (lib.rs:48-51). Studio's dep does not enable `native`. | HIGH |

**Interpretation / divergence from CONTEXT:** CONTEXT/AGENTS.md assume `nodedb-client`/`nodedb-types` are *not on crates.io* and therefore require the `.cargo/config.toml` patch to resolve at all. In **this environment they DO resolve from crates.io 0.3.0** and the baseline builds clean without any patch. Two non-exclusive explanations: (a) the crates are in fact published at 0.3.0, or (b) the environment has a registry mirror/cache. Either way, the **stated blocker (crates won't resolve) is not currently active**.

**The real Phase-1 blocker for criterion #2 is the `native` feature, not the patch.** Without `features = ["native"]` on `nodedb-client`, `use nodedb_client::NativeClient;` will not compile, so `NodeDbConnectionService` cannot wrap it.

**Recommended setup step for the planner (Wave 0 / first task):**
1. Enable the native client: change workspace dep to `nodedb-client = { version = "0.3.0", features = ["native"] }`. Re-run `cargo build` to confirm `NativeClient` is importable. *(This is the load-bearing setup action.)*
2. **Defensively** create the gitignored `.cargo/config.toml` patch (matches AGENTS.md guidance) so the build is reproducible on a developer machine where crates.io does NOT serve these crates:
   ```toml
   [patch.crates-io]
   nodedb-client = { path = "../nodedb/nodedb-client" }
   nodedb-types  = { path = "../nodedb/nodedb-types" }
   ```
   If the patch path is used, the local `native` feature still must be enabled by the studio dep (the patch only changes *source*, not *features*).
3. Run the baseline CI gate before any changes (STATE.md todo #3): `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo nextest run`. *(Baseline build + nextest already confirmed green by this research; clippy/fmt not yet run — the planner should still gate on them.)*

---

## Architecture Patterns

### Recommended Module Layout (respects <500 LOC, `mod.rs` = re-exports only)

```
nodedb-studio/src/
├── services/
│   ├── mod.rs                  # add: pub mod error; pub mod async_state;
│   ├── connection_service.rs   # trait → #[async_trait(?Send)]; Mock impl; NodeDbConnectionService stub
│   ├── error.rs                # StudioError enum + From<NodeDbError> + is_retriable()  (NEW)
│   └── async_state.rs          # AsyncState<T> enum + pure mapping fns                  (NEW, plain Rust)
├── components/
│   ├── mod.rs                  # add: pub mod async_view;
│   └── async_view.rs           # shared Dioxus component rendering Loading/Empty/Error  (NEW)
```

Alternative: top-level `src/error.rs` if the planner prefers the error type outside `services/`. Either honors the constraints; co-locating with the seam (`services/error.rs`) keeps the seam's `Result` error next to the trait. Keep `StudioError` and `From` impl together; if the mapping match grows past ~400 LOC (it won't — ~80 LOC), split the `From` impl into a sibling.

### Pattern 1: Async trait at the single seam (SEAM-01)

```rust
// services/connection_service.rs
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait ConnectionService {
    async fn list_connections(&self) -> Result<Vec<SavedConnection>, StudioError>;
    async fn notifications(&self) -> Result<Vec<Notification>, StudioError>;
    async fn connect(&self, name: &str) -> Result<ActiveConnection, StudioError>;
}

#[async_trait(?Send)]
impl ConnectionService for MockConnectionService {
    async fn list_connections(&self) -> Result<Vec<SavedConnection>, StudioError> {
        Ok(mock::connections())            // instant; stays mock-identical
    }
    async fn notifications(&self) -> Result<Vec<Notification>, StudioError> {
        Ok(mock::notifications())
    }
    async fn connect(&self, name: &str) -> Result<ActiveConnection, StudioError> {
        mock::connections().into_iter().find(|c| c.name == name)
            .and_then(|c| c.open())
            .ok_or(StudioError::NotConnected) // mock "unknown/offline" → typed error
    }
}
```

> **Signature decision (flag for planner):** the current `connect()` returns `Option<ActiveConnection>` and `list_connections`/`notifications` return bare `Vec`. Going async + typed-error means choosing return shapes. Recommended: all three return `Result<_, StudioError>`. For `connect`, "unknown name / offline" maps to a typed error (e.g. `NotConnected` or a dedicated variant) rather than `Option`, so the connection manager can render an error state (forward-looks to CONN-03). This is a behavior-preserving change *for the happy path* (mock still returns the same data); the call sites change from `if let Some(s) = …` to `match …`/`if let Ok(s) = …`.

### Pattern 2: The inert real-client stub (SEAM-02, D-06)

```rust
// services/connection_service.rs
use nodedb_client::NativeClient;

#[derive(Default)]
pub struct NodeDbConnectionService {
    client: Option<NativeClient>,   // None until Phase 2 connects
}

#[async_trait(?Send)]
impl ConnectionService for NodeDbConnectionService {
    async fn list_connections(&self) -> Result<Vec<SavedConnection>, StudioError> {
        Err(StudioError::NotConnected)  // no panic!, no todo!()
    }
    async fn notifications(&self) -> Result<Vec<Notification>, StudioError> {
        Err(StudioError::NotConnected)
    }
    async fn connect(&self, _name: &str) -> Result<ActiveConnection, StudioError> {
        Err(StudioError::NotConnected)
    }
}
```

Must instantiate in `app.rs` to satisfy criterion #2 (even though `MockConnectionService` stays the default). E.g. a `let _stub = NodeDbConnectionService::default();` behind a comment, or — cleaner — a compile-time assertion / a `#[cfg(test)]` test that constructs it and coerces to `Rc<dyn ConnectionService>`. The instantiation must be *real code that compiles*, not a doc comment.

### Pattern 3: `use_resource` + `AsyncState` proof (SEAM-04, D-05)

See target #3 above. Key shape: clone `Rc` before the `async move`, return `Result<T, StudioError>`, map to `AsyncState` via a pure function, render via the shared `AsyncView` component, wire `restart()` to a Retry button gated on `error.is_retriable()`.

### Anti-Patterns to Avoid

- **Holding a signal `.read()`/`.write()` across `.await`** — clone needed values out first; set signals only after the await resolves.
- **`Result<T, String>` anywhere in the seam** — use `StudioError`.
- **`_ =>` on the studio's own `StudioError`/`AsyncState`** — those are exhaustive domain enums; the only legitimate `_` arm is in the `From<NodeDbError>` mapper (foreign `#[non_exhaustive]` enum). Comment that arm.
- **`async fn` in trait without `async_trait`** — not dyn-safe for `Rc<dyn …>`.
- **Blocking the main thread** — async methods are instant for the mock; never add blocking IO in Phase 1.
- **Putting logic in `mod.rs`** — only `pub mod`/`pub use`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| dyn-compatible async methods | Manual `Pin<Box<dyn Future>>` returns | `#[async_trait(?Send)]` | D-01; matches `nodedb-client`; far less boilerplate/footguns. |
| Loading/pending detection | Custom `is_loading` signals per view | `Resource::read()` returning `Option` (+ `state()`/`pending()`) | Built into `use_resource`; reactive and cancel-safe. |
| Retry of a failed fetch | Re-instantiate the resource / custom retrigger | `Resource::restart()` | First-class API; gate on `StudioError::is_retriable()`. |
| Error categorization predicates | Re-implement retriable/not-found logic | `NodeDbError::is_retriable()` / `is_client_error()` / `is_not_found()` etc. | The client already exposes these; the studio delegates. |
| Error chain for logging | Concatenate messages by hand | `#[source]` on each `StudioError` variant + `tracing` | `NodeDbError` implements `source()`; `tracing` walks the chain. |

**Key insight:** Phase 1 is almost entirely *wiring existing primitives* — the only genuinely new code is the `StudioError` mapping table, the `AsyncState<T>` enum, and a small shared component. Resist building bespoke async/loading machinery.

---

## Common Pitfalls

### Pitfall 1: Borrow-then-move in the `From<NodeDbError>` impl
**What goes wrong:** matching on `e.details()` (a borrow) then trying to move `e` into the variant → borrow-checker error.
**How to avoid:** match on `e.code()` (`ErrorCode` is `Copy`) using the `ErrorCode::*` constants, so `e` is never borrowed; then move it. Or compute the category first, drop the borrow, then construct.
**Warning sign:** `cannot move out of 'e' because it is borrowed`.

### Pitfall 2: `#[async_trait]` attribute missing on an impl
**What goes wrong:** trait has `#[async_trait(?Send)]` but an impl omits it → cryptic "method not compatible" / lifetime errors.
**How to avoid:** put `#[async_trait(?Send)]` on the trait AND every impl block (Mock + NodeDb stub). Match the `?Send` exactly.

### Pitfall 3: Signal guard held across `.await`
**What goes wrong:** `active.read()` or `registry.read()` held while awaiting a seam call → panic/deadlock at runtime, or non-reactive staleness.
**How to avoid:** clone the `Rc` and copy/`.peek()` any signal value *before* the `async move` block; call `.set()` only after the await returns.
**Warning sign:** runtime borrow panic, or UI not updating.

### Pitfall 4: `NativeClient` not importable
**What goes wrong:** `use nodedb_client::NativeClient;` fails to resolve because the `native` feature is off.
**How to avoid:** enable `features = ["native"]` on the workspace `nodedb-client` dep (Wave 0). Verify with a build before writing the stub.
**Warning sign:** `unresolved import nodedb_client::NativeClient` / `no NativeClient in nodedb_client`.

### Pitfall 5: Treating empty result as loading
**What goes wrong:** an empty `Vec` (legit "no notifications") shown as a spinner forever.
**How to avoid:** `AsyncState::from_value` distinguishes `None` (pending → Loading) from `Some(Ok(empty))` (→ Empty). Unit-test both.

### Pitfall 6: `connect()` return shape churn breaks 3 call sites
**What goes wrong:** changing `connect` to async + `Result` breaks `connection_manager.rs`, `command_palette.rs`, `connection_popover.rs` which all use `if let Some(s) = service.connect(&name)` synchronously.
**How to avoid:** migrate all three to `spawn`/`use_action` handlers awaiting `service.connect(name).await` and matching `Result`. Inventory below — none can be missed or the build breaks.

---

## Code Examples

### Pure `AsyncState` mapping (unit-testable, no renderer)
```rust
// services/async_state.rs
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn none_is_loading()       { assert!(matches!(AsyncState::<Vec<u8>>::from_value(None), AsyncState::Loading)); }
    #[test] fn empty_vec_is_empty()    { assert!(matches!(AsyncState::from_value(Some(Ok(Vec::<u8>::new()))), AsyncState::Empty)); }
    #[test] fn nonempty_is_loaded()    { assert!(matches!(AsyncState::from_value(Some(Ok(vec![1u8]))), AsyncState::Loaded(_))); }
    #[test] fn err_is_error()          { assert!(matches!(AsyncState::from_value(Some(Err(StudioError::NotConnected))), AsyncState::Error(_))); }
}
```

### Async trait method exercised in a test (`#[tokio::test]`)
```rust
#[tokio::test]
async fn mock_notifications_returns_data() {
    let svc = MockConnectionService;
    let n = svc.notifications().await.expect("mock never errors"); // test code: expect OK
    assert!(!n.is_empty());
}

#[tokio::test]
async fn stub_returns_not_connected() {
    let svc = NodeDbConnectionService::default();
    assert!(matches!(svc.notifications().await, Err(StudioError::NotConnected)));
}
```
*(Source: `nodedb-client/src/traits/core/trait_def.rs` tests use the identical `#[tokio::test]` + `Arc<dyn NodeDb>` pattern — mirror it with `Rc<dyn ConnectionService>`.)*

### Error mapping test
```rust
#[test]
fn maps_collection_not_found_to_not_found() {
    let e = nodedb_client::NodeDbError::collection_not_found("users"); // public ctor (verified)
    assert!(matches!(StudioError::from(e), StudioError::NotFound(_)));
}
#[test]
fn maps_write_conflict_to_conflict_and_retriable() {
    let e = nodedb_client::NodeDbError::write_conflict("orders", "id1");
    let s = StudioError::from(e);
    assert!(matches!(s, StudioError::Conflict(_)));
    assert!(s.is_retriable()); // delegated to NodeDbError::is_retriable()
}
```
*(`NodeDbError::collection_not_found`, `write_conflict`, `bad_request`, etc. are public constructors — verified in `types.rs` tests.)*

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Sync trait returning bare values / `Option` | `#[async_trait(?Send)]` trait returning `Result<_, StudioError>` | This phase | Call sites migrate to `use_resource`/`spawn`. |
| No error type in studio | First `thiserror` enum (`StudioError`) mapped from `NodeDbError` | This phase | Establishes the seam's `Result` error for all later phases. |
| Per-view ad-hoc state | Shared `AsyncState<T>` + `AsyncView` | This phase | Phases 3–6 reuse, no per-view state matches. |

**Deprecated/outdated assumptions corrected by this research:**
- "`nodedb-client`/`nodedb-types` won't resolve without `.cargo/config.toml`" — in this environment they resolve from crates.io 0.3.0 and the baseline builds clean. The patch is still recommended *defensively* per AGENTS.md, but it is **not** the active blocker. (MEDIUM — environment-dependent.)
- "The patch is the only setup blocker" — the **`native` feature** is the real gate for `NativeClient` (criterion #2). (HIGH.)

---

## Open Questions

1. **`connect()` return shape: `Result<ActiveConnection, StudioError>` vs `Result<Option<ActiveConnection>, StudioError>`?**
   - What we know: current sync version returns `Option`; "unknown/offline" = `None`.
   - What's unclear: whether Phase 1 should collapse "not found/offline" into a typed error now or keep `Option` inside `Ok`.
   - Recommendation: return `Result<ActiveConnection, StudioError>`, mapping unknown/offline to a typed error (forward-compatible with CONN-03's "show a clear error"). Planner's call; either compiles.

2. **Where the `StudioError` type lives: `services/error.rs` vs `src/error.rs`.**
   - Recommendation: `services/error.rs` (co-located with the seam it serves). Discretion per D-02.

3. **Whether to add `dioxus-ssr` for a render-level test of `AsyncView`.**
   - What we know: `dioxus-ssr` is NOT in the lockfile; the `ssr` feature is off; AGENTS.md mentions `dioxus_ssr::render_element`.
   - Recommendation: NOT required for Phase 1. Validate `AsyncState` via plain-Rust unit tests + compile/clippy gate. Add `dioxus-ssr` only if a render assertion is explicitly wanted (it's a dep addition → ask-first).

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All compilation | ✓ | rustc/cargo 1.96.0 (== MSRV) | — |
| `cargo-nextest` | Test gate | ✓ | 0.9.137 | `cargo test` (AGENTS.md prefers nextest) |
| `../nodedb` checkout | `NativeClient`, error types, version match | ✓ | workspace 0.3.0 | — |
| `nodedb-client` / `nodedb-types` 0.3.0 resolution | Build | ✓ (resolves from crates.io in this env; builds clean) | 0.3.0 | `.cargo/config.toml [patch.crates-io]` → `../nodedb` (defensive, recommended) |
| `nodedb-client` `native` feature (for `NativeClient`) | SEAM-02 stub | ✗ (not enabled in workspace dep) | — | **No fallback — must enable `features = ["native"]`** |
| `async-trait` 0.1 | SEAM-01 | ✓ (in lock 0.1.89, transitive) | 0.1.89 | — |
| `dioxus` / `dioxus-hooks` 0.7.9 | SEAM-04 | ✓ | 0.7.9 | — |
| `dioxus-ssr` | render-level tests (optional) | ✗ (not in lock, feature off) | — | plain-Rust unit tests (sufficient) |
| Live NodeDB server :6433 | — | n/a | — | **Not needed** — Phase 1 wires no real connection (stub is inert). |

**Missing dependencies with no fallback:**
- `nodedb-client` `native` feature — **must** be enabled in `[workspace.dependencies]` before the stub can compile (`NativeClient` is `#[cfg(feature = "native")]`).

**Missing dependencies with fallback:**
- `.cargo/config.toml` patch — crates currently resolve without it, but create it defensively (AGENTS.md, reproducibility on other machines).
- `dioxus-ssr` — plain-Rust unit tests cover Phase 1 validation; SSR is optional.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo-nextest` 0.9.137 (over Rust test harness); `#[tokio::test]` for async methods |
| Config file | none (uses nextest defaults; tests are inline `#[cfg(test)] mod tests`) |
| Quick run command | `cargo nextest run -p nodedb-studio` |
| Full suite command | `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SEAM-01 | Async trait compiles; `MockConnectionService` satisfies it; returns same data | unit (`#[tokio::test]`) | `cargo nextest run -E 'test(mock_notifications_returns_data)'` | ❌ Wave 0 |
| SEAM-01 | App still compiles + renders mock identically | compile gate | `cargo build -p nodedb-studio` (+ manual `cargo run`) | ✓ (baseline green) |
| SEAM-02 | `NodeDbConnectionService` exists, wraps `NativeClient`, instantiable, returns `NotConnected` | unit + compile | `cargo nextest run -E 'test(stub_returns_not_connected)'` | ❌ Wave 0 |
| SEAM-03 | `NodeDbError` → `StudioError` mapping correct per category; `is_retriable()` delegated; no `unwrap`/`String` | unit | `cargo nextest run -E 'test(maps_)'` + `clippy -D warnings` | ❌ Wave 0 |
| SEAM-04 | `AsyncState` maps None/empty/ok/err correctly | unit | `cargo nextest run -E 'test(from_value)'` | ❌ Wave 0 |
| SEAM-04 | `use_resource` wiring renders loading/empty/error; no blocked thread | compile gate + manual render | `cargo build` + `cargo run` (visual); optional `dioxus_ssr` if added | ✓ compile / ❌ render test |

### Sampling Rate
- **Per task commit:** `cargo nextest run -p nodedb-studio`
- **Per wave merge:** `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run`
- **Phase gate:** full suite green before `/gsd:verify-work`; plus a manual `cargo run -p nodedb-studio` confirming the studio still renders mock data and the proven `AsyncState` view shows Loaded (and Error on a forced failure).

### Wave 0 Gaps
- [ ] `services/error.rs` tests — covers SEAM-03 (per-category mapping + retriable delegation)
- [ ] `services/async_state.rs` tests — covers SEAM-04 (`from_value` state transitions)
- [ ] `services/connection_service.rs` tests — covers SEAM-01 (mock async) + SEAM-02 (stub `NotConnected`); add `#[tokio::test]`
- [ ] Enable `nodedb-client` `native` feature (Cargo.toml) — prerequisite for SEAM-02 to compile
- [ ] (optional) `dioxus-ssr` dep + `ssr` feature if a render-level `AsyncView` test is desired (ask-first dep)
- [ ] Framework install: none — `cargo-nextest` 0.9.137 already present

*Note: the existing 2 mock tests must keep passing (regression guard for "renders mock identically").*

---

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **No `.unwrap()` / `.expect()` / `panic!` / `todo!()` in non-test code.** Typed `thiserror`, propagate with `?`. Never `Result<T, String>`. (Test code may `expect`.)
- **`mod.rs` = only `pub mod` / `pub use`** — no logic. New modules: add `pub mod error; pub mod async_state;` to `services/mod.rs`, `pub mod async_view;` to `components/mod.rs`.
- **Files < 500 LOC** — split by concern. `StudioError` + `From` ≈ 80–120 LOC (fits); `connection_service.rs` will grow (trait + 2 impls) — watch the limit, split impls to siblings if needed.
- **`sonic_rs` only, never `serde_json`** for runtime JSON. (Note: `nodedb-client`'s `native` feature pulls `serde_json` *internally* — that is the dependency's concern, not studio code. The studio still must not import `serde_json`.)
- **`nodedb_types::Value`** for DB values (not relevant to Phase 1 — no data path).
- **No `_ =>` on the studio's own exhaustive domain enums.** The single allowed `_` arm is in `From<NodeDbError>` (foreign `#[non_exhaustive]` `ErrorDetails`) — comment it.
- **Dioxus 0.7:** never hold `.read()`/`.write()` across `.await`; `.peek()` in handlers; stable list keys; async IO via `spawn`/`use_resource`/`use_action`; logic in plain Rust for testability; forms `e.prevent_default()`.
- **Seam discipline:** async lands at `ConnectionService` only; mock must keep working alongside the real one.
- **Ask-first deps:** `async-trait` is **approved** (D-01). Enabling the `native` feature on an existing dep is a feature change (recommend the planner note it; it's not a *new* crate). `dioxus-ssr` would be a new dep — ask first.
- **CI gate:** `cargo fmt --all` + `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo nextest run` must all pass.

---

## Sources

### Primary (HIGH confidence)
- `../nodedb/nodedb-types/src/error/details.rs` — `ErrorDetails` enum (`#[non_exhaustive]`, all variants)
- `../nodedb/nodedb-types/src/error/types.rs` — `NodeDbError` struct, `is_retriable()`/`is_client_error()`/category predicates, `Display`, `source()`, public ctors, `pub(super)` fields
- `../nodedb/nodedb-types/src/error/code.rs` — `ErrorCode` constants
- `../nodedb/nodedb-client/src/lib.rs` — feature gates (`native`/`remote`), re-exports of `NativeClient`/`ConnectionBuilder`/`NodeDbError`
- `../nodedb/nodedb-client/src/traits/core/trait_def.rs` — `NodeDb` trait `#[async_trait]` cfg-swap; `Arc<dyn NodeDb>` object-safety tests; `#[tokio::test]` patterns
- `../nodedb/nodedb-client/src/traits/core/marker.rs` — `NodeDbMarker: Send + Sync` on native (proves `NativeClient: Send + Sync`)
- `../nodedb/nodedb-client/src/native/client/core.rs` — `NativeClient { pool: Pool }`, `connect`/`new`
- `../nodedb/nodedb-client/src/native/client/dispatch.rs` — `impl NodeDb for NativeClient` async_trait cfg-swap
- `../nodedb/nodedb-client/Cargo.toml` — `default = []`, `native`/`remote` feature definitions, `async-trait` dep
- nodedb-studio source: `services/connection_service.rs`, `app.rs`, `views/connection_manager.rs`, `components/command_palette.rs`, `components/popovers/connection_popover.rs`, `data/mock.rs`, `state/connection.rs`, `state/connections_registry.rs`, `models/notification.rs`, `views/studio_shell.rs`, all `mod.rs`
- `Cargo.lock` — `async-trait` 0.1.89, `dioxus`/`dioxus-hooks` 0.7.9, `thiserror` 2.0.18, `nodedb-types` 0.3.0
- Observed builds: `cargo build -p nodedb-studio` (green, 53s), `cargo nextest run -p nodedb-studio` (2 passed), `cargo metadata` (crate sources), toolchain 1.96.0
- Dioxus 0.7 async docs — single-threaded runtime, `!Send` futures allowed: https://dioxuslabs.com/learn/0.7/essentials/basics/async/
- `dioxus_hooks::use_resource` signature (`F: Future + 'static`, no Send): https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_resource.html
- `dioxus_hooks::Resource` methods (`restart`/`read`/`value`/`state`/`finished`/`pending`/`cancel`/`clear`): https://docs.rs/dioxus-hooks/latest/dioxus_hooks/struct.Resource.html

### Secondary (MEDIUM confidence)
- Dioxus async-hook restart behavior on prop changes (issue #2784): https://github.com/DioxusLabs/dioxus/issues/2784
- Crates resolve from crates.io in this environment without the patch — environment-dependent observation (`cargo metadata` + clean build), flagged as MEDIUM because CONTEXT asserts they are unpublished.

### Tertiary (LOW confidence)
- None — all critical claims verified against source or official docs.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions verified in `Cargo.lock` + `../nodedb` manifests; builds observed green.
- Error mapping: HIGH — full `ErrorDetails`/`ErrorCode`/`NodeDbError` source read; `#[non_exhaustive]` and `pub(super)` field constraints confirmed.
- `?Send` decision: HIGH — Dioxus single-threaded runtime + `use_resource` signature (no Send bound) + `NodeDbMarker` Send-ness all verified.
- `use_resource`/`AsyncState` pattern: HIGH — `Resource` API (`restart`/`read`/`state`) confirmed via docs.rs for 0.7.9.
- Setup prerequisite: HIGH for the `native`-feature blocker and version match; MEDIUM on the crates.io-resolution observation (environment-dependent).
- Validation architecture: HIGH — nextest + baseline tests observed; Wave 0 gaps concrete.

**Research date:** 2026-06-14
**Valid until:** 2026-07-14 (stable — pinned versions; re-verify if `nodedb` or Dioxus 0.7 minor bumps)
