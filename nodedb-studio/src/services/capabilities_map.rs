//! Server capability bitmask -> studio `Capabilities` mapping (CONN-05).
//!
//! The server reports its feature set as a `u64` bitmask (the `CAP_*` constants
//! from `nodedb_types::protocol`). This module maps that mask, bit-by-bit, onto
//! the studio's `Capabilities` struct so the rail, admin sub-tabs, and routed
//! views gate on what the server actually confirms.
//!
//! Mapping (validated against `../nodedb` on 2026-06-14):
//! `CAP_GRAPHRAG -> graph`, `CAP_FTS -> fts`, `CAP_SPATIAL -> spatial`,
//! `CAP_STREAMING -> streams`, `CAP_TIMESERIES -> timeseries`, `CAP_CRDT -> sync`.
//! `CAP_COLUMNAR`/`CAP_MSGPACK` have no studio field and are ignored.
//!
//! Three studio flags have no server bit (per CONTEXT.md D-05/D-06):
//! - `vector` is a core, always-on engine -> `true`.
//! - `cluster` is indeterminate from the native handshake -> `false` (hidden).
//! - `readonly` is indeterminate at connect time -> `false` (lean writable).
//!
//! Conservative gating (D-04): an absent bit yields `false`, so a never-probed
//! or all-zero mask hides every optional feature rather than showing a tab that
//! would error on use.

use nodedb_client::Capabilities as ClientCaps;

use crate::state::connection::Capabilities;

/// Derive the studio `Capabilities` from a server capability bitmask.
///
/// Pure and total: every studio field is set explicitly from a confirmed bit or
/// a documented default. No catch-all, no panic.
///
/// Consumed by the live connect path (Plan 02) and the mock connect fallback
/// (wired in Task 2 of this plan); scaffolded + bit-by-bit unit-tested here.
#[allow(dead_code)]
pub fn derive_capabilities(bits: u64) -> Capabilities {
    let caps = ClientCaps::from_raw(bits);
    Capabilities {
        graph: caps.supports_graphrag(), // CAP_GRAPHRAG (the graph bit IS GraphRAG)
        fts: caps.supports_fts(),        // CAP_FTS
        spatial: caps.supports_spatial(), // CAP_SPATIAL
        streams: caps.supports_streaming(), // CAP_STREAMING
        timeseries: caps.supports_timeseries(), // CAP_TIMESERIES
        sync: caps.supports_crdt(),      // CAP_CRDT (peer replication == studio sync)
        vector: true,                    // core engine, no bit (D-06)
        cluster: false,                  // no native bit, indeterminate (D-06)
        readonly: false,                 // indeterminate at connect; lean writable (D-05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodedb_types::protocol::{
        CAP_CRDT, CAP_FTS, CAP_GRAPHRAG, CAP_SPATIAL, CAP_STREAMING, CAP_TIMESERIES,
    };

    #[test]
    fn maps_graphrag_bit_to_graph_capability() {
        let caps = derive_capabilities(CAP_GRAPHRAG | CAP_FTS);
        assert!(caps.graph);
        assert!(caps.fts);
        assert!(!caps.sync);
    }

    #[test]
    fn maps_all_bits() {
        let bits = CAP_STREAMING | CAP_GRAPHRAG | CAP_FTS | CAP_CRDT | CAP_SPATIAL | CAP_TIMESERIES;
        let caps = derive_capabilities(bits);
        assert!(caps.streams);
        assert!(caps.graph);
        assert!(caps.fts);
        assert!(caps.sync);
        assert!(caps.spatial);
        assert!(caps.timeseries);
    }

    #[test]
    fn maps_zero_bits_to_all_false_features() {
        let caps = derive_capabilities(0);
        assert!(!caps.graph);
        assert!(!caps.fts);
        assert!(!caps.spatial);
        assert!(!caps.streams);
        assert!(!caps.timeseries);
        assert!(!caps.sync);
    }

    #[test]
    fn vector_is_always_on() {
        // Core engine, no server bit: vector stays on even with an empty mask.
        assert!(derive_capabilities(0).vector);
    }

    #[test]
    fn cluster_defaults_off() {
        // No native bit -> indeterminate -> hidden.
        assert!(!derive_capabilities(0).cluster);
    }

    #[test]
    fn readonly_defaults_writable() {
        // Indeterminate at connect -> lean writable.
        assert!(!derive_capabilities(0).readonly);
    }
}
