mod common;

use axilog_api::v1::entities::Role;
use axilog_api::v1::envelope::Coverage;

// `Coverage::get` returns the `CoverageState` enum, not a `String`; go
// through `serde_json` (already a dev-dependency) to compare against the
// same snake_case strings the wire format uses. Mirrors the pattern in
// axilog-api's own `parse_report_v1.rs` test.
fn coverage_str(coverage: &Coverage, block: &str) -> Option<String> {
    let value = serde_json::to_value(coverage).expect("coverage serializes");
    value.get(block).and_then(|v| v.as_str()).map(str::to_owned)
}

#[test]
fn fixtures_describe_the_same_fight() {
    let n = common::native();
    let e = common::ei();
    let delta = (n.encounter.duration_ms as i64 - e.duration_ms as i64).abs();
    assert!(delta < 1000, "durations differ by {delta}ms");
}

#[test]
fn rosters_agree() {
    let n = common::native();
    let e = common::ei();
    let mut native_accounts: Vec<&str> = n
        .entities
        .iter()
        .filter(|x| x.role == Role::Squad)
        .filter_map(|x| x.account.as_deref())
        .collect();
    let mut ei_accounts: Vec<&str> = e
        .players
        .iter()
        .filter(|p| !p.not_in_squad)
        .map(|p| p.account.as_str())
        .collect();
    native_accounts.sort_unstable();
    ei_accounts.sort_unstable();
    assert_eq!(native_accounts, ei_accounts);
}

#[test]
fn every_block_this_plugin_reads_is_present() {
    let n = common::native();
    for block in [
        "damage",
        "defenses",
        "cc",
        "boons",
        "support",
        "contribution",
        "healing",
        "rotation",
        "replay",
        "series",
    ] {
        assert_eq!(
            coverage_str(&n.coverage, block).as_deref(),
            Some("present"),
            "block {block} not computed -- PARSE_OPTS has drifted"
        );
    }
}

#[test]
fn local_player_resolves_to_an_entity_id() {
    let n = common::native();
    let id = n.encounter.recorded_by.expect("fixture has a recorder");
    assert!(n.entities.iter().any(|e| e.id == id));
}
