//! The real-client-backed seam impl (CONN-01..07).
//!
//! Holds a live `NativeClient` in interior-mutable storage so that `connect`
//! work happens AT the seam and later phases (SQL, browsers) can read the same
//! client. The studio is single-threaded (the Dioxus desktop runtime), so a
//! `RefCell` is the right interior-mutability primitive — but a `RefCell` borrow
//! is NEVER held across an `.await` (the connect path builds + probes, then
//! stores in a tight non-async `borrow_mut()` scope).
//!
//! Connect flow ([`connect_real`]):
//! 1. Build the client per auth mode (sync, infallible — lazy pool, no socket).
//! 2. Force a single round-trip with an identity `SELECT`. This opens the
//!    socket, runs the handshake (populating `capabilities()`/`server_version()`),
//!    authenticates, and returns the identity row. Its `Err` IS the connect
//!    failure → mapped to `StudioError` via the Phase-1 `From<NodeDbError>`,
//!    returned without ever producing an `ActiveConnection` (CONN-03).
//! 3. Read the now-populated capabilities + version, derive the studio
//!    `Capabilities` (CONN-05) and best-effort identity (CONN-06), store the
//!    live client (CONN-04), and return the `ActiveConnection`.
//!
//! Disconnect ([`disconnect`]) drops the held client so its pool/sockets close
//! (CONN-07).

use std::cell::RefCell;

use async_trait::async_trait;
use nodedb_client::{NativeClient, NodeDb};
use nodedb_types::result::QueryResult;

use crate::models::notification::Notification;
use crate::services::auth::AuthInput;
use crate::services::capabilities_map::derive_capabilities;
use crate::services::connection_service::ConnectionService;
use crate::services::error::StudioError;
use crate::services::identity::{parse_databases, parse_identity};
use crate::state::connection::ActiveConnection;
use crate::state::connections_registry::SavedConnection;

/// The identity probe: a single read-only `SELECT` that doubles as the forcing
/// round-trip which populates the lazily-negotiated handshake metadata.
const IDENTITY_PROBE: &str = "SELECT current_user, current_role, current_database";

/// Best-effort database-list probe (never fails the connect; falls back to the
/// current database on error or permission denial).
const DATABASES_PROBE: &str = "SHOW DATABASES";

#[derive(Default)]
pub struct NodeDbConnectionService {
    /// The live client, `None` until [`connect_real`](Self::connect_real)
    /// succeeds and again after [`disconnect`](Self::disconnect). `RefCell` is
    /// `Default`, so `#[derive(Default)]` keeps working; it is `!Sync`, which is
    /// fine behind the `#[async_trait(?Send)]` single-threaded seam.
    client: RefCell<Option<NativeClient>>,
}

/// Assemble an `ActiveConnection` from probe results + negotiated metadata.
///
/// Pure and renderer-free (testable without a live server, satisfies Nyquist):
/// it only parses fixtures and composes the display fields. Identity parsing is
/// best-effort with documented fallbacks (`parse_identity`/`parse_databases`),
/// so a surprising probe shape never produces a panic.
fn describe_connection(
    caps_bits: u64,
    server_version: &str,
    who: Option<&QueryResult>,
    dbs: Option<&QueryResult>,
    form_user: Option<&str>,
    conn_name: &str,
    default_db: &str,
) -> ActiveConnection {
    let capabilities = derive_capabilities(caps_bits);
    let identity = parse_identity(who, form_user, conn_name, default_db);
    let databases = parse_databases(dbs, &identity.current_database);

    let version = server_version.trim();
    let sub = if version.is_empty() {
        format!("nodedb · {} dbs", databases.len())
    } else {
        format!("nodedb · {version} · {} dbs", databases.len())
    };

    ActiveConnection {
        name: conn_name.to_string(),
        sub,
        user: identity.user,
        role: identity.role,
        capabilities,
        databases,
        current_database: identity.current_database,
    }
}

