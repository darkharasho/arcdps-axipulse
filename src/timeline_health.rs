//! Sample `PlayerData::health_percents` (a step function) at 1Hz.

use crate::fight_data::PlayerData;

/// One sample per second from 0 to `duration_ms` inclusive. Each value
/// is the player's health percentage AT that second, from the most
/// recent state at or before that timestamp.
///
/// Returns an EMPTY vec when the player has no health states at all --
/// `FightData::from_report` folds both "the pass never ran" and "the
/// pass ran and saw no `HEALTH_UPDATE` for this entity" into an empty
/// `health_percents`, and neither of those is a measurement of full
/// health. A squad member who died must not draw as a flat 100% lane, so
/// the absence is handed to the caller as an empty lane (which
/// `ui::timeline` routes to `draw_empty_lane`) rather than fabricated.
///
/// The pre-first-sample fill is a different thing and stays: seconds
/// before the first state carry `states[0].1`, which IS a measurement --
/// the earliest one this player has.
pub fn sample_health_per_second(p: &PlayerData, duration_ms: u64) -> Vec<f64> {
    if duration_ms == 0 {
        return Vec::new();
    }
    let states = &p.health_percents;
    let Some(first) = states.first() else {
        return Vec::new();
    };

    let seconds = (duration_ms / 1000) as usize + 1;
    let mut out = Vec::with_capacity(seconds);
    let mut state_idx = 0usize;
    let mut current = first.1;

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
