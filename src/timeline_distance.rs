//! Euclidean distance between the local player and the commander at
//! each 1-second tick, in in-game inches.

use crate::ei_model::EiJson;

pub fn distance_to_commander_per_second(json: &EiJson, self_idx: usize, duration_ms: u64) -> Vec<f64> {
    let Some(meta) = json.combat_replay_meta_data.as_ref() else { return Vec::new() };
    let polling_rate = meta.polling_rate.unwrap_or(150).max(1) as f64;
    let inch_to_pixel = meta.inch_to_pixel.unwrap_or(1.0).max(1e-6);

    let Some(me) = json.players.get(self_idx) else { return Vec::new() };
    let Some(me_replay) = me.combat_replay_data.as_ref() else { return Vec::new() };
    if me_replay.positions.is_empty() { return Vec::new(); }

    let Some(commander) = json.players.iter().find(|p| p.has_commander_tag) else { return Vec::new() };
    let Some(cmdr_replay) = commander.combat_replay_data.as_ref() else { return Vec::new() };
    if cmdr_replay.positions.is_empty() { return Vec::new(); }

    let samples_per_sec = (1000.0 / polling_rate).max(1.0).round() as usize;
    let seconds = (duration_ms / 1000) as usize + 1;
    let n = me_replay.positions.len().min(cmdr_replay.positions.len());

    let mut out = Vec::with_capacity(seconds);
    for sec in 0..seconds {
        let start = sec * samples_per_sec;
        if start >= n { out.push(out.last().copied().unwrap_or(0.0)); continue; }
        let end = (start + samples_per_sec).min(n);
        let mut sum = 0.0;
        let mut count = 0.0;
        for i in start..end {
            let (mx, my) = if me_replay.positions[i].len() >= 2 {
                (me_replay.positions[i][0], me_replay.positions[i][1])
            } else {
                continue;
            };
            let (cx, cy) = if cmdr_replay.positions[i].len() >= 2 {
                (cmdr_replay.positions[i][0], cmdr_replay.positions[i][1])
            } else {
                continue;
            };
            let dx = mx - cx;
            let dy = my - cy;
            sum += (dx * dx + dy * dy).sqrt() / inch_to_pixel;
            count += 1.0;
        }
        out.push(if count > 0.0 { sum / count } else { 0.0 });
    }
    out
}
