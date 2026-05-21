use arcdps_axipulse::ei_model::EiJson;
use arcdps_axipulse::squad_rank::{rank_in_squad, RankMetric};

fn json_with(values: &[(&str, bool, u64)]) -> EiJson {
    let mut players = String::new();
    for (i, (acc, not_in_squad, dmg)) in values.iter().enumerate() {
        if i > 0 { players.push(','); }
        players.push_str(&format!(
            r#"{{"name":"p{i}","account":"{acc}","profession":"Guardian",
              "notInSquad":{not_in_squad},"dpsAll":[{{"damage":{dmg},"dps":0}}]}}"#
        ));
    }
    let s = format!(r#"{{"fightName":"t","durationMS":1,"players":[{players}],"targets":[]}}"#);
    serde_json::from_str(&s).expect("parse")
}

#[test]
fn ranks_only_among_squad_members() {
    let j = json_with(&[
        (":InSquadLow.1",  false, 100),
        (":InSquadHigh.2", false, 500),
        (":NonSquadTop.3", true, 9999),
        (":InSquadMid.4",  false, 300),
    ]);
    assert_eq!(rank_in_squad(&j, 0, RankMetric::Damage), Some(3));
    assert_eq!(rank_in_squad(&j, 1, RankMetric::Damage), Some(1));
    assert_eq!(rank_in_squad(&j, 2, RankMetric::Damage), None);
    assert_eq!(rank_in_squad(&j, 3, RankMetric::Damage), Some(2));
}

#[test]
fn out_of_range_returns_none() {
    let j = json_with(&[(":A.1", false, 100)]);
    assert_eq!(rank_in_squad(&j, 99, RankMetric::Damage), None);
}

#[test]
fn solo_squad_ranks_first() {
    let j = json_with(&[(":Solo.1", false, 100)]);
    assert_eq!(rank_in_squad(&j, 0, RankMetric::Damage), Some(1));
}
