use arcdps_axipulse::ei_model::EiJson;
use arcdps_axipulse::timeline_boons::{
    active_segments, offensive_boons, defensive_boons, Segment,
};

#[test]
fn active_segments_finds_runs_of_positive_values() {
    let states = vec![
        vec![0.0,    0.0],
        vec![500.0,  3.0],
        vec![2000.0, 0.0],
        vec![3000.0, 1.0],
    ];
    let segs = active_segments(&states, 5000);
    assert_eq!(segs, vec![
        Segment { start_ms: 500, end_ms: 2000 },
        Segment { start_ms: 3000, end_ms: 5000 },
    ]);
}

#[test]
fn active_segments_no_states_yields_empty() {
    assert!(active_segments(&[], 5000).is_empty());
}

#[test]
fn active_segments_starting_active_at_zero() {
    let states = vec![vec![0.0, 5.0], vec![1500.0, 0.0]];
    let segs = active_segments(&states, 3000);
    assert_eq!(segs, vec![Segment { start_ms: 0, end_ms: 1500 }]);
}

#[test]
fn offensive_boons_returns_might_fury_quickness_alacrity() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":2000,
        "players":[{
            "name":"me","account":":me.1","profession":"Guardian",
            "buffUptimes":[
                {"id":740,"buffData":[],"states":[[0,5],[1000,0]]},
                {"id":725,"buffData":[],"states":[[0,1],[1500,0]]},
                {"id":1187,"buffData":[],"states":[[500,1],[1500,0]]},
                {"id":30328,"buffData":[],"states":[]},
                {"id":999,"buffData":[],"states":[[0,1]]}
            ]
        }],"targets":[]
    }"#).unwrap();
    let series = offensive_boons(&j.players[0], 2000);
    assert_eq!(series.len(), 4);
    assert_eq!(series[0].id, 740);
    assert_eq!(series[0].name, "Might");
    assert_eq!(series[0].segments, vec![Segment { start_ms: 0, end_ms: 1000 }]);
    assert_eq!(series[3].id, 30328);
    assert_eq!(series[3].name, "Alacrity");
    assert!(series[3].segments.is_empty());
}

#[test]
fn defensive_boons_returns_prot_resistance_stability_aegis() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":2000,
        "players":[{
            "name":"me","account":":me.1","profession":"Guardian",
            "buffUptimes":[
                {"id":717,"buffData":[],"states":[[0,1],[800,0]]},
                {"id":1122,"buffData":[],"states":[[100,2],[500,0]]}
            ]
        }],"targets":[]
    }"#).unwrap();
    let series = defensive_boons(&j.players[0], 2000);
    assert_eq!(series.len(), 4);
    assert_eq!(series[0].id, 717);
    assert_eq!(series[0].name, "Protection");
    assert_eq!(series[0].segments, vec![Segment { start_ms: 0, end_ms: 800 }]);
    assert_eq!(series[2].id, 1122);
    assert_eq!(series[2].name, "Stability");
}
