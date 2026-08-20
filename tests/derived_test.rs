//! `Derived::compute` over the real fixture — the one place every leaf
//! module this migration rewired is exercised together, the way
//! `plugin::on_new_log` exercises them.

mod common;

use arcdps_axipulse::derived::Derived;
use arcdps_axipulse::fight_composition::GroupKey;
use arcdps_axipulse::fight_data::{FightData, PlayerData};

fn fixture() -> FightData {
    let n = common::native();
    FightData::from_report(&n)
}

#[test]
fn every_derived_surface_is_populated_for_the_fixture() {
    let f = fixture();
    let d = Derived::compute(&f);

    assert_eq!(d.self_idx, f.self_idx, "self_idx is copied, not re-guessed");
    assert!(d.rank_damage.is_some());
    assert!(d.rank_down_contribution.is_some());
    assert!(d.rank_strips.is_some());
    assert!(d.rank_cleanses.is_some());
    assert!(d.rank_damage_taken.is_some());

    assert!(!d.top_damage.is_empty(), "no top damage skills");
    assert!(!d.top_down_contribution.is_empty(), "no down-contribution skills");
    assert!(!d.boon_uptimes.is_empty(), "no boon uptimes");

    // One health sample per second, inclusive of both ends.
    assert_eq!(d.health_samples.len(), (f.duration_ms / 1000) as usize + 1);
    assert!(!d.dmg_dealt_samples.is_empty());
    assert!(!d.dmg_taken_samples.is_empty());
    assert_eq!(d.off_boons.len(), 4);
    assert_eq!(d.def_boons.len(), 4);

    // The distance lane is one entry per second like the others, but its
    // entries are `Option` -- an unmeasured second must arrive at the UI
    // as absent rather than as a neighbour's value or a zero.
    assert_eq!(d.distance_samples.len(), (f.duration_ms / 1000) as usize + 1);
    let summary = arcdps_axipulse::timeline_distance::summarize(&d.distance_samples)
        .expect("the fixture measures some seconds");
    assert!(summary.measured_secs <= summary.total_secs);
    assert!(summary.avg > 0.0 && summary.avg <= summary.max);
}

/// The fight-composition card must name a Squad group and at least one
/// enemy team, and every enemy key must be a resolved colour name rather
/// than the raw team id the Elite Insights version keyed on.
#[test]
fn the_composition_groups_the_squad_and_the_enemy_teams() {
    let f = fixture();
    let d = Derived::compute(&f);
    assert!(!d.composition.is_empty());

    let squad = d
        .composition
        .iter()
        .find(|g| g.key == GroupKey::Squad)
        .expect("no Squad group");
    assert_eq!(squad.count as usize, f.players.iter().filter(|p| p.in_squad).count());
    assert!(!squad.class_counts.is_empty());
    assert_eq!(
        squad.class_counts.iter().map(|(_, n)| n).sum::<u32>(),
        squad.count,
        "class counts must partition the group",
    );

    let enemy_keys: Vec<&String> = d
        .composition
        .iter()
        .filter_map(|g| match &g.key {
            GroupKey::Enemy(team) => Some(team),
            _ => None,
        })
        .collect();
    assert!(!enemy_keys.is_empty(), "no enemy teams in a WvW fixture");
    for team in &enemy_keys {
        assert!(
            ["red", "green", "blue", "unknown"].contains(&team.as_str()),
            "enemy group keyed on {team:?}, not a resolved colour",
        );
    }
    // Enemy teams are ordered largest first, so "T1" is the biggest.
    let enemy_counts: Vec<u32> = d
        .composition
        .iter()
        .filter(|g| matches!(g.key, GroupKey::Enemy(_)))
        .map(|g| g.count)
        .collect();
    assert!(enemy_counts.windows(2).all(|w| w[0] >= w[1]));
}

/// A fight whose recorder never resolved must still produce a usable
/// `Derived` — empty ranks, no panic — rather than indexing player 0 and
/// showing someone else's numbers as yours.
#[test]
fn a_fight_with_no_local_player_derives_nothing_personal() {
    let f = FightData {
        duration_ms: 1000,
        players: vec![PlayerData { in_squad: true, damage: 5, ..PlayerData::default() }],
        ..FightData::default()
    };
    let d = Derived::compute(&f);
    assert_eq!(d.self_idx, None);
    assert_eq!(d.rank_damage, None);
    assert!(d.top_damage.is_empty());
    assert!(d.health_samples.is_empty());
}
