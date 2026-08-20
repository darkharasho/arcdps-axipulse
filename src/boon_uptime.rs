//! Standard-WvW boon table + uptime extraction from
//! `PlayerData::boons`.
//!
//! The table below is the canonical display ORDER and label set, and
//! nothing else: this list is a curated WvW subset in a deliberate
//! order, not a catalog dump, and the panel's layout depends on it.
//! Which of a row's two numbers the bar shows is decided by
//! `BoonRow::stacking` -- axilog's own per-buff answer -- so there is
//! only ever one source of truth for that.

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

/// Display ORDER and LABELS only. This table is deliberately NOT the
/// decider of stacking mode any more: `BoonRow::stacking` carries
/// axilog's own per-buff answer, straight from `catalogs.buffs`, and
/// that is what [`collect_uptimes`] reads. Two tables that can disagree
/// would misdraw a bar rather than fail.
pub const KNOWN_BOONS: &[(u32, &str)] = &[
    (740,   "Might"),
    (725,   "Fury"),
    (1187,  "Quickness"),
    (30328, "Alacrity"),
    (717,   "Protection"),
    (718,   "Regeneration"),
    (726,   "Vigor"),
    (719,   "Swiftness"),
    (26980, "Resistance"),
    (1122,  "Stability"),
    (743,   "Aegis"),
    (873,   "Resolution"),
    (757,   "Retaliation"),
];

pub fn boon_name(id: u32) -> Option<&'static str> {
    KNOWN_BOONS.iter().find(|(i, _)| *i == id).map(|(_, n)| *n)
}

/// `BoonRow::stacking` as native writes it -- `"intensity"` or
/// `"duration"`, copied out of `BuffEntry::stacking` at projection time.
///
/// `None` for anything else. An unrecognised mode is not a default to
/// fall back on: `uptime` means average STACKS under one mode and
/// percent UPTIME under the other, and the renderer divides by 25 or by
/// 100 accordingly, so guessing would silently misdraw the bar. The
/// caller omits the row instead.
pub fn parse_stacking(s: &str) -> Option<BoonStacking> {
    match s {
        "intensity" => Some(BoonStacking::Intensity),
        "duration" => Some(BoonStacking::Duration),
        _ => None,
    }
}

/// One row per known boon the player actually held, in `KNOWN_BOONS`
/// order. A boon with no row at all is OMITTED rather than reported as
/// zero -- the same shape the Elite Insights reader had, and the same
/// reason: the panel should not claim a measured 0% for a buff the log
/// never mentions.
///
/// `uptime` means two different things by `stacking`: average STACKS for
/// an intensity buff (`BoonRow::avg_stacks`) and percent UPTIME for a
/// duration buff (`BoonRow::uptime_pct`). The renderer divides by 25 or
/// by 100 accordingly, so mixing the two up would silently misdraw the
/// bar rather than fail -- which is exactly why the mode comes from
/// `BoonRow::stacking`, axilog's own authoritative per-buff answer,
/// rather than from a second hardcoded table here.
///
/// A row whose `stacking` native did not fill in with a mode this crate
/// recognises is OMITTED, for the same reason a missing row is: nothing
/// says what its number means.
///
/// An intensity buff with no `avg_stacks` at all (native leaves it
/// `None` when the player never held it long enough to average) reports
/// 0.0 stacks -- that IS the measurement, not a stand-in for one.
pub fn collect_uptimes(p: &PlayerData) -> Vec<BoonUptime> {
    KNOWN_BOONS.iter()
        .filter_map(|(id, name)| {
            let row = p.boons.iter().find(|b| b.buff_id == *id)?;
            let stacking = parse_stacking(&row.stacking)?;
            let uptime = match stacking {
                BoonStacking::Intensity => row.avg_stacks.unwrap_or(0.0),
                BoonStacking::Duration => row.uptime_pct,
            };
            Some(BoonUptime { id: *id, name, uptime, stacking })
        })
        .collect()
}
