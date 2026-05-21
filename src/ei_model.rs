//! Strongly-typed subset of EI's JSON output. Only the fields used by
//! Pulse and Timeline plans are deserialised; everything else is ignored.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EiJson {
    pub fight_name: String,
    #[serde(default)]
    pub zone: Option<String>,
    #[serde(default, alias = "mapName", alias = "map")]
    pub map_name: Option<String>,
    #[serde(rename = "durationMS")]
    pub duration_ms: u64,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub time_start_std: Option<String>,
    #[serde(default)]
    pub recorded_by: Option<String>,
    #[serde(default)]
    pub recorded_account_by: Option<String>,
    pub players: Vec<EiPlayer>,
    #[serde(default)]
    pub targets: Vec<EiTarget>,
    #[serde(default)]
    pub combat_replay_meta_data: Option<EiReplayMeta>,
    /// Skill ID (string-prefixed with `s`, e.g. `"s5535"`) → metadata.
    /// EI emits this at the top level; `totalDamageDist[].name` is blank
    /// in WvW logs so this is the authoritative source for display names.
    #[serde(default)]
    pub skill_map: HashMap<String, SkillMapEntry>,
    /// Buff ID (string-prefixed with `b`, e.g. `"b740"`) → metadata.
    #[serde(default)]
    pub buff_map: HashMap<String, BuffMapEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMapEntry {
    #[serde(default)]
    pub name: String,
    /// Absolute URL to a 64×64 PNG hosted on render.guildwars2.com.
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuffMapEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EiPlayer {
    pub name: String,
    pub account: String,
    pub profession: String,
    #[serde(default)]
    pub elite_spec: Option<String>,
    #[serde(default)]
    pub group: i64,
    #[serde(default)]
    pub has_commander_tag: bool,
    #[serde(default)]
    pub not_in_squad: bool,
    #[serde(default, alias = "teamID", alias = "teamId")]
    pub team_id: Option<i64>,

    #[serde(default)]
    pub dps_all: Vec<DpsAll>,
    #[serde(default)]
    pub stats_all: Vec<StatsAll>,
    #[serde(default)]
    pub defenses: Vec<Defenses>,
    #[serde(default)]
    pub support: Vec<Support>,

    /// Phases × seconds cumulative damage (2-D).
    #[serde(default, rename = "damage1S")]
    pub damage_1s: Vec<Vec<u64>>,
    /// Targets × phases × seconds cumulative damage (3-D).
    #[serde(default, rename = "targetDamage1S")]
    pub target_damage_1s: Vec<Vec<Vec<u64>>>,
    /// Phases × seconds cumulative damage taken (2-D).
    #[serde(default, rename = "damageTaken1S")]
    pub damage_taken_1s: Vec<Vec<u64>>,

    #[serde(default)]
    pub total_damage_dist: Vec<Vec<DamageDistEntry>>,

    #[serde(default)]
    pub buff_uptimes: Vec<BuffEntry>,

    /// `[[time_ms, hp_percent], …]` — inner arrays, not tuples.
    #[serde(default)]
    pub health_percents: Vec<Vec<f64>>,
    #[serde(default)]
    pub combat_replay_data: Option<ReplayData>,

    /// Populated when the arcdps healing addon is loaded. Absent for
    /// players whose client didn't have the addon running.
    #[serde(default)]
    pub ext_healing_stats: Option<ExtHealingStats>,
    #[serde(default)]
    pub ext_barrier_stats: Option<ExtBarrierStats>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtHealingStats {
    /// Outer index = recipient ally, inner = phase. `[0].healing` is
    /// the total this player healed that ally in phase 0.
    #[serde(default)]
    pub outgoing_healing_allies: Vec<Vec<OutgoingHealEntry>>,
    /// Phases × skill entries.
    #[serde(default)]
    pub total_healing_dist: Vec<Vec<HealDistEntry>>,
    /// Per-second cumulative incoming healing — `[phase][sec]`.
    #[serde(default, rename = "healingReceived1S")]
    pub healing_received_1s: Vec<Vec<u64>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingHealEntry {
    #[serde(default)]
    pub healing: u64,
    #[serde(default)]
    pub hps: u64,
    #[serde(default)]
    pub downed_healing: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealDistEntry {
    pub id: i64,
    #[serde(default)]
    pub total_healing: u64,
    #[serde(default)]
    pub total_downed_healing: u64,
    #[serde(default)]
    pub hits: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtBarrierStats {
    #[serde(default)]
    pub outgoing_barrier_allies: Vec<Vec<OutgoingBarrierEntry>>,
    #[serde(default)]
    pub total_barrier_dist: Vec<Vec<BarrierDistEntry>>,
    #[serde(default, rename = "barrierReceived1S")]
    pub barrier_received_1s: Vec<Vec<u64>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingBarrierEntry {
    #[serde(default)]
    pub barrier: u64,
    #[serde(default)]
    pub bps: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarrierDistEntry {
    pub id: i64,
    #[serde(default)]
    pub total_barrier: u64,
    #[serde(default)]
    pub hits: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DpsAll {
    pub damage: u64,
    pub dps: u64,
    /// EI emits this as a float (e.g. `15.2`) — breakbar damage is
    /// computed across fractional time slices, not whole hits.
    #[serde(default)]
    pub breakbar_damage: f64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsAll {
    #[serde(default)]
    pub down_contribution: u64,
    #[serde(default)]
    pub dist_to_com: f64,
    #[serde(default)]
    pub stack_dist: f64,
    #[serde(default)]
    pub applied_crowd_control: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Defenses {
    #[serde(default)]
    pub damage_taken: u64,
    #[serde(default)]
    pub dead_count: u32,
    #[serde(default)]
    pub down_count: u32,
    #[serde(default)]
    pub dodge_count: u32,
    #[serde(default)]
    pub blocked_count: u32,
    #[serde(default)]
    pub evaded_count: u32,
    #[serde(default)]
    pub missed_count: u32,
    #[serde(default)]
    pub invulned_count: u32,
    #[serde(default)]
    pub interrupted_count: u32,
    #[serde(default)]
    pub received_crowd_control: u64,
    #[serde(default)]
    pub boon_strips: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Support {
    #[serde(default)]
    pub condi_cleanse: u64,
    #[serde(default)]
    pub condi_cleanse_self: u64,
    #[serde(default)]
    pub boon_strips: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageDistEntry {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub total_damage: u64,
    #[serde(default)]
    pub down_contribution: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuffEntry {
    pub id: i64,
    #[serde(default)]
    pub buff_data: Vec<BuffData>,
    /// `[[time_ms, state], …]` — inner arrays, not tuples.
    #[serde(default)]
    pub states: Vec<Vec<f64>>,
}

// NOTE: field names differ from axipulse/src/shared/types.ts.
// Real EI v3.22 emits `generated`/`overstacked` as per-player objects (map of
// account name → f64); the TS types incorrectly named them `generation`/
// `overstack` and assumed plain f64 scalars. Match the real JSON using HashMap.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuffData {
    #[serde(default)]
    pub uptime: f64,
    /// Map of account name → generated seconds. EI v3.22 emits an object, not a scalar.
    #[serde(default)]
    pub generated: HashMap<String, f64>,
    /// Map of account name → overstacked seconds.
    #[serde(default)]
    pub overstacked: HashMap<String, f64>,
    /// Map of account name → wasted seconds.
    #[serde(default)]
    pub wasted: HashMap<String, f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EiTarget {
    pub name: String,
    #[serde(default)]
    pub enemy_player: bool,
    #[serde(default, alias = "teamID", alias = "teamId")]
    pub team_id: Option<i64>,
    #[serde(default)]
    pub profession: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EiReplayMeta {
    #[serde(default)]
    pub inch_to_pixel: Option<f64>,
    #[serde(default)]
    pub polling_rate: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayData {
    #[serde(default)]
    pub positions: Vec<Vec<f64>>,
    #[serde(default)]
    pub start: Option<i64>,
}
