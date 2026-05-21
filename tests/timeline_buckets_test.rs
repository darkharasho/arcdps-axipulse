use arcdps_axipulse::timeline_buckets::{cumulative_to_per_second, extract_damage_dealt, extract_damage_taken};
use arcdps_axipulse::ei_model::EiJson;

#[test]
fn cumulative_to_per_second_takes_first_difference() {
    let cum = vec![0u64, 100, 250, 400, 400, 550];
    let per = cumulative_to_per_second(&cum);
    assert_eq!(per, vec![0, 100, 150, 150, 0, 150]);
}

#[test]
fn cumulative_to_per_second_handles_empty() {
    let per = cumulative_to_per_second(&[]);
    assert!(per.is_empty());
}

#[test]
fn extract_damage_dealt_uses_phase_zero() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":5000,
        "players":[{
            "name":"me","account":":me.1","profession":"Guardian",
            "damage1S":[[0,100,300,300,500,500]]
        }],"targets":[]
    }"#).unwrap();
    let per = extract_damage_dealt(&j.players[0]);
    assert_eq!(per, vec![0, 100, 200, 0, 200, 0]);
}

#[test]
fn extract_damage_taken_uses_phase_zero() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":5000,
        "players":[{
            "name":"me","account":":me.1","profession":"Guardian",
            "damageTaken1S":[[0,50,75,75]]
        }],"targets":[]
    }"#).unwrap();
    let per = extract_damage_taken(&j.players[0]);
    assert_eq!(per, vec![0, 50, 25, 0]);
}

#[test]
fn extract_damage_dealt_returns_empty_when_absent() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":5000,
        "players":[{"name":"x","account":":x.1","profession":"Guardian"}],
        "targets":[]
    }"#).unwrap();
    assert_eq!(extract_damage_dealt(&j.players[0]), Vec::<u64>::new());
}
