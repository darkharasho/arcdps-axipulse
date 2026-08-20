//! `lerp_position` / `sample_index_at`.
//!
//! Both take `track_start_ms` because position tracks do NOT share a
//! grid origin — see `lerp_position`'s own doc comment for the bug that
//! motivated the parameter.

use arcdps_axipulse::ui::map::{lerp_position, sample_index_at};

const T0: [(f32, f32); 2] = [(10.0, 20.0), (110.0, 220.0)];

#[test]
fn at_zero_returns_first_sample() {
    assert_eq!(lerp_position(&T0, 0, 0, 500), Some((10.0, 20.0)));
}

#[test]
fn at_polling_rate_returns_second_sample() {
    assert_eq!(lerp_position(&T0, 0, 500, 500), Some((110.0, 220.0)));
}

#[test]
fn between_samples_lerps_linearly() {
    let samples = [(0.0, 0.0), (100.0, 200.0)];
    assert_eq!(lerp_position(&samples, 0, 250, 500), Some((50.0, 100.0)));
}

#[test]
fn past_last_sample_clamps_to_last() {
    assert_eq!(lerp_position(&T0, 0, 5000, 500), Some((110.0, 220.0)));
}

#[test]
fn empty_samples_returns_none() {
    assert_eq!(lerp_position(&[], 0, 0, 500), None);
}

#[test]
fn single_sample_returns_it() {
    assert_eq!(lerp_position(&[(7.0, 8.0)], 0, 1234, 500), Some((7.0, 8.0)));
}

#[test]
fn zero_polling_rate_returns_first_sample() {
    let samples = [(1.0, 2.0), (3.0, 4.0)];
    assert_eq!(lerp_position(&samples, 0, 100, 0), Some((1.0, 2.0)));
}

/// **The regression the `track_start_ms` parameter exists for.** A track
/// that starts at t=1000 must read its OWN sample 0 at t=1000, not at
/// t=0. The old signature had no way to express this and divided
/// `t_ms` by the polling rate directly, which is only correct for a
/// track anchored at zero.
#[test]
fn a_late_starting_track_is_read_from_its_own_origin() {
    let samples = [(10.0, 20.0), (110.0, 220.0)];
    // Before the track begins: hold at the first sample rather than
    // index negatively or wrap.
    assert_eq!(lerp_position(&samples, 1000, 0, 500), Some((10.0, 20.0)));
    assert_eq!(lerp_position(&samples, 1000, 1000, 500), Some((10.0, 20.0)));
    assert_eq!(lerp_position(&samples, 1000, 1250, 500), Some((60.0, 120.0)));
    assert_eq!(lerp_position(&samples, 1000, 1500, 500), Some((110.0, 220.0)));
}

#[test]
fn sample_index_is_relative_to_the_tracks_own_start() {
    assert_eq!(sample_index_at(5, 1000, 0, 500), 0);
    assert_eq!(sample_index_at(5, 1000, 1000, 500), 0);
    assert_eq!(sample_index_at(5, 1000, 2000, 500), 2);
    // Clamped to the last real sample.
    assert_eq!(sample_index_at(5, 1000, 99_000, 500), 4);
    assert_eq!(sample_index_at(0, 0, 1000, 500), 0);
    assert_eq!(sample_index_at(5, 0, 1000, 0), 0);
}
