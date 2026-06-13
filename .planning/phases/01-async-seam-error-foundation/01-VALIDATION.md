---
phase: 1
slug: async-seam-error-foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-14
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo-nextest` 0.9.137 over the Rust test harness; `#[tokio::test]` for async trait methods |
| **Config file** | none (nextest defaults; tests are inline `#[cfg(test)] mod tests`) |
| **Quick run command** | `cargo nextest run -p nodedb-studio` |
| **Full suite command** | `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run` |
| **Estimated runtime** | ~60 seconds (cold build dominates; tests are near-instant) |

---

## Sampling Rate

- **After every task commit:** Run `cargo nextest run -p nodedb-studio`
- **After every plan wave:** Run `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run`
- **Before `/gsd:verify-work`:** Full suite green + manual `cargo run -p nodedb-studio` confirming mock data still renders and the `AsyncState` proof view shows Loaded (and Error on a forced failure)
- **Max feedback latency:** ~60 seconds

---

## Per-Task Verification Map

| Requirement | Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|----------|-----------|-------------------|-------------|--------|
| SEAM-01 | Async trait compiles; `MockConnectionService` satisfies it; returns same data | unit (`#[tokio::test]`) | `cargo nextest run -E 'test(mock_notifications_returns_data)'` | ❌ W0 | ⬜ pending |
| SEAM-01 | App still compiles + renders mock identically (regression) | compile gate + manual | `cargo build -p nodedb-studio` then `cargo run -p nodedb-studio` | ✅ baseline green | ⬜ pending |
| SEAM-02 | `NodeDbConnectionService` exists, wraps `NativeClient`, instantiable, returns `NotConnected` | unit + compile | `cargo nextest run -E 'test(stub_returns_not_connected)'` | ❌ W0 | ⬜ pending |
| SEAM-03 | `NodeDbError` → `StudioError` mapping correct per category; `is_retriable()` delegated; no `unwrap`/`String` | unit + lint | `cargo nextest run -E 'test(maps_)'` then `cargo clippy ... -D warnings` | ❌ W0 | ⬜ pending |
| SEAM-04 | `AsyncState` maps None/empty/ok/err correctly | unit | `cargo nextest run -E 'test(async_state)'` | ❌ W0 | ⬜ pending |
| SEAM-04 | `use_resource` wiring renders loading/empty/error; main thread never blocked | compile gate + manual render | `cargo build -p nodedb-studio` then `cargo run -p nodedb-studio` | ✅ compile / ❌ render test | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Enable `nodedb-client` `native` feature in `Cargo.toml` `[workspace.dependencies]` — **prerequisite** for SEAM-02 to compile (`NativeClient` is `#[cfg(feature = "native")]`)
- [ ] `services/error.rs` tests — SEAM-03 (per-category mapping + `is_retriable()` delegation)
- [ ] `services/async_state.rs` tests — SEAM-04 (`AsyncState` state transitions)
- [ ] `services/connection_service.rs` tests — SEAM-01 (mock async, `#[tokio::test]`) + SEAM-02 (stub `NotConnected`)
- [ ] Framework install: none — `cargo-nextest` 0.9.137 already present
- [ ] (defensive) `.cargo/config.toml` `[patch.crates-io]` block — crates currently resolve without it, create for reproducibility (AGENTS.md)

*The existing 2 mock tests in `data/mock.rs` must keep passing — regression guard for "renders mock identically."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Studio renders mock data identically after async conversion | SEAM-01 | Visual render parity has no headless assertion this phase (no `dioxus-ssr`) | `cargo run -p nodedb-studio`; confirm connection manager + studio shell render mock data as before |
| `AsyncState` proof view shows Loading→Loaded, and Error on forced failure | SEAM-04 | Live render-loop state transitions; render-test dep (`dioxus-ssr`) deferred | `cargo run -p nodedb-studio`; observe the proven read (notifications feed) render Loaded; temporarily force the service method to return `Err` and confirm the Error state + Retry affordance |

*`dioxus-ssr` would convert these to automated render tests but is an ask-first new dependency — out of scope for Phase 1.*

---

## Validation Sign-Off

- [ ] All requirements have an `<automated>` verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (native feature, the 3 test files)
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter once plans satisfy the above

**Approval:** pending
