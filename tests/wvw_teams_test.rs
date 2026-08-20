//! `count_teams` over the real fixture.
//!
//! Through Task 7, axilog's resolved colour (`EntityOut::team`, which
//! preferred the log's own CBTS_WVWTEAMS ids and fell back to the same
//! fixed table this crate used to carry locally) was additionally proved
//! against an equality oracle that re-inlined the deleted table and
//! checked it against Elite Insights' team ids. That oracle -- and the
//! `ei_model` types it needed -- is deleted by Task 8; the table has
//! moved upstream for good. What remains native-only: a WvW squad is
//! reported as being on exactly one team's colour, and the fixture's
//! enemies span the other two.

mod common;

use arcdps_axipulse::fight_data::FightData;
use arcdps_axipulse::wvw_teams::count_teams;

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

/// A WvW squad fights on exactly one team's colour, and this fixture's
/// enemies are drawn from the other two -- the strongest native-only
/// invariant available once the id-table oracle is gone. A regression
/// that resolved squad members to `Unknown`, or split them across
/// colours, would fail this even though `count_teams` still summed to
/// the right total.
#[test]
fn the_squad_shares_one_colour_and_enemies_span_the_other_two() {
    use arcdps_axipulse::wvw_teams::{team_color, TeamColor};
    use std::collections::HashSet;

    let n = common::native();
    let f = FightData::from_report(&n);

    let squad_colors: HashSet<TeamColor> = f
        .players
        .iter()
        .filter(|p| p.in_squad)
        .map(|p| team_color(&p.team))
        .collect();
    assert_eq!(squad_colors.len(), 1, "squad is split across colours: {squad_colors:?}");
    let squad_color = *squad_colors.iter().next().unwrap();
    assert_ne!(squad_color, TeamColor::Unknown, "squad resolved to Unknown");

    let enemy_colors: HashSet<TeamColor> = f
        .enemies
        .iter()
        .map(|e| team_color(&e.team))
        .collect();
    assert!(!enemy_colors.is_empty(), "fixture has no enemies to check");
    assert!(
        enemy_colors.iter().all(|c| *c != TeamColor::Unknown && *c != squad_color),
        "an enemy resolved to Unknown or to the squad's own colour: {enemy_colors:?}",
    );
}
