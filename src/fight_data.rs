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

use axilog_api::v1::catalogs::Catalogs;
use axilog_api::v1::entities::Role;
use axilog_api::v1::envelope::{Coverage, CoverageState};
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
    /// Whether this LOG carries the arcdps healing addon extension, i.e.
    /// whether `PlayerData::healing_out`/`barrier_out`/`downed_healing_out`
    /// are real measurements rather than a zero standing in for "unknown".
    ///
    /// `false` exactly when `blocks.healing`'s own coverage state is
    /// `Unsupported` -- axilog reports `blocks.healing` as `Some(..)`
    /// unconditionally, even on a log recorded without the addon
    /// (`axilog_schema::v1::mod::build_report`, and its doc comment at
    /// `v1/mod.rs:224-235`: "a log with no healing extension reported
    /// `healing: present` with zero rows"), so every player's healing
    /// fields silently read 0 on such a log unless something upstream of
    /// the UI can tell the two cases apart. This is that signal --
    /// log-wide, not per-player, because the healing extension is a
    /// property of the LOG (whoever was running the addon that reports
    /// it), not of any one squad member. Wiring it into the UI (e.g.
    /// rendering "--" instead of "0") is migration Task 7's job; this
    /// task only populates the field.
    pub healing_available: bool,
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

    // -- Healing (blocks.healing.by_entity[id]) -- 0 when this player has
    // no row (a measured zero, per the block-vs-row distinction in
    // `require`'s doc comment) OR when `FightData::healing_available` is
    // `false` (the log has no healing addon data at all, so these three
    // fields are a placeholder zero, not a measurement -- callers must
    // check `healing_available` before trusting them).
    pub healing_out: u64,
    pub barrier_out: u64,
    pub downed_healing_out: u64,

    // -- Contribution (blocks.contribution.by_entity[id]) --
    pub down_contribution: u64,

    // -- Per-skill distributions --
    /// `blocks.damage.by_entity[id].by_skill`. Sparse -- only skills that
    /// dealt damage appear.
    pub damage_by_skill: Vec<SkillRow>,
    /// `blocks.contribution.by_entity[id].downs_contribution_by_skill`.
    /// Deliberately UNGATED relative to `damage_by_skill`: the contribution
    /// pass is always-on (never rides `--skill-damage`), so this reads
    /// straight off `contribution` without checking the skill-damage
    /// suboption the damage/healing rows below depend on. See
    /// `ContributionEntity::downs_contribution_by_skill`'s own doc comment
    /// for why it lives off `contribution` rather than joined onto
    /// `damage_by_skill`.
    pub down_contribution_by_skill: Vec<SkillRow>,
    /// `blocks.healing.by_entity[id].detail.by_skill`. Empty when
    /// `healing_available` is `false` (this log has no healing addon data)
    /// or when this player has no healing row at all.
    pub healing_by_skill: Vec<SkillRow>,
    /// `blocks.healing.by_entity[id].detail.barrier_by_skill`. Same
    /// availability caveats as `healing_by_skill`.
    pub barrier_by_skill: Vec<SkillRow>,
}

