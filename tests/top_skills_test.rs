//! `top_skills` / `top_heals` ordering and limiting, over a synthetic
//! `PlayerData` for the rules and the real fixture for the shape.

mod common;

use arcdps_axipulse::fight_data::{FightData, PlayerData, SkillRow};
use arcdps_axipulse::top_skills::{top_damage, top_down_contribution, SkillEntry};

fn row(id: u32, name: &str, total: u64) -> SkillRow {
    SkillRow { skill_id: id, name: name.to_string(), total, ..SkillRow::default() }
}

fn player_with_dist() -> PlayerData {
    PlayerData {
        damage_by_skill: vec![
            row(1, "Skill A", 500),
            row(2, "Skill B", 1000),
            row(3, "Skill C", 0),
            row(4, "Skill D", 200),
        ],
        // Down contribution is its own native distribution, not a second
        // column on the damage rows -- see `top_skills`'s module doc.
        down_contribution_by_skill: vec![
            row(1, "Skill A", 50),
            row(2, "Skill B", 10),
            row(3, "Skill C", 0),
            row(4, "Skill D", 80),
        ],
        ..PlayerData::default()
    }
}

#[test]
fn top_damage_sorts_descending_and_filters_zero() {
    let p = player_with_dist();
    let top = top_damage(&p, 10);
    assert_eq!(top.len(), 3);
    assert_eq!(top[0].name, "Skill B");
    assert_eq!(top[0].damage, 1000);
    assert_eq!(top[1].name, "Skill A");
    assert_eq!(top[2].name, "Skill D");
}

#[test]
fn top_damage_respects_limit() {
    let p = player_with_dist();
    let top = top_damage(&p, 2);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].name, "Skill B");
    assert_eq!(top[1].name, "Skill A");
}

#[test]
fn top_down_contribution_sorts_and_filters() {
    let p = player_with_dist();
    let top = top_down_contribution(&p, 10);
    assert_eq!(top.len(), 3);
    assert_eq!(top[0].name, "Skill D");
    assert_eq!(top[1].name, "Skill A");
    assert_eq!(top[2].name, "Skill B");
}

#[test]
fn empty_dist_returns_empty() {
    assert_eq!(top_damage(&PlayerData::default(), 10), Vec::<SkillEntry>::new());
}

/// A catalog miss yields "Skill <id>", never a blank row label.
#[test]
fn an_unnamed_skill_falls_back_to_its_id() {
    let p = PlayerData { damage_by_skill: vec![row(4242, "", 7)], ..PlayerData::default() };
    assert_eq!(top_damage(&p, 1)[0].name, "Skill 4242");
}

/// On the real fixture the local player's top-damage list must be
/// non-empty, strictly non-increasing, and capped at the limit.
#[test]
fn the_fixtures_local_player_has_an_ordered_top_damage_list() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let p = &f.players[f.self_idx.expect("fixture resolves a local player")];
    let top = top_damage(p, 8);
    assert!(!top.is_empty(), "local player dealt no per-skill damage");
    assert!(top.len() <= 8);
    for pair in top.windows(2) {
        assert!(pair[0].damage >= pair[1].damage);
    }
    assert!(top.iter().all(|e| !e.name.is_empty()));
}

/// **Equality oracle.** The summed per-skill damage must equal Elite
/// Insights' own `totalDamageDist` sum for the same player.
#[test]
fn per_skill_damage_sums_match_the_ei_oracle() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let e = common::ei();
    let p = &f.players[f.self_idx.expect("fixture resolves a local player")];

    let native_total: u64 = p.damage_by_skill.iter().map(|r| r.total).sum();
    let ei = e
        .players
        .iter()
        .find(|x| x.account == p.account)
        .expect("the local player appears in the EI baseline");
    let ei_total: u64 = ei.total_damage_dist.iter().flatten().map(|x| x.total_damage).sum();
    assert_eq!(native_total, ei_total);
}
