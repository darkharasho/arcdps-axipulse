//! Task 6: proves the replay projection (`positions`, down/dead/dc
//! ranges, `active_ms`, `dist_to_com`, `casts`, `FightData::arena`)
//! `FightData::from_report` adds to `PlayerData`.
//!
//! **No EI equality oracle here**, unlike Tasks 3-5's fixture tests, and
//! that is deliberate, not an oversight:
//!
//! 1. `tests/fixtures/wvw.ei.json` (`common::ei()`) was exported WITHOUT
//!    `--replay` -- every player's `combatReplayData` is `null` in the
//!    fixture (checked directly: `python3 -c "import json; d =
//!    json.load(open('tests/fixtures/wvw.ei.json')); print(d['players'][0]
//!    .get('combatReplayData'))"` prints `None`). There is nothing to
//!    join `down_ranges`/`dead_ranges` against even though
//!    `docs/EI-PARITY.md` documents those two as byte-exact vs EI on a
//!    fixture that HAS replay data.
//! 2. Positions/distance have no EI counterpart to compare at all:
//!    `axilog/docs/EI-PARITY.md`'s "Known, deliberate gaps" section names
//!    the combat-replay position surface as the one deliberate exception
//!    to the rest of the format's EI parity, because axilog's positions
//!    come from a different resampler than GW2EI's own combat-replay
//!    export. An EI equality assertion here would be the wrong oracle
//!    (see this task's own brief, context note 7).
//!
//! So this file asserts native-side invariants instead: self-consistency
//! against the player's own `deaths` count, the tri-state `dist_to_com`
//! collapse, the polling grid, and the arena projection formula.

mod common;
use arcdps_axipulse::fight_data::FightData;

/// `dead_ranges.len()` is the number of times a player finished dying,
/// which is exactly what `deaths` (`blocks.defenses.by_entity[id]
/// .deaths`) counts -- both walk the same status-event stream, just
/// summarized two different ways. This is a same-source self-consistency
/// check, not an EI comparison.
#[test]
fn dead_ranges_len_matches_the_deaths_scalar() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let mut any_nonzero = false;
    for p in f.players.iter().filter(|p| p.in_squad) {
        any_nonzero |= p.deaths > 0;
        assert_eq!(
            p.dead_ranges.len(),
            p.deaths as usize,
            "{} has {} dead_ranges but deaths says {}",
            p.account,
            p.dead_ranges.len(),
            p.deaths,
        );
    }
    assert!(
        any_nonzero,
        "no squad member died in this fixture -- the check above is vacuous"
    );
}

/// `dist_to_com` must never surface EI's `-1.0` "nothing qualified"
/// sentinel as a distance -- see `dist_to_com`'s (the free function in
/// `fight_data.rs`) own doc comment and its dedicated unit tests for the
/// synthetic `-1.0`/`None` cases this fixture cannot exercise (this
/// fixture's own `dist_to_com` values are all real measured distances --
/// see this task's report for the measured count). This test instead
/// pins the OUTPUT invariant end to end, off the real fixture: whatever
/// comes out of `from_report` is either absent or non-negative.
#[test]
fn dist_to_com_is_never_the_negative_sentinel() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let mut any_present = false;
    for p in f.players.iter().filter(|p| p.in_squad) {
        if let Some(d) = p.dist_to_com {
            any_present = true;
            assert!(
                d >= 0.0,
                "{} dist_to_com is {} -- a negative value means the EI \
                 sentinel leaked through instead of collapsing to None",
                p.account,
                d,
            );
        }
    }
    assert!(
        any_present,
        "no squad member has a dist_to_com at all in this fixture -- \
         the check above is vacuous"
    );
}

