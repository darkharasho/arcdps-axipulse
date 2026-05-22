use arcdps_axipulse::ei_model::EiJson;

const REPLAY_JSON: &str = r#"{
    "fightName": "Detailed WvW - Blue Alpine Borderlands",
    "durationMS": 60000,
    "success": false,
    "players": [{
        "name": "TestPlayer",
        "account": ":TestAcc",
        "profession": "Firebrand",
        "group": 1,
        "weapons": [],
        "weaponSets": [],
        "combatReplayData": {
            "positions": [[10.0, 20.0], [11.0, 21.0]],
            "dead": [[40000.0, 60000.0]],
            "down": [[30000.0, 40000.0]]
        },
        "rotation": [
            {
                "id": 12345,
                "skills": [
                    { "castTime": 1000, "duration": 500 },
                    { "castTime": 5000, "duration": 800 }
                ]
            }
        ]
    }],
    "targets": [],
    "skillMap": {},
    "buffMap": {}
}"#;

#[test]
fn parses_dead_down_and_rotation() {
    let j: EiJson = serde_json::from_str(REPLAY_JSON).expect("EI JSON parses");
    let p = &j.players[0];
    let rd = p.combat_replay_data.as_ref().expect("replay data present");
    assert_eq!(rd.dead, vec![vec![40000.0, 60000.0]]);
    assert_eq!(rd.down, vec![vec![30000.0, 40000.0]]);
    assert_eq!(p.rotation.len(), 1);
    assert_eq!(p.rotation[0].id, 12345);
    assert_eq!(p.rotation[0].skills.len(), 2);
    assert_eq!(p.rotation[0].skills[0].cast_time, 1000);
    assert_eq!(p.rotation[0].skills[0].duration, 500);
}

#[test]
fn replay_data_dead_down_default_empty_when_absent() {
    const NO_RANGES: &str = r#"{
        "fightName":"X","durationMS":1000,"success":false,
        "players":[{
            "name":"P","account":":A","profession":"X","group":1,
            "weapons":[],"weaponSets":[],
            "combatReplayData":{"positions":[[0,0]]}
        }],
        "targets":[],"skillMap":{},"buffMap":{}
    }"#;
    let j: EiJson = serde_json::from_str(NO_RANGES).unwrap();
    let rd = j.players[0].combat_replay_data.as_ref().unwrap();
    assert!(rd.dead.is_empty());
    assert!(rd.down.is_empty());
    assert!(j.players[0].rotation.is_empty());
}
