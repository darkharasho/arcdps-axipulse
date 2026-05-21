//! Roll up `EiPlayer.totalDamageDist` into sortable per-skill entries.

use crate::ei_model::EiPlayer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    pub id: i64,
    pub name: String,
    pub damage: u64,
    pub down_contribution: u64,
}

fn flatten(p: &EiPlayer) -> Vec<SkillEntry> {
    let mut entries: Vec<SkillEntry> = Vec::new();
    for phase in &p.total_damage_dist {
        for e in phase {
            entries.push(SkillEntry {
                id: e.id,
                name: e.name.clone(),
                damage: e.total_damage,
                down_contribution: e.down_contribution,
            });
        }
    }
    entries
}

pub fn top_damage(p: &EiPlayer, limit: usize) -> Vec<SkillEntry> {
    let mut entries = flatten(p);
    entries.retain(|e| e.damage > 0);
    entries.sort_by(|a, b| b.damage.cmp(&a.damage));
    entries.truncate(limit);
    entries
}

pub fn top_down_contribution(p: &EiPlayer, limit: usize) -> Vec<SkillEntry> {
    let mut entries = flatten(p);
    entries.retain(|e| e.down_contribution > 0);
    entries.sort_by(|a, b| b.down_contribution.cmp(&a.down_contribution));
    entries.truncate(limit);
    entries
}
