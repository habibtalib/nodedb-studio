# Testing Patterns

**Analysis Date:** 2026-06-13

## Test Framework & Runner

**Framework:**
- `cargo nextest run` — The test runner (NOT `cargo test`)
- Install once: `cargo install cargo-nextest --locked`
- Native Rust test framework (`#[test]` attribute)

**Run Commands:**

```bash
# Run all tests
cargo nextest run

# Run one test by name
cargo nextest run -E 'test(payload_serializes_with_fields_in_declared_order)'

# Run tests matching a pattern
cargo nextest run -E 'test(payload_)'

# Format check (CI gate)
cargo fmt --all

# Lint check (CI gate)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Full pre-push verification
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run
```

## Test File Organization

**Location Pattern:**
- Unit tests: **inline** in source files via `#[cfg(test)] mod tests { ... }`
- Integration tests: `tests/` directory (not yet present; create when needed)
- Shared test helpers: `tests/common/mod.rs` (not `tests/common.rs`)

**Rationale (from AGENTS.md):**
- Inline unit tests can reach private items
- Can be tested without a renderer
- Keeps test code close to implementation

## Current Test Coverage

### Inline Unit Tests

**File: `src/data/mock.rs` lines 500–523**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_serializes_with_fields_in_declared_order() {
        // The orders INSERT row: keys must come out in insertion order, not
        // the alphabetical/HashMap order a `Value::Object` would impose.
        let json = sonic_rs::to_string(&cdc_events()[3].payload).unwrap();
        assert_eq!(
            json,
            r#"{"id":442004,"user_id":"u_77103","total":89.4,"currency":"USD"}"#
        );
    }

    #[test]
    fn nested_document_serializes() {
        let json = sonic_rs::to_string(&cdc_events()[0].payload).unwrap();
        assert_eq!(
            json,
            r#"{"_id":"evt_01HMNJ…","type":"page_view","user_id":"u_44182","props":{"path":"/dashboard"}}"#
        );
    }
}
```

**Coverage:**
- Two tests verify serialization of mock data to JSON
- Use `sonic_rs::to_string()` (never `serde_json`)
- Test mock data generation logic
- Currently the only test module in the codebase

### What is Tested

- **Mock data serialization:** Fields serialize in insertion order (not HashMap alphabetical order)
- **Nested document handling:** Complex nested structures serialize correctly

### Test Patterns Observed

**Pattern 1: Access to private functions**
```rust
#[cfg(test)]
mod tests {
    use super::*;  // ✓ Brings private items into scope
    
    #[test]
    fn test_name() {
        let data = cdc_events();  // ✓ Private function, visible to test
        let json = sonic_rs::to_string(&data[3].payload).unwrap();
        assert_eq!(json, expected);
    }
}
```

**Pattern 2: Assertion via string comparison**
```rust
assert_eq!(json, r#"{"id":442004,"user_id":"u_77103",...}"#);
```

## Tests NOT Yet Present

The following test gaps exist (mock-only codebase, real backend not integrated):

### Missing Unit Test Categories

- **State management:** `state/connection.rs`, `state/notifications.rs`, `state/preferences.rs` have no tests
  - Testable functions: `Capabilities::has()`, `unread_count()`, `visible()`, `ActiveConnection::avatar_letter()`
  - No side effects; should have fast unit tests
- **Component rendering:** No render checks via `dioxus_ssr::render_element()`
  - Will be needed when real behavior lands (e.g., form submission, event handlers)
- **Modal state:** `modals/preferences.rs` theme switching is untested
- **Data fixtures:** No test factories for `SavedConnection`, `Notification`, `Collection`

### Missing Integration Tests

No `tests/` directory exists. When the real `ConnectionService` backend lands:
- Integration tests should verify the seam: mock impl vs. real client behavior
- Shared helpers would go in `tests/common/mod.rs`

### Missing Render-Level Tests

**Not used yet:** `dioxus_ssr::render_element(...)` to assert on output without a window

Example (pattern for future use):
```rust
#[test]
fn topbar_renders_connection_chip() {
    // When component render testing lands, this pattern applies:
    let element = rsx! { Topbar {} };
    let html = dioxus_ssr::render_element(element);
    assert!(html.contains("local-nodedb-dev"));
}
```

**When this applies:** After real backend integration, when components have testable behavior beyond static layout.

## CI Gate Requirements

From AGENTS.md, CI must pass:
1. `cargo fmt --all --check` — Code formatting
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — Linting (warnings block merge)
3. `cargo nextest run` — All tests pass

**Note:** No doctests today (binary crate, no lib). If a library crate is introduced, add `cargo test --doc`.

## Test Execution Timing

**Local verification before push:**
```bash
# Run these in order
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run
```

All must pass before pushing (mirrors CI).

## Test Data Strategy

**Current approach:** Mock data is hardcoded Rust in `src/data/mock.rs`.

File: `src/data/mock.rs` lines 1–9:
```rust
//! All hardcoded mock data lives here, in one place (CLAUDE.md §10). When the
//! backend lands, the `ConnectionService` mock impl is the only thing that
//! reads this module; nothing else should hardcode data.
```

**Connections (lines 19–126):**
- 4 saved connections: `local-nodedb-dev`, `staging-cluster`, `prod-replica-eu`, `test-nodedb`
- Each has: name, metadata, status, profile (user, role, capabilities, databases)
- Seeded into `Signal<Vec<SavedConnection>>` at app root

**Notifications (lines 128–180):**
- 16 notification structs with severity, group, required capability, title, description
- Grouped by domain: Sync, Streams, Admin
- Seeded into `Signal<Vec<Notification>>` at app root

**CDC Events (lines 189–320):**
- 4 example change-data-capture payloads
- Use `nodedb_types::Value` (the real wire format)
- Demonstrates nested documents and field ordering

## Pattern: Keep Logic Testable

**File: `src/state/notifications.rs` — Pure, testable logic**

```rust
pub fn visible<'a>(
    items: &'a [Notification],
    caps: &Capabilities,
) -> impl Iterator<Item = &'a Notification> {
    let caps = *caps;
    items
        .iter()
        .filter(move |n| n.required_cap.is_none_or(|c| caps.has(c)))
}

