//! Per-fight derived data — every expensive walk over EI JSON the UI
//! used to do every frame is computed exactly once here (on the parser
//! worker thread, right after EI returns) and stored on `FightRecord`
//! behind an `Arc`. Render paths become cheap reads.
//!
//! All fields are public so callers can read them without methods.

use crate::ei_model::EiJson;

#[derive(Debug, Default)]
pub struct Derived {
    /// Resolved local-player index, or `None` if EI's `recordedAccountBy`
    /// didn't match any player.
    pub self_idx: Option<usize>,

    // --- Pulse Overview / Damage / Support squad ranks --------------
    pub rank_damage:            Option<u32>,
    pub rank_down_contribution: Option<u32>,
    pub rank_strips:            Option<u32>,
    pub rank_cleanses:          Option<u32>,
    pub rank_damage_taken:      Option<u32>,

    // --- Fight composition ------------------------------------------
    pub composition: Vec<crate::fight_composition::Group>,

    // --- Top skills (each subview's "top 8") ------------------------
    pub top_damage:            Vec<crate::top_skills::SkillEntry>,
    pub top_down_contribution: Vec<crate::top_skills::SkillEntry>,
    pub top_healing:           Vec<crate::top_heals::HealEntry>,
    pub top_downed_healing:    Vec<crate::top_heals::HealEntry>,
    pub top_barrier:           Vec<crate::top_heals::BarrierEntry>,

    // --- Pulse Boons subview ----------------------------------------
    pub boon_uptimes: Vec<crate::boon_uptime::BoonUptime>,

    // --- Timeline lane samples --------------------------------------
    pub health_samples:    Vec<f64>,
    pub dmg_dealt_samples: Vec<u64>,
    pub dmg_taken_samples: Vec<u64>,
    pub distance_samples:  Vec<f64>,
    pub off_boons:         Vec<crate::timeline_boons::BoonSeries>,
    pub def_boons:         Vec<crate::timeline_boons::BoonSeries>,
}

impl Derived {
    pub fn compute(json: &EiJson) -> Self {
        use crate::boon_uptime::collect_uptimes;
        use crate::fight_composition::compute as compute_comp;
        use crate::self_identify::find_self_index;
        use crate::squad_rank::{rank_in_squad, RankMetric};
        use crate::timeline_boons::{defensive_boons, offensive_boons};
        use crate::timeline_buckets::{extract_damage_dealt, extract_damage_taken};
        use crate::timeline_distance::distance_to_commander_per_second;
        use crate::timeline_health::sample_health_per_second;
        use crate::top_heals::{top_barrier, top_downed_healing, top_healing};
        use crate::top_skills::{top_damage, top_down_contribution};

        let self_idx = find_self_index(json);
        let mut d = Derived { self_idx, ..Derived::default() };
        d.composition = compute_comp(json, self_idx.unwrap_or(0));

        let Some(idx) = self_idx else { return d; };
        let Some(p) = json.players.get(idx) else { return d; };

        d.rank_damage            = rank_in_squad(json, idx, RankMetric::Damage);
        d.rank_down_contribution = rank_in_squad(json, idx, RankMetric::DownContribution);
        d.rank_strips            = rank_in_squad(json, idx, RankMetric::Strips);
        d.rank_cleanses          = rank_in_squad(json, idx, RankMetric::Cleanses);
        d.rank_damage_taken      = rank_in_squad(json, idx, RankMetric::DamageTaken);

        d.top_damage            = top_damage(p, 8);
        d.top_down_contribution = top_down_contribution(p, 8);
        d.top_healing           = top_healing(p, 8);
        d.top_downed_healing    = top_downed_healing(p, 8);
        d.top_barrier           = top_barrier(p, 8);

        d.boon_uptimes = collect_uptimes(p);

        let dur = json.duration_ms;
        d.health_samples    = sample_health_per_second(p, dur);
        d.dmg_dealt_samples = extract_damage_dealt(p);
        d.dmg_taken_samples = extract_damage_taken(p);
        d.distance_samples  = distance_to_commander_per_second(json, idx, dur);
        d.off_boons         = offensive_boons(p, dur);
        d.def_boons         = defensive_boons(p, dur);

        d
    }
}
