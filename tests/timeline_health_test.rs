use arcdps_axipulse::fight_data::PlayerData;
use arcdps_axipulse::timeline_health::sample_health_per_second;

#[test]
fn samples_step_function_at_each_second() {
    let p = PlayerData {
        health_percents: vec![(0, 100.0), (2500, 80.0), (4000, 60.0)],
        ..PlayerData::default()
    };
    let samples = sample_health_per_second(&p, 6000);
    assert_eq!(samples, vec![100.0, 100.0, 100.0, 80.0, 60.0, 60.0, 60.0]);
}

/// No health states at all means the health pass never saw this entity.
/// That is an ABSENCE, and the lane is empty so the Timeline routes it
/// to `draw_empty_lane` -- exactly as the Heal In lane already does. It
/// must NOT come back as a full-health lane: a squad member who died
/// would then be drawn flat at 100%, indistinguishable from a real
/// measurement.
#[test]
fn empty_health_yields_an_empty_lane_not_a_full_one() {
    let samples = sample_health_per_second(&PlayerData::default(), 3000);
    assert!(
        samples.is_empty(),
        "an unmeasured health series must not be filled in: got {samples:?}",
    );
}

/// The pre-first-sample fill is a different thing and stays. Seconds
/// before the first state carry `states[0]`, which IS this player's
/// earliest measurement -- not a fabricated 100%.
#[test]
fn seconds_before_the_first_sample_carry_that_first_sample() {
    let p = PlayerData {
        health_percents: vec![(3000, 40.0)],
        ..PlayerData::default()
    };
    assert_eq!(sample_health_per_second(&p, 4000), vec![40.0, 40.0, 40.0, 40.0, 40.0]);
}

#[test]
fn zero_duration_returns_empty() {
    let p = PlayerData { health_percents: vec![(0, 100.0)], ..PlayerData::default() };
    assert!(sample_health_per_second(&p, 0).is_empty());
}