/// One skill's contribution to one of `PlayerData`'s four per-skill
/// distributions (damage/down-contribution/healing/barrier).
///
/// `name`/`icon` are resolved here, at build time, against
/// `ReportV1::catalogs` -- this format's rule that no human-readable name
/// appears outside `catalogs`/`entities` means every OTHER consumer of
/// this row would otherwise have to carry the catalog around just to
/// render it. `icon` stays `Option` (a small, already-diagnosed set of
/// skill ids has no art in axilog's catalog yet -- an upstream gap, not a
/// bug in this projection); `name` does not, because the catalog's own
/// invariant ("every id any row references resolves to an entry") means a
/// name is always resolvable for an id these rows actually reference.
///
/// `hits` and `downed` mean different things depending on which
/// distribution a row came from -- neither native source this struct
/// draws from carries a "hits and downed, always" pair:
/// - `damage_by_skill`: `hits` is the native `SkillRow::hits` contributing
///   count; `downed` has no native per-skill equivalent for damage, so it
///   is always `0`.
/// - `down_contribution_by_skill`: the native source is a bare
///   `BTreeMap<u32, u64>` (skill id -> credited damage), with no hit count
///   or downed-subset of its own; both `hits` and `downed` are `0`.
/// - `healing_by_skill` / `barrier_by_skill`: `hits` is the native
///   `HealSkillRow::hits` count and `downed` is `HealSkillRow::total_downed`
///   (healing/barrier landed while the target was downed). Per
///   `HealSkillRow::total_downed`'s own doc comment this is always `0` on
///   a barrier row -- GW2EI's barrier distribution has no downed field to
///   measure it from at all -- so `barrier_by_skill` rows carry `downed:
///   0` structurally, not because this projection dropped anything.
#[derive(Debug, Clone)]
pub struct SkillRow {
    pub skill_id: u32,
    pub name: String,
    pub icon: Option<String>,
    pub total: u64,
    pub hits: u32,
    pub downed: u64,
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
        // (`tests/common/mod.rs::PARSE_OPTS`) none of them can read
        // `not_computed`. `require` panics if one ever does (see its own
        // doc comment) -- that would mean the native report's shape
        // changed under us, not that the fight had nothing to report.
        // "Nothing to report" is `Empty` (an always-on block computed and
        // found nothing) or, for `healing` alone, `Unsupported` (this LOG
        // has no addon data) -- neither is fatal; see `require` and
        // `healing_available` below for how those two are told apart from
        // a real absence.
        let damage = require(&r.blocks.damage, "damage", &r.coverage);
        let defenses = require(&r.blocks.defenses, "defenses", &r.coverage);
        let cc = require(&r.blocks.cc, "cc", &r.coverage);
        let support = require(&r.blocks.support, "support", &r.coverage);
        let healing = require(&r.blocks.healing, "healing", &r.coverage);
        let contribution = require(&r.blocks.contribution, "contribution", &r.coverage);
        // Log-wide, not per-entity -- see `FightData::healing_available`'s
        // doc comment. `NotComputed` is already fatal via the `require`
        // call above (this crate's `PARSE_OPTS` never leaves a gate
        // `healing` depends on off), so the only two states left here are
        // `Unsupported` (no addon on this log -- not available) and
        // `Present`/`Empty` (addon ran -- available, whether or not it
        // measured anything).
        let healing_available =
            !matches!(r.coverage.get("healing"), Some(CoverageState::Unsupported));

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

                        // `by_skill`/`detail` are `Option` because they
                        // ride the crate's OWN `skill_damage` ParseOpts
                        // suboption, not `r.coverage` -- there is no
                        // `BlockName` for it to gate on, so `require`'s
                        // coverage-map check does not apply here. Under
                        // this crate's fixed `PARSE_OPTS`
                        // (`tests/common/mod.rs`, `skill_damage: true`)
                        // both are always `Some`; `.unwrap_or_default()`
                        // degrades to an empty `Vec` rather than panicking
                        // if that ever changes, since an empty per-skill
                        // breakdown is a defensible fallback (unlike a
                        // silently-zeroed scalar) and this projection has
                        // no coverage signal to check instead.
                        damage_by_skill: dmg
                            .and_then(|d| d.by_skill.as_ref())
                            .map(|by_skill| {
                                skill_rows(
                                    by_skill.iter().map(|(id, row)| {
                                        (*id, row.total, row.hits.unwrap_or_default(), 0)
                                    }),
                                    &r.catalogs,
                                )
                            })
                            .unwrap_or_default(),

                        down_contribution_by_skill: contrib
                            .map(|c| {
                                skill_rows(
                                    c.downs_contribution_by_skill
                                        .iter()
                                        .map(|(id, total)| (*id, *total, 0, 0)),
                                    &r.catalogs,
                                )
                            })
                            .unwrap_or_default(),

                        healing_by_skill: heal
                            .and_then(|h| h.detail.as_ref())
                            .map(|d| {
                                skill_rows(
                                    d.by_skill.iter().map(|(id, row)| {
                                        (*id, row.total, row.hits, row.total_downed)
                                    }),
                                    &r.catalogs,
                                )
                            })
                            .unwrap_or_default(),

