//! The disconnected state: the entry screen. Pick a saved connection (or add
//! one) to enter the studio. Preferences is reachable here via the topbar link.
//!
//! Connect runs the REAL path at the seam (D-03, CONN-01..04): each card
//! collects the in-session secret it needs (Password/ApiKey/OidcBearer; Trust
//! needs none), builds an [`AuthInput`], and hands the full [`SavedConnection`]
//! + `AuthInput` up to the manager via an `EventHandler`. The manager runs a
//! one-shot `spawn` calling [`NodeDbConnectionService::connect_real`]; on `Ok`
//! it sets the active session (→ Studio shell), on `Err` it surfaces an inline
//! error + Retry on the connecting card (Retry gated on `is_retriable`) and
//! NEVER enters the connected state (CONN-03). The in-session secret is created
//! at the call site from a card-local signal and moved into `connect_real` — it
//! is never written to `SavedConnection`, disk, or a long-lived signal (D-01).

use std::rc::Rc;

use dioxus::prelude::*;

use crate::services::auth::{AuthInput, Secret};
use crate::services::nodedb_service::NodeDbConnectionService;
use crate::state::connection::ActiveConnection;
use crate::state::connections_registry::{AuthMode, ConnStatus, SavedConnection};
use crate::state::ui::ModalKind;

/// Per-card connect-error payload: `(connection name, message, retriable)`.
type ConnectError = (String, String, bool);

#[component]
pub fn ConnectionManager() -> Element {
    let registry = use_context::<Signal<Vec<SavedConnection>>>();
    let mut active = use_context::<Signal<Option<ActiveConnection>>>();
    let mut modal = use_context::<Signal<Option<ModalKind>>>();
    let real = use_context::<Rc<NodeDbConnectionService>>();

    // The card currently connecting (its name), or None.
    let mut connecting = use_signal(|| None::<String>);
    // The last connect error, keyed by connection name so only the right card
    // renders it: (name, message, retriable).
    let mut connect_error = use_signal(|| None::<ConnectError>);

    rsx! {
        div { class: "conn-manager",
            div { class: "cm-topbar",
                div { class: "cm-brand-mini",
                    div { class: "logo", "N" }
                    div { "NodeDB " span { "Studio" } }
                }
                div { class: "cm-topbar-actions",
                    a { onclick: move |_| modal.set(Some(ModalKind::Preferences)), "Preferences" }
                    a { "Docs" }
                    span { class: "version", "dev" }
                }
            }

            div { class: "cm-content",
                div { class: "cm-hero",
                    h1 { "Connect to a database" }
                    p { "Pick a saved connection or add a new one. Workspace, tools, and admin open after you connect." }
                }

                div { class: "cm-section-head",
                    h2 { "Saved connections" }
                    button { class: "btn", onclick: move |_| modal.set(Some(ModalKind::NewConnection)), "+ New connection" }
                }

                div { class: "cm-grid",
                    for conn in registry.read().iter().cloned() {
                        ConnectionCard {
                            key: "{conn.name}",
                            conn: conn.clone(),
                            connecting,
                            connect_error,
                            on_connect: {
                                let real = real.clone();
                                move |(saved, auth): (SavedConnection, AuthInput)| {
                                    // Run the real connect at the seam. Clone the
                                    // Rc + saved into the task; touch the (Copy)
                                    // signals only AFTER the await resolves — no
                                    // guard is ever held across `.await`.
                                    let real = real.clone();
                                    connecting.set(Some(saved.name.clone()));
                                    connect_error.set(None);
                                    spawn(async move {
                                        match real.connect_real(&saved, auth).await {
                                            Ok(session) => {
                                                active.set(Some(session)); // → Studio (CONN-04)
                                                connecting.set(None);
                                                connect_error.set(None);
                                            }
                                            Err(e) => {
                                                // CONN-03: never touch `active` on failure.
                                                connect_error.set(Some((
                                                    saved.name.clone(),
                                                    e.to_string(),
                                                    e.is_retriable(),
                                                )));
                                                connecting.set(None);
                                            }
                                        }
                                    });
                                }
                            },
                        }
                    }
                    button { class: "cm-card cm-new-card", onclick: move |_| modal.set(Some(ModalKind::NewConnection)),
                        div { class: "plus", "+" }
                        div { "New connection" }
                    }
                }

                div { class: "cm-section-head", h2 { "Recent activity" } }
                div { class: "cm-recent",
                    RecentItem { fail: false, what_connected: "local-nodedb-dev", detail: " · 142ms last query", when: "2m ago" }
                    RecentItem { fail: true, what_connected: "test-nodedb", detail: " · password expired", when: "1h ago", verb: "Auth failed on" }
                    RecentItem { fail: false, what_connected: "prod-replica-eu", detail: " · added TLS cert", when: "yesterday", verb: "Edited" }
                    RecentItem { fail: false, what_connected: "staging-cluster", detail: " · ran 14 queries", when: "yesterday" }
                }
            }
        }
    }
}

/// Build the per-mode [`AuthInput`] from the saved connection's [`AuthMode`] and
/// the card-local secret/provider the user just typed. No `_ =>` arm — `AuthMode`
/// is the studio's own exhaustive enum. The secret string is moved into
/// [`Secret::new`] and never stored beyond the call (D-01).
fn build_auth_input(conn: &SavedConnection, secret: String, provider: String) -> AuthInput {
    let username = conn.username.clone().unwrap_or_default();
    match conn.auth_mode {
        AuthMode::Trust => AuthInput::Trust { username },
        AuthMode::Password => AuthInput::Password {
            username,
            password: Secret::new(secret),
        },
        AuthMode::ApiKey => AuthInput::ApiKey {
            token: Secret::new(secret),
        },
        AuthMode::OidcBearer => AuthInput::Oidc {
            token: Secret::new(secret),
            provider: if provider.is_empty() {
                None
            } else {
                Some(provider)
            },
        },
    }
}

