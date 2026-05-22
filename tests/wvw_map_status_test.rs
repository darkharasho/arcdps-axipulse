use arcdps_axipulse::ui::map::{status_at, health_at, MemberStatus};

fn range(start: f64, end: f64) -> Vec<f64> { vec![start, end] }

#[test]
fn status_alive_with_no_ranges() {
    let dead: Vec<Vec<f64>> = vec![];
    let down: Vec<Vec<f64>> = vec![];
    assert_eq!(status_at(&dead, &down, 5000), MemberStatus::Alive);
}

#[test]
fn status_down_when_t_in_down_range() {
    let dead: Vec<Vec<f64>> = vec![];
    let down = vec![range(2000.0, 4000.0)];
    assert_eq!(status_at(&dead, &down, 3000), MemberStatus::Down);
}

#[test]
fn status_dead_overrides_down() {
    let dead = vec![range(2000.0, 8000.0)];
    let down = vec![range(2000.0, 4000.0)];
    assert_eq!(status_at(&dead, &down, 3000), MemberStatus::Dead);
}

#[test]
fn status_alive_outside_ranges() {
    let dead = vec![range(2000.0, 4000.0)];
    let down: Vec<Vec<f64>> = vec![];
    assert_eq!(status_at(&dead, &down, 5000), MemberStatus::Alive);
}

#[test]
fn status_inclusive_boundaries() {
    let dead = vec![range(1000.0, 2000.0)];
    let down: Vec<Vec<f64>> = vec![];
    assert_eq!(status_at(&dead, &down, 1000), MemberStatus::Dead);
    assert_eq!(status_at(&dead, &down, 2000), MemberStatus::Dead);
}

#[test]
fn health_at_empty_returns_100() {
    let samples: Vec<Vec<f64>> = vec![];
    assert_eq!(health_at(&samples, 0), 100.0);
}

#[test]
fn health_at_picks_last_sample_at_or_before_t() {
    let samples = vec![vec![0.0, 100.0], vec![1000.0, 80.0], vec![2000.0, 50.0]];
    assert_eq!(health_at(&samples, 500), 100.0);
    assert_eq!(health_at(&samples, 1500), 80.0);
    assert_eq!(health_at(&samples, 5000), 50.0);
}

#[test]
fn health_at_returns_first_when_before_first_sample() {
    let samples = vec![vec![1000.0, 80.0]];
    assert_eq!(health_at(&samples, 0), 80.0);
}

use arcdps_axipulse::ui::map::{boon_stacks_at, recent_skill_casts};
use arcdps_axipulse::ei_model::{RotationEntry, SkillCast};

#[test]
fn boon_stacks_picks_last_state_at_or_before_t() {
    let states = vec![vec![0.0, 0.0], vec![1000.0, 3.0], vec![2000.0, 5.0]];
    assert_eq!(boon_stacks_at(&states, 500), 0);
    assert_eq!(boon_stacks_at(&states, 1500), 3);
    assert_eq!(boon_stacks_at(&states, 5000), 5);
}

#[test]
fn boon_stacks_empty_returns_zero() {
    let states: Vec<Vec<f64>> = vec![];
    assert_eq!(boon_stacks_at(&states, 1000), 0);
}

fn cast(t: i64, dur: u32) -> SkillCast {
    SkillCast { cast_time: t, duration: dur }
}

fn rot(id: i64, skills: Vec<SkillCast>) -> RotationEntry {
    RotationEntry { id, skills }
}

#[test]
fn recent_casts_returns_empty_when_no_rotation() {
    let rotation: Vec<RotationEntry> = vec![];
    let out = recent_skill_casts(&rotation, 5000, 4);
    assert!(out.is_empty());
}

#[test]
fn recent_casts_returns_casts_before_t_in_descending_order() {
    let rotation = vec![rot(101, vec![cast(1000, 500), cast(3000, 500), cast(8000, 500)])];
    let out = recent_skill_casts(&rotation, 4000, 4);
    assert_eq!(out, vec![(101, 3000), (101, 1000)]);
}

#[test]
fn recent_casts_ignores_negative_cast_time() {
    let rotation = vec![rot(101, vec![cast(-500, 200), cast(1000, 500)])];
    let out = recent_skill_casts(&rotation, 4000, 4);
    assert_eq!(out, vec![(101, 1000)]);
}

#[test]
fn recent_casts_caps_at_max_results() {
    let mut casts = Vec::new();
    for t in (1000..10000).step_by(1000) { casts.push(cast(t as i64, 100)); }
    let rotation = vec![rot(7, casts)];
    let out = recent_skill_casts(&rotation, 20000, 3);
    assert_eq!(out.len(), 3);
    assert_eq!(out, vec![(7, 9000), (7, 8000), (7, 7000)]);
}

#[test]
fn recent_casts_merges_multiple_skill_ids_in_time_order() {
    let rotation = vec![
        rot(101, vec![cast(2000, 500)]),
        rot(202, vec![cast(3000, 500)]),
        rot(303, vec![cast(1000, 500)]),
    ];
    let out = recent_skill_casts(&rotation, 5000, 4);
    assert_eq!(out, vec![(202, 3000), (101, 2000), (303, 1000)]);
}