                        barrier_by_skill: heal
                            .and_then(|h| h.detail.as_ref())
                            .map(|d| {
                                skill_rows(
                                    d.barrier_by_skill.iter().map(|(id, row)| {
                                        (*id, row.total, row.hits, row.total_downed)
                                    }),
                                    &r.catalogs,
                                )
                            })
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
            healing_available,
        }
    }
}

/// Joins a `(skill_id, total, hits, downed)` iterator against
/// `catalogs.skills` to produce `SkillRow`s. Shared by all four of
/// `PlayerData`'s per-skill distributions -- see `SkillRow`'s own doc
/// comment for what `hits`/`downed` mean for each caller.
fn skill_rows(
    rows: impl Iterator<Item = (u32, u64, u32, u64)>,
    catalogs: &Catalogs,
) -> Vec<SkillRow> {
    rows.map(|(skill_id, total, hits, downed)| {
        let entry = catalogs.skills.get(&skill_id);
        SkillRow {
            skill_id,
            name: entry.map(|e| e.name.clone()).unwrap_or_default(),
            icon: entry.and_then(|e| e.icon.clone()),
            total,
            hits,
            downed,
        }
    })
    .collect()
}

/// Fetches a block this projection needs, panicking when its OWN
/// coverage entry says `NotComputed`.
///
/// Gated on the coverage MAP ENTRY, not on `block.is_none()`. The two are
/// not interchangeable: axilog's `blocks.healing` is `Some(..)`
/// unconditionally (`axilog_schema::v1::mod::build_report`, `v1/mod.rs`
/// around line 669), even when its coverage is `Unsupported` -- a log
/// recorded without the arcdps healing addon, which is the common case.
/// The block's own doc comment (`v1/mod.rs:224-235`) says as much: "a log
/// with no healing extension reported `healing: present` with zero
/// rows". A version of this function that only checked
/// `Option::is_none()` would never fire for `healing` at all, and every
/// player's healing fields would silently resolve to a plain `0` on an
/// addon-less log -- exactly the "coverage says absent but we rendered a
/// zero anyway" outcome this project forbids. Reading `coverage.get(name)`
/// first is what lets `NotComputed` (fatal) and `Unsupported` (not
/// fatal, see below) be told apart even when both pair with a `None`
/// block on some hypothetical future block, and even though today only
/// `healing` can be `Unsupported` at all.
///
/// Only `NotComputed` (and a name absent from `coverage` entirely, which
/// should not be possible given `Coverage::new`'s `BlockName::ALL` seed)
/// panics. Every block this projection reads is either always-on under
/// this crate's fixed `PARSE_OPTS` (damage, defenses, cc, support,
/// contribution -- see `axilog_schema::v1::mod::build_report`'s
/// "Always-on blocks" comment) or, for `healing` alone, gated on a
/// property of the LOG rather than of `ParseOpts` (whether the arcdps
/// healing addon ran). Neither case can produce `NotComputed` under this
/// crate's parse options, so seeing it here means a real regression: a
/// `ParseOpts` gate this projection depends on got left off. `Empty`
/// (block ran, found nothing) and `Unsupported` (this log cannot produce
/// the block at all) are NOT fatal -- both still carry a real `Some`
/// block whose `by_entity` map a caller reads as a measured zero per
/// row; `Unsupported` additionally needs a way to tell "zero" from
/// "unknown" apart, which is what `FightData::healing_available` is for
/// (derived the same way, straight from `coverage.get("healing")`,
/// rather than folded into this function's boolean return).
fn require<'a, T>(block: &'a Option<T>, name: &str, coverage: &Coverage) -> &'a T {
    let state = coverage.get(name);
    assert!(
        !matches!(state, Some(CoverageState::NotComputed) | None),
        "axilog report's \"{name}\" block is not_computed under this crate's PARSE_OPTS -- \
         a parse gate this projection depends on must have been left off (coverage: {state:?})",
    );
    block.as_ref().unwrap_or_else(|| {
        panic!(
            "axilog report's \"{name}\" block is absent despite coverage {state:?} -- \
             the native report's shape no longer matches this projection's assumptions",
        )
    })
}
