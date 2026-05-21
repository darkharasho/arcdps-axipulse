use arcdps_axipulse::ei_model::EiJson;
use arcdps_axipulse::pulse_metrics::*;

fn load_fixture() -> EiJson {
    let bytes = std::fs::read("fixtures/sample-fight.json").expect("fixture");
    serde_json::from_slice(&bytes).expect("parse")
}

#[test]
fn derives_for_first_player() {
    let json = load_fixture();
    let p = &json.players[0];

    let dmg = damage(p);
    let dps = dps_value(p);
    assert!(dps == 0 || (dmg * 1000) / json.duration_ms.max(1).max(1000) > 0,
        "dps/damage relationship sane");
    let _ = cleanses(p);
    let _ = strips(p);
    let _ = dist_to_tag(p);
    let _ = damage_taken(p);
    let _ = deaths(p);
    let _ = downs(p);
    let _ = down_contribution(p);
}

#[test]
fn down_contribution_falls_back_to_dist_sum() {
    let json: EiJson = serde_json::from_str(r#"{
        "fightName": "test",
        "durationMS": 1000,
        "players": [{
            "name": "X", "account": ":x", "profession": "Guardian",
            "statsAll": [{ "downContribution": 0 }],
            "totalDamageDist": [[{ "id": 1, "name": "skill", "totalDamage": 100, "downContribution": 42 }]]
        }],
        "targets": []
    }"#).unwrap();
    assert_eq!(down_contribution(&json.players[0]), 42);
}
