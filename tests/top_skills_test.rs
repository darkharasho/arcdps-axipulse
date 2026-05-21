use arcdps_axipulse::ei_model::EiJson;
use arcdps_axipulse::top_skills::{top_damage, top_down_contribution, SkillEntry};

fn json_with_dist() -> EiJson {
    serde_json::from_str(r#"{
        "fightName":"t","durationMS":1,
        "players":[{
            "name":"me","account":":me.1","profession":"Guardian",
            "totalDamageDist":[[
                {"id":1,"name":"Skill A","totalDamage":500,"downContribution":50},
                {"id":2,"name":"Skill B","totalDamage":1000,"downContribution":10},
                {"id":3,"name":"Skill C","totalDamage":0,"downContribution":0},
                {"id":4,"name":"Skill D","totalDamage":200,"downContribution":80}
            ]]
        }],
        "targets":[]
    }"#).unwrap()
}

#[test]
fn top_damage_sorts_descending_and_filters_zero() {
    let j = json_with_dist();
    let top = top_damage(&j.players[0], 10);
    assert_eq!(top.len(), 3);
    assert_eq!(top[0].name, "Skill B");
    assert_eq!(top[0].damage, 1000);
    assert_eq!(top[1].name, "Skill A");
    assert_eq!(top[2].name, "Skill D");
}

#[test]
fn top_damage_respects_limit() {
    let j = json_with_dist();
    let top = top_damage(&j.players[0], 2);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].name, "Skill B");
    assert_eq!(top[1].name, "Skill A");
}

#[test]
fn top_down_contribution_sorts_and_filters() {
    let j = json_with_dist();
    let top = top_down_contribution(&j.players[0], 10);
    assert_eq!(top.len(), 3);
    assert_eq!(top[0].name, "Skill D");
    assert_eq!(top[1].name, "Skill A");
    assert_eq!(top[2].name, "Skill B");
}

#[test]
fn empty_dist_returns_empty() {
    let j: EiJson = serde_json::from_str(r#"{
        "fightName":"t","durationMS":1,
        "players":[{"name":"x","account":":x.1","profession":"Guardian"}],
        "targets":[]
    }"#).unwrap();
    assert_eq!(top_damage(&j.players[0], 10), Vec::<SkillEntry>::new());
}
