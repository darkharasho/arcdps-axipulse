//! Post-derive slimming of a parsed Elite Insights fight.
//!
//! **Orphaned as of the axilog cutover and deleted by migration Task 8.**
//! Nothing calls it: `plugin.rs` parses with `crate::parse::parse_log`
//! now, and a `FightData` has no oversized ext-stats trees to collapse
//! because the `ReportV1` it was projected from is dropped inside
//! `parse_log`. Its `tests/slim_test.rs` coverage is kept until Task 8
//! removes both together.
//!
//! A retained `EiJson` for a mid-size WvW fight measured ~78 MB, and
//! ~80% of that was `extHealingStats`/`extBarrierStats` arrays that the
//! per-frame UI only ever reads through the scalar accessors in
//! `pulse_metrics` (`Derived` has already consumed the rest on the
//! worker thread). Keeping full trees for every history slot inflated
//! the game process by gigabytes over a session, which pushed the box
//! into a zram swap storm on each parse — the post-fight lag.
//!
//! Call `slim_after_derive` after `Derived::compute`: it rewrites each
//! player's ext stats to the smallest structure for which every
//! `pulse_metrics` accessor returns the same value, and drops the
//! per-skill dist tables outright. Fields the map/timeline views read
//! directly (`combat_replay_data`, `buff_uptimes`, `health_percents`,
//! `rotation`) are left untouched.

use crate::ei_model::{
    EiJson, EiPlayer, ExtBarrierStats, ExtHealingStats, OutgoingBarrierEntry, OutgoingHealEntry,
};

// These five were `crate::pulse_metrics` accessors until that module
// moved onto `FightData`. Inlined here verbatim rather than deleted
// with their old home, so this module keeps compiling unchanged until
// Task 8 removes it whole. Do not add callers.
mod pm {
    use super::EiPlayer;

    pub fn healing(p: &EiPlayer) -> u64 {
        p.ext_healing_stats.as_ref()
            .map(|h| h.outgoing_healing_allies.iter()
                .filter_map(|recip| recip.first().map(|e| e.healing))
                .sum())
            .unwrap_or(0)
    }

    pub fn hps(p: &EiPlayer) -> u64 {
        p.ext_healing_stats.as_ref()
            .map(|h| h.outgoing_healing_allies.iter()
                .filter_map(|recip| recip.first().map(|e| e.hps))
                .sum())
            .unwrap_or(0)
    }

    pub fn healing_downed(p: &EiPlayer) -> u64 {
        p.ext_healing_stats.as_ref()
            .map(|h| h.outgoing_healing_allies.iter()
                .filter_map(|recip| recip.first().map(|e| e.downed_healing))
                .sum())
            .unwrap_or(0)
    }

    pub fn barrier(p: &EiPlayer) -> u64 {
        p.ext_barrier_stats.as_ref()
            .map(|h| h.outgoing_barrier_allies.iter()
                .filter_map(|recip| recip.first().map(|e| e.barrier))
                .sum())
            .unwrap_or(0)
    }

    pub fn incoming_healing(p: &EiPlayer) -> u64 {
        p.ext_healing_stats.as_ref()
            .and_then(|h| h.healing_received_1s.get(0))
            .and_then(|arr| arr.last().copied())
            .unwrap_or(0)
    }

    pub fn incoming_barrier(p: &EiPlayer) -> u64 {
        p.ext_barrier_stats.as_ref()
            .and_then(|h| h.barrier_received_1s.get(0))
            .and_then(|arr| arr.last().copied())
            .unwrap_or(0)
    }
}

pub fn slim_after_derive(json: &mut EiJson) {
    for p in &mut json.players {
        // Compute through the same accessors the UI uses, so the
        // collapsed form is equivalent by construction.
        if p.ext_healing_stats.is_some() {
            let (healing, hps, downed) = (pm::healing(p), pm::hps(p), pm::healing_downed(p));
            let incoming = pm::incoming_healing(p);
            p.ext_healing_stats = Some(ExtHealingStats {
                outgoing_healing_allies: vec![vec![OutgoingHealEntry {
                    healing,
                    hps,
                    downed_healing: downed,
                }]],
                total_healing_dist: Vec::new(),
                healing_received_1s: vec![vec![incoming]],
            });
        }
        if p.ext_barrier_stats.is_some() {
            let (barrier, incoming) = (pm::barrier(p), pm::incoming_barrier(p));
            p.ext_barrier_stats = Some(ExtBarrierStats {
                outgoing_barrier_allies: vec![vec![OutgoingBarrierEntry { barrier, bps: 0 }]],
                total_barrier_dist: Vec::new(),
                barrier_received_1s: vec![vec![incoming]],
            });
        }
    }
}
