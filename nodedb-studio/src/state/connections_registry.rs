//! The registry of saved connections shown on the Connection Manager and in
//! the connection-switch popover.
//!
//! A saved entry is **connect-config only** (D-09): host, port, auth-mode,
//! username, default database, TLS, and an optional connect-timeout override.
//! It carries **no secret** (D-01) — the password / api_key / OIDC token is
//! entered at connect time, held in memory for the live session only, and never
//! written to disk, `Debug`, or `tracing`. Identity (user, role, databases) and
//! capabilities now come from the **live server after connect**, not from the
//! saved entry. The connect path in Plan 02 builds an `ActiveConnection` from a
//! live probe; this struct is no longer the source of session identity.
//!
//! A saved connection is not necessarily reachable. `ConnStatus::Offline`
//! renders as a disabled card, mirroring the mockup's `test-nodedb`.

use serde::{Deserialize, Serialize};

/// Reachability/credential status as shown by the card status pill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnStatus {
    Online,
    ReadOnly,
    Offline,
}

impl ConnStatus {
    /// Whether a card with this status can be connected to at all.
    pub fn is_connectable(self) -> bool {
        !matches!(self, ConnStatus::Offline)
    }
}

/// How the studio authenticates to the server. mTLS is **not** an auth mode —
/// it is transport/TLS config (see [`TlsSettings`]). The secret that pairs with
/// each mode (password / api_key / OIDC token) is supplied at connect time and
/// never stored on the saved entry (D-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMode {
    /// No secret — the server trusts the username.
    Trust,
    /// Username + password.
    Password,
    /// API-key token.
    ApiKey,
    /// OIDC bearer token (+ optional provider hint).
    OidcBearer,
}

/// Transport-layer TLS settings for a saved connection. Mirrors the client's
/// `TlsConfig` shape (research Target 5); the conversion to
/// `nodedb_client::native::connection::TlsConfig` happens at the connect seam
/// in Plan 02. Studio-owned so it can be serialized into the saved registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TlsSettings {
    pub enabled: bool,
    pub ca_cert_path: Option<String>,
    pub server_name: Option<String>,
    pub danger_accept_invalid_certs: bool,
}

/// One entry in the saved-connections registry — connect-config only (D-09).
///
/// Carries no secret field (D-01). Identity and capabilities are derived from
/// the live server after connect, not pre-baked here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedConnection {
    pub name: String,
    /// Meta line on the card (e.g. "nodedb · single-node").
    pub meta: String,
    /// Secondary chip line; may be recomputed from the live session in Plan 03.
    pub sub: String,
    pub status: ConnStatus,
    /// Card stats. `None` renders as the mockup's em-dash placeholder.
    pub db_count: Option<u32>,
    pub ping: Option<String>,
    pub server: String,

    // ── connect-config (D-09) ───────────────────────────────────────────────
    pub host: String,
    /// Native MessagePack port; defaults to `6433` (`DEFAULT_NATIVE_PORT`).
    pub port: u16,
    pub auth_mode: AuthMode,
    /// Username for trust/password modes; `None` for api_key / oidc_bearer.
    pub username: Option<String>,
    /// Default database to select on connect; `None` => server default.
    pub default_database: Option<String>,
    pub tls: TlsSettings,
    /// Per-connection connect-timeout override; `None` => client default (5s).
    pub connect_timeout_secs: Option<u64>,
}
