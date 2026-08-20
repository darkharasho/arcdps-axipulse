//! Per-player Pulse derives. Pure functions of a `&PlayerData`; never
//! mutate.
//!
//! Most of these are now a straight field read -- the native projection
//! already did the phase-0 indexing and the `Option` unwrapping the
//! Elite Insights reader had to do here. The ones that are still
//! functions either combine two fields (`cleanses`) or have to decide
//! how to report an ABSENT measurement, which is the only interesting
//! thing left in this module. Nothing here ever substitutes a zero for
//! an unknown.

use crate::fight_data::PlayerData;

pub fn damage(p: &PlayerData) -> u64 { p.damage }
pub fn dps_value(p: &PlayerData) -> u64 { p.dps }
pub fn breakbar_damage(p: &PlayerData) -> u64 { p.breakbar_damage }

/// Total cleanses: allies plus self. Native splits the two
/// (`support.cleanses` counts allies only), matching Elite Insights'
/// `condiCleanse` / `condiCleanseSelf` pair, so the sum is still the
/// "cleanses" the Pulse card means.
pub fn cleanses(p: &PlayerData) -> u64 {
    u64::from(p.cleanses) + u64::from(p.cleanses_self)
}
pub fn cleanse_self(p: &PlayerData) -> u64 { u64::from(p.cleanses_self) }
pub fn strips(p: &PlayerData) -> u64 { u64::from(p.strips) }

/// Mean distance to the commander over this player's active polls, in
/// world inches -- or `None` when there is no such measurement.
///
/// `None` is NOT zero and must not be rendered as one: it means either
/// that no commander reference existed for this player's polls or that
/// the replay pass never ran. `PlayerData::dist_to_com` has already
/// absorbed Elite Insights' `-1` sentinel, so a negative value can never
/// reach here.
pub fn dist_to_tag(p: &PlayerData) -> Option<f32> { p.dist_to_com }

pub fn damage_taken(p: &PlayerData) -> u64 { p.damage_taken }
pub fn deaths(p: &PlayerData)       -> u32 { p.deaths }
pub fn downs(p: &PlayerData)        -> u32 { p.downs }
pub fn dodges(p: &PlayerData)       -> u32 { p.dodges }
pub fn blocked(p: &PlayerData)      -> u32 { p.blocked }
pub fn evaded(p: &PlayerData)       -> u32 { p.evaded }
pub fn missed(p: &PlayerData)       -> u32 { p.missed }
pub fn invulned(p: &PlayerData)     -> u32 { p.invulned }
pub fn interrupted(p: &PlayerData)  -> u32 { p.interrupted }
pub fn incoming_cc(p: &PlayerData)  -> u64 { u64::from(p.incoming_cc) }
pub fn incoming_strips(p: &PlayerData) -> u64 { u64::from(p.incoming_strips) }

/// `blocks.contribution.by_entity[id].downs_contribution.damage`.
///
/// The Elite Insights reader needed a fallback here (`statsAll[0]
/// .downContribution` came through as 0 on WvW logs, so it re-summed
/// `totalDamageDist`). The native contribution block is always-on and
/// always populated, so the fallback is gone rather than kept as dead
/// code -- see `PlayerData::down_contribution_by_skill`'s doc comment.
pub fn down_contribution(p: &PlayerData) -> u64 { p.down_contribution }

// --- arcdps healing addon derives ----------------------------------
//
// These four are only meaningful when `FightData::healing_available` is
// true. When it is false they are all a structural 0 standing in for
// "the log has no healing addon data at all", and the caller must show
// its not-available treatment instead of the number. The gate is
// log-wide, not per-player, so it lives on `FightData`, not here.

pub fn healing(p: &PlayerData) -> u64 { p.healing_out }

/// Healing per second over the fight's own duration.
///
/// Elite Insights published an `hps` field; axilog does not, because it
/// is a pure function of the two numbers already on the row. Deriving it
/// here keeps the single definition of "per second" (fight duration, not
/// active time) visible instead of trusting two upstreams to agree.
/// A zero-length fight reports 0 rather than dividing by zero.
pub fn hps(p: &PlayerData, duration_ms: u64) -> u64 {
    if duration_ms == 0 { return 0; }
    (p.healing_out as u128 * 1000 / duration_ms as u128) as u64
}

pub fn healing_downed(p: &PlayerData) -> u64 { p.downed_healing_out }
pub fn barrier(p: &PlayerData) -> u64 { p.barrier_out }

/// Total incoming healing: the last bucket of the CUMULATIVE
/// `healing_received_1s` series, which is what that series' final value
/// means. Empty series (no healing addon, or no series row) reports 0 --
/// the caller gates on `FightData::healing_available` before showing it.
pub fn incoming_healing(p: &PlayerData) -> u64 {
    p.healing_received_1s.last().copied().unwrap_or(0)
}

pub fn incoming_barrier(p: &PlayerData) -> u64 {
    p.barrier_received_1s.last().copied().unwrap_or(0)
}
