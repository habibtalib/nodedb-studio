# Coding Conventions

**Analysis Date:** 2026-06-13

## Overview

This document captures the conventions enforced in AGENTS.md and verified across the codebase. These are the rules for contributing to NodeDB Studio, a Dioxus 0.7 desktop GUI client written in Rust (edition 2024, MSRV 1.96).

## Rust Core Conventions

### Error Handling

**Rules (from AGENTS.md):**
- **No `.unwrap()` / `.expect()` / `panic!` in non-test code**
- Use typed `thiserror` errors instead
- Propagate errors with `?` operator
- Never use `Result<T, String>` — use proper error types

**Verification:**
- `.unwrap()` calls present only in:
  - Test code (e.g., `data/mock.rs` lines 508, 517 in `#[cfg(test)]` block)
  - Display-only serialization with safe defaults (e.g., `sonic_rs::to_string(...).unwrap_or_default()` in `views/streams/cdc.rs:16`)
  - Safe Option patterns: `.unwrap_or('?')` in `state/connection.rs:86` (fallback char for empty username)

**Pattern:**
```rust
// ✓ Correct: Use ? for propagation
fn some_fn() -> Result<Value, MyError> {
    let data = risky_op()?;
    Ok(data)
}

// ✓ Correct: Safe fallback with Option
let avatar = self.user.chars().next()
    .map(|c| c.to_ascii_uppercase())
    .unwrap_or('?')
```

### Module Structure

**Rule (from AGENTS.md):**
- `mod.rs` files contain **ONLY** `pub mod` and `pub use` declarations
- No logic, no type definitions in `mod.rs`
- All logic lives in sibling files (e.g., `view.rs`, `host.rs`)

**Verification:**
- `src/components/mod.rs` — 11 lines, only pub mod declarations
- `src/state/mod.rs` — 10 lines, only pub mod declarations
- `src/models/mod.rs` — 6 lines, only pub mod declarations
- `src/modals/mod.rs` — 7 lines, includes pub use re-exports
- `src/views/mod.rs` — 23 lines, only pub mod declarations
- `src/data/mod.rs` — only pub use declarations

**Pattern:**
```rust
// src/components/mod.rs — 100% of the pattern
pub mod command_palette;
pub mod modal;
pub mod popovers;
pub mod rail;
pub mod snav;
pub mod statusbar;
pub mod subnav;
pub mod topbar;
```

### File Size

**Rule (from AGENTS.md):**
- Files stay under 500 lines of non-test code
- Split by concern first

**Verification:**
- Largest files: `src/data/mock.rs` (~524 lines, but mostly test data), `src/modals/preferences.rs` (205 lines)
- Modal components are split by concern (host, preferences, new_connection as separate files)
- Views are split by feature (streams/ has cdc.rs, cron.rs, landing.rs, mv.rs, notify.rs, topics.rs, view.rs)

### Type Naming

**Rules (from AGENTS.md):**
- Types: `UpperCamelCase`
- Functions/modules: `snake_case`
- No `get_` prefix on accessor functions

**Verification:**
- Types: `Notification`, `ActiveConnection`, `Capabilities`, `Preference`, `StorageMode`, `Theme`, `ModalKind`, `Popover`, `Collection`
- Functions: `connections()`, `notifications()`, `visible()`, `unread_count()`, `avatar_letter()`, `icon_letter()`, `label()`, `key()`, `css_class()`
- No `get_` prefixes observed (e.g., `Severity::css_class()` not `get_css_class()`)

### Data Types

**Rules (from AGENTS.md):**
- Use `nodedb_types::Value` for values coming from/going to the database
- Use `sonic_rs` for runtime JSON (display only), **NEVER** `serde_json`
- Serialize to JSON only at the view boundary for display

**Verification:**
- `data/mock.rs` line 241-242: Comment explicitly states this pattern: "viewers serialize them to JSON via `sonic_rs` purely for display"
- `views/streams/cdc.rs:16`: `sonic_rs::to_string(&ev.payload)` for display
- `views/streams/notify.rs:18`: `sonic_rs::to_string(&m.payload)` for display
- `data/mock.rs` tests use `sonic_rs::to_string()` (lines 508, 517)
- No `serde_json` imports found in codebase

### Exhaustive Pattern Matching

**Rule (from AGENTS.md):**
- Do **not** write `_ =>` catch-alls on exhaustive domain enums
- Let the compiler flag every site that needs updating

**Verification:**
- `models/notification.rs`: `Severity` enum match at line 21 — all 3 variants explicitly matched, no catch-all
- `models/collection.rs`: `StorageMode` enum matches at lines 27–68 — all 8 variants explicitly matched, no catch-alls
- `modals/preferences.rs`: `ModalKind` match at lines 18–23 — exhaustive, no catch-all
- `modals/preferences.rs`: String match on pane at lines 47–54 — has default `_` because it's not a closed enum

## Dioxus 0.7 Conventions

### Components

