//! Roll up a player's per-skill damage / down-contribution distributions
//! into sortable entries.
//!
//! The two distributions come from two different native blocks
//! (`blocks.damage.by_entity[id].by_skill` and
//! `blocks.contribution.by_entity[id].downs_contribution_by_skill`), not
//! from one fused row the way Elite Insights' `totalDamageDist` did --
//! see `PlayerData::down_contribution_by_skill`'s doc comment for why
//! contribution lives off its own always-on block. Each `top_*` function
//! therefore reads only the list that measures its own quantity, and
//! leaves the other field at 0.

use crate::fight_data::{PlayerData, SkillRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    pub id: u32,
    pub name: String,
    pub damage: u64,
    pub down_contribution: u64,
}

/// Renders a row's label. `SkillRow::name` is resolved from
/// `catalogs.skills` at projection time and is empty only for an id the
/// catalog has no entry for at all -- rare, and "Skill <id>" is a more
/// honest label for it than a blank.
///
/// A name that is nothing but digits is rejected too, not just an empty
/// one. This carries over a rule from the Elite Insights reader
/// (`ui/pulse.rs::resolve_skill_name`, deleted in the cutover): a
/// catalog whose "name" for an id is that id -- or any bare number --
/// has not actually named it, and rendering `12345` as a skill label
/// reads as a value rather than an identifier. `Skill 12345` at least
/// says what it is.
pub fn skill_label(row: &SkillRow) -> String {
    if row.name.is_empty() || row.name.parse::<i64>().is_ok() {
        format!("Skill {}", row.skill_id)
    } else {
        row.name.clone()
    }
}

pub fn top_damage(p: &PlayerData, limit: usize) -> Vec<SkillEntry> {
    let mut entries: Vec<SkillEntry> = p
        .damage_by_skill
        .iter()
        .filter(|r| r.total > 0)
        .map(|r| SkillEntry {
            id: r.skill_id,
            name: skill_label(r),
            damage: r.total,
            down_contribution: 0,
        })
        .collect();
    entries.sort_by(|a, b| b.damage.cmp(&a.damage));
    entries.truncate(limit);
    entries
}

pub fn top_down_contribution(p: &PlayerData, limit: usize) -> Vec<SkillEntry> {
    let mut entries: Vec<SkillEntry> = p
        .down_contribution_by_skill
        .iter()
        .filter(|r| r.total > 0)
        .map(|r| SkillEntry {
            id: r.skill_id,
            name: skill_label(r),
            damage: 0,
            down_contribution: r.total,
        })
        .collect();
    entries.sort_by(|a, b| b.down_contribution.cmp(&a.down_contribution));
    entries.truncate(limit);
    entries
}
