//! Per-auth-mode `NativeClient` construction + in-session secret hygiene
//! (CONN-01/CONN-02).
//!
//! The studio builds a live `NativeClient` from a saved connect-config plus an
//! in-session secret. Trust / password / api_key go through the client's
//! `ConnectionBuilder` (its `build()` precedence is api_key → password → trust).
//! OIDC has **no builder setter**, so it is wired by constructing a `PoolConfig`
//! directly with `AuthMethod::OidcBearer` and calling `NativeClient::new`
//! (research Target 5).
//!
//! Secret hygiene (D-01): the password / api_key / OIDC token is held only in
//! the [`Secret`] wrapper, which deliberately does **not** derive `Debug`,
//! `Serialize`, or `Clone`. Its manual `Debug` redacts the value, so the secret
//! never leaks into `tracing`, panic output, or the saved registry. The only
//! call site that reads the raw bytes is [`Secret::expose`], used solely to feed
//! the builder / `PoolConfig`.
//!
//! Build is **synchronous and infallible** and opens no socket — the pool is
//! lazy, so the network round-trip (and any connect/auth error) happens later on
//! the first request (the identity probe in `nodedb_service`).
//!
//! NOTE: this module's public surface (`Secret`, `AuthInput`, `build_client`) is
//! consumed by the live connect path in Task 2 of this same plan
//! (`nodedb_service::connect_real`). Until that wiring lands in the next commit,
//! clippy's `dead_code` lint (a `-D warnings` CI gate failure) is suppressed by
//! the scoped allow below — mirroring the Plan 01 precedent for the identity /
//! capability helpers. The allow is removed in Task 2 once `connect_real` calls
//! `build_client`.
#![allow(dead_code)]

use std::time::Duration;

use nodedb_client::native::connection::TlsConfig;
use nodedb_client::native::pool::PoolConfig;
use nodedb_client::{ConnectionBuilder, NativeClient};
use nodedb_types::protocol::AuthMethod;

use crate::state::connections_registry::TlsSettings;

/// Default connect timeout when a saved connection has no override.
///
/// Matches the client's own `PoolConfig`/`ConnectionBuilder` default (5s).
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// A redacting wrapper around an in-session secret (password / api_key / OIDC
/// token).
///
/// Does **not** derive `Debug` / `Serialize` / `Clone`: the manual `Debug` impl
/// prints `Secret(***)` so the raw value can never reach `tracing`, logs, the
/// saved registry, or disk (D-01). The value is read exactly once, at the
/// builder / `PoolConfig` call site, via [`expose`](Self::expose).
pub struct Secret(String);

impl Secret {
    /// Wrap an in-session secret.
    pub fn new(s: impl Into<String>) -> Self {
        Secret(s.into())
    }

    /// Read the raw secret. The ONLY intended call site is feeding the
    /// `ConnectionBuilder` / `PoolConfig` at connect time.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// The auth input for a single connect attempt: the auth mode plus the
/// in-session secret it needs (studio-owned, exhaustive — no `_ =>`).
///
/// Built by the connect form (Plan 03) from the saved [`AuthMode`] + the secret
/// the user just typed, then consumed by [`build_client`].
///
/// [`AuthMode`]: crate::state::connections_registry::AuthMode
#[derive(Debug)]
pub enum AuthInput {
    /// No secret — the server trusts the username.
    Trust { username: String },
    /// Username + password.
    Password { username: String, password: Secret },
    /// API-key token.
    ApiKey { token: Secret },
    /// OIDC bearer token (+ optional provider hint).
    Oidc {
        token: Secret,
        provider: Option<String>,
    },
}

/// Convert the studio's serializable [`TlsSettings`] into the client's
/// `TlsConfig`. All fields matched explicitly (no `..Default`) so a new client
/// TLS field forces a compile error here rather than silently defaulting.
fn to_client_tls(t: &TlsSettings) -> TlsConfig {
    TlsConfig {
        enabled: t.enabled,
        ca_cert_path: t.ca_cert_path.as_ref().map(std::path::PathBuf::from),
        server_name: t.server_name.clone(),
        danger_accept_invalid_certs: t.danger_accept_invalid_certs,
    }
}

/// Start a `ConnectionBuilder` with the shared (auth-independent) options.
fn base(addr: &str, db: Option<String>, tls: TlsConfig, timeout: Duration) -> ConnectionBuilder {
    let mut b = ConnectionBuilder::new(addr)
        .tls(tls)
        .connect_timeout(timeout);
    if let Some(d) = db {
        b = b.database(d);
    }
    b
}

/// Build a `NativeClient` for one connect attempt.
///
/// Trust / password / api_key go through `ConnectionBuilder`; OIDC is wired via
/// a direct `PoolConfig` (the builder has no OIDC setter). Synchronous and
/// infallible — no socket is opened here (lazy pool); connect/auth errors
/// surface on the first request (the identity probe).
///
/// `timeout` of `None` uses the 5s client default.
pub fn build_client(
    host: &str,
    port: u16,
    default_db: Option<String>,
    tls: &TlsSettings,
    timeout: Option<Duration>,
    mode: AuthInput,
) -> NativeClient {
    let addr = format!("{host}:{port}");
    let timeout = timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT);
    let tls = to_client_tls(tls);