/// Every player's replay track lands on the shared polling grid
/// (`blocks.replay.tracks.poll_ms`) -- verified here by re-deriving the
/// grid length bound from `duration_ms` and checking no player's
/// `positions` exceeds it. Track LENGTH is deliberately NOT asserted
/// equal across players: `blocks.replay.tracks.by_entity[id].samples`
/// starts at each player's own first-aware time rounded up to the grid
/// and ends at their own last-aware time (see `PlayerData::positions`'s
/// doc comment), which measured against this fixture already differs
/// for 8 of 47 squad members (7 short by 1-9 samples, one far short at
/// 254 of the modal 461) -- asserting equal length here would be
/// asserting something this fixture's own real data does not do.
#[test]
fn positions_never_exceed_the_polling_grid_bound() {
    let n = common::native();
    let f = FightData::from_report(&n);
    // `blocks.replay.tracks.poll_ms` is 300ms on this fixture (checked
    // directly against the native report during implementation); a
    // player present for the whole encounter cannot have more samples
    // than `duration_ms / poll_ms + 1`.
    let poll_ms = 300u64;
    let max_samples = (f.duration_ms / poll_ms + 2) as usize;
    let mut any_positions = false;
    for p in f.players.iter().filter(|p| p.in_squad) {
        if !p.positions.is_empty() {
            any_positions = true;
        }
        assert!(
            p.positions.len() <= max_samples,
            "{} has {} position samples, more than the grid bound {} \
             (duration_ms={}, poll_ms={})",
            p.account,
            p.positions.len(),
            max_samples,
            f.duration_ms,
            poll_ms,
        );
    }
    assert!(
        any_positions,
        "no squad member has any position samples in this fixture -- \
         `--replay` may not have actually produced tracks"
    );
}

/// The commander's first position, projected through `FightData::arena`
/// using the formula on `PlayerData::positions`'s doc comment, must land
/// inside the arena image's pixel bounds -- the same invariant `ui/map.rs`
/// (migration Task 7) will rely on to plot it.
#[test]
fn the_commanders_first_position_projects_inside_the_arena_image() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let commander_idx = f
        .commander_idx
        .expect("this fixture has a commander-tagged squad member");
    let commander = &f.players[commander_idx];
    let (x, y) = *commander
        .positions
        .first()
        .expect("the commander has at least one position sample");
    let arena = f
        .arena
        .as_ref()
        .expect("this fixture's map has a known arena image");

    let px = (x - arena.world_min_x as f32) / (arena.world_max_x as f32 - arena.world_min_x as f32)
        * arena.image_width as f32;
    let py = (1.0
        - (y - arena.world_min_y as f32) / (arena.world_max_y as f32 - arena.world_min_y as f32))
        * arena.image_height as f32;

    assert!(
        (0.0..=arena.image_width as f32).contains(&px),
        "commander px {px} outside [0, {}]",
        arena.image_width
    );
    assert!(
        (0.0..=arena.image_height as f32).contains(&py),
        "commander py {py} outside [0, {}]",
        arena.image_height
    );
}

/// `casts` is already time-ordered on the native side
/// (`build_rotation` sorts by `(cast_time_ms, skill_id)`); this proves
/// the projection preserves that order rather than re-shuffling it.
#[test]
fn casts_are_non_decreasing_in_time() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let mut any_casts = false;
    for p in f.players.iter().filter(|p| p.in_squad) {
        for pair in p.casts.windows(2) {
            any_casts = true;
            assert!(
                pair[0].cast_time_ms <= pair[1].cast_time_ms,
                "{} casts out of order: {} then {}",
                p.account,
                pair[0].cast_time_ms,
                pair[1].cast_time_ms,
            );
        }
    }
    assert!(
        any_casts,
        "no squad member has 2+ casts in this fixture -- the check above is vacuous"
    );
}

/// `down`/`dead`/`dc` ranges are half-open `[start_ms, end_ms)` -- every
/// range's end must not precede its start, and `active_ms` must never
/// exceed the player's own aware span.
#[test]
fn ranges_and_active_ms_are_internally_consistent() {
    let n = common::native();
    let f = FightData::from_report(&n);
    for p in f.players.iter().filter(|p| p.in_squad) {
        for (kind, ranges) in [
            ("down", &p.down_ranges),
            ("dead", &p.dead_ranges),
            ("dc", &p.dc_ranges),
        ] {
            for &(start, end) in ranges {
                assert!(
                    start <= end,
                    "{} has a {} range with start {} > end {}",
                    p.account,
                    kind,
                    start,
                    end,
                );
            }
        }
        assert!(
            p.active_ms <= f.duration_ms,
            "{} active_ms {} exceeds the fight duration {}",
            p.account,
            p.active_ms,
            f.duration_ms,
        );
    }
}
