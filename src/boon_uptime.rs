//! Standard-WvW boon table + uptime extraction from `EiPlayer.buff_uptimes`.

use crate::ei_model::EiPlayer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoonStacking {
    Intensity,
    Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoonUptime {
    pub id: i64,
    pub name: &'static str,
    pub uptime: f64,
    pub stacking: BoonStacking,
}

pub const KNOWN_BOONS: &[(i64, &str, BoonStacking)] = &[
    (740,   "Might",       BoonStacking::Intensity),
    (725,   "Fury",        BoonStacking::Duration),
    (1187,  "Quickness",   BoonStacking::Duration),
    (30328, "Alacrity",    BoonStacking::Duration),
    (717,   "Protection",  BoonStacking::Duration),
    (718,   "Regeneration",BoonStacking::Duration),
    (726,   "Vigor",       BoonStacking::Duration),
    (719,   "Swiftness",   BoonStacking::Duration),
    (26980, "Resistance",  BoonStacking::Duration),
    (1122,  "Stability",   BoonStacking::Intensity),
    (743,   "Aegis",       BoonStacking::Duration),
    (873,   "Resolution",  BoonStacking::Duration),
    (757,   "Retaliation", BoonStacking::Duration),
];

pub fn boon_name(id: i64) -> Option<&'static str> {
    KNOWN_BOONS.iter().find(|(i, _, _)| *i == id).map(|(_, n, _)| *n)
}

pub fn boon_stacking(id: i64) -> BoonStacking {
    KNOWN_BOONS.iter().find(|(i, _, _)| *i == id)
        .map(|(_, _, s)| *s)
        .unwrap_or(BoonStacking::Duration)
}

pub fn collect_uptimes(p: &EiPlayer) -> Vec<BoonUptime> {
    let mut by_id: std::collections::HashMap<i64, f64> =
        std::collections::HashMap::with_capacity(p.buff_uptimes.len());
    for entry in &p.buff_uptimes {
        let uptime = entry.buff_data.first().map(|d| d.uptime).unwrap_or(0.0);
        by_id.insert(entry.id, uptime);
    }
    KNOWN_BOONS.iter()
        .filter_map(|(id, name, stacking)| {
            by_id.get(id).map(|uptime| BoonUptime {
                id: *id, name, uptime: *uptime, stacking: *stacking,
            })
        })
        .collect()
}
