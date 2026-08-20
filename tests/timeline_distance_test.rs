//! `distance_to_commander_per_second`, including the regression that
//! motivated it: two players' position tracks do NOT start at the same
//! time, so they must be matched by TIME, never by sample index.

mod common;

use arcdps_axipulse::fight_data::{FightData, PlayerData};
use arcdps_axipulse::timeline_distance::{distance_to_commander_per_second, position_at};

fn fight(poll_ms: u64, players: Vec<PlayerData>) -> FightData {
    let commander_idx = players.iter().position(|p| p.is_commander);
    FightData { poll_ms, commander_idx, players, ..FightData::default() }
}

fn track(start_ms: u64, positions: Vec<(f32, f32)>, is_commander: bool) -> PlayerData {
    PlayerData { track_start_ms: start_ms, positions, is_commander, in_squad: true, ..PlayerData::default() }
}

#[test]
fn computes_distance_in_inches() {
    let f = fight(1000, vec![
        track(0, vec![(0.0, 0.0), (3.0, 4.0), (6.0, 8.0)], false),
        track(0, vec![(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)], true),
    ]);
    let samples = distance_to_commander_per_second(&f, 0, 3000);
    assert_eq!(samples.len(), 4);
    assert!((samples[0] - 0.0).abs() < 0.01);
    assert!((samples[1] - 5.0).abs() < 0.01);
    assert!((samples[2] - 10.0).abs() < 0.01);
}

/// **The regression this rewrite exists for.** The commander's track
/// starts 2s after the local player's. Sample index 0 means t=0 for one
/// and t=2000 for the other, so index-aligned subtraction would pair
/// `(0,0)` with `(100,0)` and report 100 at t=0. Matching by time pairs
/// t=2000 with t=2000, where both sit at `(100, 0)` -- distance 0.
#[test]
fn tracks_with_different_start_times_are_matched_by_time_not_index() {
    let f = fight(1000, vec![
        // me: t=0..3000 walking +100/s along x
        track(0, vec![(0.0, 0.0), (100.0, 0.0), (200.0, 0.0), (300.0, 0.0)], false),
        // commander: t=2000..3000, sitting exactly where I am at those
        // two instants.
        track(2000, vec![(200.0, 0.0), (300.0, 0.0)], true),
    ]);
    let samples = distance_to_commander_per_second(&f, 0, 3000);
    assert_eq!(samples.len(), 4);
    for (sec, d) in samples.iter().enumerate() {
        assert!(
            d.abs() < 0.01,
            "second {sec} reported distance {d}; the tracks were aligned by index, not by time",
        );
    }
}

#[test]
fn position_at_reads_a_tracks_own_grid() {
    let p = track(2000, vec![(10.0, 0.0), (20.0, 0.0), (30.0, 0.0)], false);
    // Before the track starts: holds at the first sample.
    assert_eq!(position_at(&p, 0, 1000), Some((10.0, 0.0)));
    assert_eq!(position_at(&p, 2000, 1000), Some((10.0, 0.0)));
    assert_eq!(position_at(&p, 3000, 1000), Some((20.0, 0.0)));
    assert_eq!(position_at(&p, 4000, 1000), Some((30.0, 0.0)));
    // Past the end: holds at the last sample.
    assert_eq!(position_at(&p, 99_000, 1000), Some((30.0, 0.0)));
    assert_eq!(position_at(&PlayerData::default(), 0, 1000), None);
}

#[test]
fn returns_empty_when_no_commander() {
    let f = fight(1000, vec![track(0, vec![(0.0, 0.0), (1.0, 0.0)], false)]);
    assert!(
        distance_to_commander_per_second(&f, 0, 2000).is_empty(),
        "no commander -> no distance lane",
    );
}

#[test]
fn returns_empty_when_self_lacks_a_track() {
    let f = fight(1000, vec![
        PlayerData { in_squad: true, ..PlayerData::default() },
        track(0, vec![(0.0, 0.0), (1.0, 0.0)], true),
    ]);
    assert!(distance_to_commander_per_second(&f, 0, 2000).is_empty());
}

/// Being the commander is not a distance-to-commander measurement; a
/// flat zero lane would claim one.
#[test]
fn returns_empty_when_the_local_player_is_the_commander() {
    let f = fight(1000, vec![track(0, vec![(0.0, 0.0), (1.0, 0.0)], true)]);
    assert!(distance_to_commander_per_second(&f, 0, 2000).is_empty());
}

#[test]
fn returns_empty_when_the_tracks_never_overlap() {
    let f = fight(1000, vec![
        track(0, vec![(0.0, 0.0), (1.0, 0.0)], false),
        track(50_000, vec![(0.0, 0.0), (1.0, 0.0)], true),
    ]);
    assert!(distance_to_commander_per_second(&f, 0, 60_000).is_empty());
}

/// On the real fixture the lane must be one sample per second, every
/// value finite and non-negative, and its mean must sit in the same
/// neighbourhood as the native `dist_to_com` scalar computed by an
/// entirely separate pass. The two are close relatives rather than the
/// same number -- the scalar averages over the player's ACTIVE polls at
/// 300ms, this lane averages over whole seconds clamped into the overlap
/// window -- so the band is ±10%. Measured on this fixture: lane mean
/// 411.7 vs scalar 413.7, ratio 0.995.
#[test]
fn the_fixture_lane_agrees_with_the_native_scalar() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let idx = f.self_idx.expect("fixture resolves a local player");
    let lane = distance_to_commander_per_second(&f, idx, f.duration_ms);
    assert_eq!(lane.len(), (f.duration_ms / 1000) as usize + 1);
    assert!(lane.iter().all(|d| d.is_finite() && *d >= 0.0));

    let scalar = f.players[idx].dist_to_com.expect("fixture has a dist_to_com") as f64;
    let mean = lane.iter().sum::<f64>() / lane.len() as f64;
    let ratio = mean / scalar;
    assert!(
        (0.90..=1.10).contains(&ratio),
        "lane mean {mean:.1} vs native dist_to_com {scalar:.1} (ratio {ratio:.3})",
    );
}
