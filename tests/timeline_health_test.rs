use arcdps_axipulse::timeline_health::sample_health_per_second;
use arcdps_axipulse::ei_model::EiJson;

#[test]
fn samples_step_function_at_each_second() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":6000,
        "players":[{
            "name":"me","account":":me.1","profession":"Guardian",
            "healthPercents":[[0,100],[2500,80],[4000,60]]
        }],"targets":[]
    }"#).unwrap();
    let samples = sample_health_per_second(&j.players[0], 6000);
    assert_eq!(samples, vec![100.0, 100.0, 100.0, 80.0, 60.0, 60.0, 60.0]);
}

#[test]
fn empty_health_yields_full() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":3000,
        "players":[{"name":"x","account":":x.1","profession":"Guardian"}],
        "targets":[]
    }"#).unwrap();
    let samples = sample_health_per_second(&j.players[0], 3000);
    assert_eq!(samples, vec![100.0, 100.0, 100.0, 100.0]);
}

#[test]
fn zero_duration_returns_empty() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":0,
        "players":[{
            "name":"me","account":":me.1","profession":"Guardian",
            "healthPercents":[[0,100]]
        }],"targets":[]
    }"#).unwrap();
    let samples = sample_health_per_second(&j.players[0], 0);
    assert!(samples.is_empty());
}
