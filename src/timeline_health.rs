//! Sample `PlayerData::health_percents` (a step function) at 1Hz.

use crate::fight_data::PlayerData;

/// One sample per second from 0 to `duration_ms` inclusive. Each value
/// is the player's health percentage AT that second, from the most
/// recent state at or before that timestamp. Pre-fight state is 100%.
pub fn sample_health_per_second(p: &PlayerData, duration_ms: u64) -> Vec<f64> {
    if duration_ms == 0 {
        return Vec::new();
    }
    let states = &p.health_percents;

    let seconds = (duration_ms / 1000) as usize + 1;
    let mut out = Vec::with_capacity(seconds);
    let mut state_idx = 0usize;
    let mut current = states.first().map(|s| s.1).unwrap_or(100.0);

    for sec in 0..seconds {
        let t = (sec as u64) * 1000;
        while state_idx < states.len() && states[state_idx].0 <= t {
            current = states[state_idx].1;
            state_idx += 1;
        }
        out.push(current);
    }
    out
}
