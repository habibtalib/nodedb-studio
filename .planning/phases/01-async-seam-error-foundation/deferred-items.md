# Deferred Items — Phase 01 (async-seam-error-foundation)

Out-of-scope discoveries logged during execution. Do NOT fix as part of the
discovering plan; route to a follow-up.

## Pre-existing `cargo fmt` violation in `app.rs` (found during 01-03)

- **File:** `nodedb-studio/src/app.rs:12`
- **Issue:** Import ordering is not rustfmt-clean — `use crate::modals::ModalHost;`
  and `use crate::models::notification::Notification;` are out of alphabetical order.
  `cargo fmt --all -- --check` exits 1 on this single file.
- **Introduced by:** commit `60f6ad1` (plan 01-02, "convert ConnectionService to async…").
  Not caused by 01-03 — 01-03's new files (`services/async_state.rs`,
  `components/async_view.rs`) are rustfmt-clean.
- **Why deferred:** Out of scope for 01-03 (unrelated file, prior plan). The CI
  fmt gate is shared, so this should be fixed promptly — recommend a one-line
  `cargo fmt --all` cleanup in the next plan (01-04) or a `chore` fix.
- **Fix:** Run `cargo fmt --all`; it reorders the two imports. Zero behavior change.
