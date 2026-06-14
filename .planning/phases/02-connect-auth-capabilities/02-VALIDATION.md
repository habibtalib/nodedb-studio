---
phase: 2
slug: connect-auth-capabilities
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-14
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `02-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo-nextest` 0.9.x; `#[tokio::test]` for async seam methods; inline `#[cfg(test)] mod tests` |
| **Config file** | none (nextest defaults) |
| **Quick run command** | `cargo nextest run -p nodedb-studio` |
| **Full suite command** | `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run` |
| **Estimated runtime** | ~30 seconds (quick); ~2–3 min (full gate, clippy-dominated) |

---

## Sampling Rate

- **After every task commit:** Run `cargo nextest run -p nodedb-studio`
- **After every plan wave:** Run full gate `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds (quick run)

---

## Per-Task Verification Map

> Planner fills `Task ID` / `Plan` / `Wave` columns when plans are written. Requirement → test-type rows are pre-derived from research.

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | CONN-01 | unit | `cargo nextest run -E 'test(builds_client_)'` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CONN-02 | unit | `cargo nextest run -E 'test(oidc_builds_via_poolconfig)'` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CONN-02 | manual/e2e | manual `cargo run` against live `:6433` | ❌ needs server | ⬜ pending |
| TBD | TBD | TBD | CONN-03 | unit + render | `cargo nextest run -E 'test(connect_error_)'` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CONN-04 | manual/e2e | manual `cargo run` against live `:6433` | ❌ needs server | ⬜ pending |
| TBD | TBD | TBD | CONN-05 | unit | `cargo nextest run -E 'test(maps_)'` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CONN-06 | unit | `cargo nextest run -E 'test(identity_parse_)'` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CONN-07 | unit (signal) | `cargo nextest run -E 'test(disconnect_releases)'` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `services/nodedb_service.rs` tests — client build per auth mode (CONN-01/02), connect-error mapping (CONN-03)
- [ ] capability-mapping tests — raw `u64` / client `Capabilities` → studio `Capabilities` per bit (CONN-05)
- [ ] identity-parse tests — fixture `QueryResult` rows → `ActiveConnection`, with fallbacks (CONN-06)
- [ ] `state/connections_registry.rs` reshape + tests — D-09 `SavedConnection` shape (remove/replace `ConnectionProfile`)
- [ ] `modals/new_connection.rs` rework — port `6433`, auth-mode dropdown, field-swap (D-02)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Each auth mode connects end-to-end to a live server | CONN-02 | Requires a running NodeDB `:6433` instance; no in-process server harness | `cargo run -p nodedb-studio`, connect with trust/password/api_key/oidc against live `:6433`; each reaches the shell |
| Successful connect transitions to the connected `Studio` shell | CONN-04 | Requires live handshake + capability/identity probe | `cargo run`, connect → shell renders with real caps/version |
| Topbar chip shows real identity + ⌘D releases session | CONN-06/07 | Identity probe + window-level ⌘D handler need a live session | `cargo run`, connect → chip shows user·role·db·version; ⌘D → returns to manager |

> Note: per research, identity (user/role/current db/databases) is **not** exposed by the client API (`authenticate()` discards `AuthResponse`). CONN-06 relies on an identity probe (SQL/system query) or documented fallbacks — manual e2e confirms the real values render.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
