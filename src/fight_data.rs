//! The purpose-built projection of axilog's native `ReportV1` that the UI
//! reads instead of the Elite Insights JSON tree it replaced.
//!
//! `FightData::from_report` must not retain the `&ReportV1` it is handed
//! -- it reads it once and returns owned data, per the migration's
//! binding constraint that the native report is never stored past the
//! call that produced it.

use axilog_api::v1::catalogs::Catalogs;
use axilog_api::v1::entities::Role;
use axilog_api::v1::envelope::{Coverage, CoverageState};
use axilog_api::v1::series::SeriesOut;
use axilog_api::v1::ReportV1;
use std::collections::HashMap;

/// Upper bound on a fight's reported duration, in milliseconds.
///
/// Native computes `encounter.duration_ms` as `last event time -
/// log_start_ms` and never clamps it, so a single corrupt trailing
/// timestamp yields an arbitrarily large `u64`. Downstream, both
/// `timeline_health::sample_health_per_second` and
/// `timeline_distance::distance_to_commander_per_second` size a
/// per-second `Vec` from it (`duration_ms / 1000 + 1`), which scales
/// linearly and unboundedly: a 10^12 ms duration asks for ~10^9 entries
/// per lane. An allocation that large fails, and an allocation failure
/// ABORTS rather than unwinds -- it takes the game process down, where
/// the parse path's `catch_unwind` cannot help.
///
/// Six hours. The bound is deliberately far above anything real (a WvW
/// log is minutes; arcdps rotates logs long before this) so it can only
/// ever fire on a corrupt timestamp, while still capping every
/// downstream per-second allocation at ~21,601 entries.
///
/// It CLAMPS rather than rejects: every scalar in the report is still a
/// real measurement, only the timeline's axis is truncated, and a
/// truncated timeline beats discarding the log.
pub const MAX_DURATION_MS: u64 = 6 * 60 * 60 * 1_000;

/// The single place [`MAX_DURATION_MS`] is applied. Every consumer sizes
/// its per-second buffers from `FightData::duration_ms`, so bounding it
/// here bounds all of them.
pub fn clamp_duration_ms(duration_ms: u64) -> u64 {
    duration_ms.min(MAX_DURATION_MS)
}

/// Upper bound on a decoded [`SeriesOut`]'s length, derived from
/// [`MAX_DURATION_MS`] rather than picked independently.
///
/// A series is axilog's per-second curve over the fight, so its longest
/// legitimate length follows directly from the same six-hour ceiling:
/// one bucket per second of `MAX_DURATION_MS`, plus one. That `+ 1` is
/// not slack for corruption -- axilog's per-second grid is a CEILING
/// grid (`axilog_schema::v1::series::SeriesOut`'s own contract), so a
/// fight that runs any part of a trailing second still gets a full
/// bucket for it. Without the `+ 1` a legitimate maximum-length series
/// would trip this bound, which is the failure this constant exists to
/// avoid causing.
///
/// [`decode_series`] is the only place this is checked, and it checks it
/// BEFORE allocating: an unclamped `len` (or, worse, an unclamped RLE
/// `run` that grows a `Vec` past this regardless of its initial
/// capacity) risks the same hazard `MAX_DURATION_MS` documents -- an
/// allocation failure ABORTS rather than unwinds, taking the game
/// process down where `parse_log`'s `catch_unwind` cannot help.
pub const MAX_SERIES_LEN: u64 = MAX_DURATION_MS / 1000 + 1;

