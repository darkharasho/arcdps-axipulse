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
