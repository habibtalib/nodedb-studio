//! New-connection modal body.
//!
//! Per CLAUDE.md §1 this is a single-engine client: there is NO Engine picker.
//! The Protocol field is also omitted — NodeDB's transport is the native
//! MessagePack port (`6433`).
//!
//! The form uses a single **Auth method** dropdown that swaps the visible field
//! set per [`AuthMode`] (D-02):
//!   - Trust       → Username only
//!   - Password    → Username + Password
//!   - ApiKey      → Token only
//!   - OidcBearer  → Token (+ optional Provider hint)
//!
//! mTLS is **not** an auth method — it is a TLS transport toggle (captured as a
//! saved-connection setting), so it does not appear here. The secret is held in
//! memory for the live session only (D-01): the live connect entry point is the
//! connection-manager card (see `views/connection_manager.rs`); this modal owns
//! the form structure (fields + dropdown + copy).

use dioxus::prelude::*;

use crate::state::connections_registry::AuthMode;
use crate::state::ui::ModalKind;

/// Parse the auth-method `select`'s value back into the studio's [`AuthMode`].
/// The `<option value>`s below are the only sources, so the fallback is never
/// hit in practice — it keeps the parse total without a panic.
fn parse_auth_mode(value: &str) -> AuthMode {
    match value {
        "trust" => AuthMode::Trust,
        "api_key" => AuthMode::ApiKey,
        "oidc_bearer" => AuthMode::OidcBearer,
        // "password" and any unexpected value default to Password.
        _ => AuthMode::Password,
    }
}

#[component]
pub fn NewConnectionForm() -> Element {
    let mut modal = use_context::<Signal<Option<ModalKind>>>();
    let auth_mode = use_signal(|| AuthMode::Password);

    rsx! {
        div { class: "modal-body",
            div { class: "form-field",
                label { "Name" }
                input { value: "local-nodedb-dev-2" }
            }
            div { class: "form-row",
                div { class: "form-field", label { "Host" } input { value: "localhost" } }
                div { class: "form-field", label { "Port" } input { value: "6433" } }
            }
            div { class: "form-field",
                label { "Auth method" }
                select {
                    onchange: move |e| {
                        let mut auth_mode = auth_mode;
                        auth_mode.set(parse_auth_mode(&e.value()));
                    },
                    option { value: "trust", "Trust" }
                    option { value: "password", selected: true, "Password" }
                    option { value: "api_key", "API key" }
                    option { value: "oidc_bearer", "OIDC bearer" }
                }
            }
            // Field-swap per the selected auth mode. No `_ =>` arm: AuthMode is
            // the studio's own exhaustive enum, so every variant is explicit.
            match *auth_mode.read() {
                AuthMode::Trust => rsx! {
                    div { class: "form-field", label { "Username" } input { value: "root" } }
                },
                AuthMode::Password => rsx! {
                    div { class: "form-row",
                        div { class: "form-field", label { "Username" } input { value: "root" } }
                        div { class: "form-field", label { "Password" } input { r#type: "password", value: "" } }
                    }
                },
                AuthMode::ApiKey => rsx! {
                    div { class: "form-field", label { "Token" } input { r#type: "password", value: "" } }
                },
                AuthMode::OidcBearer => rsx! {
                    div { class: "form-field", label { "Token" } input { r#type: "password", value: "" } }
                    div { class: "form-field", label { "Provider (optional)" } input { value: "" } }
                },
            }
            div { style: "display: flex; gap: 8px; align-items: center; padding-top: 6px;",
                span { class: "pill info", "i" }
                span { style: "font-size: 11px; color: var(--text-secondary);", "Secrets are kept in memory for this session only — re-entered on reconnect." }
            }
        }
        div { class: "modal-footer",
            button { class: "btn ghost", onclick: move |_| modal.set(None), "Cancel" }
            button { class: "btn", "Test" }
            button { class: "btn", "Save" }
            button { class: "btn primary", onclick: move |_| modal.set(None), "Save & connect" }
        }
    }
}
