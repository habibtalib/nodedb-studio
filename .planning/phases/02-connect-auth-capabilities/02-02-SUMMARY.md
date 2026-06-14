---
phase: 02-connect-auth-capabilities
plan: 02
subsystem: services (live connect-and-probe path)
tags: [connect, auth, capabilities, identity, secret-hygiene, oidc, wave-2, tdd]
dependency_graph:
  requires:
    - "Plan 02-01: derive_capabilities, parse_identity/parse_databases, reshaped SavedConnection (AuthMode/TlsSettings)"
    - "Phase 1: StudioError + From<NodeDbError> + is_retriable; async ConnectionService seam; NodeDbConnectionService stub"
    - "nodedb_client::{ConnectionBuilder, NativeClient, NativeClient::new, NodeDb, NodeDbError}"
    - "nodedb_client::native::{pool::PoolConfig, connection::TlsConfig}; nodedb_types::protocol::AuthMethod::OidcBearer"
  provides:
    - "services::auth::{Secret, AuthInput, build_client} — per-auth-mode NativeClient constructor (CONN-01/02)"
    - "services::nodedb_service::NodeDbConnectionService::connect_real — live connect-and-probe (CONN-03/04/05/06)"
    - "services::nodedb_service::NodeDbConnectionService::disconnect — client release (CONN-07)"
    - "services::nodedb_service::describe_connection — pure probe->ActiveConnection assembly (testable)"
  affects:
    - "Plan 02-03 (connect-manager UI calls connect_real with the in-session secret; ⌘D calls disconnect; removes the scoped dead_code allows)"
tech_stack:
  added: []
  patterns:
    - "Redacting Secret newtype (no derived Debug/Serialize/Clone; manual Debug -> Secret(***)) for in-session secret hygiene (D-01)"
    - "OIDC via direct PoolConfig + NativeClient::new (builder has no OIDC setter)"
    - "Lazy-pool forcing round-trip: identity SELECT runs the handshake BEFORE reading capabilities()/server_version()"
    - "Interior-mutable client (RefCell<Option<NativeClient>>) preserving #[derive(Default)] + Rc-object-safety; borrow_mut never held across .await"
    - "Pure describe_connection assembly unit-tested with fixture QueryResults (no live server)"
key_files:
  created:
    - "nodedb-studio/src/services/auth.rs"
  modified:
    - "nodedb-studio/src/services/mod.rs"
    - "nodedb-studio/src/services/nodedb_service.rs"
decisions:
  - "Real connect is an inherent connect_real(&saved, AuthInput) method, NOT a trait change (trait shape locked, ask-first); the name-only trait connect() still returns NotConnected"
  - "Secret newtype carries the in-session secret; AuthInput enum pairs it with each auth mode; neither reaches SavedConnection/disk/Debug/tracing (D-01)"
  - "Single identity SELECT is the forcing round-trip (Target 2/6): it opens the socket, runs the handshake, authenticates, and returns identity in one trip; SHOW DATABASES is best-effort (.ok())"
  - "Client held in RefCell<Option<NativeClient>> (research choice B); Default preserved, object-safe behind Rc; disconnect sets it to None to drop pool/sockets (CONN-07)"
  - "Scoped #[allow(dead_code)] on the connect surface (auth module + connect_real/disconnect + probe consts) until Plan 03 wires the UI — Plan 01 precedent"
metrics:
  duration: "5 min"
  completed: "2026-06-14"
  tasks: 2
  files: 3
---

# Phase 2 Plan 02: Live Connect-and-Probe Path Summary

Filled the inert `NodeDbConnectionService` stub with a real connect path: a per-auth-mode `NativeClient` constructor with a redacting in-session `Secret` (builder for trust/password/api_key, direct `PoolConfig` + `AuthMethod::OidcBearer` for OIDC), a lazy-pool-aware connect-and-probe that forces one identity `SELECT` round-trip before reading real capabilities/version, best-effort identity derivation into an `ActiveConnection`, probe-failure mapping to `StudioError` that never enters the connected state, and a disconnect that drops the held client — all unit-tested without a live server, clippy `-D warnings` clean.

## What Was Built

### Task 1 — per-auth-mode client builder + Secret hygiene (TDD) — `f09526f`
- `services/auth.rs` (new): `Secret(String)` newtype with **no derived `Debug`/`Serialize`/`Clone`** and a manual `Debug` that prints `Secret(***)`; `expose()` is the single sanctioned raw-read site (feeds the builder/PoolConfig). `AuthInput { Trust, Password, ApiKey, Oidc }` carries the secret per mode (studio-owned, exhaustive — no `_ =>`).
- `build_client(host, port, default_db, &TlsSettings, Option<Duration>, AuthInput) -> NativeClient`: trust/password/api_key via `ConnectionBuilder` (its `build()` precedence is api_key→password→trust); OIDC via a direct `PoolConfig { auth: AuthMethod::OidcBearer { token, provider }, .. }` + `NativeClient::new` (the builder has no OIDC setter, research Target 5). `to_client_tls` maps `TlsSettings` → client `TlsConfig` with all four fields explicit (no `..Default`). 5s default timeout when unset.
- Registered `pub mod auth;` in `services/mod.rs`.
- 6 tests: `builds_client_trust/password/api_key`, `oidc_builds_via_poolconfig`, `secret_is_not_in_debug`, `secret_expose_returns_raw_value`.