/// Everything the UI needs about one fight, read once from a native
/// `ReportV1`.
///
/// `Default` is derived so tests can build a minimal fight with
/// functional-update syntax. A defaulted `FightData` is EMPTY, not a
/// fallback: nothing in production constructs one this way, and
/// `from_report` sets every field explicitly.
#[derive(Debug, Clone, Default)]
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
    /// `EntityOut::id` -> index into `players`. Built once here so
    /// anything that joins a row keyed by entity id back onto a roster
    /// row doesn't have to re-derive it.
    ///
    /// Public, like every other field, so a caller (in practice a test)
    /// can construct a `FightData` with functional-update syntax. It
    /// guards no invariant `from_report` would be free to break: it is
    /// derived from `players`, and the only thing that reads it,
    /// `self_idx`'s resolution, has already run by the time this struct
    /// exists.
    pub entity_index: HashMap<u32, usize>,
    /// The WvW map's fixed world rect and arena image --
    /// `blocks.replay.tracks.arena`. `None` for a map axilog has no
    /// hand-authored arena image for (`ArenaOut::for_map_id`'s own doc
    /// comment), OR when `--replay` did not run at all (`tracks` itself is
    /// `None` then) -- this projection does not distinguish the two,
    /// because neither leaves anything to project `PlayerData::positions`
    /// onto. `ui/map.rs`'s job (migration Task 7) is to project a raw
    /// world position onto this rect's image pixel space:
    /// `px = (x - world_min_x) / (world_max_x - world_min_x) * image_width`,
    /// `py = (1 - (y - world_min_y) / (world_max_y - world_min_y)) *
    /// image_height` -- world y grows northward, image y grows downward,
    /// hence the flip.
    pub arena: Option<Arena>,
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
    /// `blocks.replay.tracks.poll_ms` -- the ONE sample interval shared by
    /// every [`PlayerData::positions`] and [`EnemyData::positions`] track.
    /// `0` when `--replay` did not run (there are no tracks to describe).
    ///
    /// Carried because [`PlayerData::positions`] drops each sample's own
    /// timestamp: with `poll_ms` and the track's own
    /// [`PlayerData::track_start_ms`], `positions[i]` is the position at
    /// `track_start_ms + i * poll_ms` EXACTLY -- axilog's downsampler
    /// (`axilog_core::analysis::replay::downsample`) emits one sample per
    /// grid tick from `first_aware.div_ceil(poll_ms) * poll_ms` to the
    /// last observed position with no gaps, so the two scalars are a
    /// lossless stand-in for the dropped timestamps (verified over all 93
    /// tracks in this crate's fixture: every sample satisfies
    /// `t == samples[0].t + i * poll_ms`).
    ///
    /// **This pair is the only correct way to turn a track index into a
    /// time.** Tracks do NOT share a start: this crate's fixture has
    /// starts spread from 0ms to 100800ms and lengths from 105 to 462
    /// samples, so `positions[i]` for two different players is generally
    /// two different instants. Anything that compares two players'
    /// samples must match them by TIME, never by index.
    pub poll_ms: u64,
    /// `catalogs.skills[id].icon` for the ids that have art, copied out so
    /// the UI can resolve an icon for any skill id (including a
    /// [`CastRow::skill_id`], which carries no row of its own) without the
    /// `ReportV1` -- which this projection must not retain.
    pub skill_icons: HashMap<u32, String>,
    /// `catalogs.buffs[id].icon`, same rationale. Separate from
    /// `skill_icons` because a buff id and a skill id share a namespace
    /// only by accident: a boon nobody's damage came from has a buff entry
    /// and no skill entry at all (see `BuffEntry::icon`'s own doc comment).
    pub buff_icons: HashMap<u32, String>,
}

