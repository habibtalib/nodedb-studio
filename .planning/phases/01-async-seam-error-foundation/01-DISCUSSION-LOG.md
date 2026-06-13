# Phase 1: Async Seam & Error Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-14
**Phase:** 01-async-seam-error-foundation
**Areas discussed:** Async mechanism / dep, Error model shape, Loading/empty/error pattern, Stub service shape

---

## Gray Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| Async mechanism / dep | async_trait (new dep, ask-first) vs hand-boxed futures | ✓ |
| Error model shape | thin passthrough vs categorized studio error | ✓ |
| Loading/empty/error pattern | reusable shape + which view proves it in Phase 1 | ✓ |
| Stub service shape | what NodeDbConnectionService holds/returns pre-Phase-2 | ✓ |

**User's choice:** All four areas.

---

## Async mechanism / dependency

| Option | Description | Selected |
|--------|-------------|----------|
| Add async-trait crate | Idiomatic; nodedb-client uses it; clean trait + impls; one new dep (ask-first gate) | ✓ |
| Hand-boxed futures | No dep; Pin<Box<dyn Future>> returns by hand; boilerplate, error-prone | |

**User's choice:** Add async-trait crate.
**Notes:** Seam is `Rc<dyn ConnectionService>`, so native async-fn-in-trait is not dyn-compatible. Version to be pinned `0.1`. `?Send` vs `Send` left to executor to verify against `Rc`/`NativeClient` under Dioxus desktop's tokio runtime.

---

## Error model shape

| Option | Description | Selected |
|--------|-------------|----------|
| Categorized studio error | thiserror enum (Connection/Auth/NotFound/Conflict/ReadOnly/Server/Setup) mapped from NodeDbError | ✓ |
| Thin passthrough | One variant wrapping NodeDbError + a couple studio-only variants | |

**User's choice:** Categorized studio error.

### Error detail / retry follow-up

| Option | Description | Selected |
|--------|-------------|----------|
| Preserve source + retriable | Each variant keeps NodeDbError as #[source]; expose is_retriable() for Retry UX | ✓ |
| Message-only variants | Variants carry just a String; loses error chain + retriable signal | |

**User's choice:** Preserve source + retriable.
**Notes:** Variant set to be validated against the real `ErrorDetails` enum during planning.

---

## Loading / empty / error pattern

| Option | Description | Selected |
|--------|-------------|----------|
| AsyncState enum + component | AsyncState<T> { Loading, Empty, Loaded(T), Error(StudioError) } + shared Dioxus component | ✓ |
| Per-view ad-hoc states | Each phase writes its own match on use_resource result | |

**User's choice:** AsyncState enum + component.

### Phase-1 proof scope

| Option | Description | Selected |
|--------|-------------|----------|
| Prove on mock via async | Route one existing read (connection list / notifications) through async seam + use_resource + AsyncState | ✓ |
| Helper only, no view changes | Build + unit-test the helper; wire no view; pattern ships unproven | |

**User's choice:** Prove on mock via async.
**Notes:** Planner picks the lowest-risk read (connection-manager list vs notifications).

---

## Stub service shape

| Option | Description | Selected |
|--------|-------------|----------|
| Client slot + NotConnected | Holds Option<NativeClient> (None until Phase 2); data methods return typed NotConnected; no panics | ✓ |
| Empty marker struct | Zero-field struct returning Unimplemented/NotConnected; Phase 2 redesigns internals | |

**User's choice:** Client slot + NotConnected.
**Notes:** Real connect/auth wiring explicitly deferred to Phase 2.

---

## Claude's Discretion

- Exact `async-trait` `0.1.*` patch version; `?Send` vs `Send`.
- Final categorized-error variant names, validated against `ErrorDetails`.
- Which single read view demonstrates `AsyncState` in Phase 1.
- Module/file layout for the new error type and `AsyncState`.

## Deferred Ideas

- Real connect/auth + capability negotiation + identity chip — Phase 2.
- Making `NodeDbConnectionService` the active impl — Phase 2.
- Wiring any data-path method — Phases 3–6.
- Simulated mock latency for loading states — deferred unless needed.

## Flagged (not a design decision)

- `.cargo/config.toml` patch file is missing and must be created (pointing at `../nodedb`) before the stub compiles. STATE.md pre-Phase-1 todos must be done first.
