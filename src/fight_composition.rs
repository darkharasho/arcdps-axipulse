//! Compute Squad / Allies / Enemy-team groupings + per-class breakdowns
//! from `FightData`. Pure function; tested on the host.

use std::collections::HashMap;

use crate::fight_data::FightData;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GroupKey {
    Squad,
    Allies,
    /// Keyed by axilog's resolved team NAME (`"red"`/`"green"`/`"blue"`/
    /// `"unknown"`), not by a raw team id -- the native projection never
    /// carries the id. `"unknown"` is a legitimate key: it groups the
    /// enemies whose team the log did not identify, which is different
    /// from having no enemies.
    Enemy(String),
}

#[derive(Debug, Clone)]
pub struct Group {
    pub key: GroupKey,
    pub label: String,
    pub color: [f32; 4],
    pub count: u32,
    /// (spec_name, count) sorted descending by count.
    pub class_counts: Vec<(String, u32)>,
}

/// Our own team's palette colour: Squad wears it directly, Allies wear a
/// dimmed version so the two stay distinguishable while still reading as
/// the same side.
fn home_colors(self_team: &str) -> ([f32; 4], [f32; 4]) {
    let squad = match crate::wvw_teams::team_color(self_team) {
        // No resolved team colour (PvE, or a WvW log with no team
        // data) — the green the card used to hard-code for everyone.
        crate::wvw_teams::TeamColor::Unknown => crate::ui::series::NO_TEAM,
        c => c.rgba(),
    };
    let allies = [squad[0] * 0.62, squad[1] * 0.62, squad[2] * 0.62, squad[3]];
    (squad, allies)
}

pub fn compute(fight: &FightData, self_idx: usize) -> Vec<Group> {
    let self_team = fight.players.get(self_idx).map(|p| p.team.as_str()).unwrap_or("");
    let (squad_color, ally_color) = home_colors(self_team);

    let mut squad_specs: HashMap<String, u32> = HashMap::new();
    let mut ally_specs: HashMap<String, u32> = HashMap::new();
    let mut squad_count = 0u32;
    let mut ally_count = 0u32;

    for p in &fight.players {
        let spec = p.spec_label().to_string();
        if p.in_squad {
            squad_count += 1;
            *squad_specs.entry(spec).or_insert(0) += 1;
        } else if p.team == self_team {
            ally_count += 1;
            *ally_specs.entry(spec).or_insert(0) += 1;
        }
    }

    // Enemies live in `fight.enemies` (`Role::EnemyPlayer` only -- the
    // projection already dropped NPCs). Group by resolved team colour
    // and order by count desc so the larger team gets "T1".
    let mut enemy_team_specs: HashMap<String, HashMap<String, u32>> = HashMap::new();
    for e in &fight.enemies {
        if e.team == self_team { continue; }
        // Spec, then core class, then the "<Spec> pl-1992" display-name
        // prefix -- see `EnemyData::spec_label`. An enemy with none of
        // the three is counted as "Unknown" rather than under "".
        let spec = match e.spec_label() {
            "" => "Unknown".to_string(),
            s => s.to_string(),
        };
        *enemy_team_specs.entry(e.team.clone()).or_default().entry(spec).or_insert(0) += 1;
    }

    let mut enemy_team_totals: Vec<(String, u32)> = enemy_team_specs.iter()
        .map(|(team, specs)| (team.clone(), specs.values().sum()))
        .collect();
    enemy_team_totals.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut groups: Vec<Group> = Vec::new();
    if squad_count > 0 {
        let mut specs: Vec<(String, u32)> = squad_specs.into_iter().collect();
        specs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        groups.push(Group {
            key: GroupKey::Squad,
            label: "Squad".to_string(),
            color: squad_color,
            count: squad_count,
            class_counts: specs,
        });
    }
    if ally_count > 0 {
        let mut specs: Vec<(String, u32)> = ally_specs.into_iter().collect();
        specs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        groups.push(Group {
            key: GroupKey::Allies,
            label: "Allies".to_string(),
            color: ally_color,
            count: ally_count,
            class_counts: specs,
        });
    }
    for (i, (team, count)) in enemy_team_totals.into_iter().enumerate() {
        let mut specs: Vec<(String, u32)> = enemy_team_specs.remove(&team)
            .unwrap_or_default().into_iter().collect();
        specs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        groups.push(Group {
            // The enemy team's own colour now that axilog resolves it --
            // the old rotating red/orange/pink palette existed only
            // because an EI team id carried no colour.
            color: crate::wvw_teams::team_color(&team).rgba(),
            label: format!("Enemy T{}", i + 1),
            key: GroupKey::Enemy(team),
            count,
            class_counts: specs,
        });
    }
    groups
}
