//! Extract per-buff active-interval lists from `EiPlayer.buff_uptimes`.

use crate::ei_model::EiPlayer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone)]
pub struct BoonSeries {
    pub id: i64,
    pub name: &'static str,
    pub segments: Vec<Segment>,
}

const OFFENSIVE_IDS: &[(i64, &str)] = &[
    (740,   "Might"),
    (725,   "Fury"),
    (1187,  "Quickness"),
    (30328, "Alacrity"),
];

const DEFENSIVE_IDS: &[(i64, &str)] = &[
    (717,   "Protection"),
    (26980, "Resistance"),
    (1122,  "Stability"),
    (743,   "Aegis"),
];

pub fn active_segments(states: &[Vec<f64>], duration_ms: u64) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut active_start: Option<u64> = None;
    for pair in states {
        if pair.len() < 2 { continue; }
        let t = pair[0].max(0.0) as u64;
        let v = pair[1];
        match (active_start, v > 0.0) {
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

fn series_for(p: &EiPlayer, list: &[(i64, &'static str)], duration_ms: u64) -> Vec<BoonSeries> {
    list.iter().map(|(id, name)| {
        let segments = p.buff_uptimes.iter()
            .find(|b| b.id == *id)
            .map(|b| active_segments(&b.states, duration_ms))
            .unwrap_or_default();
        BoonSeries { id: *id, name, segments }
    }).collect()
}

pub fn offensive_boons(p: &EiPlayer, duration_ms: u64) -> Vec<BoonSeries> {
    series_for(p, OFFENSIVE_IDS, duration_ms)
}

pub fn defensive_boons(p: &EiPlayer, duration_ms: u64) -> Vec<BoonSeries> {
    series_for(p, DEFENSIVE_IDS, duration_ms)
}