pub fn unread_count(items: &[Notification], caps: &Capabilities) -> usize {
    visible(items, caps).filter(|n| n.unread).count()
}
```

**Testable without a renderer:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn unread_count_filters_by_capability() {
        let caps = Capabilities { graph: false, ..Default::default() };
        let notifs = vec![
            Notification { required_cap: Some(Capability::Graph), unread: true, ... },
            Notification { required_cap: None, unread: true, ... },
        ];
        assert_eq!(unread_count(&notifs, &caps), 1);  // Only the one without required_cap
    }
}
```

This pattern applies everywhere: put logic in `state/`, `services/`, `data/`, not in components.

## Dependencies for Testing

From `Cargo.toml` (workspace level):
```toml
[workspace.dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
dioxus = { version = "0.7", features = ["desktop", "router"] }
sonic-rs = "0.5"
```

**For future use:**
- `dioxus_ssr` crate (not yet in workspace) for render testing
- Test frameworks beyond native test attribute: not yet adopted

## Test Coverage Status

**Current:** 2 tests in mock.rs (data validation only)

**Coverage by module:**
| Module | Tests | Gap |
|--------|-------|-----|
| `src/data/mock.rs` | 2 (serialization) | CRCs, CDC event fixtures untested |
| `src/state/notifications.rs` | 0 | `visible()`, `unread_count()` should have tests |
| `src/state/connection.rs` | 0 | `Capabilities::has()`, `avatar_letter()` untested |
| `src/state/preferences.rs` | 0 | No side effects; low priority |
| `src/models/*.rs` | 0 | Pure data structures; low priority |
| `src/components/*.rs` | 0 | Render testing not yet adopted |
| `src/views/*.rs` | 0 | Behavior tests blocked on real backend |
| `src/services/*.rs` | 0 | Trait seam; tested via mock/real impl pairs later |

**Priority for next phase:**
1. Unit tests for `state/notifications.rs` (pure, testable, logic-heavy)
2. Unit tests for `state/connection.rs::Capabilities::has()`
3. Integration test suite for `ConnectionService` (when real backend lands)

## Recommended Test Template for Future

When adding new logic, use this structure:

```rust
// src/state/my_state.rs

/// Pure, testable business logic
pub fn process_data(input: &[Item]) -> Result<Output, MyError> {
    // ... logic
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_data_handles_edge_case() {
        let input = vec![...];
        let result = process_data(&input).expect("should not fail");
        assert_eq!(result, expected);
    }

    #[test]
    fn process_data_returns_error_when() {
        let input = vec![...];
        let result = process_data(&input);
        assert!(result.is_err());
    }
}
```

---

*Testing analysis: 2026-06-13*
