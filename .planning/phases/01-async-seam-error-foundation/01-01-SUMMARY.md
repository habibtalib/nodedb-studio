---
phase: 01-async-seam-error-foundation
plan: 01
subsystem: infra
tags: [rust, thiserror, async-trait, nodedb-client, error-handling, cargo]

# Dependency graph
requires: []
provides:
  - "StudioError: the studio's first typed thiserror enum (8 categorized variants) — the seam's Result error"
  - "From<NodeDbError> mapping every ErrorDetails category to a StudioError category, origin preserved via #[source]"
  - "StudioError::is_retriable() delegating to the wrapped NodeDbError (drives the Retry affordance)"
  - "nodedb-client native feature enabled — NativeClient is importable for the later stub (plan 01-02)"
  - "async-trait workspace + crate dependency for the async ConnectionService seam (plan 01-02)"
  - "defensive gitignored .cargo/config.toml [patch.crates-io] for reproducible local builds"
affects: [01-02-async-seam-stub, 01-03-asyncstate-pattern, 01-04, connect, auth, query, collections, browser]

# Tech tracking
tech-stack:
  added: [async-trait 0.1, "nodedb-client native feature"]
  patterns:
    - "Categorized typed error mapped from a foreign error type; #[source] preserves the cause chain; is_retriable() delegated"
    - "Foreign #[non_exhaustive] enum match: a single documented `_` catch-all arm is allowed (studio's own enums stay exhaustive)"

key-files:
  created:
    - nodedb-studio/src/services/error.rs
    - .cargo/config.toml
  modified:
    - Cargo.toml
    - nodedb-studio/Cargo.toml
    - nodedb-studio/src/services/mod.rs

key-decisions:
  - "Matched on borrowed e.details() then moved e into the variant (compiles cleanly; code()-based fallback not needed)"
  - "Scoped #[allow(dead_code)] on StudioError + is_retriable() — binary crate, the type is consumed by later plans, not yet referenced from non-test code in this plan"
  - "Activated the .cargo/config.toml patch so the local ../nodedb checkout is the source of truth for reproducibility"

patterns-established:
  - "StudioError + From<NodeDbError>: the canonical seam Result error every later plan consumes"
  - "Documented foreign-enum catch-all: the ONLY allowed `_ =>` arm, with a rationale comment"

requirements-completed: [SEAM-03]

# Metrics
duration: 6min
completed: 2026-06-13
---

# Phase 1 Plan 1: Async Seam & Error Foundation Summary

**StudioError — the studio's first typed thiserror enum (8 categories) mapped from nodedb-client's NodeDbError with #[source] preservation and delegated is_retriable(), plus the native client feature + async-trait dependency that unblock the rest of Phase 1.**

## Performance

- **Duration:** ~6 min
- **Completed:** 2026-06-13
- **Tasks:** 2
- **Files modified/created:** 6 (3 created, 3 modified; Cargo.lock also updated)

## Accomplishments
- Enabled `nodedb-client`'s `native` feature so `NativeClient` is importable — the load-bearing prerequisite for plan 01-02's stub.
- Added `async-trait = "0.1"` to the workspace and the studio crate (approved per D-01) for the upcoming async `ConnectionService` trait.
- Created the defensive gitignored `.cargo/config.toml` `[patch.crates-io]` pointing at `../nodedb` (now the active build source).
- Introduced `StudioError`: 8 categorized variants (Connection / Auth / NotFound / Conflict / ReadOnly / Setup / Server / NotConnected) with `From<NodeDbError>` mapping and delegated `is_retriable()`.
- 12 unit tests covering every documented mapping category plus retriable delegation; all 14 studio tests pass; clippy clean under `-D warnings`.

## Task Commits

1. **Task 1: Enable native feature, add async-trait, create defensive .cargo/config.toml** - `b95d259` (chore)
2. **Task 2: Create StudioError enum + From<NodeDbError> + is_retriable() with tests** - `621b776` (feat, TDD: impl + tests together)

## Files Created/Modified
- `nodedb-studio/src/services/error.rs` - `StudioError` enum, `From<NodeDbError>` category mapping, `is_retriable()`, and 12 unit tests.
- `nodedb-studio/src/services/mod.rs` - Registered `pub mod error;` (re-exports only).
- `Cargo.toml` - `nodedb-client` `features = ["native"]`; added `async-trait = "0.1"`.
- `nodedb-studio/Cargo.toml` - `async-trait = { workspace = true }`.
- `.cargo/config.toml` - gitignored defensive `[patch.crates-io]` for `nodedb-client`/`nodedb-types`.

## Decisions Made
- Matched on the borrowed `e.details()` and moved `e` into the variant after the arm is selected — this compiles cleanly, so the `e.code()`-based fallback the plan offered was not required.
- The only `_ =>` arm lives in `From<NodeDbError>` for the foreign `#[non_exhaustive]` `ErrorDetails` enum, with a rationale comment; the studio's own `StudioError` has no catch-all.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Scoped `#[allow(dead_code)]` on `StudioError` and `is_retriable()`**
- **Found during:** Task 2 (clippy gate)
- **Issue:** `cargo clippy -D warnings` failed with `enum StudioError is never used` and `method is_retriable is never used`. This is a binary crate, so `pub` does not suppress dead-code; the type is foundational and only consumed in tests until later plans (01-02..04) wire it into the seam.
- **Fix:** Added a documented `#[allow(dead_code)]` on the enum and on `is_retriable()`, with a comment explaining it is a foundation type consumed by later plans.
- **Files modified:** nodedb-studio/src/services/error.rs
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0.
- **Committed in:** `621b776` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary to pass the CI clippy gate for a binary-crate foundation type. No scope creep — the allow attributes are scoped and self-documenting, and will become unnecessary once 01-02 references `StudioError`.

## Issues Encountered
None beyond the dead-code clippy gate documented above.

## User Setup Required
None - no external service configuration required. (`.cargo/config.toml` is created automatically and gitignored.)

## Next Phase Readiness
- `StudioError` is ready as the async seam's `Result` error for plan 01-02.
- `NativeClient` is importable and `async-trait` is available workspace-wide — both prerequisites for the async `ConnectionService` trait + `NodeDbConnectionService` stub.
- No blockers. Wave 1 complete; the rest of Phase 1 is unblocked.

## Known Stubs
None — this plan delivers a complete, tested error type and dependency foundation. The inert `NodeDbConnectionService` stub is plan 01-02's scope, not this plan's.

## Self-Check: PASSED

All created/modified files exist on disk; both task commits (`b95d259`, `621b776`) exist in history.

---
*Phase: 01-async-seam-error-foundation*
*Completed: 2026-06-13*