### Task 2 — connect-and-probe + derivation + disconnect (TDD) — `04d707b`
- `services/nodedb_service.rs`: reshaped the client field to `RefCell<Option<NativeClient>>` — `#[derive(Default)]` preserved (`RefCell` is `Default`), object-safe behind `Rc<dyn ConnectionService>` (`stub_is_object_safe_behind_rc` still green).
- `connect_real(&self, &SavedConnection, AuthInput) -> Result<ActiveConnection, StudioError>`: builds the client (sync, lazy), forces the handshake with `execute_sql("SELECT current_user, current_role, current_database")` (the `?` maps any `NodeDbError` → `StudioError`; an `Err` returns WITHOUT storing a client or producing a session — CONN-03), reads the now-populated `capabilities()`/`server_version()`, runs a best-effort `SHOW DATABASES` (`.ok()`), assembles via `describe_connection`, then stores the live client in a tight non-async `borrow_mut()` (never held across `.await`).
- `describe_connection(caps_bits, version, who, dbs, form_user, conn_name, default_db) -> ActiveConnection`: pure assembly over `derive_capabilities` + `parse_identity` + `parse_databases`; composes `sub` as `"nodedb · {version} · {n} dbs"` (or `"nodedb · {n} dbs"` when version is empty).
- `disconnect(&self)`: `*self.client.borrow_mut() = None;` — drops the `NativeClient` so its pool/sockets close (CONN-07). Idempotent.
- The name-only trait `connect()`/`list_connections()`/`notifications()` still return `NotConnected` (the real path is `connect_real`; the data path is a later phase).
- Removed the Task-1 module-level `dead_code` allow's "Task 2" framing and re-scoped it to "Plan 03" (the actual UI consumer); added a matching scoped allow on the connect surface in `nodedb_service.rs`.
- 7 tests: `connect_error_maps_to_studio_error` (Connection retriable / Auth not), `connect_error_does_not_set_active`, `describe_connection_builds_active_from_probe`, `describe_connection_uses_fallbacks_on_empty_probe`, `disconnect_releases`, plus the carried `stub_returns_not_connected` / `stub_is_object_safe_behind_rc`.

## Requirements Satisfied
- **CONN-01:** client built from saved config + secret via the builder — `builds_client_*`.
- **CONN-02:** OIDC builds via direct `PoolConfig` with `AuthMethod::OidcBearer` — `oidc_builds_via_poolconfig`.
- **CONN-03:** probe failure → `StudioError`, returns `Err`, no client stored, no session — `connect_error_*`.
- **CONN-04:** success returns `ActiveConnection` + holds the live client — `describe_connection_*` + `connect_real` store.
- **CONN-05:** `capabilities()` → studio `Capabilities` after the forcing probe — `derive_capabilities` wired post-probe.
- **CONN-06:** best-effort identity from the probe with fallbacks — `describe_connection_*`.
- **CONN-07:** disconnect drops the client — `disconnect_releases`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Scoped `#[allow(dead_code)]` on the Plan-03-only connect surface**
- **Found during:** Tasks 1 and 2 (clippy `-D warnings`, the CI gate)
- **Issue:** The entire connect chain (`auth` module + `connect_real`/`disconnect` + the probe consts) is unreachable from non-test code until Plan 03 wires the connection-manager UI to call `connect_real`/`disconnect`. Clippy's `dead_code` lint failed the `-D warnings` gate at each task commit.
- **Fix:** Added scoped `#[allow(dead_code)]` (module-level on `auth.rs`; on the `impl NodeDbConnectionService` block and the two probe consts in `nodedb_service.rs`) with doc notes pointing to Plan 03 — mirroring the Plan 01 precedent for the identity/capability scaffolding. To be removed in Plan 03 once the UI consumes the surface.
- **Files modified:** `nodedb-studio/src/services/auth.rs`, `nodedb-studio/src/services/nodedb_service.rs`
- **Commits:** `f09526f`, `04d707b`

## Verification
- `cargo nextest run -p nodedb-studio`: 48 passed, 0 failed (13 new this plan: 6 auth + 7 service).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean (only the unrelated transitive `block v0.1.6` future-incompat note).
- `cargo fmt --all --check`: clean.
- Acceptance greps: `auth.rs` has `pub fn build_client`/`pub struct Secret`/`pub enum AuthInput`, `Secret(***)`, `AuthMethod::OidcBearer`, `PoolConfig`/`NativeClient::new`; `nodedb_service.rs` has `connect_real`/`describe_connection`/`disconnect`/`execute_sql`, the identity probe string, and `derive_capabilities`.
- Forbidden-pattern scan over non-test code: no `.unwrap()`/`.expect(`/`panic!`/`todo!`/`serde_json`/`_ =>` (the single `_ =>` match is inside a doc comment, not code).
- Discipline: no `RefCell` borrow held across `.await` (build + two awaits, then a single `*self.client.borrow_mut() = ...`); trait shape unchanged; `MockConnectionService` still functional (its tests pass); the in-session `Secret` never reaches `SavedConnection`/`Debug`/`tracing`.

## Known Stubs
None. The trait `connect()`/`list_connections()`/`notifications()` returning `NotConnected` is intentional and documented: the real connect is `connect_real` (carries the secret the name-only trait cannot), wired into the UI in Plan 03; the live data path (registry/notifications) lands in a later phase. The scoped `dead_code` allows are scaffolding-bridges (Plan 01 precedent), not unfinished functionality — every item is fully implemented and unit-tested; only its UI caller is pending Plan 03.

## Self-Check: PASSED
All created/modified files exist on disk; both task commits (`f09526f`, `04d707b`) are present in git history.
