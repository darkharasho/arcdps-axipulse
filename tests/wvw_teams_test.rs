//! `count_teams` over the real fixture, with an equality oracle against
//! the team-id table this crate used to carry.
//!
//! The table moved upstream: axilog resolves a WvW team id to a colour
//! itself (preferring the log's own CBTS_WVWTEAMS ids, falling back to
//! the same fixed table) and publishes the answer as `EntityOut::team`.
//! This test re-inlines the deleted table ONE more time to prove the two
//! resolutions agree on real data before the local copy stays gone.

mod common;

use arcdps_axipulse::fight_data::FightData;
use arcdps_axipulse::wvw_teams::{count_teams, TeamColor};

const RED_TEAM_IDS: &[i64] = &[697, 705, 706, 707, 882, 885, 886, 2520, 2543];
const GREEN_TEAM_IDS: &[i64] = &[39, 2739, 2741, 2752, 2763, 2767];
const BLUE_TEAM_IDS: &[i64] = &[432, 433, 1277, 1282, 1989];

fn ei_team_color(
    team_id: Option<i64>,
    map: Option<&arcdps_axipulse::ei_model::WvwMapData>,
) -> TeamColor {
    let Some(tid) = team_id.filter(|t| *t > 0) else { return TeamColor::Unknown };
    if let Some(m) = map {
        if m.red_team_id > 0 && tid == m.red_team_id { return TeamColor::Red; }
        if m.green_team_id > 0 && tid == m.green_team_id { return TeamColor::Green; }
        if m.blue_team_id > 0 && tid == m.blue_team_id { return TeamColor::Blue; }
    }
    if RED_TEAM_IDS.contains(&tid) { return TeamColor::Red; }
    if GREEN_TEAM_IDS.contains(&tid) { return TeamColor::Green; }
    if BLUE_TEAM_IDS.contains(&tid) { return TeamColor::Blue; }
    TeamColor::Unknown
}

#[test]
fn counts_every_player_and_enemy_exactly_once() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let c = count_teams(&f);
    assert_eq!(
        c.total() as usize,
        f.players.len() + f.enemies.len(),
        "a player was double-counted or dropped",
    );
    assert!(c.total() > 0);
    assert!(c.segments().len() >= 2, "a WvW fight with one team is suspicious");
}

/// **Equality oracle.** Every squad member's colour, as axilog resolved
/// it, must match what the deleted local team-id table would have said
/// for the same player in the Elite Insights baseline.
#[test]
fn native_team_colours_match_the_deleted_id_tables_verdict() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let e = common::ei();
    let map = e.wvw_map_data.as_ref();

    let mut checked = 0;
    for p in f.players.iter().filter(|p| p.in_squad) {
        let ep = e
            .players
            .iter()
            .find(|x| x.account == p.account)
            .expect("squad member appears in the EI baseline");
        assert_eq!(
            arcdps_axipulse::wvw_teams::team_color(&p.team),
            ei_team_color(ep.team_id, map),
            "{} team: native {:?} vs the id table's verdict for id {:?}",
            p.account,
            p.team,
            ep.team_id,
        );
        checked += 1;
    }
    assert!(checked > 0, "no squad member compared -- the check above is vacuous");
}
