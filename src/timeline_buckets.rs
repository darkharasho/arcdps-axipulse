//! Turn the cumulative per-second damage series into deltas suitable
//! for area-chart rendering.

use crate::fight_data::PlayerData;

pub fn cumulative_to_per_second(cumulative: &[u64]) -> Vec<u64> {
    if cumulative.is_empty() { return Vec::new(); }
    let mut out = Vec::with_capacity(cumulative.len());
    out.push(cumulative[0]);
    for i in 1..cumulative.len() {
        out.push(cumulative[i].saturating_sub(cumulative[i - 1]));
    }
    out
}

pub fn extract_damage_dealt(p: &PlayerData) -> Vec<u64> {
    cumulative_to_per_second(&p.damage_1s)
}

pub fn extract_damage_taken(p: &PlayerData) -> Vec<u64> {
    cumulative_to_per_second(&p.damage_taken_1s)
}

/// Empty when the log has no healing addon data (`healing_available ==
/// false`) or this player has no series row -- `healing_received_1s` is
/// already empty in both cases, per its own doc comment, so this simply
/// carries that absence through rather than fabricating a zero-filled
/// per-second series.
pub fn extract_incoming_healing(p: &PlayerData) -> Vec<u64> {
    cumulative_to_per_second(&p.healing_received_1s)
}

/// Same absence rule as [`extract_incoming_healing`], off
/// `barrier_received_1s`.
pub fn extract_incoming_barrier(p: &PlayerData) -> Vec<u64> {
    cumulative_to_per_second(&p.barrier_received_1s)
}
