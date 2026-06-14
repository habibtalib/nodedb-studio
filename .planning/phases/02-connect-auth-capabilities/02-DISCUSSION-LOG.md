# Phase 2: Connect, Auth & Capabilities - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-14
**Phase:** 02-connect-auth-capabilities
**Areas discussed:** Saved-connection shape & credentials, Auth-mode form UX, Connect error & progress UX, Capability/identity derivation & fallback

---

## Saved-connection shape & credentials

### Secret storage
| Option | Description | Selected |
|--------|-------------|----------|
| In-session only | Saved entry stores non-secret params; secret entered at connect, in-memory for session; reconnect re-prompts | ✓ |
| Saved in registry file | Persist secret with the saved connection (plaintext/obfuscated) | |
| Hybrid (remember + optional cache) | Remember non-secret fields; per-connection 'remember secret for session' toggle, default off | |

**User's choice:** In-session only. Aligns with PROJECT.md deferring OS-keychain hardening.

### Saved connect params (multi-select)
| Option | Selected |
|--------|----------|
| Default database | ✓ |
| TLS settings | ✓ |
| Connect timeout override | ✓ |

**User's choice:** All three. A SavedConnection stores host/port/auth-mode/username + default database + TLS + connect-timeout override (no secret).

---

## Auth-mode form UX

| Option | Description | Selected |
|--------|-------------|----------|
| Dropdown that swaps fields | One auth-method select; fields change per mode (trust/password/api_key/oidc); fix port default to 6433 | ✓ |
| All fields always visible | Show username/password/token/provider together | |
| Minimal set (password + token only) | Support only password + api_key; defer trust/OIDC | |

**User's choice:** Dropdown swapping field sets.
**Notes:** Real auth modes trust/password/api_key/oidc_bearer; ConnectionBuilder lacks an OIDC setter (planning to handle via AuthMethod::OidcBearer). mTLS dropped as an auth method.

---

## Connect error & progress UX

| Option | Description | Selected |
|--------|-------------|----------|
| Inline on the card | Connecting… → inline error + Retry (reuse AsyncView/AsyncState + is_retriable); refused/timeout→Connection, bad creds→Auth | ✓ |
| Global banner/notification | Status + errors in a top banner / notifications surface | |
| Blocking modal | Modal during connect, blocks until dismissed/retried | |

**User's choice:** Inline on the card. App never freezes or enters connected state on failure.

---

## Capability/identity derivation & fallback

### Capability fallback policy
| Option | Description | Selected |
|--------|-------------|----------|
| Conservative — hide if unconfirmed | Gate only on confirmed server bits; absent/failed → hidden | ✓ |
| Permissive — show, let calls fail | Default unknown caps on; per-view errors on unsupported calls | |
| Mixed | Confirmed bits strict; vector always-on, cluster from topology, readonly from role | |

**User's choice:** Conservative. Map server u64 bitmask → studio Capabilities; unconfirmed → hidden.

### Readonly derivation
| Option | Description | Selected |
|--------|-------------|----------|
| From session role/permissions | readonly derived from identity's role/permissions | ✓ |
| Always writable in v1 | readonly=false, defer detection | |
| From a write-probe | Harmless permission check on connect | |

**User's choice:** From session role/permissions (research confirms the signal; indeterminate → lean writable).

---

## Session lifecycle

### Disconnect (⌘D)
| Option | Description | Selected |
|--------|-------------|----------|
| Immediate, no confirm | Drop client, clear active signal, return to manager | ✓ |
| Confirm first | Prompt before disconnecting | |

**User's choice:** Immediate, no confirm.

### Quick-switch (palette + popover)
| Option | Description | Selected |
|--------|-------------|----------|
| Reuse the real connect path | Switch = disconnect + connect via the same seam; route to form/prompt if secret missing | ✓ |
| Disable quick-switch in v1 | Connect only from the manager | |

**User's choice:** Reuse the real connect path.

## Claude's Discretion
- Exact OIDC wiring in ConnectionBuilder; capability-bit→flag mapping (validate vs protocol/mod.rs); identity source (user/role/databases) on the client; connect-timeout default; whether NodeDbConnectionService becomes the default impl on connect; file/module layout.

## Deferred Ideas
- OS-keychain hardening; persisting secrets to disk; mTLS as an auth method; data-path wiring (Phases 3–6); streams/admin/sync live wiring (v2).
