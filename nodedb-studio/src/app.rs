//! Root component: provides global state and the top-level state machine.
//!
//! Two states, handled by a root conditional (NOT routing — see CLAUDE.md §5):
//!   - Disconnected -> `ConnectionManager` (full screen, no studio chrome)
//!   - Connected    -> `Studio`
//!
//! Global state is exposed as fine-grained signals via context. The backend
//! seam is a `dyn ConnectionService` behind an `Rc`, so swapping the mock for a
//! real NodeDB client later is a one-line change here.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::modals::ModalHost;
use crate::models::notification::Notification;
use crate::services::connection_service::{ConnectionService, MockConnectionService};
use crate::services::nodedb_service::NodeDbConnectionService;
use crate::state::connection::ActiveConnection;
use crate::state::connections_registry::SavedConnection;
use crate::state::preferences::Preferences;
use crate::state::ui::ModalKind;
use crate::views::connection_manager::ConnectionManager;
use crate::views::studio_shell::Studio;

const STYLES: Asset = asset!("/assets/styles.css");

// Dioxus components are PascalCase by convention; `App` is the root component.
#[allow(non_snake_case)]
pub fn App() -> Element {
    // The mock seam still backs the registry/notification feed (it works
    // alongside the real connect engine — seam discipline). Provided as a trait
    // object so consumers read it the same way regardless of impl.
    let service: Rc<dyn ConnectionService> = Rc::new(MockConnectionService);

    // The real connect/disconnect engine. It owns the live `NativeClient`
    // interior-mutably; the connection-manager card calls `connect_real` on it
    // and every disconnect surface (⌘D, popover, palette) calls `disconnect`.
    // Provided concretely (not behind `dyn`) so callers can reach the inherent
    // `connect_real`/`disconnect` methods (the secret-carrying real path is not
    // on the trait — the trait shape is intentionally unchanged this phase).
    let real: Rc<NodeDbConnectionService> = Rc::new(NodeDbConnectionService::default());

    use_context_provider(|| service.clone());
    use_context_provider(|| real.clone());
    use_context_provider(|| Signal::new(None::<ActiveConnection>));
    let mut registry = use_context_provider(|| Signal::new(Vec::<SavedConnection>::new()));
    let mut notifications = use_context_provider(|| Signal::new(Vec::<Notification>::new()));
    use_context_provider(|| Signal::new(Preferences::default()));
    // Modal state is provided here (not in Studio) because Preferences is
    // reachable while disconnected and via Cmd+, in either state.
    use_context_provider(|| Signal::new(None::<ModalKind>));

    // Seed the registry + notification feed asynchronously, at the seam. The
    // mock resolves instantly; the real client awaits the network. The guard
    // rule: clone the Rc before the async block and `.set()` the signals only
    // AFTER the await resolves — never hold a read/write guard across `.await`.
    {
        let svc = service.clone();
        use_resource(move || {
            let svc = svc.clone();
            async move {
                if let Ok(conns) = svc.list_connections().await {
                    registry.set(conns);
                }
                if let Ok(notifs) = svc.notifications().await {
                    notifications.set(notifs);
                }
            }
        });
    }

    let active = use_context::<Signal<Option<ActiveConnection>>>();

    rsx! {
        document::Stylesheet { href: STYLES }
        if active.read().is_some() {
            Studio {}
        } else {
            ConnectionManager {}
        }
        // Overlays everything; renders nothing when no modal is open.
        ModalHost {}
    }
}
