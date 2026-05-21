//! Roll up `EiPlayer.ext_healing_stats.total_healing_dist` and
//! `ext_barrier_stats.total_barrier_dist` into sortable per-skill
//! entries. Mirrors the pattern in `top_skills`.

use crate::ei_model::EiPlayer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealEntry {
    pub id: i64,
    pub healing: u64,
    pub downed_healing: u64,
    pub hits: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarrierEntry {
    pub id: i64,
    pub barrier: u64,
    pub hits: u64,
}

fn flatten_heals(p: &EiPlayer) -> Vec<HealEntry> {
    let Some(stats) = p.ext_healing_stats.as_ref() else { return Vec::new(); };
    stats.total_healing_dist.iter()
        .flatten()
        .map(|e| HealEntry {
            id: e.id,
            healing: e.total_healing,
            downed_healing: e.total_downed_healing,
            hits: e.hits,
        })
        .collect()
}

fn flatten_barriers(p: &EiPlayer) -> Vec<BarrierEntry> {
    let Some(stats) = p.ext_barrier_stats.as_ref() else { return Vec::new(); };
    stats.total_barrier_dist.iter()
        .flatten()
        .map(|e| BarrierEntry {
            id: e.id,
            barrier: e.total_barrier,
            hits: e.hits,
        })
        .collect()
}

pub fn top_healing(p: &EiPlayer, limit: usize) -> Vec<HealEntry> {
    let mut entries = flatten_heals(p);
    entries.retain(|e| e.healing > 0);
    entries.sort_by(|a, b| b.healing.cmp(&a.healing));
    entries.truncate(limit);
    entries
}

pub fn top_downed_healing(p: &EiPlayer, limit: usize) -> Vec<HealEntry> {
    let mut entries = flatten_heals(p);
    entries.retain(|e| e.downed_healing > 0);
    entries.sort_by(|a, b| b.downed_healing.cmp(&a.downed_healing));
    entries.truncate(limit);
    entries
}

pub fn top_barrier(p: &EiPlayer, limit: usize) -> Vec<BarrierEntry> {
    let mut entries = flatten_barriers(p);
    entries.retain(|e| e.barrier > 0);
    entries.sort_by(|a, b| b.barrier.cmp(&a.barrier));
    entries.truncate(limit);
    entries
}
