//! The purpose-built projection of axilog's native `ReportV1` that the UI
//! will eventually read instead of the Elite Insights JSON tree.
//!
//! This module is additive for now (migration Task 2): nothing consumes
//! it yet, so the roster/identity fields it carries today are the only
//! ones proven against the frozen EI oracle. Later tasks add fields to
//! `PlayerData` and grow `from_report` to fill them; the shape of what is
//! already here does not change.
//!
//! `FightData::from_report` must not retain the `&ReportV1` it is handed
//! -- it reads it once and returns owned data, per the migration's
//! binding constraint that the native report is never stored past the
//! call that produced it.

#![allow(dead_code)] // Nothing wires this module up until migration Task 7.

use axilog_api::v1::entities::Role;
use axilog_api::v1::ReportV1;
use std::collections::HashMap;

/// Everything the UI needs about one fight, read once from a native
/// `ReportV1`.
#[derive(Debug, Clone)]
pub struct FightData {
    pub duration_ms: u64,
    pub map_name: String,
    pub map_id: Option<u32>,
    pub started_at_unix: Option<u64>,
    /// The log's `t0` in arcdps session time. Not log-relative itself --
    /// it IS the origin. Subtract this from `encounter.markers[].time_ms`
    /// and `entities[].commander.segments` before treating either as
    /// encounter-relative; nothing in this task reads those two fields.
    pub log_start_ms: u64,
    /// Index into `players` of the recording player, when the recorder
    /// resolves to a roster entity. Replaces `self_identify.rs` entirely.
    pub self_idx: Option<usize>,
    /// Index into `players` of the first roster entity that ever held the
    /// commander tag.
    pub commander_idx: Option<usize>,
    /// `Role::Squad` and `Role::FriendlyPlayer` entities, in the native
    /// report's sort order (squad first, then non-squad friendlies).
    pub players: Vec<PlayerData>,
    /// `Role::EnemyPlayer` entities. `Role::Npc` entities are dropped --
    /// this projection carries only players on both sides.
    pub enemies: Vec<EnemyData>,
    /// `EntityOut::id` -> index into `players`. Built once here so every
    /// later task that joins a `blocks.*` row (keyed by entity id) back
    /// onto a roster row doesn't have to re-derive it.
    entity_index: HashMap<u32, usize>,
}

#[derive(Debug, Clone)]
pub struct PlayerData {
    pub entity_id: u32,
    pub account: String,
    pub character: String,
    pub profession: String,
    /// Empty string when the agent has no elite spec, OR when it has one
    /// axilog's catalog cannot yet name (a diagnosed upstream gap, not a
    /// bug here -- see `from_report`'s doc comment).
    pub elite_spec: String,
    pub subgroup: i32,
    pub team: String,
    /// `true` for `Role::Squad`, `false` for `Role::FriendlyPlayer`.
    pub in_squad: bool,
    pub is_commander: bool,
}

#[derive(Debug, Clone)]
pub struct EnemyData {
    pub entity_id: u32,
    pub name: String,
    pub team: String,
    pub profession: String,
}

impl FightData {
    /// Reads a native `ReportV1` once into an owned `FightData`. Does not
    /// retain the reference.
    pub fn from_report(r: &ReportV1) -> Self {
        let mut players = Vec::new();
        let mut enemies = Vec::new();
        let mut entity_index = HashMap::new();

        for e in &r.entities {
            match e.role {
                Role::Squad | Role::FriendlyPlayer => {
                    entity_index.insert(e.id, players.len());
                    players.push(PlayerData {
                        entity_id: e.id,
                        account: e.account.clone().unwrap_or_default(),
                        character: e.character.clone().unwrap_or_default(),
                        profession: e.profession.clone().unwrap_or_default(),
                        // Never a numeric spec id (axilog-schema's
                        // contract on `EntityOut::elite_spec`): empty
                        // string is either "no elite spec" or "axilog
                        // can't name this one yet". Both cases keep the
                        // empty string -- the UI already handles it, and
                        // this task does not invent a spec lookup table
                        // to disambiguate them.
                        elite_spec: e.elite_spec.clone().unwrap_or_default(),
                        subgroup: e.subgroup.map(i32::from).unwrap_or_default(),
                        team: e.team.clone(),
                        in_squad: matches!(e.role, Role::Squad),
                        is_commander: e.commander.is_some(),
                    });
                }
                Role::EnemyPlayer => {
                    enemies.push(EnemyData {
                        entity_id: e.id,
                        name: e.name.clone().unwrap_or_default(),
                        team: e.team.clone(),
                        profession: e.profession.clone().unwrap_or_default(),
                    });
                }
                Role::Npc => {}
            }
        }

        let self_idx = r
            .encounter
            .recorded_by
            .and_then(|id| entity_index.get(&id).copied());
        let commander_idx = players.iter().position(|p| p.is_commander);

        FightData {
            duration_ms: r.encounter.duration_ms,
            map_name: r.encounter.map.clone(),
            map_id: r.encounter.map_id,
            started_at_unix: r.encounter.started_at_unix,
            log_start_ms: r.encounter.log_start_ms,
            self_idx,
            commander_idx,
            players,
            enemies,
            entity_index,
        }
    }
}
