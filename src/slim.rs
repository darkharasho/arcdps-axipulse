//! Post-derive slimming of a parsed fight.
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
    EiJson, ExtBarrierStats, ExtHealingStats, OutgoingBarrierEntry, OutgoingHealEntry,
};
use crate::pulse_metrics as pm;

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
