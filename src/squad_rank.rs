//! Rank a player within the squad subset on a single metric.

use crate::ei_model::EiJson;
use crate::pulse_metrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankMetric {
    Damage,
    DownContribution,
    Cleanses,
    Strips,
    DamageTaken,
}

/// Returns `Some(rank)` where rank is 1-indexed among squad members
/// (`not_in_squad == false`). Returns `None` if the target isn't a
/// squad member or the index is out of range. Ties keep the natural
/// roster order — the earlier player gets the better rank.
pub fn rank_in_squad(json: &EiJson, target_idx: usize, metric: RankMetric) -> Option<u32> {
    let target = json.players.get(target_idx)?;
    if target.not_in_squad {
        return None;
    }
    let value_of = |p: &crate::ei_model::EiPlayer| -> u64 {
        match metric {
            RankMetric::Damage           => pulse_metrics::damage(p),
            RankMetric::DownContribution => pulse_metrics::down_contribution(p),
            RankMetric::Cleanses         => pulse_metrics::cleanses(p),
            RankMetric::Strips           => pulse_metrics::strips(p),
            RankMetric::DamageTaken      => pulse_metrics::damage_taken(p),
        }
    };
    let target_value = value_of(target);
    let better_count = json.players.iter().enumerate()
        .filter(|(i, p)| !p.not_in_squad && *i != target_idx && value_of(p) > target_value)
        .count();
    Some(better_count as u32 + 1)
}
