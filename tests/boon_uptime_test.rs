use arcdps_axipulse::ei_model::EiJson;
use arcdps_axipulse::boon_uptime::{collect_uptimes, boon_name, BoonStacking, BoonUptime};

#[test]
fn boon_name_returns_known_names() {
    assert_eq!(boon_name(740), Some("Might"));
    assert_eq!(boon_name(725), Some("Fury"));
    assert_eq!(boon_name(1187), Some("Quickness"));
    assert_eq!(boon_name(30328), Some("Alacrity"));
    assert_eq!(boon_name(717), Some("Protection"));
    assert_eq!(boon_name(1122), Some("Stability"));
    assert_eq!(boon_name(743), Some("Aegis"));
    assert_eq!(boon_name(999_999), None);
}

#[test]
fn boon_name_classifies_stacking() {
    use arcdps_axipulse::boon_uptime::boon_stacking;
    assert_eq!(boon_stacking(740), BoonStacking::Intensity);
    assert_eq!(boon_stacking(1122), BoonStacking::Intensity);
    assert_eq!(boon_stacking(725), BoonStacking::Duration);
    assert_eq!(boon_stacking(717), BoonStacking::Duration);
    assert_eq!(boon_stacking(1187), BoonStacking::Duration);
    assert_eq!(boon_stacking(30328), BoonStacking::Duration);
    assert_eq!(boon_stacking(743), BoonStacking::Duration);
}

#[test]
fn collect_uptimes_returns_known_boons_in_canonical_order() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":1,
        "players":[{
            "name":"me","account":":me.1","profession":"Guardian",
            "buffUptimes":[
                {"id":725,"buffData":[{"uptime":85.5}]},
                {"id":740,"buffData":[{"uptime":18.3}]},
                {"id":999999,"buffData":[{"uptime":50.0}]},
                {"id":1187,"buffData":[{"uptime":42.1}]}
            ]
        }],
        "targets":[]
    }"#).unwrap();
    let ups = collect_uptimes(&j.players[0]);
    assert_eq!(ups.len(), 3);
    assert_eq!(ups[0], BoonUptime { id: 740, name: "Might", uptime: 18.3, stacking: BoonStacking::Intensity });
    assert_eq!(ups[1], BoonUptime { id: 725, name: "Fury", uptime: 85.5, stacking: BoonStacking::Duration });
    assert_eq!(ups[2], BoonUptime { id: 1187, name: "Quickness", uptime: 42.1, stacking: BoonStacking::Duration });
}

#[test]
fn missing_buff_data_yields_zero_uptime() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":1,
        "players":[{
            "name":"me","account":":me.1","profession":"Guardian",
            "buffUptimes":[{"id":740,"buffData":[]}]
        }],
        "targets":[]
    }"#).unwrap();
    let ups = collect_uptimes(&j.players[0]);
    assert_eq!(ups[0].uptime, 0.0);
}
