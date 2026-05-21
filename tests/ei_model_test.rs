use arcdps_axipulse::ei_model::EiJson;

#[test]
fn deserialises_sample_fight() {
    let bytes = std::fs::read("fixtures/sample-fight.json")
        .expect("fixtures/sample-fight.json not present");
    let parsed: EiJson = serde_json::from_slice(&bytes).expect("EiJson deserialise");
    assert!(parsed.duration_ms > 0, "duration_ms should be positive");
    assert!(!parsed.players.is_empty(), "fight should have players");
    let p0 = &parsed.players[0];
    assert!(!p0.profession.is_empty(), "first player has profession");
    let _ = p0.dps_all.get(0).map(|d| d.damage).unwrap_or(0);
}
