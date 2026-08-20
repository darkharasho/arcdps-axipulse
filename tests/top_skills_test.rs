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

/// A name that is nothing but digits has not actually named the skill,
/// and rendering it bare reads as a value rather than an identifier.
/// The Elite Insights reader rejected it (`resolve_skill_name`'s
/// `parse::<i64>().is_err()` guard) and so must this.
#[test]
fn a_purely_numeric_name_falls_back_to_its_id() {
    let p = PlayerData {
        damage_by_skill: vec![
            row(4242, "12345", 9),
            row(77, "-3", 8),
            // Not purely numeric -- these are real names and must survive.
            row(88, "Symbol of Blades", 7),
            row(99, "1000 Cuts", 6),
        ],
        ..PlayerData::default()
    };
    let top = top_damage(&p, 4);
    assert_eq!(top[0].name, "Skill 4242");
    assert_eq!(top[1].name, "Skill 77");
    assert_eq!(top[2].name, "Symbol of Blades");
    assert_eq!(top[3].name, "1000 Cuts");
}

/// The same guard has to hold for the healing and barrier lists, which
/// go through `skill_label` too.
#[test]
fn top_heals_reject_a_numeric_name_as_well() {
    use arcdps_axipulse::top_heals::{top_barrier, top_healing};
    let p = PlayerData {
        healing_by_skill: vec![row(1234, "1234", 50)],
        barrier_by_skill: vec![row(5678, "", 40)],
        ..PlayerData::default()
    };
    assert_eq!(top_healing(&p, 1)[0].name, "Skill 1234");
    assert_eq!(top_barrier(&p, 1)[0].name, "Skill 5678");
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

/// Through Task 7 the summed per-skill damage was additionally proved
/// against Elite Insights' own `totalDamageDist` sum, deleted by
/// Task 8 -- the oracle has served its purpose. What remains
/// native-only: `damage_by_skill`'s own total must agree EXACTLY with
/// `pulse_metrics::damage`, the scalar the Pulse Overview shows -- both
/// are read off the same `blocks.damage` row, just summarized two
/// different ways, so any drift between them is a projection bug.
#[test]
fn per_skill_damage_sums_to_the_overview_damage_scalar() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let p = &f.players[f.self_idx.expect("fixture resolves a local player")];

    let native_total: u64 = p.damage_by_skill.iter().map(|r| r.total).sum();
    assert_eq!(native_total, arcdps_axipulse::pulse_metrics::damage(p));
}