/// `Default` is derived for the same test-construction reason
/// [`FightData`]'s is; `from_report` never leans on it.
#[derive(Debug, Clone, Default)]
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
    /// Rounded from the native `f64` dps to a whole number for display.
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
    // The six mitigation counters the Pulse "Defense" subview shows.
    // All measured zeros when the player has no defenses row, per the
    // block-vs-row distinction in `require`'s doc comment.
    pub blocked: u32,
    pub evaded: u32,
    pub dodges: u32,
    pub missed: u32,
    pub interrupted: u32,
    pub invulned: u32,

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

    // -- Boons (blocks.boons.by_entity[id]) --
    /// One row per buff this player ever held, in the native map's key
    /// order (ascending buff id). Empty when this player has no boon row
    /// at all -- a measured "held nothing", per the block-vs-row
    /// distinction `require`'s doc comment draws (`blocks.boons` itself
    /// is always-on, see `require`'s call site below).
    ///
    /// `name`/`stacking` are resolved here, at build time, against
    /// `ReportV1::catalogs.buffs` -- the same reason `SkillRow` resolves
    /// its own name/icon here rather than making every later consumer
    /// (boon_uptime.rs/timeline_boons.rs, Task 7) carry the catalog
    /// around just to render a row.
    pub boons: Vec<BoonRow>,

    // -- Per-second series (blocks.series.by_entity[id]) --
    /// Cumulative outgoing damage per second, on the CEILING grid
    /// (`timeseries::ei_grid` -- one bucket longer than
    /// `timeline.resolution_ms`'s floor grid on a partial-second log).
    /// `blocks.series.by_entity[id].damage`, decoded with
    /// [`decode_series`]. Empty when this player has no series row.
    pub damage_1s: Vec<u64>,
    /// Cumulative incoming damage per second, same grid.
    /// `blocks.series.by_entity[id].damage_taken`.
    pub damage_taken_1s: Vec<u64>,
    /// Cumulative INCOMING healing per second from the arcdps healing
    /// addon, ally-attributed. `blocks.series.by_entity[id]
    /// .healing_received_1s` -- `Option<SeriesOut>`, gated the same way
    /// `healing_out` is (see [`FightData::healing_available`]): empty
    /// when this log has no healing extension data, OR when this player
    /// has no series row at all. New in axilog 1.1.0.
    pub healing_received_1s: Vec<u64>,
    /// Cumulative INCOMING barrier per second, same grid and gate as
    /// `healing_received_1s`. New in axilog 1.1.0.
    pub barrier_received_1s: Vec<u64>,
    /// `(time_ms_from_log_start, health_percent)` step-function pairs --
    /// NOT a [`SeriesOut`] (see `EntitySeries::health_percents`'s own doc
    /// comment on the native side for why: it is keyed off
    /// `HEALTH_UPDATE` events at their own timestamps, not resampled onto
    /// a fixed grid). Empty when the pass never saw a `HEALTH_UPDATE` for
    /// this player, which native and this projection both treat as
    /// distinct from "saw one, no transitions" (`Some(vec![])`, which
    /// this projection cannot currently distinguish from "absent" either
    /// -- both collapse to an empty `Vec` here, since `PlayerData` has no
    /// spare `Option` to carry the difference through and no consumer
    /// this task knows of needs it).
    pub health_percents: Vec<(u64, f64)>,

    // -- Replay (blocks.replay) --
    /// `blocks.replay.tracks.by_entity[id].samples`, per-sample timestamp
    /// dropped (recoverable from [`PlayerData::track_start_ms`] and
    /// [`FightData::poll_ms`] -- see those two) and
    /// `(x, y)` narrowed to `f32` -- RAW WORLD INCHES, not pixels or any
    /// other projected unit. Converting to a map pixel is the UI's job
    /// (see [`FightData::arena`]'s doc comment for the formula), not
    /// this projection's. Empty when `--replay` did not run (`tracks` is
    /// `None`) or when this player has no track row at all -- the same
    /// "measured absence" convention every other by-entity lookup in this
    /// module follows; there is no coverage signal narrower than
    /// `blocks.replay.tracks.is_some()` to distinguish those two cases
    /// from each other, and neither leaves any samples to report.
    ///
    /// **Does NOT have a uniform length across players, or a uniform
    /// start.** A track starts
    /// at this player's own first-aware time rounded up to the shared
    /// polling grid (`blocks.replay.tracks.poll_ms`) and ends at their
    /// last-aware time -- both genuinely per-player, not a shared window.
    /// Measured against this crate's fixture (47 squad members,
    /// `poll_ms == 300`): 39/47 share the modal length (461 samples,
    /// i.e. present for the whole ~138s encounter), 7 more sit 1-9
    /// samples short (453-460 -- late join / early leave / a gap in
    /// position telemetry), and one sits far short at 254 (a player who
    /// was only tracked for roughly the back half of the fight). "First/
    /// last aware" is an event-presence fact, not a fixed start/end pair,
    /// so this spread is expected, not a bug. What IS uniform is the
    /// GRID: every sample's own timestamp (not carried here, see above)
    /// is an exact multiple of `poll_ms` -- verified against this fixture
    /// for every returned track.
    ///
    /// The starts differ too, and by much more than the lengths: measured
    /// over all 93 of this fixture's tracks they range from 0ms to
    /// 100800ms. **`positions[i]` for two different players is therefore
    /// two different instants.** Anything comparing two tracks must
    /// match by time (`track_start_ms + i * poll_ms`), never by index.
    pub positions: Vec<(f32, f32)>,
    /// The timestamp (log-relative ms) of `positions[0]`, i.e. this
    /// player's own first-aware time rounded UP to the shared
    /// [`FightData::poll_ms`] grid. `0` when `positions` is empty.
    ///
    /// This is the anchor `positions`'s dropped timestamps collapse to --
    /// see [`FightData::poll_ms`] for why the pair is lossless and why
    /// index-alignment across two players is always wrong.
    pub track_start_ms: u64,
    /// `blocks.replay.by_entity[id].down` -- half-open
    /// `[start_ms, end_ms)` down-state windows, log-relative ms. Always
    /// on (Task 11's always-on half of the replay block), unlike
    /// `positions` above.
    pub down_ranges: Vec<(u64, u64)>,
    /// `blocks.replay.by_entity[id].dead`, same shape/gate as
    /// `down_ranges`.
    pub dead_ranges: Vec<(u64, u64)>,
    /// `blocks.replay.by_entity[id].dc` -- disconnect/not-yet-spawned
    /// windows. Same shape/gate as `down_ranges`; not mutually exclusive
    /// with it or `dead_ranges` (an agent can despawn while dead).
    pub dc_ranges: Vec<(u64, u64)>,
    /// `blocks.replay.by_entity[id].active_ms` -- `(end_ms - start_ms) -
    /// dead_ms`. NOT `- down_ms` too, despite the tempting reading of
    /// "active" -- down time counts as active, only dead time does not.
    /// 0 when this player has no replay row at all.
    pub active_ms: u64,
    /// `blocks.replay.by_entity[id].dist_to_com`, EI's `distToCom` --
    /// mean distance to the commander over this player's active polls, in
    /// world inches.
    ///
    /// **Tri-state on the native side, collapsed to two here on purpose.**
    /// Native's `Option<f64>` is `None` when `--replay` never ran (nothing
    /// measured) and `Some(-1.0)` when it ran but this actor had no poll
    /// that paired with a commander reference (EI's own sentinel, see
    /// `axilog_core::analysis::distance::NO_DISTANCE`) -- both distinct
    /// from `Some(x >= 0.0)`, a real measured distance. This field folds
    /// the first two into one `None`: nothing downstream of this
    /// projection needs to tell "we never looked" from "we looked and
    /// this player was never near a commander" apart, and a consumer that
    /// mapped absence to `-1.0` (or vice versa) would risk rendering the
    /// sentinel as a real distance, which is exactly the bug this
    /// projection exists to prevent. **A `-1.0` must never reach this
    /// field as `Some`.**
    pub dist_to_com: Option<f32>,
    /// `blocks.rotation.by_entity[id].casts`, already flat and sorted by
    /// `(cast_time_ms, skill_id)` on the native side
    /// (`axilog_schema::v1::blocks::activity::build_rotation`) -- this
    /// projection does not re-sort. Empty when `--rotation` did not run
    /// (the native field is `None`, the gate `RotationEntity::casts`'s own
    /// doc comment describes) or when this player cast nothing; per that
    /// same doc comment, `coverage.rotation` cannot distinguish the two
    /// cases, so neither can this field -- both collapse to an empty
    /// `Vec`, the same convention `damage_by_skill`/etc. above use for
    /// their own suboption gates.
    pub casts: Vec<CastRow>,
}