    match mode {
        AuthInput::Trust { username } => base(&addr, default_db, tls, timeout)
            .username(username)
            .build(),
        AuthInput::Password { username, password } => base(&addr, default_db, tls, timeout)
            .username(username)
            .password(password.expose())
            .build(),
        AuthInput::ApiKey { token } => base(&addr, default_db, tls, timeout)
            .api_key(token.expose())
            .build(),
        AuthInput::Oidc { token, provider } => {
            // The builder cannot produce OidcBearer — construct the PoolConfig
            // directly and reuse PoolConfig::default() for the pool-shape fields
            // the studio does not configure (research Target 5).
            let defaults = PoolConfig::default();
            NativeClient::new(PoolConfig {
                addr,
                auth: AuthMethod::OidcBearer {
                    token: token.expose().to_string(),
                    provider,
                },
                database: default_db,
                max_size: defaults.max_size,
                connect_timeout: timeout,
                idle_timeout: defaults.idle_timeout,
                tls,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tls() -> TlsSettings {
        TlsSettings::default()
    }

    #[test]
    fn builds_client_trust() {
        // Sync + infallible (lazy pool, no socket): success == it returns a
        // NativeClient without panicking.
        let _client: NativeClient = build_client(
            "localhost",
            6433,
            None,
            &tls(),
            None,
            AuthInput::Trust {
                username: "alice".to_string(),
            },
        );
    }

    #[test]
    fn builds_client_password() {
        let _client: NativeClient = build_client(
            "localhost",
            6433,
            Some("analytics".to_string()),
            &tls(),
            None,
            AuthInput::Password {
                username: "bob".to_string(),
                password: Secret::new("hunter2"),
            },
        );
    }

    #[test]
    fn builds_client_api_key() {
        let _client: NativeClient = build_client(
            "localhost",
            6433,
            None,
            &tls(),
            None,
            AuthInput::ApiKey {
                token: Secret::new("key-abc"),
            },
        );
    }

    #[test]
    fn oidc_builds_via_poolconfig() {
        // The OIDC branch must NOT use ConnectionBuilder (it cannot make
        // OidcBearer) — it goes through PoolConfig + NativeClient::new. We can
        // only assert it returns a NativeClient (build is infallible/lazy).
        let _client: NativeClient = build_client(
            "localhost",
            6433,
            None,
            &tls(),
            Some(Duration::from_secs(3)),
            AuthInput::Oidc {
                token: Secret::new("jwt-token"),
                provider: Some("okta".to_string()),
            },
        );
    }

    #[test]
    fn secret_is_not_in_debug() {
        let secret = Secret::new("hunter2");
        let shown = format!("{secret:?}");
        assert!(
            !shown.contains("hunter2"),
            "secret leaked in Debug: {shown}"
        );
        assert_eq!(shown, "Secret(***)");
    }

    #[test]
    fn secret_expose_returns_raw_value() {
        // expose() is the single sanctioned read site (feeds builder/PoolConfig).
        assert_eq!(Secret::new("hunter2").expose(), "hunter2");
    }
}
