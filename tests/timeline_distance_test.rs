use arcdps_axipulse::timeline_distance::distance_to_commander_per_second;
use arcdps_axipulse::ei_model::EiJson;

#[test]
fn computes_distance_in_inches_using_inch_to_pixel() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":3000,
        "combatReplayMetaData":{"inchToPixel":1.0,"pollingRate":1000},
        "players":[
            {"name":"me","account":":me.1","profession":"Guardian",
             "combatReplayData":{"positions":[[0.0,0.0],[3.0,4.0],[6.0,8.0]]}},
            {"name":"cmdr","account":":cmdr.1","profession":"Warrior",
             "hasCommanderTag":true,
             "combatReplayData":{"positions":[[0.0,0.0],[0.0,0.0],[0.0,0.0]]}}
        ],
        "targets":[]
    }"#).unwrap();
    let samples = distance_to_commander_per_second(&j, 0, 3000);
    assert_eq!(samples.len(), 4);
    assert!((samples[0] - 0.0).abs() < 0.01);
    assert!((samples[1] - 5.0).abs() < 0.01);
    assert!((samples[2] - 10.0).abs() < 0.01);
}

#[test]
fn returns_empty_when_no_commander() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":2000,
        "combatReplayMetaData":{"inchToPixel":1.0,"pollingRate":1000},
        "players":[
            {"name":"me","account":":me.1","profession":"Guardian",
             "combatReplayData":{"positions":[[0.0,0.0],[1.0,0.0]]}}
        ],
        "targets":[]
    }"#).unwrap();
    let samples = distance_to_commander_per_second(&j, 0, 2000);
    assert!(samples.is_empty(), "no commander → no distance lane");
}

#[test]
fn returns_empty_when_self_lacks_replay_data() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":2000,
        "combatReplayMetaData":{"inchToPixel":1.0,"pollingRate":1000},
        "players":[
            {"name":"me","account":":me.1","profession":"Guardian"},
            {"name":"c","account":":c.1","profession":"Warrior","hasCommanderTag":true,
             "combatReplayData":{"positions":[[0.0,0.0],[1.0,0.0]]}}
        ],
        "targets":[]
    }"#).unwrap();
    let samples = distance_to_commander_per_second(&j, 0, 2000);
    assert!(samples.is_empty());
}