impl PlayerData {
    /// The class label to show for this player: the elite spec when the
    /// log named one, else the core profession. [`Self::elite_spec`] is
    /// empty both for a core-only build and for a spec axilog cannot yet
    /// name, and the core profession is the honest fallback for either.
    /// Matches the bundled class-icon keys (see `ui::icons`).
    pub fn spec_label(&self) -> &str {
        if self.elite_spec.is_empty() { &self.profession } else { &self.elite_spec }
    }
}

/// One cast, `blocks.rotation.by_entity[id].casts[]`. Mirrors the native
/// `CastRow` field-for-field; carried as its own local type (rather than
/// re-exporting the native struct) for the same reason every other row
/// type in this module is local -- this projection's public shape should
/// not change just because axilog's internal representation does.
///
/// No skill name/icon resolved here, unlike [`SkillRow`]: the brief this
/// struct was built against does not ask for one, and a rotation view can
/// already join `skill_id` against a skill catalog if migration Task 7
/// gives it one.
#[derive(Debug, Clone, Default)]
pub struct CastRow {
    pub skill_id: u32,
    pub cast_time_ms: i64,
    pub duration_ms: i64,
    pub time_gained_ms: i64,
    pub quickness: f64,
}

/// The WvW map's fixed world rectangle and arena image --
/// `blocks.replay.tracks.arena`. Mirrors native `ArenaOut` field-for-field
/// (`image_url` widened from `&'static str` to `String` since this
/// projection is an owned copy, not a borrow of the native report).
///
/// See [`FightData::arena`]'s doc comment for the pixel-projection
/// formula every `(x, y)` in [`PlayerData::positions`] needs run through
/// this rect before it is plottable.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Arena {
    pub image_width: u32,
    pub image_height: u32,
    pub image_url: String,
    pub world_min_x: f64,
    pub world_min_y: f64,
    pub world_max_x: f64,
    pub world_max_y: f64,
}

impl Arena {
    /// Projects ONE raw world position onto a `canvas_w` x `canvas_h`
    /// rectangle, per the formula in this type's doc comment. World y
    /// grows northward and canvas y grows downward, hence the flip.
    ///
    /// Deliberately per-position and stateless: a position is plottable
    /// on its own, so nothing here needs a second player's track, a
    /// shared sample index, or a polling grid. That is the whole point --
    /// [`PlayerData::positions`] tracks are ragged, and any projection
    /// that needed two of them lined up would be wrong.
    ///
    /// Degenerate (zero-width or zero-height) world rects project to the
    /// canvas origin rather than a NaN; no such rect exists in axilog's
    /// map table, but a NaN would silently poison a draw call.
    pub fn project(&self, x: f32, y: f32, canvas_w: f32, canvas_h: f32) -> (f32, f32) {
        let span_x = (self.world_max_x - self.world_min_x) as f32;
        let span_y = (self.world_max_y - self.world_min_y) as f32;
        if span_x == 0.0 || span_y == 0.0 {
            return (0.0, 0.0);
        }
        let fx = (x - self.world_min_x as f32) / span_x;
        let fy = (y - self.world_min_y as f32) / span_y;
        (fx * canvas_w, (1.0 - fy) * canvas_h)
    }
}

