use arcdps_axipulse::ei_model::EiJson;
use arcdps_axipulse::self_identify::find_self_index;

fn json_with_recorded(recorded: Option<&str>, players: &[(&str, bool)]) -> EiJson {
    let mut accounts = String::new();
    for (i, (acc, cmdr)) in players.iter().enumerate() {
        if i > 0 {
            accounts.push(',');
        }
        accounts.push_str(&format!(
            r#"{{"name":"p{i}","account":"{acc}","profession":"Guardian","hasCommanderTag":{cmdr}}}"#
        ));
    }
    let rec = recorded
        .map(|s| format!(r#","recordedAccountBy":"{s}""#))
        .unwrap_or_default();
    let s = format!(
        r#"{{"fightName":"t","durationMS":1{rec},"players":[{accounts}],"targets":[]}}"#
    );
    serde_json::from_str(&s).expect("parse")
}

#[test]
fn matches_recorded_account_by_when_present() {
    let j = json_with_recorded(Some(":Alice.1234"), &[
        (":Bob.5555", false),
        (":Alice.1234", false),
        (":Carol.9999", true),
    ]);
    assert_eq!(find_self_index(&j), Some(1));
}

#[test]
fn falls_back_to_commander_tag_when_recorded_missing() {
    let j = json_with_recorded(None, &[
        (":Bob.5555", false),
        (":Alice.1234", false),
        (":Carol.9999", true),
    ]);
    assert_eq!(find_self_index(&j), Some(2));
}

#[test]
fn falls_back_to_first_player_when_no_signals() {
    let j = json_with_recorded(None, &[
        (":Bob.5555", false),
        (":Alice.1234", false),
    ]);
    assert_eq!(find_self_index(&j), Some(0));
}

#[test]
fn returns_none_for_empty_roster() {
    let j: EiJson =
        serde_json::from_str(r#"{"fightName":"t","durationMS":1,"players":[],"targets":[]}"#)
            .unwrap();
    assert_eq!(find_self_index(&j), None);
}
