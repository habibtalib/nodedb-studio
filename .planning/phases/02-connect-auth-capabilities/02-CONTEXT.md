# Phase 2: Connect, Auth & Capabilities - Context

**Gathered:** 2026-06-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Open a **real** NodeDB session from the connection manager using any supported auth mode
(trust / password / api_key / OIDC bearer), transition into the connected `Studio` shell
bound to that live `NativeClient`, and drive every shell element (rail, admin sub-tabs,
capability-gated views, topbar identity chip) from the server's **real** capabilities and
identity — not mock flags. Disconnect (⌘D) releases the client and returns to the manager.

This fills the `NodeDbConnectionService` stub from Phase 1 with a working `connect()` path.
**No data-path wiring** (SQL, collections, document/vector/graph/FTS/KV browsers) — those are
Phases 3–6. This phase is connect + auth + capability/identity negotiation only.

Requirements covered: CONN-01, CONN-02, CONN-03, CONN-04, CONN-05, CONN-06, CONN-07.

</domain>

<decisions>
## Implementation Decisions

### Credentials & saved-connection shape
- **D-01:** **Secrets live in-session only.** The saved-connection registry stores only
  non-secret connect params; the secret (password / api_key / OIDC token) is entered at
  connect time and held in memory for the live session only — never written to disk this
  milestone. Reconnect re-prompts for the secret. Rationale: PROJECT.md explicitly defers
  OS-keychain hardening to v2 ("start with in-session/registry handling; harden later").
- **D-09:** A `SavedConnection` stores the real connect target: **host, port, auth-mode,
  username, default database, TLS settings, and an optional connect-timeout override**
  (no secret). The current mock `ConnectionProfile` (pre-baked user/role/capabilities/
  databases) is **replaced** — identity and capabilities now come from the live server
  *after* connect, not from the saved entry. The saved entry is connect-config, not identity.

### Auth-mode form UX
- **D-02:** The connection form uses a **single "Auth method" dropdown that swaps the visible
  field set** per mode:
  - **trust** → username
  - **password** → username + password
  - **api_key** → token
  - **oidc_bearer** → token (+ optional provider hint)
  This replaces the stale mock picker ("Username+password / Token / mTLS") in
  `modals/new_connection.rs`, and **fixes the wrong port default** (mock shows `2480` →
  must default to NodeDB native `6433`). mTLS is transport, not an auth method — see Deferred.
- Note for planning: `ConnectionBuilder` exposes `.username()/.password()/.api_key()` but
  **no OIDC setter**, while `nodedb_types::protocol::AuthMethod::OidcBearer { token, provider }`
  exists. Wiring OIDC requires builder-level handling (construct `AuthMethod` directly or
  extend the builder) — see Claude's Discretion.

### Connect progress & error UX
- **D-03:** Connect progress + failures surface **inline on the connecting card** in the
  connection manager: card shows `Connecting…`, then on failure an inline error message + a
  **Retry** affordance — reusing the Phase 1 `AsyncState`/`AsyncView` primitive and
  `StudioError::is_retriable()`. Error categories: refused / network timeout → `Connection`;
  bad credentials / unauthorized → `Auth`. **The app never freezes and never enters the
  connected state on failure** (CONN-03). Retry re-runs the connect at the seam.

### Capability & identity derivation
- **D-04:** **Conservative capability gating.** The shell gates features only on capabilities
  the server **confirms**. The server returns a `u64` capability **bitmask**; map its bits to
  the studio's `Capabilities` struct (proposed, validate in research against
  `nodedb-types/src/protocol/mod.rs`): `CAP_GRAPHRAG→graph`, `CAP_FTS→fts`,
  `CAP_SPATIAL→spatial`, `CAP_STREAMING→streams`, `CAP_TIMESERIES→timeseries`,
  `CAP_CRDT→sync`. If `capabilities()` can't be fetched or a bit is absent, treat that
  feature as **unavailable (hidden)** — never show a tab that errors on use.