/// One buff row on [`PlayerData::boons`].
///
/// Mirrors `axilog_api::v1::blocks::support::BoonRow` field-for-field,
/// minus `id` (hoisted to `buff_id`, since native keys it as the map key
/// rather than carrying it on the row) and plus `name`/`stacking`,
/// resolved from `catalogs.buffs` at projection time for the same reason
/// [`SkillRow`] resolves its own name here -- see [`PlayerData::boons`]'s
/// doc comment.
///
/// `stacking` is `"intensity"` or `"duration"`, straight from
/// `BuffEntry::stacking` -- read `avg_stacks` for an intensity buff and
/// `uptime_pct` for a duration buff, per the brief; this projection does
/// not infer stacking from the buff id.
#[derive(Debug, Clone, Default)]
pub struct BoonRow {
    pub buff_id: u32,
    pub name: String,
    pub stacking: String,
    pub uptime_pct: f64,
    /// `None` for a duration buff, or an intensity buff this player never
    /// held long enough to average -- `BoonRow::avg_stacks` on the native
    /// side is `Option` for exactly this reason.
    pub avg_stacks: Option<f64>,
    pub gen_self: f64,
    pub gen_group: f64,
    pub gen_squad: f64,
    /// This buff's fused stack timeline, `(time_ms_from_log_start,
    /// stacks)`. Native carries stacks as `u32`
    /// (`axilog_api::v1::blocks::StateTimeline = Vec<(u64, u32)>`); this
    /// projection widens to `i32` per this task's own interface contract,
    /// which is otherwise a lossless cast since a stack count is never
    /// negative. Empty when `--timeseries` did not run (not the case
    /// under this crate's fixed `PARSE_OPTS`) or when this buff was never
    /// held.
    pub states: Vec<(u64, i32)>,
}

