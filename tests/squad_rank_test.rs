//! `rank_in_squad` over both a synthetic roster (for the ordering rules)
//! and the real fixture (for the range invariant, plus an equality
//! oracle against Elite Insights' own damage ranking).

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

/// **Equality oracle.** The local player's damage rank computed off
/// `FightData` must equal the rank produced by sorting Elite Insights'
/// own squad by `dpsAll[0].damage`. This is the one place the two
/// pipelines' damage numbers are compared end to end through a
/// consumer, rather than field by field.
#[test]
fn the_local_players_damage_rank_matches_the_ei_oracle() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let e = common::ei();

    let idx = f.self_idx.expect("fixture resolves a local player");
    let me = &f.players[idx];
    let native_rank = rank_in_squad(&f, idx, RankMetric::Damage).expect("local player is in squad");

    let my_ei = e
        .players
        .iter()
        .find(|p| p.account == me.account)
        .expect("the local player appears in the EI baseline");
    let my_ei_damage = my_ei.dps_all.first().map(|d| d.damage).unwrap_or(0);
    // Same tie rule as `rank_in_squad`: count strictly-better players.
    let ei_rank = e
        .players
        .iter()
        .filter(|p| !p.not_in_squad && p.account != me.account)
        .filter(|p| p.dps_all.first().map(|d| d.damage).unwrap_or(0) > my_ei_damage)
        .count() as u32
        + 1;

    assert_eq!(
        native_rank, ei_rank,
        "native ranked the local player {native_rank}, EI ranked them {ei_rank}",
    );
}
