//! `boon_stacks_at` / `recent_skill_casts` over the native row shapes.

use arcdps_axipulse::fight_data::CastRow;
use arcdps_axipulse::ui::map::{boon_stacks_at, recent_skill_casts};

#[test]
fn boon_stacks_picks_last_state_at_or_before_t() {
    let states = [(0, 0), (1000, 3), (2000, 5)];
    assert_eq!(boon_stacks_at(&states, 500), 0);
    assert_eq!(boon_stacks_at(&states, 1500), 3);
    assert_eq!(boon_stacks_at(&states, 5000), 5);
}

#[test]
fn boon_stacks_empty_returns_zero() {
    assert_eq!(boon_stacks_at(&[], 1000), 0);
}

fn cast(id: u32, t: i64) -> CastRow {
    CastRow { skill_id: id, cast_time_ms: t, ..CastRow::default() }
}

#[test]
fn recent_casts_returns_empty_when_no_rotation() {
    assert!(recent_skill_casts(&[], 5000, 4).is_empty());
}

#[test]
fn recent_casts_returns_casts_before_t_in_descending_order() {
    let casts = [cast(101, 1000), cast(101, 3000), cast(101, 8000)];
    assert_eq!(recent_skill_casts(&casts, 4000, 4), vec![(101, 3000), (101, 1000)]);
}

#[test]
fn recent_casts_ignores_negative_cast_time() {
    let casts = [cast(101, -500), cast(101, 1000)];
    assert_eq!(recent_skill_casts(&casts, 4000, 4), vec![(101, 1000)]);
}

#[test]
fn recent_casts_caps_at_max_results() {
    let casts: Vec<CastRow> = (1000..10000).step_by(1000).map(|t| cast(7, t as i64)).collect();
    let out = recent_skill_casts(&casts, 20000, 3);
    assert_eq!(out, vec![(7, 9000), (7, 8000), (7, 7000)]);
}

#[test]
fn recent_casts_merges_multiple_skill_ids_in_time_order() {
    // Native emits `casts` already sorted by (cast_time_ms, skill_id);
    // this list is in that order and must come back newest-first.
    let casts = [cast(303, 1000), cast(101, 2000), cast(202, 3000)];
    assert_eq!(
        recent_skill_casts(&casts, 5000, 4),
        vec![(202, 3000), (101, 2000), (303, 1000)],
    );
}