/// Decodes a native `SeriesOut` into its full-length value array.
///
/// `enc: "raw"` is a plain per-bucket array; `enc: "rle"` is
/// `[value, run_length]` pairs. `len` is documented as the DECODED
/// length, which need not equal `data.len()` in either encoding -- e.g. a
/// `raw` series a future encoder change truncated, or a hand-built
/// `rle` series whose run lengths do not sum to what `len` claims. This
/// function does not guess which of `len`/`data` is authoritative in that
/// case: `len` is the format's documented contract
/// (`axilog_schema::v1::series::SeriesOut`'s own doc comment: "`len` is
/// the DECODED length in both cases, so a consumer can allocate before
/// decoding and validate after"), so decoding and checking the result
/// against `len` is that validation, not an extra opinion this function
/// adds. A silent truncation or pad would let a caller plot a series
/// against the wrong number of buckets with no signal that anything was
/// wrong; panicking is the same "wrong shape is a bug, not data" stance
/// `require` takes for a missing block.
///
/// Before any of that, this function checks the declared `len` and, as
/// it accumulates, every RLE run against [`MAX_SERIES_LEN`] -- both are
/// log-controlled `u64`s with no upper bound of their own, and unlike the
/// `len`/`data` disagreement above, an oversized one is a hazard at
/// ALLOCATION time, not after: an allocation failure aborts the process
/// rather than unwinding, so `parse_log`'s `catch_unwind` cannot turn it
/// into a `ParseError` the way it can a panic. These checks panic (same
/// stance, same reason) but do so before the allocation they are
/// guarding against, not after.
pub fn decode_series(s: &SeriesOut) -> Vec<u64> {
    assert!(
        s.len <= MAX_SERIES_LEN,
        "SeriesOut declares len {} but a legitimate series cannot exceed \
         {} buckets (MAX_DURATION_MS's six hours, in whole seconds, plus \
         the ceiling grid's trailing bucket) -- refusing to allocate for \
         a corrupt `len` (enc: {:?})",
        s.len,
        MAX_SERIES_LEN,
        s.enc,
    );
    let out: Vec<u64> = match s.enc {
        "rle" => {
            let mut out = Vec::with_capacity(s.len as usize);
            for pair in &s.data {
                let value = pair[0].as_u64().unwrap_or_default();
                let run = pair[1].as_u64().unwrap_or_default();
                assert!(
                    out.len() as u64 + run <= MAX_SERIES_LEN,
                    "SeriesOut RLE run of {} at value {} would grow the \
                     decoded series past {} buckets ({} already decoded) \
                     -- MAX_SERIES_LEN bounds this regardless of the \
                     declared `len`, so a corrupt run cannot run away the \
                     allocation",
                    run,
                    value,
                    MAX_SERIES_LEN,
                    out.len(),
                );
                out.extend(std::iter::repeat(value).take(run as usize));
            }
            out
        }
        _ => s
            .data
            .iter()
            .map(|v| v.as_u64().unwrap_or_default())
            .collect(),
    };
    assert_eq!(
        out.len() as u64,
        s.len,
        "SeriesOut decoded to {} values but its own `len` says expected {} \
         (enc: {:?}) -- the encoding and the declared length disagree",
        out.len(),
        s.len,
        s.enc,
    );
    out
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
#[derive(Debug, Clone, Default)]
pub struct SkillRow {
    pub skill_id: u32,
    pub name: String,
    pub icon: Option<String>,
    pub total: u64,
    pub hits: u32,
    pub downed: u64,
}

#[derive(Debug, Clone, Default)]
pub struct EnemyData {
    pub entity_id: u32,
    pub name: String,
    /// `"red"` / `"green"` / `"blue"` / `"unknown"` -- axilog resolves the
    /// WvW team id to a colour itself (`axilog_core::wvw::team_color`),
    /// which is why this crate no longer carries a team-id table.
    pub team: String,
    pub profession: String,
    /// Same split, and the same empty-string ambiguity, as
    /// [`PlayerData::elite_spec`]. Read [`Self::spec_label`], not this
    /// field, when you want the class name to render.
    pub elite_spec: String,
    /// `blocks.replay.tracks.by_entity[id].samples`, same shape, grid and
    /// caveats as [`PlayerData::positions`] -- the track roster is WIDER
    /// than the always-on interval roster and deliberately includes enemy
    /// players (see `ReplayTrack::down_intervals`'s own doc comment).
    pub positions: Vec<(f32, f32)>,
    /// Anchor for `positions`, exactly as [`PlayerData::track_start_ms`].
    pub track_start_ms: u64,
    /// `blocks.replay.tracks.by_entity[id].down_intervals` /
    /// `dead_intervals`. Read off the TRACK rather than
    /// `blocks.replay.by_entity` because that always-on half covers squad
    /// players only -- an enemy has no row there at all.
    pub down_ranges: Vec<(u64, u64)>,
    pub dead_ranges: Vec<(u64, u64)>,
}

impl EnemyData {
    /// [`PlayerData::spec_label`] for an enemy, with one extra fallback:
    /// when the report names neither profession nor spec, the display
    /// name is shaped `"<Spec> pl-1992"`, so its first token is the
    /// best label available.
    pub fn spec_label(&self) -> &str {
        if !self.elite_spec.is_empty() {
            &self.elite_spec
        } else if !self.profession.is_empty() {
            &self.profession
        } else {
            self.name.split(" pl-").next().unwrap_or("")
        }
    }
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
        // `blocks.boons` is a two-gate block (see `BoonRow`'s doc comment
        // on the native side): `coverage.boons` answers for the UPTIME
        // half, which is always-on, same as the six blocks above. The
        // `states`/`per_source` half rides `--timeseries` alone, with no
        // `BlockName` of its own to gate on -- under this crate's fixed
        // `PARSE_OPTS` (`timeseries: true`) that half is always populated
        // too, so `require` on the block as a whole is the right check.
        let boons = require(&r.blocks.boons, "boons", &r.coverage);
        // `blocks.series`'s squad rollup is always-on, so its own
        // coverage entry is a real always-on signal too -- the
        // `healing_received_1s`/`barrier_received_1s`/`health_percents`
        // fields on each row are the ones that ride `--timeseries` and/or
        // the healing addon, which their own `Option`s encode per-row
        // rather than a second coverage entry.
        let series = require(&r.blocks.series, "series", &r.coverage);
        // `replay`'s intervals half (`by_entity`) is always-on (Task 11),
        // so under this crate's fixed `PARSE_OPTS` its coverage can only
        // ever be `Present`/`Empty`, same as the six blocks above --
        // `require` is the right check even though the OTHER half
        // (`tracks`, gated on `--replay`) is a per-field `Option` this
        // function branches on separately below, not a second coverage
        // entry. See `ReplayBlock`'s own doc comment for the two-gate
        // split.
        let replay = require(&r.blocks.replay, "replay", &r.coverage);
        // Same story for `rotation`: the block itself is built
        // unconditionally (`aftercast` is always-on), so `coverage.rotation`
        // can only read `Present`/`Empty` here -- the `--rotation` gate is
        // `RotationEntity::casts`'s own `Option`, read per-row below.
        let rotation = require(&r.blocks.rotation, "rotation", &r.coverage);
        let arena = replay.tracks.as_ref().and_then(|t| t.arena).map(|a| Arena {
            image_width: a.image_width,
            image_height: a.image_height,
            image_url: a.image_url.to_string(),
            world_min_x: a.world_min_x,
            world_min_y: a.world_min_y,
            world_max_x: a.world_max_x,
            world_max_y: a.world_max_y,
        });
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
                    let boon_map = boons.by_entity.get(e.id);
                    let series_row = series.by_entity.get(e.id);
                    let replay_row = replay.by_entity.get(e.id);
                    let track_row = replay.tracks.as_ref().and_then(|t| t.by_entity.get(e.id));
                    let rotation_row = rotation.by_entity.get(e.id);

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
                        // Native carries dps as `f64`; round rather than
                        // truncate for a more representative whole number.
                        dps: dmg.map(|d| d.dps.round() as u64).unwrap_or_default(),
                        damage_taken: dmg.map(|d| d.taken).unwrap_or_default(),
                        breakbar_damage: dmg.map(|d| d.breakbar_damage_dealt).unwrap_or_default(),
                        downs_dealt: dmg.map(|d| d.downs_dealt).unwrap_or_default(),
                        kills_dealt: dmg.map(|d| d.kills_dealt).unwrap_or_default(),

                        deaths: def.map(|d| d.deaths).unwrap_or_default(),
                        downs: def.map(|d| d.downs_taken).unwrap_or_default(),
                        incoming_cc: def.map(|d| d.received_cc_count).unwrap_or_default(),
                        incoming_strips: def.map(|d| d.boon_strips_taken).unwrap_or_default(),
                        blocked: def.map(|d| d.blocked_count).unwrap_or_default(),
                        evaded: def.map(|d| d.evaded_count).unwrap_or_default(),
                        dodges: def.map(|d| d.dodge_count).unwrap_or_default(),
                        missed: def.map(|d| d.missed_count).unwrap_or_default(),
                        interrupted: def.map(|d| d.interrupted_count).unwrap_or_default(),
                        invulned: def.map(|d| d.invulned_count).unwrap_or_default(),

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

                        boons: boon_map
                            .map(|m| {
                                m.iter()
                                    .map(|(id, row)| {
                                        let entry = r.catalogs.buffs.get(id);
                                        BoonRow {
                                            buff_id: *id,
                                            name: entry.map(|e| e.name.clone()).unwrap_or_default(),
                                            stacking: entry
                                                .map(|e| e.stacking.to_string())
                                                .unwrap_or_default(),
                                            uptime_pct: row.uptime_pct,
                                            avg_stacks: row.avg_stacks,
                                            gen_self: row.generation.self_pct,
                                            gen_group: row.generation.group_pct,
                                            gen_squad: row.generation.squad_pct,
                                            states: row
                                                .states
                                                .as_ref()
                                                .map(|s| {
                                                    s.iter().map(|(t, v)| (*t, *v as i32)).collect()
                                                })
                                                .unwrap_or_default(),
                                        }
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),

                        // `series_row` is `None` for a player with no
                        // series row at all -- an empty `Vec` per field
                        // is the same "measured absence" convention every
                        // other by-skill/by-entity lookup in this
                        // function follows. The three `Option<SeriesOut>`
                        // fields inside a present row (healing/barrier
                        // received, health_percents) additionally fold
                        // "not measured" (no healing addon / no
                        // `HEALTH_UPDATE` seen) into that same empty
                        // `Vec` -- see the field doc comments on
                        // `PlayerData` for why neither has a spare
                        // `Option` to keep the two apart.
                        damage_1s: series_row
                            .map(|s| decode_series(&s.damage))
                            .unwrap_or_default(),
                        damage_taken_1s: series_row
                            .map(|s| decode_series(&s.damage_taken))
                            .unwrap_or_default(),
                        healing_received_1s: series_row
                            .and_then(|s| s.healing_received_1s.as_ref())
                            .map(decode_series)
                            .unwrap_or_default(),
                        barrier_received_1s: series_row
                            .and_then(|s| s.barrier_received_1s.as_ref())
                            .map(decode_series)
                            .unwrap_or_default(),
                        health_percents: series_row
                            .and_then(|s| s.health_percents.clone())
                            .unwrap_or_default(),

                        positions: track_row
                            .map(|t| {
                                t.samples
                                    .iter()
                                    .map(|(_, x, y)| (*x as f32, *y as f32))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        track_start_ms: track_row
                            .and_then(|t| t.samples.first().map(|(t_ms, _, _)| *t_ms))
                            .unwrap_or_default(),
                        down_ranges: replay_row.map(|iv| iv.down.clone()).unwrap_or_default(),
                        dead_ranges: replay_row.map(|iv| iv.dead.clone()).unwrap_or_default(),
                        dc_ranges: replay_row.map(|iv| iv.dc.clone()).unwrap_or_default(),
                        active_ms: replay_row.map(|iv| iv.active_ms).unwrap_or_default(),
                        dist_to_com: replay_row.and_then(|iv| dist_to_com(iv.dist_to_com)),

                        casts: rotation_row
                            .and_then(|r| r.casts.as_ref())
                            .map(|casts| {
                                casts
                                    .iter()
                                    .map(|c| CastRow {
                                        skill_id: c.skill_id,
                                        cast_time_ms: c.cast_time_ms,
                                        duration_ms: c.duration_ms,
                                        time_gained_ms: c.time_gained_ms,
                                        quickness: c.quickness,
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    });
                }
                Role::EnemyPlayer => {
                    let track_row = replay.tracks.as_ref().and_then(|t| t.by_entity.get(e.id));
                    enemies.push(EnemyData {
                        entity_id: e.id,
                        name: e.name.clone().unwrap_or_default(),
                        team: e.team.clone(),
                        profession: e.profession.clone().unwrap_or_default(),
                        elite_spec: e.elite_spec.clone().unwrap_or_default(),
                        positions: track_row
                            .map(|t| {
                                t.samples
                                    .iter()
                                    .map(|(_, x, y)| (*x as f32, *y as f32))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        track_start_ms: track_row
                            .and_then(|t| t.samples.first().map(|(t_ms, _, _)| *t_ms))
                            .unwrap_or_default(),
                        down_ranges: track_row
                            .map(|t| t.down_intervals.clone())
                            .unwrap_or_default(),
                        dead_ranges: track_row
                            .map(|t| t.dead_intervals.clone())
                            .unwrap_or_default(),
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
            duration_ms: clamp_duration_ms(r.encounter.duration_ms),
            map_name: r.encounter.map.clone(),
            map_id: r.encounter.map_id,
            started_at_unix: r.encounter.started_at_unix,
            log_start_ms: r.encounter.log_start_ms,
            self_idx,
            commander_idx,
            players,
            enemies,
            entity_index,
            arena,
            healing_available,
            poll_ms: replay.tracks.as_ref().map(|t| t.poll_ms).unwrap_or_default(),
            skill_icons: r
                .catalogs
                .skills
                .iter()
                .filter_map(|(id, e)| e.icon.as_ref().map(|i| (*id, i.clone())))
                .collect(),
            buff_icons: r
                .catalogs
                .buffs
                .iter()
                .filter_map(|(id, e)| e.icon.as_ref().map(|i| (*id, i.clone())))
                .collect(),
        }
    }
}

/// Collapses native's tri-state `dist_to_com`/`stack_dist` convention
/// (`None` = pass never ran, `Some(-1.0)` = ran and nothing qualified,
/// `Some(x >= 0.0)` = a real distance) to this projection's two-state
/// `Option<f32>` -- see [`PlayerData::dist_to_com`]'s doc comment for why
/// the two absent cases are safe to fold together here. `-1.0` is an EXACT
/// sentinel on the native side (`axilog_core::analysis::distance::
/// NO_DISTANCE`, a `const` assigned directly rather than the result of any
/// averaging that could land near but not on it -- confirmed by that
/// module's own tests, which assert `== NO_DISTANCE` rather than an
/// epsilon comparison), so an exact equality check here is not a
/// float-comparison hazard.
fn dist_to_com(raw: Option<f64>) -> Option<f32> {
    const NO_DISTANCE: f64 = -1.0;
    match raw {
        Some(d) if d == NO_DISTANCE => None,
        Some(d) => Some(d as f32),
        None => None,
    }
}

#[cfg(test)]
mod dist_to_com_tests {
    use super::dist_to_com;

    #[test]
    fn absent_pass_stays_none() {
        assert_eq!(dist_to_com(None), None);
    }

    #[test]
    fn the_ei_sentinel_collapses_to_none_not_a_negative_distance() {
        assert_eq!(dist_to_com(Some(-1.0)), None);
    }

    #[test]
    fn a_real_zero_distance_survives_as_some() {
        // `0.0` must NOT be treated as absent -- only the exact `-1.0`
        // sentinel is. A commander standing on top of themselves is a
        // real, meaningful `Some(0.0)`.
        assert_eq!(dist_to_com(Some(0.0)), Some(0.0));
    }

    #[test]
    fn a_positive_distance_narrows_to_f32() {
        assert_eq!(dist_to_com(Some(123.5)), Some(123.5_f32));
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
