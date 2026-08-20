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
use axilog_api::v1::envelope::Coverage;
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

    // -- Damage (blocks.damage.by_entity[id]) --
    pub damage: u64,
    /// Rounded from the native `f64` dps (matches `ei_model::DpsAll::dps`,
    /// which the EI adapter carries as a whole-number `u64` too).
    pub dps: u64,
    pub damage_taken: u64,
    pub breakbar_damage: u64,
    pub downs_dealt: u32,
    pub kills_dealt: u32,

    // -- Defenses (blocks.defenses.by_entity[id]) --
    pub deaths: u32,
    pub downs: u32,
    pub incoming_cc: u32,
    pub incoming_strips: u32,

    // -- CC (blocks.cc.by_entity[id]) --
    pub applied_cc: u32,

    // -- Support (blocks.support.by_entity[id]) --
    pub strips: u32,
    pub cleanses: u32,
    pub cleanses_self: u32,
    pub resurrects: u32,

    // -- Healing (blocks.healing.by_entity[id]) --
    pub healing_out: u64,
    pub barrier_out: u64,
    pub downed_healing_out: u64,

    // -- Contribution (blocks.contribution.by_entity[id]) --
    pub down_contribution: u64,
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
        // These six blocks are all "always-on" per `axilog_schema::v1::mod::build_report`
        // (see its "Always-on blocks" comment) -- none of them ride a
        // `ParseOpts` gate, so under this codebase's fixed `PARSE_OPTS`
        // (`tests/common/mod.rs::PARSE_OPTS`) they are computed
        // unconditionally. `require` still panics rather than defaulting
        // to zero if one is ever missing, because a missing block here
        // would mean the native report changed shape under us, not that
        // the fight had nothing to report -- that "nothing" case is
        // already expressed by the block being PRESENT but its
        // `by_entity` map lacking this entity's row (handled below by
        // defaulting each field, not the whole block).
        let damage = require(&r.blocks.damage, "damage", &r.coverage);
        let defenses = require(&r.blocks.defenses, "defenses", &r.coverage);
        let cc = require(&r.blocks.cc, "cc", &r.coverage);
        let support = require(&r.blocks.support, "support", &r.coverage);
        let healing = require(&r.blocks.healing, "healing", &r.coverage);
        let contribution = require(&r.blocks.contribution, "contribution", &r.coverage);

        let mut players = Vec::new();
        let mut enemies = Vec::new();
        let mut entity_index = HashMap::new();

        for e in &r.entities {
            match e.role {
                Role::Squad | Role::FriendlyPlayer => {
                    entity_index.insert(e.id, players.len());

                    // A player entity absent from a block's `by_entity`
                    // map is a MEASURED ZERO (the block itself is
                    // present -- see the `require` calls above) -- e.g. a
                    // squad member who never resurrected anyone has no
                    // `support` row at all, and that means 0 resurrects,
                    // not "unknown".
                    let dmg = damage.by_entity.get(e.id);
                    let def = defenses.by_entity.get(e.id);
                    let cc_row = cc.by_entity.get(e.id);
                    let sup = support.by_entity.get(e.id);
                    let heal = healing.by_entity.get(e.id);
                    let contrib = contribution.by_entity.get(e.id);

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

                        damage: dmg.map(|d| d.total).unwrap_or_default(),
                        // Native carries dps as `f64`; the EI oracle
                        // (`ei_model::DpsAll::dps`) is already `u64`, so
                        // this rounds rather than truncates to match it
                        // as closely as a whole number can.
                        dps: dmg.map(|d| d.dps.round() as u64).unwrap_or_default(),
                        damage_taken: dmg.map(|d| d.taken).unwrap_or_default(),
                        breakbar_damage: dmg.map(|d| d.breakbar_damage_dealt).unwrap_or_default(),
                        downs_dealt: dmg.map(|d| d.downs_dealt).unwrap_or_default(),
                        kills_dealt: dmg.map(|d| d.kills_dealt).unwrap_or_default(),

                        deaths: def.map(|d| d.deaths).unwrap_or_default(),
                        downs: def.map(|d| d.downs_taken).unwrap_or_default(),
                        incoming_cc: def.map(|d| d.received_cc_count).unwrap_or_default(),
                        incoming_strips: def.map(|d| d.boon_strips_taken).unwrap_or_default(),

                        applied_cc: cc_row.map(|c| c.applied_total).unwrap_or_default(),

                        strips: sup.map(|s| s.strips).unwrap_or_default(),
                        cleanses: sup.map(|s| s.cleanses).unwrap_or_default(),
                        cleanses_self: sup.map(|s| s.cleanses_self).unwrap_or_default(),
                        resurrects: sup.map(|s| s.resurrects).unwrap_or_default(),

                        healing_out: heal.map(|h| h.outgoing_allies).unwrap_or_default(),
                        barrier_out: heal.map(|h| h.barrier_out).unwrap_or_default(),
                        downed_healing_out: heal.map(|h| h.downed_healing_out).unwrap_or_default(),

                        down_contribution: contrib
                            .map(|c| c.downs_contribution.damage)
                            .unwrap_or_default(),
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

/// Fetches a block this projection needs, panicking with the block's own
/// coverage state when it is absent.
///
/// A missing block here is never "the fight had nothing" -- that case is
/// `Some(block)` with an empty `by_entity` map, which `Coverage::get`
/// reports as `Empty` and which callers already treat as a measured zero
/// per-entity. `None` means the block never ran at all (`NotComputed`,
/// because a `ParseOpts` gate it depends on was off) or cannot run on this
/// log (`Unsupported`) -- both are real absences this projection must not
/// paper over by rendering a zero or falling back to Elite Insights.
fn require<'a, T>(block: &'a Option<T>, name: &str, coverage: &Coverage) -> &'a T {
    block.as_ref().unwrap_or_else(|| {
        panic!(
            "axilog report is missing the \"{name}\" block (coverage: {:?})",
            coverage.get(name),
        )
    })
}
