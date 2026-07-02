//! WvW team → color mapping + per-color player counts, ported from
//! axibridge's `src/shared/wvwTeams.ts`.
//!
//! Preferred source: EI's authoritative `wvWMapData` (built from the
//! arcdps CBTS_WVWTEAMS statechange event), which gives the exact
//! red/green/blue team ids for the log. Older logs (pre-~May 2026)
//! lack the event, so we fall back to the well-known fixed team-id
//! table below.
//!
//! Fixed table reconciled from two community tools that predate the
//! event:
//!   - Drevarr/EVTC_parser/gw2_data.py
//!   - Drevarr/GW2_EI_log_combiner/config.py

use crate::ei_model::{EiJson, WvwMapData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeamColor {
    Red,
    Green,
    Blue,
    Unknown,
}

const RED_TEAM_IDS: &[i64] = &[697, 705, 706, 707, 882, 885, 886, 2520, 2543];
const GREEN_TEAM_IDS: &[i64] = &[39, 2739, 2741, 2752, 2763, 2767];
const BLUE_TEAM_IDS: &[i64] = &[432, 433, 1277, 1282, 1989];

impl TeamColor {
    pub fn label(self) -> &'static str {
        match self {
            TeamColor::Red => "Red",
            TeamColor::Green => "Green",
            TeamColor::Blue => "Blue",
            TeamColor::Unknown => "Unknown",
        }
    }

    /// axibridge hex palette (#f87171 / #4ade80 / #60a5fa / #9ca3af)
    /// as normalized RGBA.
    pub fn rgba(self) -> [f32; 4] {
        match self {
            TeamColor::Red => [0.973, 0.443, 0.443, 1.0],
            TeamColor::Green => [0.290, 0.871, 0.502, 1.0],
            TeamColor::Blue => [0.376, 0.647, 0.980, 1.0],
            TeamColor::Unknown => [0.612, 0.639, 0.686, 1.0],
        }
    }
}

/// Resolve a team id to its color. Prefers the authoritative per-log
/// map, then the fixed id-table, else Unknown.
pub fn team_color(team_id: Option<i64>, map: Option<&WvwMapData>) -> TeamColor {
    let Some(tid) = team_id.filter(|t| *t > 0) else {
        return TeamColor::Unknown;
    };
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

/// Everyone in the fight bucketed by team color: allied players from
/// `players` plus enemy players from `targets`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TeamCounts {
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub unknown: u32,
}

impl TeamCounts {
    pub fn total(&self) -> u32 {
        self.red + self.green + self.blue + self.unknown
    }
}

pub fn count_teams(json: &EiJson) -> TeamCounts {
    let map = json.wvw_map_data.as_ref();
    let mut counts = TeamCounts::default();
    let mut bump = |color: TeamColor| match color {
        TeamColor::Red => counts.red += 1,
        TeamColor::Green => counts.green += 1,
        TeamColor::Blue => counts.blue += 1,
        TeamColor::Unknown => counts.unknown += 1,
    };
    for p in &json.players {
        bump(team_color(p.team_id, map));
    }
    for t in &json.targets {
        if !t.enemy_player || t.is_fake { continue; }
        bump(team_color(t.team_id, map));
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(red: i64, green: i64, blue: i64) -> WvwMapData {
        WvwMapData { red_team_id: red, green_team_id: green, blue_team_id: blue }
    }

    #[test]
    fn authoritative_map_wins_over_fixed_table() {
        // 39 is Green in the fixed table, but the log says it's red.
        let m = map(39, 0, 0);
        assert_eq!(team_color(Some(39), Some(&m)), TeamColor::Red);
    }

    #[test]
    fn falls_back_to_fixed_table() {
        assert_eq!(team_color(Some(705), None), TeamColor::Red);
        assert_eq!(team_color(Some(39), None), TeamColor::Green);
        assert_eq!(team_color(Some(1277), None), TeamColor::Blue);
        assert_eq!(team_color(Some(123456), None), TeamColor::Unknown);
    }

    #[test]
    fn missing_or_nonpositive_ids_are_unknown() {
        assert_eq!(team_color(None, None), TeamColor::Unknown);
        assert_eq!(team_color(Some(0), None), TeamColor::Unknown);
        assert_eq!(team_color(Some(-5), Some(&map(705, 39, 432))), TeamColor::Unknown);
    }

    #[test]
    fn zero_slot_in_map_does_not_match() {
        // red slot is 0 (team absent); a 0 team id must not become Red.
        let m = map(0, 39, 432);
        assert_eq!(team_color(Some(0), Some(&m)), TeamColor::Unknown);
    }

    #[test]
    fn counts_players_and_enemy_players() {
        let json: EiJson = serde_json::from_str(
            r#"{
                "fightName": "Detailed WvW - Eternal Battlegrounds",
                "durationMS": 1000,
                "wvWMapData": { "redTeamID": 100, "greenTeamID": 200, "blueTeamID": 300 },
                "players": [
                    {"name": "a", "account": "a.1", "profession": "Guardian", "teamID": 200, "wasted": {}},
                    {"name": "b", "account": "b.1", "profession": "Necromancer", "teamID": 200, "wasted": {}}
                ],
                "targets": [
                    {"name": "Tempest pl-1", "enemyPlayer": true, "teamID": 100},
                    {"name": "Scrapper pl-2", "enemyPlayer": true, "teamID": 300},
                    {"name": "Dummy", "enemyPlayer": true, "isFake": true, "teamID": 100},
                    {"name": "Keep Lord", "enemyPlayer": false, "teamID": 100},
                    {"name": "Weaver pl-3", "enemyPlayer": true}
                ]
            }"#,
        ).unwrap();
        let c = count_teams(&json);
        assert_eq!(c, TeamCounts { red: 1, green: 2, blue: 1, unknown: 1 });
        assert_eq!(c.total(), 5);
    }
}
