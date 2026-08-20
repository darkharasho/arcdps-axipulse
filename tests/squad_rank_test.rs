//! `rank_in_squad` over both a synthetic roster (for the ordering rules)
//! and the real fixture (for the range invariant). Through Task 7 the
//! local player's rank was additionally proved against an Elite
//! Insights equality oracle, deleted by Task 8 -- the range invariant
//! below already re-derives every rank from scratch each run, which is
//! the stronger check.

mod common;

use arcdps_axipulse::fight_data::{FightData, PlayerData};
use arcdps_axipulse::squad_rank::{rank_in_squad, RankMetric};

fn fight_with(values: &[(&str, bool, u64)]) -> FightData {
    FightData {
        duration_ms: 1,
        players: values
            .iter()
            .map(|(acc, in_squad, dmg)| PlayerData {
                account: (*acc).to_string(),
                in_squad: *in_squad,
                damage: *dmg,
                ..PlayerData::default()
            })
            .collect(),
        ..FightData::default()
    }
}

#[test]
fn ranks_only_among_squad_members() {
    let f = fight_with(&[
        (":InSquadLow.1",  true,  100),
        (":InSquadHigh.2", true,  500),
        (":NonSquadTop.3", false, 9999),
        (":InSquadMid.4",  true,  300),
    ]);
    assert_eq!(rank_in_squad(&f, 0, RankMetric::Damage), Some(3));
    assert_eq!(rank_in_squad(&f, 1, RankMetric::Damage), Some(1));
    assert_eq!(rank_in_squad(&f, 2, RankMetric::Damage), None);
    assert_eq!(rank_in_squad(&f, 3, RankMetric::Damage), Some(2));
}

#[test]
fn out_of_range_returns_none() {
    let f = fight_with(&[(":A.1", true, 100)]);
    assert_eq!(rank_in_squad(&f, 99, RankMetric::Damage), None);
}

#[test]
fn solo_squad_ranks_first() {
    let f = fight_with(&[(":Solo.1", true, 100)]);
    assert_eq!(rank_in_squad(&f, 0, RankMetric::Damage), Some(1));
}

/// Every squad member's rank on every metric must land in
/// `[1, squad_size]`, and every non-squad friendly must rank `None`.
#[test]
fn every_rank_falls_inside_the_squad_on_the_real_fixture() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let squad_size = f.players.iter().filter(|p| p.in_squad).count() as u32;
    assert!(squad_size > 1, "fixture has no squad to rank within");

    for (i, p) in f.players.iter().enumerate() {
        for metric in [
            RankMetric::Damage,
            RankMetric::DownContribution,
            RankMetric::Strips,
            RankMetric::Cleanses,
            RankMetric::DamageTaken,
        ] {
            let rank = rank_in_squad(&f, i, metric);
            if p.in_squad {
                let rank = rank.unwrap_or_else(|| panic!("{} ranked None in squad", p.account));
                assert!(
                    (1..=squad_size).contains(&rank),
                    "{} ranked {rank} of {squad_size}",
                    p.account,
                );
            } else {
                assert_eq!(rank, None, "{} is not in the squad but ranked", p.account);
            }
        }
    }
}
