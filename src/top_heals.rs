//! Roll up a player's per-skill healing / barrier distributions into
//! sortable entries. Mirrors the pattern in `top_skills`.
//!
//! Both source lists (`PlayerData::healing_by_skill` /
//! `barrier_by_skill`) are empty when the log carries no arcdps healing
//! addon data at all -- callers must check `FightData::healing_available`
//! before reading an empty result as "this player healed nothing".

use crate::fight_data::PlayerData;
use crate::top_skills::skill_label;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealEntry {
    pub id: u32,
    pub name: String,
    pub healing: u64,
    pub downed_healing: u64,
    pub hits: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarrierEntry {
    pub id: u32,
    pub name: String,
    pub barrier: u64,
    pub hits: u64,
}

fn heal_entries(p: &PlayerData) -> Vec<HealEntry> {
    p.healing_by_skill
        .iter()
        .map(|r| HealEntry {
            id: r.skill_id,
            name: skill_label(r),
            healing: r.total,
            downed_healing: r.downed,
            hits: u64::from(r.hits),
        })
        .collect()
}

pub fn top_healing(p: &PlayerData, limit: usize) -> Vec<HealEntry> {
    let mut entries = heal_entries(p);
    entries.retain(|e| e.healing > 0);
    entries.sort_by(|a, b| b.healing.cmp(&a.healing));
    entries.truncate(limit);
    entries
}

pub fn top_downed_healing(p: &PlayerData, limit: usize) -> Vec<HealEntry> {
    let mut entries = heal_entries(p);
    entries.retain(|e| e.downed_healing > 0);
    entries.sort_by(|a, b| b.downed_healing.cmp(&a.downed_healing));
    entries.truncate(limit);
    entries
}

pub fn top_barrier(p: &PlayerData, limit: usize) -> Vec<BarrierEntry> {
    let mut entries: Vec<BarrierEntry> = p
        .barrier_by_skill
        .iter()
        .filter(|r| r.total > 0)
        .map(|r| BarrierEntry {
            id: r.skill_id,
            name: skill_label(r),
            barrier: r.total,
            hits: u64::from(r.hits),
        })
        .collect();
    entries.sort_by(|a, b| b.barrier.cmp(&a.barrier));
    entries.truncate(limit);
    entries
}
