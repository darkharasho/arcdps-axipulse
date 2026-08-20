//! `distance_to_commander_per_second`, including the regression that
//! motivated it: two players' position tracks do NOT start at the same
//! time, so they must be matched by TIME, never by sample index.

mod common;

use arcdps_axipulse::fight_data::{FightData, PlayerData};
use arcdps_axipulse::timeline_distance::{
    distance_to_commander_per_second, position_at, summarize,
};

fn fight(poll_ms: u64, players: Vec<PlayerData>) -> FightData {
    let commander_idx = players.iter().position(|p| p.is_commander);
    FightData { poll_ms, commander_idx, players, ..FightData::default() }
}

fn track(start_ms: u64, positions: Vec<(f32, f32)>, is_commander: bool) -> PlayerData {
    PlayerData { track_start_ms: start_ms, positions, is_commander, in_squad: true, ..PlayerData::default() }
}

/// Unwraps a second that must have been measured. Panics with the index
/// rather than the opaque `Option::unwrap` message.
fn at(samples: &[Option<f64>], sec: usize) -> f64 {
    samples[sec].unwrap_or_else(|| panic!("second {sec} was expected to be measured"))
}

#[test]
fn computes_distance_in_inches() {
    let f = fight(1000, vec![
        track(0, vec![(0.0, 0.0), (3.0, 4.0), (6.0, 8.0)], false),
        track(0, vec![(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)], true),
    ]);
    let samples = distance_to_commander_per_second(&f, 0, 3000);
    assert_eq!(samples.len(), 4);
    assert!((at(&samples, 0) - 0.0).abs() < 0.01);
    assert!((at(&samples, 1) - 5.0).abs() < 0.01);
    assert!((at(&samples, 2) - 10.0).abs() < 0.01);
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
    // Seconds 0 and 1 precede the commander's track and are unmeasured;
    // 2 and 3 are inside the overlap and must both read 0.
    assert_eq!(samples[0], None);
    assert_eq!(samples[1], None);
    for sec in [2usize, 3] {
        let d = at(&samples, sec);
        assert!(
            d.abs() < 0.01,
            "second {sec} reported distance {d}; the tracks were aligned by index, not by time",
        );
    }
}

/// **The fabricated-measurement regression.** The commander tags up 100s
/// into a 300s fight. The 100 seconds before that were never measured
/// and must come back `None` -- an earlier draft clamped them into the
/// overlap window, so every one of them reported the distance measured
/// at t=100s, and the inspector card then averaged those 100 invented
/// values in with the real ones.
#[test]
fn seconds_before_the_commander_tagged_up_are_absent_not_invented() {
    // me: the whole 300s, drifting steadily away from the origin.
    let mine: Vec<(f32, f32)> = (0..=300).map(|s| (s as f32 * 10.0, 0.0)).collect();
    // commander: parked at the origin, but only from t=100s.
    let theirs: Vec<(f32, f32)> = (100..=300).map(|_| (0.0, 0.0)).collect();
    let f = fight(1000, vec![
        track(0, mine, false),
        track(100_000, theirs, true),
    ]);

    let samples = distance_to_commander_per_second(&f, 0, 300_000);
    assert_eq!(samples.len(), 301);
    for sec in 0..100 {
        assert_eq!(
            samples[sec], None,
            "second {sec} predates the commander's track but reported a distance",
        );
    }
    for sec in 100..=300 {
        let d = at(&samples, sec);
        assert!(
            (d - sec as f64 * 10.0).abs() < 0.01,
            "second {sec} reported {d}, expected {}",
            sec as f64 * 10.0,
        );
    }

    // And the summary the Position card reads must exclude them from
    // BOTH the numerator and the denominator.
    let d = summarize(&samples).expect("some seconds were measured");
    assert_eq!(d.measured_secs, 201);
    assert_eq!(d.total_secs, 301);
    assert!(d.is_partial());
    // Mean of 1000, 1010, ... 3000 -- the true mean of the 201 measured
    // seconds. Averaging over 301 would give 1336.2; carrying the t=100s
    // value backwards over the first 100 seconds would give 1671.6.
    let expected: f64 = (100..=300).map(|s| s as f64 * 10.0).sum::<f64>() / 201.0;
    assert!(
        (d.avg - expected).abs() < 0.01,
        "avg {} is not the mean of the measured seconds ({expected})",
        d.avg,
    );
    assert!((d.max - 3000.0).abs() < 0.01);
}

/// `summarize` is what stands between an unmeasured second and the
/// user's "Avg distance", so pin its arithmetic directly.
#[test]
fn summarize_ignores_unmeasured_seconds_in_both_terms() {
    let s = summarize(&[None, Some(10.0), None, Some(30.0), None]).expect("two measured");
    assert_eq!(s.measured_secs, 2);
    assert_eq!(s.total_secs, 5);
    assert!((s.avg - 20.0).abs() < 1e-9, "avg {} used the wrong denominator", s.avg);
    assert!((s.max - 30.0).abs() < 1e-9);
    assert!(s.is_partial());

    // Fully-covered lane: not partial, so the card shows no coverage note.
    let full = summarize(&[Some(1.0), Some(3.0)]).expect("both measured");
    assert!(!full.is_partial());
    assert!((full.avg - 2.0).abs() < 1e-9);

    // Nothing measured is not a zero average -- it is no average.
    assert_eq!(summarize(&[]), None);
    assert_eq!(summarize(&[None, None]), None);
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
    assert!(lane.iter().flatten().all(|d| d.is_finite() && *d >= 0.0));

    let scalar = f.players[idx].dist_to_com.expect("fixture has a dist_to_com") as f64;
    let d = summarize(&lane).expect("the fixture measures some seconds");
    // This fixture's local player and commander are both present for
    // essentially the whole encounter: 138 of its 139 seconds are
    // measured, the odd one out being the last (the fight runs 138333ms,
    // and both tracks end before the 138s tick). Coverage that near-total
    // is exactly why no fixture-driven test can catch a fabricated
    // out-of-overlap value -- clamping one second changes the mean by
    // 0.7% -- so the partial-overlap case is constructed above instead.
    assert_eq!((d.measured_secs, d.total_secs), (138, 139));
    let ratio = d.avg / scalar;
    let mean = d.avg;
    assert!(
        (0.90..=1.10).contains(&ratio),
        "lane mean {mean:.1} vs native dist_to_com {scalar:.1} (ratio {ratio:.3})",
    );
}
