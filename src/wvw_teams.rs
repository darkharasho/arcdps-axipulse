//! WvW team colour + per-colour player counts.
//!
//! # The team-id table moved upstream
//!
//! This module used to carry its own red/green/blue team-id table plus a
//! preference for Elite Insights' `wvWMapData` (built from the arcdps
//! CBTS_WVWTEAMS statechange) when the log had one. axilog does exactly
//! that resolution itself -- `axilog_core::wvw::team_color_with` prefers
//! the log's own dynamic team ids and falls back to the same fixed
//! table -- and publishes the ANSWER as `EntityOut::team`, one of
//! `"red"` / `"green"` / `"blue"` / `"unknown"`.
//!
//! Keeping a second copy of that table here would be two tables to drift
//! apart, so this module now only maps the resolved colour NAME onto the
//! plugin's palette. `"unknown"` is preserved as its own colour, never
//! silently folded into one of the three.

use crate::fight_data::FightData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeamColor {
    Red,
    Green,
    Blue,
    Unknown,
}

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

/// Maps axilog's resolved team name onto this palette. Any name outside
/// the three known colours -- including `"unknown"`, the empty string,
/// and any future value -- is `Unknown`, which the UI renders as its own
/// grey segment rather than dropping.
pub fn team_color(team: &str) -> TeamColor {
    match team {
        "red" => TeamColor::Red,
        "green" => TeamColor::Green,
        "blue" => TeamColor::Blue,
        _ => TeamColor::Unknown,
    }
}

/// Everyone in the fight bucketed by team colour: allied players from
/// `players` plus enemy players from `enemies`.
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

    /// Nonzero (color, count) pairs in display order. Shared by the
    /// team bar and the notifier toast so both surfaces always agree
    /// on ordering and (via `TeamColor::rgba`) on colors.
    pub fn segments(&self) -> Vec<(TeamColor, u32)> {
        [
            (TeamColor::Red, self.red),
            (TeamColor::Green, self.green),
            (TeamColor::Blue, self.blue),
            (TeamColor::Unknown, self.unknown),
        ]
        .into_iter()
        .filter(|(_, c)| *c > 0)
        .collect()
    }
}

/// `FightData::enemies` is already `Role::EnemyPlayer` only -- NPCs are
/// dropped by the projection -- so this no longer needs the
/// `enemy_player` / `is_fake` filtering the Elite Insights `targets`
/// list required.
pub fn count_teams(fight: &FightData) -> TeamCounts {
    let mut counts = TeamCounts::default();
    let mut bump = |color: TeamColor| match color {
        TeamColor::Red => counts.red += 1,
        TeamColor::Green => counts.green += 1,
        TeamColor::Blue => counts.blue += 1,
        TeamColor::Unknown => counts.unknown += 1,
    };
    for p in &fight.players {
        bump(team_color(&p.team));
    }
    for e in &fight.enemies {
        bump(team_color(&e.team));
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_the_three_known_colour_names() {
        assert_eq!(team_color("red"), TeamColor::Red);
        assert_eq!(team_color("green"), TeamColor::Green);
        assert_eq!(team_color("blue"), TeamColor::Blue);
    }

    /// axilog's own fallback-of-last-resort. It must stay a distinct
    /// colour: folding it into one of the three would put players on a
    /// team the log never identified.
    #[test]
    fn unknown_is_preserved_not_guessed() {
        assert_eq!(team_color("unknown"), TeamColor::Unknown);
        assert_eq!(team_color(""), TeamColor::Unknown);
        assert_eq!(team_color("Red"), TeamColor::Unknown);
        assert_eq!(team_color("purple"), TeamColor::Unknown);
    }

    #[test]
    fn segments_keep_display_order_and_skip_zero_teams() {
        let c = TeamCounts { red: 3, green: 0, blue: 7, unknown: 1 };
        assert_eq!(
            c.segments(),
            vec![(TeamColor::Red, 3), (TeamColor::Blue, 7), (TeamColor::Unknown, 1)],
        );
        assert_eq!(TeamCounts::default().segments(), vec![]);
    }
}