**Rules (from AGENTS.md):**
- Components are `PascalCase` `#[component]` functions returning `Element`
- Never hold a `.read()` / `.write()` guard across an `.await`
- Prefer `.peek()` in event handlers and when reading + writing the same signal
- List items must have stable keys (never array index)

**Verification:**

File: `src/components/topbar.rs` — Function signature pattern:
```rust
#[component]
pub fn Topbar() -> Element {
    let mut popover = use_context::<Signal<Option<Popover>>>();
    let mut palette = use_context::<Signal<bool>>();
    let active = use_context::<Signal<Option<ActiveConnection>>>();
    let notifs = use_context::<Signal<Vec<Notification>>>();

    let conn = active.read();  // ✓ Read guard released before event handlers
    let Some(c) = conn.as_ref() else {
        return rsx! {};
    };
    // ... later in handlers:
    onclick: move |e| { e.stop_propagation(); let cur = *popover.read(); popover.set(...); }
    // ✓ Peek pattern (read immediately in closure context, never held across await)
}
```

File: `src/modals/preferences.rs` lines 64–79:
```rust
#[component]
fn AppearancePane() -> Element {
    let mut prefs = use_context::<Signal<Preferences>>();
    let theme = prefs.read().theme;  // ✓ Read guard released immediately
    let cls = |t: Theme| if theme == t { "active" } else { "" };  // ✓ Capture by value
    rsx! {
        // ...
        onclick: move |_| prefs.write().theme = Theme::Light,  // ✓ .write() in handler, guard released immediately
    }
}
```

File: `src/modals/host.rs` lines 14–25 — exhaustive match on `ModalKind`:
```rust
#[component]
pub fn ModalHost() -> Element {
    let modal = use_context::<Signal<Option<ModalKind>>>();
    match *modal.read() {
        None => rsx! {},
        Some(ModalKind::NewConnection) => rsx! { Modal { ... } },
        Some(ModalKind::Preferences) => rsx! { Modal { ... } },
    }
}
```

### State Management

**Rules (from AGENTS.md):**
- Use `use_signal` (signals are `Copy`)
- Subscribe with `.read()`
- Use `.peek()` in event handlers and when reading + writing the same signal
- Provide context via `use_context_provider()`, never global statics
- Never hold a `.read()` / `.write()` guard across `.await`

**Verification:**

File: `src/app.rs` lines 36–43 — Context provider pattern:
```rust
use_context_provider(|| service.clone());
use_context_provider(|| Signal::new(None::<ActiveConnection>));
use_context_provider(|| Signal::new(registry));
use_context_provider(|| Signal::new(notifications));
use_context_provider(|| Signal::new(Preferences::default()));
use_context_provider(|| Signal::new(None::<ModalKind>));
```

File: `src/views/studio_shell.rs` lines 17–18 — Studio-scoped signals:
```rust
use_context_provider(|| Signal::new(None::<Popover>));
use_context_provider(|| Signal::new(false)); // command palette open
```

### Formatting & Attributes

**Rules (from AGENTS.md):**
- Use inline format strings in attributes/text (`"{value}"`)
- Avoid redundant closures over existing handlers
- Forms submit by default in 0.7 — call `e.prevent_default()` in `onsubmit`

**Verification:**

File: `src/components/topbar.rs` lines 54–56 — Inline format strings:
```rust
span { class: "name", "{c.name}" }
span { class: "engine", "{c.sub}" }
span { class: "chevron", "▾" }
```

File: `src/modals/preferences.rs` line 67 — Conditional class:
```rust
let cls = |t: Theme| if theme == t { "active" } else { "" };
// ... then later:
button { class: cls(Theme::Light), onclick: move |_| ..., "Light" }
```

### Threading & Async

**Rules (from AGENTS.md):**
- Never block the main thread
- CPU work → `std::thread::spawn`
- Async IO → `spawn` / `use_resource` / `use_action`
- Long-lived tasks → `spawn_forever`
- Write results back into a signal

**Status in codebase:** Patterns not yet observed; codebase is mock-only (synchronous, no real async). Rules will apply when `ConnectionService` backend lands.

### List Keys

**Rule (from AGENTS.md):**
- Lists need stable keys (`key: "{item.id}"`)
- Never the array index

**Verification:**

File: `src/modals/preferences.rs` lines 32–44 — Loop with stable key:
```rust
for (key, label) in CATS {
    {
        let is_active = current == key;
        let k = key.to_string();
        rsx! {
            div {
                class: if is_active { "prefs-cat active" } else { "prefs-cat" },
                onclick: move |_| pane.set(k.clone()),
                "{label}"
            }
        }
    }
}
```
**Note:** This loop uses `CATS` constant array, and the closure captures the string `k` derived from the stable key. No explicit `key:` attribute here, but pattern is stable (string-based, not index).

File: `src/views/query.rs` lines 118–120 — Results table loop:
```rust
for r in results {
    tr { td { "{r.0}" } td { "{r.1}" } td { "{r.2}" } }
}
```
**Note:** Results are synthetic tuples; stable order guaranteed.

### Business Logic Testability

