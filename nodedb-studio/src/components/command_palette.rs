//! Command palette (Cmd+K). Opens only while connected.
//!
//! Rendered inside the router (via `StudioLayout`) so navigation items can use
//! the navigator. Open state is the shared `Signal<bool>` provided by `Studio`.
//!
//! Switch-connection items reuse the REAL connect path (D-08): they look the
//! target up in the saved registry, drop the current client, then connect a
//! trust-mode target directly or route back to the connection manager when the
//! target needs a secret not held in session. Disconnect (CONN-07) releases the
//! live client.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::routes::Route;
use crate::services::auth::AuthInput;
use crate::services::nodedb_service::NodeDbConnectionService;
use crate::state::connection::ActiveConnection;
use crate::state::connections_registry::{AuthMode, SavedConnection};
use crate::state::ui::ModalKind;

#[component]
pub fn CommandPalette() -> Element {
    let mut open = use_context::<Signal<bool>>();
    let mut active = use_context::<Signal<Option<ActiveConnection>>>();
    let mut modal = use_context::<Signal<Option<ModalKind>>>();
    let registry = use_context::<Signal<Vec<SavedConnection>>>();
    let real = use_context::<Rc<NodeDbConnectionService>>();
    let nav = use_navigator();

    if !*open.read() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "palette-overlay open",
            onclick: move |_| open.set(false),
            div {
                class: "palette",
                onclick: move |e| e.stop_propagation(),
                input { placeholder: "Search or run command…" }
                div { class: "palette-results",
                    div { class: "palette-section", "Navigate" }
                    div { class: "palette-item", onclick: move |_| { nav.push(Route::Explorer {}); open.set(false); },
                        "Open Explorer" span { class: "meta", "G E" }
                    }
                    div { class: "palette-item", onclick: move |_| { nav.push(Route::Query {}); open.set(false); },
                        "Open Query" span { class: "meta", "G Q" }
                    }
                    div { class: "palette-item", onclick: move |_| { nav.push(Route::GraphExplorer {}); open.set(false); },
                        "Open Graph Explorer" span { class: "meta", "G G" }
                    }
                    div { class: "palette-item", onclick: move |_| { nav.push(Route::Streams { tab: "landing".to_string() }); open.set(false); },
                        "Open Streams & Events" span { class: "meta", "G S" }
                    }

                    div { class: "palette-section", "Actions" }
                    div { class: "palette-item", onclick: move |_| { modal.set(Some(ModalKind::NewConnection)); open.set(false); },
                        "New connection…" span { class: "meta", "⌘N" }
                    }
                    div { class: "palette-item",
                        "Run current query" span { class: "meta", "⌘↵" }
                    }
                    div { class: "palette-item", onclick: move |_| { modal.set(Some(ModalKind::Preferences)); open.set(false); },
                        "Open preferences" span { class: "meta", "⌘," }
                    }
                    div { class: "palette-item", onclick: move |_| { modal.set(Some(ModalKind::Preferences)); open.set(false); },
                        "Toggle theme" span { class: "meta", "⌘⇧L" }
                    }

                    div { class: "palette-section", "Connections" }
                    // Switch items, one per other saved connection. Each reuses
                    // the real connect path (D-08).
                    for sc in registry.read().iter().cloned() {
                        {
                            let target = sc.clone();
                            let real = real.clone();
                            rsx! {
                                div {
                                    key: "{sc.name}",
                                    class: "palette-item",
                                    onclick: move |_| {
                                        switch_connection(real.clone(), target.clone(), active);
                                        open.set(false);
                                    },
                                    "Switch to {sc.name}"
                                }
                            }
                        }
                    }
                    div { class: "palette-item", onclick: {
                            let real = real.clone();
                            move |_| {
                                // Release the live client (CONN-07) then disconnect.
                                real.disconnect();
                                active.set(None);
                                open.set(false);
                            }
                        },
                        "Disconnect" span { class: "meta", "⌘D" }
                    }
                }
            }
        }
    }
}

/// Quick-switch to `target` via the real connect path (D-08): drop the current
/// client, then either connect a trust-mode target directly or — when the target
/// needs a secret not held in session — route back to the connection manager so
/// the secret can be re-entered on a card (never connect with an empty secret).
fn switch_connection(
    real: Rc<NodeDbConnectionService>,
    target: SavedConnection,
    mut active: Signal<Option<ActiveConnection>>,
) {
    real.disconnect();

    match target.auth_mode {
        AuthMode::Trust => {
            let auth = AuthInput::Trust {
                username: target.username.clone().unwrap_or_default(),
            };
            spawn(async move {
                if let Ok(session) = real.connect_real(&target, auth).await {
                    active.set(Some(session));
                }
            });
        }
        AuthMode::Password | AuthMode::ApiKey | AuthMode::OidcBearer => {
            // Secret not held in session (D-01) — route to the manager to
            // re-enter it rather than connecting silently (D-08).
            active.set(None);
        }
    }
}
