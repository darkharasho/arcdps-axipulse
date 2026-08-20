//! Extract per-buff active-interval lists from `PlayerData::boons`.

use crate::fight_data::PlayerData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone)]
pub struct BoonSeries {
    pub id: u32,
    pub name: &'static str,
    pub segments: Vec<Segment>,
}

const OFFENSIVE_IDS: &[(u32, &str)] = &[
    (740,   "Might"),
    (725,   "Fury"),
    (1187,  "Quickness"),
    (30328, "Alacrity"),
];

const DEFENSIVE_IDS: &[(u32, &str)] = &[
    (717,   "Protection"),
    (26980, "Resistance"),
    (1122,  "Stability"),
    (743,   "Aegis"),
];

/// `states` is `BoonRow::states` -- `(time_ms_from_log_start, stacks)`
/// transitions. A trailing run that never returns to zero is closed at
/// `duration_ms`.
pub fn active_segments(states: &[(u64, i32)], duration_ms: u64) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut active_start: Option<u64> = None;
    for (t, v) in states {
        let t = *t;
        let v = *v;
        match (active_start, v > 0) {
            (None, true) => active_start = Some(t),
            (Some(start), false) => {
                out.push(Segment { start_ms: start, end_ms: t });
                active_start = None;
            }
            _ => {}
        }
    }
    if let Some(start) = active_start {
        out.push(Segment { start_ms: start, end_ms: duration_ms });
    }
    out
}

fn series_for(p: &PlayerData, list: &[(u32, &'static str)], duration_ms: u64) -> Vec<BoonSeries> {
    list.iter().map(|(id, name)| {
        let segments = p.boons.iter()
            .find(|b| b.buff_id == *id)
            .map(|b| active_segments(&b.states, duration_ms))
            .unwrap_or_default();
        BoonSeries { id: *id, name, segments }
    }).collect()
}

pub fn offensive_boons(p: &PlayerData, duration_ms: u64) -> Vec<BoonSeries> {
    series_for(p, OFFENSIVE_IDS, duration_ms)
}

pub fn defensive_boons(p: &PlayerData, duration_ms: u64) -> Vec<BoonSeries> {
    series_for(p, DEFENSIVE_IDS, duration_ms)
}