- **D-05:** **`readonly` is derived from the session role/permissions** (no capability bit
  exists for it). Research confirms the exact client-side signal; when indeterminate, lean
  writable (`readonly=false`) and flag for follow-up rather than blocking legitimate writes.
- **D-06:** Flags with **no server bit** — `vector`, `cluster` — are resolved in research:
  `vector` is expected to be a core engine (lean always-on once confirmed); `cluster` from
  server topology/limits if available, else off. Default off only when genuinely
  indeterminate (per D-04 conservative policy).

### Session lifecycle
- **D-07:** **Disconnect (⌘D) is immediate, no confirm** — close/drop the `NativeClient`,
  clear the active-session signal, return to the connection manager, release the session
  (CONN-07). Matches the existing ⌘D handler.
- **D-08:** The command-palette and connection-popover **"switch connection" actions reuse
  the real connect path** (disconnect current + connect to the chosen saved connection via
  the same seam). If the target needs a secret not held in session, route to the connect
  form/prompt rather than failing silently.

### Claude's Discretion
- Exact `ConnectionBuilder` wiring for OIDC (`AuthMethod::OidcBearer` direct construction vs
  a builder extension) — pick whatever compiles cleanly and keeps the seam clean.
- The exact server-bit → `Capabilities` mapping, validated against `protocol/mod.rs` `CAP_*`.
- The client-side source of identity (user, role, current database, databases list) for the
  topbar chip — research determines which client/trait methods or handshake fields provide
  these (only `capabilities()`, `limits()`, `server_version()` are confirmed present).
- Connect-timeout default (client default is 5s) when no per-connection override is set.
- Whether `NodeDbConnectionService` becomes the **default** service impl on a real connect
  (vs. `MockConnectionService` staying default until a connection is opened) — keep the mock
  working alongside per seam discipline.
- File/module layout (respect <500 LOC, `mod.rs` re-exports only).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 2 goal + the 5 success criteria (source of truth for "done")
- `.planning/REQUIREMENTS.md` — CONN-01..CONN-07 wording
- `.planning/PROJECT.md` — keychain deferral (Out of Scope), constraints, key decisions

### Carried-forward foundation (Phase 1)
- `.planning/phases/01-async-seam-error-foundation/01-CONTEXT.md` — async seam, `StudioError`, `AsyncState`, stub decisions
- `.planning/phases/01-async-seam-error-foundation/01-RESEARCH.md` — `StudioError` mapping table, `use_resource` API, native-feature notes, `?Send`
- `nodedb-studio/src/services/error.rs` — `StudioError` categories (Connection/Auth/Setup/NotConnected) connect errors map to
- `nodedb-studio/src/services/async_state.rs`, `nodedb-studio/src/components/async_view.rs` — loading/empty/error + Retry primitive to reuse
- `nodedb-studio/src/services/nodedb_service.rs` — the stub wrapping `Option<NativeClient>` to fill with a real connect

### Studio surfaces this phase modifies
- `nodedb-studio/src/services/connection_service.rs` — the async trait + `MockConnectionService`
- `nodedb-studio/src/app.rs` — service provision / context; where a real client session is held
- `nodedb-studio/src/views/connection_manager.rs` — connect entry point + inline card states (D-03)
- `nodedb-studio/src/modals/new_connection.rs` — the connection form to rework (D-02)
- `nodedb-studio/src/components/command_palette.rs`, `nodedb-studio/src/components/popovers/connection_popover.rs` — quick-switch (D-08)
- `nodedb-studio/src/components/topbar.rs` — identity chip (real user/role/db/version) (CONN-06)
- `nodedb-studio/src/state/connection.rs` — `ActiveConnection` + `Capabilities`/`Capability`
- `nodedb-studio/src/state/connections_registry.rs` — `SavedConnection` + `ConnectionProfile` (reshape per D-09)
- `nodedb-studio/src/routes.rs` — capability-gated routing / fallback (CONN-05)

