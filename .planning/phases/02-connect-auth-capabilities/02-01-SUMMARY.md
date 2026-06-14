---
phase: 02-connect-auth-capabilities
plan: 01
subsystem: services + state (connect-config foundation)
tags: [capabilities, identity, saved-connection, auth-mode, tls, wave-0, tdd]
dependency_graph:
  requires:
    - "Phase 1: StudioError, async ConnectionService seam, MockConnectionService"
    - "nodedb_client::Capabilities::from_raw / supports_* predicates"
    - "nodedb_types::{QueryResult, Value, protocol::CAP_*}"
  provides:
    - "services::capabilities_map::derive_capabilities(u64) -> Capabilities (CONN-05)"
    - "services::identity::{Identity, parse_identity, parse_databases} (CONN-06)"
    - "state::connections_registry::{SavedConnection (connect-config), AuthMode, TlsSettings}"
  affects:
    - "Plan 02 (live connect path consumes derive_capabilities + identity parsers + SavedConnection fields)"
    - "Plan 03 (connect form reads AuthMode/TlsSettings; topbar chip uses Identity)"
tech_stack:
  added: []
  patterns:
    - "Pure-Rust, renderer-free unit tests for capability mapping and identity parse"
    - "Safe Value::as_str accessors with documented fallback chains (no unwrap/panic)"
    - "Mock synthesizes a session at the seam after the connect-config reshape (seam discipline)"
key_files:
  created:
    - "nodedb-studio/src/services/capabilities_map.rs"
    - "nodedb-studio/src/services/identity.rs"
  modified:
    - "nodedb-studio/src/services/mod.rs"
    - "nodedb-studio/src/state/connections_registry.rs"
    - "nodedb-studio/src/data/mock.rs"
    - "nodedb-studio/src/services/connection_service.rs"
decisions:
  - "derive_capabilities sets vector=true / cluster=false / readonly=false (D-04/05/06): no server bit, conservative defaults"
  - "SavedConnection carries no secret field (D-01); secrets are in-session only (Plan 02)"
  - "ConnectionProfile + SavedConnection::open() removed; mock connect synthesizes a session via derive_capabilities(u64::MAX)"
  - "AuthMode is studio-owned 4-variant enum; mTLS demoted to TlsSettings (transport, not auth)"
metrics:
  duration: "6 min"
  completed: "2026-06-14"
  tasks: 2
  files: 6
---

# Phase 2 Plan 01: Connect-Config Data Foundation Summary

Pure-Rust, fully unit-tested Wave 0 scaffolding for Phase 2: a bit-by-bit server-capability→studio mapping (`derive_capabilities`), a panic-free identity/databases parser over `QueryResult` probe rows with documented fallbacks, and a reshaped secret-free `SavedConnection` (connect-config only) with `AuthMode`/`TlsSettings` — all green under nextest, with `MockConnectionService` still connecting after the reshape.

## What Was Built

### Task 1 — capability-map + identity-parse seam helpers (TDD)
- `services/capabilities_map.rs`: `derive_capabilities(bits: u64) -> Capabilities` maps the server bitmask through `nodedb_client::Capabilities::from_raw().supports_*()` — `CAP_GRAPHRAG→graph`, `CAP_FTS→fts`, `CAP_SPATIAL→spatial`, `CAP_STREAMING→streams`, `CAP_TIMESERIES→timeseries`, `CAP_CRDT→sync`; `vector=true` (core engine, D-06), `cluster=false` (D-06), `readonly=false` (D-05). `CAP_COLUMNAR`/`CAP_MSGPACK` ignored (no studio field). 6 bit-level tests.
- `services/identity.rs`: `Identity { user, role, current_database }`, `parse_identity(who, form_user, conn_name, default_db)` and `parse_databases(dbs, current_database)`. All parsing uses `Value::as_str()` + `.iter().position(...)`; fallback chain user→form_user→conn_name, role→"", current_database→default_db, databases→`vec![current_database]`. 4 fixture tests.
- Registered both in `services/mod.rs` (pub mod only).
- Commit: `b061bf3`

### Task 2 — SavedConnection reshape (D-09) + mock update
- `state/connections_registry.rs`: `SavedConnection` reshaped to connect-config (host, port, auth_mode, username, default_database, tls, connect_timeout_secs) with the card-display fields retained; **no secret field** (D-01). Added `AuthMode { Trust, Password, ApiKey, OidcBearer }` and `TlsSettings`. Removed `ConnectionProfile` and `SavedConnection::open()`.
- `data/mock.rs`: rebuilt the 4 literals to the new shape; fixed port `2480→6433`; dropped the now-unused `Capabilities`/`ConnectionProfile` imports (kept `Capability` for notifications).
- `services/connection_service.rs`: `MockConnectionService::connect` synthesizes an `ActiveConnection` for connectable entries via `derive_capabilities(u64::MAX)` (then narrows `readonly` by `ConnStatus`), falling back to `NotConnected` for unknown/offline names. Removed the temporary `dead_code` allow on `derive_capabilities` (now a live consumer).
- Commit: `8296a58`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Scoped `#[allow(dead_code)]` on Plan-02-only identity items**
- **Found during:** Task 1 (clippy `-D warnings`)
- **Issue:** `Identity`, `first_row_cell`, `parse_identity`, `parse_databases` are consumed by the live connect path in Plan 02, so clippy's `dead_code` lint failed the CI gate at Task 1 commit time.
- **Fix:** Added scoped `#[allow(dead_code)]` with a doc note pointing to Plan 02 (mirrors the Phase 1 precedent for `AsyncState`/`from_value`). `derive_capabilities` carried a temporary allow for the Task 1 commit, removed in Task 2 once the mock connect wired it.
- **Files modified:** `nodedb-studio/src/services/identity.rs`, `nodedb-studio/src/services/capabilities_map.rs`
- **Commits:** `b061bf3`, `8296a58`

## Verification

- `cargo nextest run -p nodedb-studio`: 37 passed, 0 failed (10 new this plan: 6 capability-map + 4 identity).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean (only the unrelated transitive `block v0.1.6` future-incompat note).
- `cargo fmt --all --check`: clean.
- Acceptance greps: `AuthMode`/`TlsSettings`/all connect-config fields present; zero secret-field declarations; zero `ConnectionProfile`; zero `2480` and six `6433` in mock.
- Seam discipline: `MockConnectionService` connects (`mock_connect_known_name_returns_session`) and rejects unknown/offline (`mock_connect_unknown_name_is_not_connected`).

## Known Stubs

None. (The two "placeholder" string matches in the scan are pre-existing doc comments describing UI em-dash display and the neutral server tag, not code stubs.)

## Self-Check: PASSED

All created/modified files exist on disk; both task commits (`b061bf3`, `8296a58`) are present in git history.
