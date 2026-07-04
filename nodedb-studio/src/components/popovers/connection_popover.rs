//! Connection switch popover: current connection, a switch list of the others
//! (self excluded), edit, and disconnect.
//!
//! Switching reuses the REAL connect path (D-08): disconnect the current client,
//! then connect to the target via `connect_real`. For trust-mode targets the
//! secret-free `AuthInput::Trust` is built directly; for targets that need a
//! secret not held in session, route the user back to the connection manager
//! (clear `active`) so the secret can be re-entered on a card — never connect
//! silently with an empty secret. Disconnect (CONN-07) releases the live client.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::services::auth::AuthInput;
use crate::services::nodedb_service::NodeDbConnectionService;
use crate::state::connection::ActiveConnection;
use crate::state::connections_registry::{AuthMode, ConnStatus, SavedConnection};
use crate::state::ui::{ModalKind, Popover};

#[component]
pub fn ConnectionPopover() -> Element {
    let mut active = use_context::<Signal<Option<ActiveConnection>>>();
    let mut popover = use_context::<Signal<Option<Popover>>>();
    let mut modal = use_context::<Signal<Option<ModalKind>>>();
    let registry = use_context::<Signal<Vec<SavedConnection>>>();
    let real = use_context::<Rc<NodeDbConnectionService>>();

    let conn = active.read();
    let Some(c) = conn.as_ref() else {
        return rsx! {};
    };
    let current_name = c.name.clone();
    let current_sub = c.sub.clone();

    // Switch list excludes the current connection.
    let others: Vec<SavedConnection> = registry
        .read()
        .iter()
        .filter(|s| s.name != current_name)
        .cloned()
        .collect();

    let disconnect_real = real.clone();

    rsx! {
        div { class: "conn-popover open", onclick: move |e| e.stop_propagation(),
            div { class: "cp-current",
                div { class: "name", span { class: "dot" } span { "{current_name}" } }
                div { class: "sub", "{current_sub}" }
            }
            div { class: "cp-section", "Switch to" }
            for sc in others {
                {
                    let (dot_class, meta, disabled) = match sc.status {
                        ConnStatus::Online => ("ok", sc.ping.clone().unwrap_or_default(), false),
                        ConnStatus::ReadOnly => ("warn", "read-only".to_string(), false),
                        ConnStatus::Offline => ("off", "offline".to_string(), true),
                    };
                    let target = sc.clone();
                    let real = real.clone();
                    let item_class = if disabled { "cp-item disabled" } else { "cp-item" };
                    rsx! {
                        div {
                            class: "{item_class}",
                            onclick: move |_| {
                                if !disabled {
                                    switch_connection(real.clone(), target.clone(), active);
                                    popover.set(None);
                                }
                            },
                            span { class: "dot {dot_class}" }
                            " {sc.name} "
                            span { class: "meta", "{meta}" }
                        }
                    }
                }
            }
            div { class: "cp-divider" }
            div {
                class: "cp-action",
                onclick: move |_| { popover.set(None); modal.set(Some(ModalKind::NewConnection)); },
                "Edit connection…"
            }
            div {
                class: "cp-action danger",
                onclick: move |_| {
                    // Release the live client (CONN-07) before clearing the session.
                    disconnect_real.disconnect();
                    active.set(None);
                },
                "Disconnect " span { class: "kbd", "⌘D" }
            }
        }
    }
}

/// Quick-switch to `target` via the real connect path (D-08): drop the current
/// client, then either connect a trust-mode target directly or — when the target
/// needs a secret not held in session — return to the connection manager so the
/// secret can be re-entered on a card (never connect with an empty secret).
fn switch_connection(
    real: Rc<NodeDbConnectionService>,
    target: SavedConnection,
    mut active: Signal<Option<ActiveConnection>>,
) {
    // Drop the current live client first.
    real.disconnect();

    match target.auth_mode {
        AuthMode::Trust => {
            // Trust needs no secret — reuse the real connect path directly.
            let auth = AuthInput::Trust {
                username: target.username.clone().unwrap_or_default(),
            };
            spawn(async move {
                if let Ok(session) = real.connect_real(&target, auth).await {
                    active.set(Some(session));
                }
                // On failure, stay disconnected (the manager surfaces errors).
            });
        }
        AuthMode::Password | AuthMode::ApiKey | AuthMode::OidcBearer => {
            // The secret is not held in session (D-01) — route back to the
            // connection manager so it can be re-entered on a card, rather than
            // connecting silently with an empty secret (D-08).
            active.set(None);
        }
    }
}
