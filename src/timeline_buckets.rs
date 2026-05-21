//! Turn EI's cumulative per-second damage arrays into deltas suitable
//! for area-chart rendering.

use crate::ei_model::EiPlayer;

pub fn cumulative_to_per_second(cumulative: &[u64]) -> Vec<u64> {
    if cumulative.is_empty() { return Vec::new(); }
    let mut out = Vec::with_capacity(cumulative.len());
    out.push(cumulative[0]);
    for i in 1..cumulative.len() {
        out.push(cumulative[i].saturating_sub(cumulative[i - 1]));
    }
    out
}

pub fn extract_damage_dealt(p: &EiPlayer) -> Vec<u64> {
    let Some(phase) = p.damage_1s.get(0) else { return Vec::new(); };
    cumulative_to_per_second(phase)
}

pub fn extract_damage_taken(p: &EiPlayer) -> Vec<u64> {
    let Some(phase) = p.damage_taken_1s.get(0) else { return Vec::new(); };
    cumulative_to_per_second(phase)
}