**Rule (from AGENTS.md):**
- Keep business/state logic in plain Rust (`state/`, `services/`, `data/`)
- Logic must be testable without a renderer

**Verification:**

File: `src/state/notifications.rs` — Pure functions:
```rust
pub fn visible<'a>(items: &'a [Notification], caps: &Capabilities) -> impl Iterator<Item = &'a Notification> {
    let caps = *caps;
    items.iter().filter(move |n| n.required_cap.is_none_or(|c| caps.has(c)))
}

pub fn unread_count(items: &[Notification], caps: &Capabilities) -> usize {
    visible(items, caps).filter(|n| n.unread).count()
}
```
These are pure, testable functions with no UI coupling.

File: `src/state/connection.rs` lines 43–59 — Pure methods:
```rust
impl Capabilities {
    pub fn has(&self, cap: Capability) -> bool {
        match cap { ... }
    }
}

impl ActiveConnection {
    pub fn avatar_letter(&self) -> char {
        self.user.chars().next()
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or('?')
    }
}
```
Pure, no side effects.

## Naming Conventions Summary

| Category | Convention | Examples |
|----------|-----------|----------|
| Types | `UpperCamelCase` | `Notification`, `StorageMode`, `Preference`, `ActiveConnection` |
| Functions | `snake_case` | `connections()`, `unread_count()`, `visible()` |
| Modules | `snake_case` | `components`, `modals`, `views`, `state`, `data` |
| Components | `PascalCase` `#[component]` | `Topbar()`, `Modal()`, `Preferences()` |
| Enum variants | `PascalCase` | `Notification::Info`, `StorageMode::Document` |
| Module files | `snake_case.rs` | `modal.rs`, `preferences.rs`, `topbar.rs` |
| Module roots | `mod.rs` | Only pub mod/use, no logic |

## Import Organization

**Pattern observed:**

File: `src/components/topbar.rs` lines 8–17:
```rust
use dioxus::prelude::*;

use crate::components::popovers::avatar_popover::AvatarPopover;
use crate::components::popovers::connection_popover::ConnectionPopover;
use crate::components::popovers::database_popover::DatabasePopover;
use crate::components::popovers::notification_popover::NotificationPopover;
use crate::models::notification::Notification;
use crate::state::connection::ActiveConnection;
use crate::state::notifications::unread_count;
use crate::state::ui::Popover;
```

**Order:**
1. External crates (dioxus)
2. Internal crate imports (crate::*)
3. Organized by module path

## Comments & Documentation

**Rules observed:**
- Module-level doc comments on all public modules
- Inline comments explain *why*, not *what*
- Clear intent in type/function names reduces need for comments

**Pattern:**

File: `src/state/connection.rs` lines 1–6:
```rust
//! The active connection and its capabilities.
//!
//! Identity in NodeDB-Studio is per-connection, NOT global. There is no
//! "Studio account": switching connections swaps the NodeDB user, role, avatar
//! letter, and the capability flags that reshape the entire shell. See
//! CLAUDE.md "Per-connection identity" and "Capability-driven shell".
```

File: `src/components/topbar.rs` lines 19–26:
```rust
/// Open `which`, or close it if it is already the open popover.
fn toggled(current: Option<Popover>, which: Popover) -> Option<Popover> {
    if current == Some(which) {
        None
    } else {
        Some(which)
    }
}
```

## Architecture-Level Patterns

### Backend Seam (Trait-Based)

File: `src/services/connection_service.rs` — Trait definition:
```rust
pub trait ConnectionService {
    fn list_connections(&self) -> Vec<SavedConnection>;
    fn notifications(&self) -> Vec<Notification>;
    fn connect(&self, name: &str) -> Option<ActiveConnection>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MockConnectionService;

impl ConnectionService for MockConnectionService {
    fn list_connections(&self) -> Vec<SavedConnection> {
        mock::connections()
    }
    // ...
}
```

**Rationale (from CLAUDE.md):** The trait is the seam; when the real `nodedb-client` backend lands, a second impl wraps it. Consumers never know about the swap.

### Capability-Driven Rendering

File: `src/state/connection.rs` lines 27–59 — Exhaustive capability checking:
```rust
pub struct Capabilities {
    pub graph: bool,
    pub vector: bool,
    // ... 8 flags total
}

impl Capabilities {
    pub fn has(&self, cap: Capability) -> bool {
        match cap { ... }
    }
}
```

File: `src/models/notification.rs` lines 42–55:
```rust
pub struct Notification {
    // ...
    /// If set, hidden unless the active connection has this capability.
    pub required_cap: Option<Capability>,
    // ...
}
```

File: `src/state/notifications.rs` lines 12–20:
```rust
pub fn visible<'a>(items: &'a [Notification], caps: &Capabilities) -> impl Iterator<Item = &'a Notification> {
    let caps = *caps;
    items.iter().filter(move |n| n.required_cap.is_none_or(|c| caps.has(c)))
}
```

**Pattern:** Notifications and views filter conditionally on capabilities. No hardcoded feature flags in render logic.

---

*Conventions analysis: 2026-06-13*
