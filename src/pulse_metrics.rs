//! Per-player Pulse derives. Port of axipulse/src/shared/dashboardMetrics.ts.
//! Pure functions of an &EiPlayer; never mutate.

use crate::ei_model::EiPlayer;

pub fn damage(p: &EiPlayer) -> u64        { p.dps_all.get(0).map(|d| d.damage).unwrap_or(0) }
pub fn dps_value(p: &EiPlayer) -> u64     { p.dps_all.get(0).map(|d| d.dps).unwrap_or(0) }
pub fn breakbar_damage(p: &EiPlayer) -> u64 {
    p.dps_all.get(0).map(|d| d.breakbar_damage).unwrap_or(0)
}

pub fn cleanses(p: &EiPlayer) -> u64 {
    match p.support.get(0) {
        Some(s) => s.condi_cleanse + s.condi_cleanse_self,
        None => 0,
    }
}
pub fn cleanse_self(p: &EiPlayer) -> u64 {
    p.support.get(0).map(|s| s.condi_cleanse_self).unwrap_or(0)
}
pub fn strips(p: &EiPlayer) -> u64 {
    p.support.get(0).map(|s| s.boon_strips).unwrap_or(0)
}

pub fn dist_to_tag(p: &EiPlayer) -> f64 {
    let s = match p.stats_all.get(0) { Some(s) => s, None => return 0.0 };
    if s.dist_to_com > 0.0 { s.dist_to_com } else { s.stack_dist }
}

pub fn damage_taken(p: &EiPlayer) -> u64 { p.defenses.get(0).map(|d| d.damage_taken).unwrap_or(0) }
pub fn deaths(p: &EiPlayer)       -> u32 { p.defenses.get(0).map(|d| d.dead_count).unwrap_or(0) }
pub fn downs(p: &EiPlayer)        -> u32 { p.defenses.get(0).map(|d| d.down_count).unwrap_or(0) }
pub fn dodges(p: &EiPlayer)       -> u32 { p.defenses.get(0).map(|d| d.dodge_count).unwrap_or(0) }
pub fn blocked(p: &EiPlayer)      -> u32 { p.defenses.get(0).map(|d| d.blocked_count).unwrap_or(0) }
pub fn evaded(p: &EiPlayer)       -> u32 { p.defenses.get(0).map(|d| d.evaded_count).unwrap_or(0) }
pub fn missed(p: &EiPlayer)       -> u32 { p.defenses.get(0).map(|d| d.missed_count).unwrap_or(0) }
pub fn invulned(p: &EiPlayer)     -> u32 { p.defenses.get(0).map(|d| d.invulned_count).unwrap_or(0) }
pub fn interrupted(p: &EiPlayer)  -> u32 { p.defenses.get(0).map(|d| d.interrupted_count).unwrap_or(0) }
pub fn incoming_cc(p: &EiPlayer)  -> u64 { p.defenses.get(0).map(|d| d.received_crowd_control).unwrap_or(0) }
pub fn incoming_strips(p: &EiPlayer) -> u64 { p.defenses.get(0).map(|d| d.boon_strips).unwrap_or(0) }

/// statsAll[0] is authoritative when populated. In WvW EI may aggregate
/// targets and leave the field at 0; fall back to summing
/// `downContribution` across totalDamageDist (matches axipulse).
pub fn down_contribution(p: &EiPlayer) -> u64 {
    let from_stats = p.stats_all.get(0).map(|s| s.down_contribution).unwrap_or(0);
    if from_stats > 0 { return from_stats; }
    p.total_damage_dist.iter().flatten().map(|e| e.down_contribution).sum()
}