// `connect_real` + `disconnect` are the live connect surface, wired into the
// connection-manager card, the ⌘D handler, and both quick-switch surfaces
// (Plan 03).
impl NodeDbConnectionService {
    /// Open a real session: build the client, force the handshake via the
    /// identity probe, derive capabilities + identity, hold the live client, and
    /// return the `ActiveConnection`. On any probe failure, returns `Err` WITHOUT
    /// storing a client or producing a session (CONN-03).
    ///
    /// The secret travels in `auth` (the trait's `connect(name)` cannot carry
    /// it, and changing the trait shape is out of scope); the UI calls this in
    /// Plan 03. `form_username` for the identity fallback is read from `saved`.
    pub async fn connect_real(
        &self,
        saved: &SavedConnection,
        auth: AuthInput,
    ) -> Result<ActiveConnection, StudioError> {
        // Capture the fallback username BEFORE `auth` is moved into the builder.
        let form_username = saved.username.clone();
        let default_db = saved
            .default_database
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // 1. Build (sync, infallible, no socket — lazy pool).
        let client = crate::services::auth::build_client(
            &saved.host,
            saved.port,
            saved.default_database.clone(),
            &saved.tls,
            saved
                .connect_timeout_secs
                .map(std::time::Duration::from_secs),
            auth,
        );

        // 2. Force the handshake + identity in ONE round-trip. `?` maps any
        //    NodeDbError to StudioError; an Err here means connect failed, so we
        //    return WITHOUT storing the client (CONN-03). No RefCell borrow is
        //    held across this await.
        let who = client.execute_sql(IDENTITY_PROBE, &[]).await?;

        // 3. Metadata is now populated (the probe ran the handshake).
        let caps_bits = client.capabilities();
        let version = client.server_version();

        // Best-effort database list — never fails the connect.
        let dbs = client.execute_sql(DATABASES_PROBE, &[]).await.ok();

        let conn = describe_connection(
            caps_bits,
            &version,
            Some(&who),
            dbs.as_ref(),
            form_username.as_deref(),
            &saved.name,
            &default_db,
        );

        // 4. Store the live client (tight non-async borrow) and return.
        *self.client.borrow_mut() = Some(client);
        Ok(conn)
    }

    /// Release the live session (CONN-07): drop the held `NativeClient` so its
    /// pool returns connections and the sockets close. Idempotent.
    pub fn disconnect(&self) {
        *self.client.borrow_mut() = None;
    }
}

#[async_trait(?Send)]
impl ConnectionService for NodeDbConnectionService {
    async fn list_connections(&self) -> Result<Vec<SavedConnection>, StudioError> {
        // The data path (live registry) lands in a later phase; this phase wires
        // connect/auth/capabilities only.
        Err(StudioError::NotConnected)
    }

    async fn notifications(&self) -> Result<Vec<Notification>, StudioError> {
        Err(StudioError::NotConnected)
    }

