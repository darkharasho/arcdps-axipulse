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

#[test]
fn empty_health_yields_full() {
    let samples = sample_health_per_second(&PlayerData::default(), 3000);
    assert_eq!(samples, vec![100.0, 100.0, 100.0, 100.0]);
}

#[test]
fn zero_duration_returns_empty() {
    let p = PlayerData { health_percents: vec![(0, 100.0)], ..PlayerData::default() };
    assert!(sample_health_per_second(&p, 0).is_empty());
}
