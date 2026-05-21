//! Compute Squad / Allies / Enemy-team groupings + per-class breakdowns
//! from `EiJson`. Pure function; tested on the host.

use std::collections::HashMap;

use crate::ei_model::EiJson;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GroupKey {
    Squad,
    Allies,
    Enemy(i64),
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

pub fn compute(json: &EiJson, self_idx: usize) -> Vec<Group> {
    let self_team = json.players.get(self_idx).and_then(|p| p.team_id);

    let mut squad_specs: HashMap<String, u32> = HashMap::new();
    let mut ally_specs: HashMap<String, u32> = HashMap::new();
    let mut squad_count = 0u32;
    let mut ally_count = 0u32;

    for p in &json.players {
        let spec = p.elite_spec.clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| p.profession.clone());
        if !p.not_in_squad {
            squad_count += 1;
            *squad_specs.entry(spec).or_insert(0) += 1;
        } else if p.team_id == self_team {
            ally_count += 1;
            *ally_specs.entry(spec).or_insert(0) += 1;
        }
    }

    // Enemies live in `targets` with `enemy_player == true`. Each enemy
    // team has its own `team_id`; we group by that and order by count
    // desc so the larger team gets "T1".
    let mut enemy_team_specs: HashMap<i64, HashMap<String, u32>> = HashMap::new();
    for t in &json.targets {
        if !t.enemy_player { continue; }
        let Some(tid) = t.team_id else { continue; };
        if tid == 0 { continue; }
        if Some(tid) == self_team { continue; }
        // EI usually leaves enemy `profession` empty. The target's
        // display name is shaped like "<Spec> <random>" (e.g.
        // "Tempest pl-1992"), so the first token is a reasonable
        // best-effort spec label.
        let spec = t.profession.clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                t.name.split_whitespace().next().unwrap_or("Unknown").to_string()
            });
        *enemy_team_specs.entry(tid).or_default().entry(spec).or_insert(0) += 1;
    }

    let mut enemy_team_totals: Vec<(i64, u32)> = enemy_team_specs.iter()
        .map(|(tid, specs)| (*tid, specs.values().sum()))
        .collect();
    enemy_team_totals.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut groups: Vec<Group> = Vec::new();
    if squad_count > 0 {
        let mut specs: Vec<(String, u32)> = squad_specs.into_iter().collect();
        specs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        groups.push(Group {
            key: GroupKey::Squad,
            label: "Squad".to_string(),
            color: [0.29, 0.86, 0.50, 1.0],
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
            color: [0.32, 0.78, 0.92, 1.0],
            count: ally_count,
            class_counts: specs,
        });
    }
    let enemy_palette = [
        [0.95, 0.38, 0.38, 1.0],
        [0.95, 0.55, 0.20, 1.0],
        [0.86, 0.30, 0.55, 1.0],
    ];
    for (i, (tid, count)) in enemy_team_totals.into_iter().enumerate() {
        let mut specs: Vec<(String, u32)> = enemy_team_specs.remove(&tid)
            .unwrap_or_default().into_iter().collect();
        specs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        groups.push(Group {
            key: GroupKey::Enemy(tid),
            label: format!("Enemy T{}", i + 1),
            color: enemy_palette[i.min(enemy_palette.len() - 1)],
            count,
            class_counts: specs,
        });
    }
    groups
}