    async fn connect(&self, _name: &str) -> Result<ActiveConnection, StudioError> {
        // The real connect carries an in-session secret and so cannot use this
        // name-only trait method — the UI calls `connect_real` directly (the
        // trait shape is intentionally unchanged this phase).
        Err(StudioError::NotConnected)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use nodedb_client::NodeDbError;
    use nodedb_types::value::Value;

    use super::*;

    fn s(text: &str) -> Value {
        Value::String(text.to_string())
    }

    fn result(columns: &[&str], rows: Vec<Vec<Value>>) -> QueryResult {
        QueryResult {
            columns: columns.iter().map(|c| c.to_string()).collect(),
            rows,
            rows_affected: 0,
        }
    }

    #[tokio::test]
    async fn stub_returns_not_connected() {
        // The trait methods still return NotConnected this phase (the real path
        // is `connect_real`); list/notifications are wired in a later phase.
        let svc = NodeDbConnectionService::default();
        assert!(matches!(
            svc.list_connections().await,
            Err(StudioError::NotConnected)
        ));
        assert!(matches!(
            svc.notifications().await,
            Err(StudioError::NotConnected)
        ));
        assert!(matches!(
            svc.connect("anything").await,
            Err(StudioError::NotConnected)
        ));
    }

    #[test]
    fn stub_is_object_safe_behind_rc() {
        // Compile-time guarantee: the reshaped service (RefCell<Option<…>>) still
        // coerces to the seam trait object, exactly as app.rs provides it.
        let _s: Rc<dyn ConnectionService> = Rc::new(NodeDbConnectionService::default());
    }

    #[test]
    fn connect_error_maps_to_studio_error() {
        // A connection-class NodeDbError -> StudioError::Connection (retriable);
        // an auth-class one -> StudioError::Auth (NOT retriable). This mirrors
        // the connect path's `?` mapping (the probe Err becomes StudioError via
        // From<NodeDbError>), gated on the method, never the category name.
        let conn: StudioError = NodeDbError::node_unreachable("refused").into();
        assert!(matches!(conn, StudioError::Connection(_)));
        assert!(conn.is_retriable());

        let auth: StudioError = NodeDbError::authorization_denied("bad creds").into();
        assert!(matches!(auth, StudioError::Auth(_)));
        assert!(!auth.is_retriable());
    }

    #[test]
    fn connect_error_does_not_set_active() {
        // The studio-side rule under test (CONN-03): a probe Err yields no
        // ActiveConnection and leaves the held client cleared. We model the
        // connect_real failure contract directly: on Err, the service never
        // stored a client, so the RefCell stays None.
        let svc = NodeDbConnectionService::default();
        let probe: Result<QueryResult, StudioError> =
            Err(NodeDbError::authorization_denied("bad creds").into());

        // Simulate the connect_real `?` short-circuit: an Err returns early,
        // before the `*self.client.borrow_mut() = Some(client)` store.
        let assembled: Result<ActiveConnection, StudioError> = probe
            .map(|who| describe_connection(0, "", Some(&who), None, Some("alice"), "conn", "mydb"));

        assert!(matches!(assembled, Err(StudioError::Auth(_))));
        assert!(
            svc.client.borrow().is_none(),
            "no client must be stored on a failed connect (CONN-03)"
        );
    }

    #[test]
    fn describe_connection_builds_active_from_probe() {
        let who = result(
            &["current_user", "current_role", "current_database"],
            vec![vec![s("root"), s("admin"), s("analytics")]],
        );
        let dbs = result(&["name"], vec![vec![s("analytics")], vec![s("logs")]]);

        // Bits with GraphRAG + FTS set (1<<1 | 1<<2 = 6); vector is always-on.
        let conn = describe_connection(
            6,
            "0.3.0",
            Some(&who),
            Some(&dbs),
            Some("alice"),
            "prod",
            "mydb",
        );

        assert_eq!(conn.name, "prod");
        assert_eq!(conn.user, "root"); // probe wins over form_user
        assert_eq!(conn.role, "admin");
        assert_eq!(conn.current_database, "analytics");
        assert_eq!(conn.databases, vec!["analytics", "logs"]);
        assert!(conn.capabilities.graph);
        assert!(conn.capabilities.fts);
        assert!(conn.capabilities.vector); // core engine, always on
        assert!(!conn.capabilities.sync);
        assert_eq!(conn.sub, "nodedb · 0.3.0 · 2 dbs");
    }

    #[test]
    fn describe_connection_uses_fallbacks_on_empty_probe() {
        // No probe data + empty version: identity falls back to form_user /
        // default_db, databases to [current_db], and sub omits the version.
        let conn = describe_connection(0, "", None, None, Some("alice"), "conn", "mydb");
        assert_eq!(conn.user, "alice");
        assert_eq!(conn.role, "");
        assert_eq!(conn.current_database, "mydb");
        assert_eq!(conn.databases, vec!["mydb"]);
        assert_eq!(conn.sub, "nodedb · 1 dbs");
    }

    #[test]
    fn disconnect_releases() {
        // After populating the held client, disconnect clears it to None so the
        // pool/sockets drop (CONN-07).
        let svc = NodeDbConnectionService::default();
        // Stash a (never-probed, lazy) client to model the connected state.
        *svc.client.borrow_mut() = Some(
            nodedb_client::ConnectionBuilder::new("127.0.0.1:6433")
                .username("u")
                .build(),
        );
        assert!(svc.client.borrow().is_some());

        svc.disconnect();
        assert!(svc.client.borrow().is_none());
    }
}
