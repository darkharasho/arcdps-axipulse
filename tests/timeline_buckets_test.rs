use arcdps_axipulse::fight_data::PlayerData;
use arcdps_axipulse::timeline_buckets::{
    cumulative_to_per_second, extract_damage_dealt, extract_damage_taken,
};

#[test]
fn cumulative_to_per_second_takes_first_difference() {
    let cum = vec![0u64, 100, 250, 400, 400, 550];
    let per = cumulative_to_per_second(&cum);
    assert_eq!(per, vec![0, 100, 150, 150, 0, 150]);
}

#[test]
fn cumulative_to_per_second_handles_empty() {
    assert!(cumulative_to_per_second(&[]).is_empty());
}

/// The native series is already the one phase this plugin cares about,
/// so there is no `[0]` phase index to pick any more -- the whole vec is
/// the series.
#[test]
fn extract_damage_dealt_differences_the_series() {
    let p = PlayerData {
        damage_1s: vec![0, 100, 300, 300, 500, 500],
        ..PlayerData::default()
    };
    assert_eq!(extract_damage_dealt(&p), vec![0, 100, 200, 0, 200, 0]);
}

#[test]
fn extract_damage_taken_differences_the_series() {
    let p = PlayerData { damage_taken_1s: vec![0, 50, 75, 75], ..PlayerData::default() };
    assert_eq!(extract_damage_taken(&p), vec![0, 50, 25, 0]);
}

#[test]
fn extract_damage_dealt_returns_empty_when_absent() {
    assert_eq!(extract_damage_dealt(&PlayerData::default()), Vec::<u64>::new());
}
