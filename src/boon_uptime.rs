//! Standard-WvW boon table + uptime extraction from
//! `PlayerData::boons`.
//!
//! The table below stays the canonical display ORDER and label set even
//! though `BoonRow` now carries axilog's own name and stacking mode:
//! this list is a curated WvW subset in a deliberate order, not a
//! catalog dump, and the panel's layout depends on it.

use crate::fight_data::PlayerData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoonStacking {
    Intensity,
    Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoonUptime {
    pub id: u32,
    pub name: &'static str,
    pub uptime: f64,
    pub stacking: BoonStacking,
}

pub const KNOWN_BOONS: &[(u32, &str, BoonStacking)] = &[
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

pub fn boon_name(id: u32) -> Option<&'static str> {
    KNOWN_BOONS.iter().find(|(i, _, _)| *i == id).map(|(_, n, _)| *n)
}

pub fn boon_stacking(id: u32) -> BoonStacking {
    KNOWN_BOONS.iter().find(|(i, _, _)| *i == id)
        .map(|(_, _, s)| *s)
        .unwrap_or(BoonStacking::Duration)
}

/// One row per known boon the player actually held, in `KNOWN_BOONS`
/// order. A boon with no row at all is OMITTED rather than reported as
/// zero -- the same shape the Elite Insights reader had, and the same
/// reason: the panel should not claim a measured 0% for a buff the log
/// never mentions.
///
/// `uptime` means two different things by `stacking`, exactly as it did
/// before: average STACKS for an intensity buff (`BoonRow::avg_stacks`)
/// and percent UPTIME for a duration buff (`BoonRow::uptime_pct`). The
/// renderer divides by 25 or by 100 accordingly, so mixing the two up
/// would silently misdraw the bar rather than fail.
///
/// An intensity buff with no `avg_stacks` at all (native leaves it
/// `None` when the player never held it long enough to average) reports
/// 0.0 stacks -- that IS the measurement, not a stand-in for one.
pub fn collect_uptimes(p: &PlayerData) -> Vec<BoonUptime> {
    KNOWN_BOONS.iter()
        .filter_map(|(id, name, stacking)| {
            let row = p.boons.iter().find(|b| b.buff_id == *id)?;
            let uptime = match stacking {
                BoonStacking::Intensity => row.avg_stacks.unwrap_or(0.0),
                BoonStacking::Duration => row.uptime_pct,
            };
            Some(BoonUptime { id: *id, name, uptime, stacking: *stacking })
        })
        .collect()
}