### Client surface to wrap (../nodedb)
- `../nodedb/nodedb-client/src/native/builder.rs` — `ConnectionBuilder` (`username`/`password`/`api_key`/`database`/`tls`/`connect_timeout`/`build`); **no OIDC setter**
- `../nodedb/nodedb-types/src/protocol/auth.rs` — `AuthMethod` enum incl. `OidcBearer { token, provider }`
- `../nodedb/nodedb-types/src/protocol/mod.rs` — `CAP_*` capability bit constants (drives D-04 mapping)
- `../nodedb/nodedb-client/src/traits/core/trait_def.rs` — `capabilities() -> u64`, `limits() -> Limits`, `server_version() -> String`
- `../nodedb/nodedb-client/src/native/client/dispatch.rs` — `NativeClient` impls of the above
- `../nodedb/nodedb-types` `Limits` struct — `max_vector_dim`, `max_top_k`, `max_graph_depth`, etc.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `AsyncState<T>` + `AsyncView` (Phase 1) — drive the inline connecting/error/Retry card (D-03).
- `StudioError` categories — connect failures map to `Connection`/`Auth`/`Setup`; `is_retriable()` gates Retry.
- `NodeDbConnectionService` stub (`Option<NativeClient>`) — Phase 2 fills `connect()` via `ConnectionBuilder`.
- `ConnectionBuilder` fluent API — `addr`, `username`, `password`, `api_key`, `database`, `tls`, `connect_timeout`.
- `use_action`/`spawn` at the seam — connect is a one-shot action (use_action), not a use_resource read.
- `Capabilities`/`Capability` + `Capabilities::has()` — already gate the rail/admin/views; feed them real flags.

### Established Patterns
- Single seam, `Rc<dyn ConnectionService>` via context; mock must keep working (seam discipline).
- Capability-driven rendering: rail items, admin sub-tabs, routed views gate on `Capabilities` (CONN-05).
- Per-connection identity (no global account): topbar chip reflects the live session (CONN-06).
- Async at the seam only; never hold a signal guard across `.await`.

### Integration Points
- `app.rs` — holds the active `NativeClient`/service; transitions disconnected→connected on success (CONN-04).
- `connection_manager.rs` on_connect — real connect, inline states, never enters connected on failure (CONN-03).
- `topbar.rs` identity chip + ⌘D handler — show real identity; ⌘D releases the session (CONN-06/07).
- `routes.rs` — capability fallback redirect when a view's required cap is absent (CONN-05).

</code_context>

<specifics>
## Specific Ideas

- Fix the mock connection form: port default `2480` → `6433`; replace the auth picker with the
  D-02 dropdown (trust/password/api_key/oidc) that swaps fields.
- Inline card connect lifecycle: `Connecting…` → success enters shell / failure shows inline
  error + Retry (no freeze, no connected state on failure).
- Topbar chip must show real `user · role · current_database · N dbs · server_version`.

</specifics>

<deferred>
## Deferred Ideas

- **OS-keychain credential hardening** — PROJECT.md Out of Scope; in-session secrets only for v1.
- **Persisting secrets to disk** — explicitly rejected (D-01).
- **mTLS as an auth method** — the mock form listed it, but real auth modes are
  trust/password/api_key/oidc; TLS is transport config (`ConnectionBuilder.tls()`), captured as
  a saved-connection setting (D-09), not an auth mode.
- **Data-path wiring** — SQL (Phase 3), collections (Phase 4), document/vector/graph/FTS/KV
  browsers (Phases 5–6).
- **Streams / admin / sync / designer live wiring** — v2.

None of the discussion strayed outside the phase scope.

</deferred>

---

*Phase: 02-connect-auth-capabilities*
*Context gathered: 2026-06-14*