/// Whether this auth mode needs an in-session secret typed on the card. Trust
/// carries no secret; the other three do.
fn needs_secret(mode: AuthMode) -> bool {
    match mode {
        AuthMode::Trust => false,
        AuthMode::Password | AuthMode::ApiKey | AuthMode::OidcBearer => true,
    }
}

#[component]
fn ConnectionCard(
    conn: SavedConnection,
    connecting: Signal<Option<String>>,
    connect_error: Signal<Option<ConnectError>>,
    on_connect: EventHandler<(SavedConnection, AuthInput)>,
) -> Element {
    let connectable = conn.status.is_connectable();
    let (pill_class, pill_text, dot_style) = match conn.status {
        ConnStatus::Online => ("pill ok", "online", ""),
        ConnStatus::ReadOnly => ("pill warn", "read-only", ""),
        ConnStatus::Offline => ("pill", "offline", "background:var(--text-tertiary)"),
    };
    let dbs = conn
        .db_count
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".to_string());
    let ping = conn.ping.clone().unwrap_or_else(|| "—".to_string());

    // Card-local, ephemeral in-session secret (D-01): typed into the inputs
    // below, read with `.peek()` in the connect handler, moved into `Secret`,
    // never persisted. `provider_input` is only used for OidcBearer.
    let mut secret_input = use_signal(String::new);
    let mut provider_input = use_signal(String::new);

    let show_secret = needs_secret(conn.auth_mode);
    let show_provider = matches!(conn.auth_mode, AuthMode::OidcBearer);
    let secret_label = match conn.auth_mode {
        AuthMode::Password => "Password",
        AuthMode::ApiKey | AuthMode::OidcBearer => "Token",
        AuthMode::Trust => "",
    };

    // This card's live state, compared against its own name.
    let is_connecting = connecting.read().as_deref() == Some(conn.name.as_str());
    let card_err: Option<ConnectError> = connect_error
        .read()
        .as_ref()
        .filter(|(n, _, _)| *n == conn.name)
        .cloned();

    // Fire the connect with a freshly-built AuthInput (reused by card + Retry).
    // `conn`/`on_connect`/the secret signals are all Clone/Copy, so each onclick
    // gets its own copies — a single closure can't be shared (closures aren't
    // `Clone`). The secret is read with `.peek()` (read-without-subscribe).
    let conn_for_card = conn.clone();
    let conn_for_retry = conn.clone();

    rsx! {
        div { class: "cm-card-wrap",
            button {
                class: "cm-card",
                disabled: !connectable || is_connecting,
                onclick: move |_| {
                    if connectable && !is_connecting {
                        let auth = build_auth_input(
                            &conn_for_card,
                            secret_input.peek().clone(),
                            provider_input.peek().clone(),
                        );
                        on_connect.call((conn_for_card.clone(), auth));
                    }
                },
                div { class: "cm-card-top",
                    div { class: "cm-card-name", "{conn.name}" }
                    span { class: "{pill_class}", span { class: "dot", style: "{dot_style}" } "{pill_text}" }
                }
                div { class: "cm-card-meta", "{conn.meta}" }
                div { class: "cm-card-stats",
                    div { class: "stat", span { class: "v", "{dbs}" } span { class: "l", "DBs" } }
                    div { class: "stat", span { class: "v", "{ping}" } span { class: "l", "ping" } }
                    div { class: "stat", span { class: "v", "{conn.server}" } span { class: "l", "server" } }
                }
            }

            // In-session secret inputs for non-trust modes (D-01). Clicks here
            // must not bubble to the card's connect button.
            if connectable && show_secret {
                div { class: "cm-card-secret", onclick: move |e| e.stop_propagation(),
                    input {
                        r#type: "password",
                        placeholder: "{secret_label}",
                        value: "{secret_input}",
                        oninput: move |e| secret_input.set(e.value()),
                    }
                    if show_provider {
                        input {
                            r#type: "text",
                            placeholder: "Provider (optional)",
                            value: "{provider_input}",
                            oninput: move |e| provider_input.set(e.value()),
                        }
                    }
                }
            }

            // Connecting / error+Retry lifecycle (D-03).
            if is_connecting {
                div { class: "cm-card-state", "Connecting…" }
            } else if let Some((_, msg, retriable)) = card_err {
                div { class: "cm-card-error",
                    div { class: "cm-card-error-msg", "{msg}" }
                    if retriable {
                        button {
                            class: "btn",
                            onclick: move |e| {
                                e.stop_propagation();
                                let auth = build_auth_input(
                                    &conn_for_retry,
                                    secret_input.peek().clone(),
                                    provider_input.peek().clone(),
                                );
                                on_connect.call((conn_for_retry.clone(), auth));
                            },
                            "Retry"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RecentItem(
    fail: bool,
    what_connected: String,
    detail: String,
    when: String,
    #[props(default = "Connected to".to_string())] verb: String,
) -> Element {
    rsx! {
        div { class: "cm-recent-item",
            span { class: if fail { "dot fail" } else { "dot" } }
            span { class: "what", "{verb} " strong { "{what_connected}" } "{detail}" }
            span { class: "when", "{when}" }
        }
    }
}
