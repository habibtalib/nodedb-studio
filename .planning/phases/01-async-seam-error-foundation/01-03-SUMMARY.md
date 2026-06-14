---
phase: 01-async-seam-error-foundation
plan: 03
subsystem: ui-state
tags: [rust, dioxus, async-state, loading-empty-error, primitive, seam]

# Dependency graph
requires:
  - "StudioError (01-01): the typed seam error stored by-value in AsyncState::Error and rendered (via Display) + gated (via is_retriable) by AsyncView"
provides:
  - "AsyncState<T>: plain-Rust loading/empty/error/loaded enum with a pure, unit-tested from_value mapping (None->Loading, Some(Err)->Error, Some(Ok(empty))->Empty, Some(Ok(data))->Loaded)"
  - "IsEmpty trait (impl for Vec<T>) — lets from_value distinguish loaded-but-empty from loaded-with-data without a renderer"
  - "AsyncView Dioxus component: shared Loading/Empty/Error(+retriable-gated Retry) renderer that yields to caller markup for Loaded, via discrete Clone+PartialEq props"
affects: [01-04, query, collections, browser, "phases 3-6 wired views"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "State-mapping in plain Rust (testable without a renderer); the component holds only render logic (per D-04)"
    - "Generic AsyncState<T> kept off the component boundary: the caller decodes it and passes discrete Clone+PartialEq flags, because StudioError is not Clone/PartialEq and T is unconstrained"

key-files:
  created:
    - nodedb-studio/src/services/async_state.rs
    - nodedb-studio/src/components/async_view.rs
  modified:
    - nodedb-studio/src/services/mod.rs
    - nodedb-studio/src/components/mod.rs

key-decisions:
  - "AsyncState generic over T only; Error holds StudioError by value (StudioError is not Clone/PartialEq) — exactly as planned, no derives added to StudioError"
  - "AsyncView takes discrete props (loading/empty/error/retriable/on_retry/empty_message) rather than the generic enum, so props satisfy Dioxus's Clone+PartialEq requirement; calling convention documented in the module doc"
  - "Scoped #[allow(dead_code)] on AsyncState / from_value / AsyncView (binary crate, consumed by 01-04), mirroring how 01-01 handled StudioError"

patterns-established:
  - "AsyncState::from_value as the single, renderer-free mapping from a use_resource read to the four UI states — phases 3-6 reuse it, no per-view ad-hoc matches"
  - "AsyncView as the single loading/empty/error+Retry renderer — caller owns only the Loaded markup"

requirements-completed: [SEAM-04]

# Metrics
duration: ~4min
completed: 2026-06-14
---

# Phase 1 Plan 3: AsyncState Pattern Summary

**The reusable loading/empty/error UI-state primitive: a plain-Rust `AsyncState<T>` enum with a pure, unit-tested `from_value` mapping, plus a shared `AsyncView` Dioxus component that renders the three non-loaded states (with a retriable-gated Retry) and yields to caller markup for `Loaded`.**

## Performance

- **Duration:** ~4 min
- **Completed:** 2026-06-14
- **Tasks:** 2
- **Files modified/created:** 4 (2 created, 2 modified)

## Accomplishments
- Introduced `AsyncState<T>` (Loading / Empty / Loaded(T) / Error(StudioError)) — the studio's own exhaustive enum, no `_ =>` catch-all.
- Added the `IsEmpty` trait (impl for `Vec<T>`) so `from_value` can distinguish a loaded-but-empty result from loaded-with-data.
- Implemented the pure `from_value` mapping (`None->Loading`, `Some(Err)->Error`, `Some(Ok(empty))->Empty`, `Some(Ok(data))->Loaded`) with 4 renderer-free unit tests, one per branch.
- Built the shared `AsyncView` component: renders Loading / Empty / Error states, gates the Retry button on `retriable`, and yields to caller markup for the Loaded case. Calling convention (used by 01-04) documented in the module doc comment.
- All gates green: 4 `async_state` tests pass, `cargo build -p nodedb-studio` clean, `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean. New files are rustfmt-clean.

## Task Commits

1. **Task 1: AsyncState<T> enum + IsEmpty + pure from_value mapping (TDD, impl+tests together)** - `44d2296` (feat)
2. **Task 2: Shared AsyncView Dioxus component** - `bfc1d3c` (feat)

## Files Created/Modified
- `nodedb-studio/src/services/async_state.rs` - `AsyncState<T>` enum, `IsEmpty` trait (+ `Vec<T>` impl), pure `from_value`, 4 unit tests.
- `nodedb-studio/src/components/async_view.rs` - `AsyncView` `#[component]` + `AsyncViewProps`; renders Loading/Empty/Error(+Retry); documented calling convention.
- `nodedb-studio/src/services/mod.rs` - Registered `pub mod async_state;` (re-exports only).
- `nodedb-studio/src/components/mod.rs` - Registered `pub mod async_view;` (re-exports only).

## Decisions Made
- Kept `AsyncState` generic over `T` only and stored `StudioError` by value in `Error(StudioError)`; did not add Clone/PartialEq derives to `StudioError` (it wraps `NodeDbError` via `#[source]`).
- Designed `AsyncView` around discrete props (the caller decodes the enum) rather than the generic `AsyncState<T>`, because Dioxus props must be `Clone + PartialEq` and `StudioError`/`T` do not satisfy that. The caller renders `Loaded` itself and delegates the other three states.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Scoped `#[allow(dead_code)]` on `AsyncState` / `from_value` / `AsyncView`**
- **Found during:** Tasks 1 & 2 (clippy `-D warnings` gate)
- **Issue:** This plan builds a primitive that nothing consumes until 01-04. In a binary crate, `pub` does not suppress dead-code, so clippy would flag the unused enum/method/component.
- **Fix:** Added a scoped, documented `#[allow(dead_code)]` (`// Consumed by plan 01-04`) on `AsyncState`, `from_value`, and `AsyncView` — mirroring how 01-01 handled `StudioError`. 01-04 removes them when it wires the popover.
- **Files modified:** `nodedb-studio/src/services/async_state.rs`, `nodedb-studio/src/components/async_view.rs`
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0.
- **Committed in:** `44d2296` (Task 1), `bfc1d3c` (Task 2)

---

**Total deviations:** 1 auto-fixed (1 blocking). Anticipated by the plan's critical notes.
**Impact on plan:** None — scoped, self-documenting allows that disappear once 01-04 references the primitive.

## Deferred Issues (Out of Scope)

**Pre-existing `cargo fmt` violation in `app.rs`** — `cargo fmt --all -- --check` exits 1 on `nodedb-studio/src/app.rs:12` (import ordering). Introduced by commit `60f6ad1` (plan 01-02), NOT by 01-03; this plan's new files are rustfmt-clean. Out of scope (unrelated file, prior plan). Logged to `.planning/phases/01-async-seam-error-foundation/deferred-items.md` with a one-line `cargo fmt --all` fix for 01-04 / a follow-up `chore`. Build, clippy, and nextest are all green.

## Issues Encountered
None for this plan's scope beyond the deferred pre-existing fmt issue above.

## User Setup Required
None.

## Next Phase Readiness
- `AsyncState<T>` + `from_value` + `AsyncView` are ready for 01-04 to wire into the seam-backed views (popover) and for phases 3-6 to reuse. 01-04 will: call `from_value` on a `use_resource` read, render `Loaded` itself, delegate the other three states to `AsyncView`, and drop the `#[allow(dead_code)]` markers.
- No blockers.

## Known Stubs
None — `AsyncState` and `AsyncView` are complete, tested primitives. They are intentionally unconsumed by non-test code until 01-04 (documented `#[allow(dead_code)]`), which is the plan's design, not a stub.

## Self-Check: PASSED

All created/modified files exist on disk; both task commits (`44d2296`, `bfc1d3c`) exist in history; all acceptance greps (enum, from_value, both `mod` registrations, `#[component]`, `AsyncView`, Retry+retriable) pass.

---
*Phase: 01-async-seam-error-foundation*
*Completed: 2026-06-14*
