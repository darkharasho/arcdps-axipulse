//! Sample `EiPlayer.health_percents` (a step function) at 1Hz.

use crate::ei_model::EiPlayer;

/// One sample per second from 0 to `duration_ms` inclusive. Each value
/// is the player's health percentage AT that second, from the most
/// recent state at or before that timestamp. Pre-fight state is 100%.
pub fn sample_health_per_second(p: &EiPlayer, duration_ms: u64) -> Vec<f64> {
    if duration_ms == 0 {
        return Vec::new();
    }
    let states: Vec<(f64, f64)> = p
        .health_percents
        .iter()
        .filter_map(|pair| {
            if pair.len() >= 2 {
                Some((pair[0], pair[1]))
            } else {
                None
            }
        })
        .collect();

    let seconds = (duration_ms / 1000) as usize + 1;
    let mut out = Vec::with_capacity(seconds);
    let mut state_idx = 0usize;
    let mut current = states.first().map(|s| s.1).unwrap_or(100.0);

    for sec in 0..seconds {
        let t = (sec as f64) * 1000.0;
        while state_idx < states.len() && states[state_idx].0 <= t {
            current = states[state_idx].1;
            state_idx += 1;
        }
        out.push(current);
    }
    out
}
