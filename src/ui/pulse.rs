#![cfg(windows)]
//! Pulse tab content — five subviews (Overview/Damage/Support/Defense/
//! Boons) rendered inside the unified AxiPulse window. The outer
//! window + fight picker live in `ui::main`.

use std::sync::Mutex;

use arcdps::imgui::{StyleColor, Ui};
use once_cell::sync::Lazy;

use crate::ei_model::EiJson;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Subview { Overview, Damage, Support, Defense, Boons }

static SUBVIEW: Lazy<Mutex<Subview>> = Lazy::new(|| Mutex::new(Subview::Overview));

// --- palette -------------------------------------------------------------

const BG_CARD:        [f32; 4] = [0.085, 0.10, 0.13, 0.95];
const BG_CARD_BORDER: [f32; 4] = [1.0, 1.0, 1.0, 0.06];
const TEXT_PRIMARY:   [f32; 4] = [0.97, 0.97, 1.00, 1.0];
const TEXT_SECONDARY: [f32; 4] = [0.78, 0.78, 0.85, 1.0];
const TEXT_MUTED:     [f32; 4] = [0.52, 0.54, 0.62, 1.0];
const ACCENT_DAMAGE:  [f32; 4] = [0.95, 0.38, 0.38, 1.0];
const ACCENT_DOWN:    [f32; 4] = [0.97, 0.55, 0.42, 1.0];
const ACCENT_SUPPORT: [f32; 4] = [0.40, 0.85, 0.65, 1.0];
const ACCENT_CLEANSE: [f32; 4] = [0.32, 0.78, 0.92, 1.0];
const ACCENT_DEFEND:  [f32; 4] = [0.95, 0.62, 0.30, 1.0];
const ACCENT_SUCCESS: [f32; 4] = [0.40, 0.85, 0.55, 1.0];
const ACCENT_DANGER:  [f32; 4] = [0.95, 0.40, 0.40, 1.0];
const ACCENT_NEUTRAL: [f32; 4] = [0.55, 0.62, 0.78, 1.0];

const HERO_H: f32 = 76.0;
const CARD_H: f32 = 58.0;
const GAP:    f32 = 8.0;

// --- entry point ---------------------------------------------------------

/// Render the Pulse tab contents (no window — caller owns that).
pub fn render_content(ui: &Ui, json: &EiJson, idx: usize) {
    render_tab_strip(ui);
    ui.dummy([0.0, 4.0]);

    let subview = SUBVIEW.lock().ok().map(|g| *g).unwrap_or(Subview::Overview);
    match subview {
        Subview::Overview => render_overview(ui, json, idx),
        Subview::Damage   => render_damage(ui, json, idx),
        Subview::Support  => render_support(ui, json, idx),
        Subview::Defense  => render_defense(ui, json, idx),
        Subview::Boons    => render_boons(ui, json, idx),
    }
}

fn render_tab_strip(ui: &Ui) {
    let mut current = SUBVIEW.lock().ok().map(|g| *g).unwrap_or(Subview::Overview);
    let labels = [
        ("Overview", Subview::Overview),
        ("Damage",   Subview::Damage),
        ("Support",  Subview::Support),
        ("Defense",  Subview::Defense),
        ("Boons",    Subview::Boons),
    ];
    for (i, (label, sv)) in labels.iter().enumerate() {
        let selected = current == *sv;
        let tokens = if selected {
            vec![
                ui.push_style_color(StyleColor::Button,        [0.18, 0.22, 0.30, 1.0]),
                ui.push_style_color(StyleColor::ButtonHovered, [0.22, 0.26, 0.34, 1.0]),
                ui.push_style_color(StyleColor::ButtonActive,  [0.24, 0.28, 0.36, 1.0]),
                ui.push_style_color(StyleColor::Text,          TEXT_PRIMARY),
            ]
        } else {
            vec![
                ui.push_style_color(StyleColor::Text, TEXT_SECONDARY),
            ]
        };
        if ui.button(label) { current = *sv; }
        for t in tokens { t.pop(); }
        if i + 1 < labels.len() { ui.same_line(); }
    }
    if let Ok(mut g) = SUBVIEW.lock() { *g = current; }
}

// --- subviews ------------------------------------------------------------

fn render_overview(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::pulse_metrics::*;
    use crate::squad_rank::{rank_in_squad, RankMetric};

    let p = &json.players[idx];
    let dmg = damage(p);
    let dps_v = dps_value(p);
    let dc = down_contribution(p);
    let cl = cleanses(p);
    let st = strips(p);
    let dt = damage_taken(p);
    let d_to_tag = dist_to_tag(p);
    let deaths_n = deaths(p);
    let downs_n = downs(p);

    let dmg_rank = rank_in_squad(json, idx, RankMetric::Damage);
    hero_banner(ui,
        "DAMAGE DEALT", ACCENT_DAMAGE,
        &format_damage(dmg),
        &format!("{} DPS", format_damage(dps_v)),
        dmg_rank.map(|r| format!("{} in squad", ordinal(r))).as_deref(),
    );
    ui.dummy([0.0, 2.0]);

    let cells = [
        ("DOWN CONTRIBUTION", ACCENT_DOWN,
            format_damage(dc),
            rank_in_squad(json, idx, RankMetric::DownContribution).map(|r| ordinal(r))),
        ("DEATHS / DOWNS", if deaths_n == 0 { ACCENT_SUCCESS } else { ACCENT_DANGER },
            format!("{deaths_n} / {downs_n}"), None),
        ("STRIPS", ACCENT_SUPPORT,
            st.to_string(),
            rank_in_squad(json, idx, RankMetric::Strips).map(|r| ordinal(r))),
        ("CLEANSES", ACCENT_CLEANSE,
            cl.to_string(),
            rank_in_squad(json, idx, RankMetric::Cleanses).map(|r| ordinal(r))),
        ("DAMAGE TAKEN", ACCENT_DEFEND,
            format_damage(dt),
            rank_in_squad(json, idx, RankMetric::DamageTaken).map(|r| ordinal(r))),
        ("DISTANCE TO TAG", ACCENT_NEUTRAL,
            if d_to_tag > 0.0 { format!("{:.0}", d_to_tag) } else { "—".into() },
            None),
    ];
    draw_2col_card_grid(ui, &cells);
    ui.dummy([0.0, 8.0]);
    render_fight_composition(ui, json, idx);
}

fn render_damage(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::pulse_metrics::*;
    use crate::squad_rank::{rank_in_squad, RankMetric};
    use crate::top_skills::top_damage;

    let p = &json.players[idx];
    let dmg = damage(p);
    let dps_v = dps_value(p);
    let dc = down_contribution(p);

    let dmg_rank = rank_in_squad(json, idx, RankMetric::Damage);
    hero_banner(ui,
        "TOTAL DAMAGE", ACCENT_DAMAGE,
        &format_damage(dmg),
        &format!("{} DPS", format_damage(dps_v)),
        dmg_rank.map(|r| format!("{} in squad", ordinal(r))).as_deref(),
    );
    ui.dummy([0.0, 2.0]);

    let dc_rank = rank_in_squad(json, idx, RankMetric::DownContribution);
    let cells = [
        ("DOWN CONTRIBUTION", ACCENT_DOWN, format_damage(dc), dc_rank.map(ordinal)),
    ];
    draw_2col_card_grid(ui, &cells);

    ui.dummy([0.0, 6.0]);
    let skills = top_damage(p, 8);
    if skills.is_empty() {
        ui.text_disabled("No skill damage recorded.");
        return;
    }
    section_label(ui, "TOP SKILLS");
    let max = skills.first().map(|e| e.damage).unwrap_or(1).max(1);
    let total: u64 = skills.iter().map(|e| e.damage).sum();
    for (i, entry) in skills.iter().enumerate() {
        let frac = entry.damage as f32 / max as f32;
        let pct = if total > 0 { entry.damage as f64 / total as f64 * 100.0 } else { 0.0 };
        let name = resolve_skill_name(json, entry.id, &entry.name);
        draw_skill_bar(ui, json, i, entry.id, &name, frac, pct, &format_damage(entry.damage));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupportMode { Healing, Downed, Barrier }

static SUPPORT_MODE: Lazy<Mutex<SupportMode>> = Lazy::new(|| Mutex::new(SupportMode::Healing));

fn render_support(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::pulse_metrics::*;
    use crate::squad_rank::{rank_in_squad, RankMetric};
    use crate::top_heals::{top_healing, top_downed_healing, top_barrier};

    let p = &json.players[idx];
    let st = strips(p);
    let cl = cleanses(p);
    let cl_self = cleanse_self(p);
    let has_heal = has_healing_data(p);
    let heal = healing(p);
    let heal_hps = hps(p);
    let heal_downed = healing_downed(p);
    let barr = barrier(p);
    let inc_heal = incoming_healing(p);

    // Hero banner — defaults to Healing if the addon is producing data,
    // else falls back to Boon Strips (the prior behaviour).
    let strips_rank = rank_in_squad(json, idx, RankMetric::Strips);
    if has_heal && heal > 0 {
        hero_banner(ui,
            "HEALING OUTPUT", ACCENT_SUPPORT,
            &format_damage(heal),
            &format!("{} HPS", format_damage(heal_hps)),
            None,
        );
    } else {
        hero_banner(ui,
            "BOON STRIPS", ACCENT_SUPPORT,
            &st.to_string(), "",
            strips_rank.map(|r| format!("{} in squad", ordinal(r))).as_deref(),
        );
    }
    ui.dummy([0.0, 2.0]);

    let cl_rank = rank_in_squad(json, idx, RankMetric::Cleanses);
    let strips_rank2 = rank_in_squad(json, idx, RankMetric::Strips);
    let cells: Vec<(&str, [f32; 4], String, Option<String>)> = if has_heal {
        vec![
            ("BARRIER",       [0.65, 0.51, 0.91, 1.0], format_damage(barr),       None),
            ("DOWNED HEALING",ACCENT_DOWN,             format_damage(heal_downed), None),
            ("STRIPS",        ACCENT_SUPPORT,          st.to_string(),            strips_rank2.map(ordinal)),
            ("CLEANSES",      ACCENT_CLEANSE,          cl.to_string(),            cl_rank.map(ordinal)),
            ("SELF CLEANSE",  ACCENT_CLEANSE,          cl_self.to_string(),       None),
            ("INCOMING HEAL", ACCENT_SUCCESS,          format_damage(inc_heal),   None),
        ]
    } else {
        vec![
            ("CLEANSES",      ACCENT_CLEANSE, cl.to_string(),      cl_rank.map(ordinal)),
            ("SELF CLEANSE",  ACCENT_CLEANSE, cl_self.to_string(), None),
            ("STRIPS / SEC",  ACCENT_SUPPORT,
                format!("{:.2}", st as f64 / (json.duration_ms.max(1) as f64 / 1000.0)),
                None),
            ("INCOMING HEAL", ACCENT_SUCCESS,
                "no addon".to_string(), None),
        ]
    };
    let cells_refs: Vec<(&str, [f32; 4], String, Option<String>)> = cells;
    draw_2col_card_grid(ui, &cells_refs);

    if !has_heal {
        ui.dummy([0.0, 4.0]);
        ui.text_disabled("Per-skill heal / barrier breakdowns require the arcdps");
        ui.text_disabled("healing addon (arcdps_healing_stats.dll).");
        return;
    }

    // Mode toggle for top-skills.
    ui.dummy([0.0, 6.0]);
    render_support_mode_toggle(ui);

    let mode = SUPPORT_MODE.lock().ok().map(|g| *g).unwrap_or(SupportMode::Healing);
    section_label(ui, match mode {
        SupportMode::Healing => "TOP HEALING SKILLS",
        SupportMode::Downed  => "TOP DOWNED-HEALING SKILLS",
        SupportMode::Barrier => "TOP BARRIER SKILLS",
    });

    match mode {
        SupportMode::Healing => {
            let skills = top_healing(p, 8);
            render_value_bars(ui, json, &skills.iter().map(|s| (s.id, s.healing)).collect::<Vec<_>>(),
                              "heal", [0.40, 0.85, 0.65, 0.55]);
        }
        SupportMode::Downed => {
            let skills = top_downed_healing(p, 8);
            render_value_bars(ui, json, &skills.iter().map(|s| (s.id, s.downed_healing)).collect::<Vec<_>>(),
                              "down", [0.97, 0.55, 0.42, 0.55]);
        }
        SupportMode::Barrier => {
            let skills = top_barrier(p, 8);
            render_value_bars(ui, json, &skills.iter().map(|s| (s.id, s.barrier)).collect::<Vec<_>>(),
                              "barr", [0.65, 0.51, 0.91, 0.55]);
        }
    }
}

fn render_support_mode_toggle(ui: &Ui) {
    let mut current = SUPPORT_MODE.lock().ok().map(|g| *g).unwrap_or(SupportMode::Healing);
    for (i, (label, mode)) in [
        ("Healing", SupportMode::Healing),
        ("Downed",  SupportMode::Downed),
        ("Barrier", SupportMode::Barrier),
    ].iter().enumerate() {
        let selected = current == *mode;
        let tokens = if selected {
            vec![
                ui.push_style_color(StyleColor::Button,        [0.18, 0.22, 0.30, 1.0]),
                ui.push_style_color(StyleColor::ButtonHovered, [0.22, 0.26, 0.34, 1.0]),
                ui.push_style_color(StyleColor::ButtonActive,  [0.24, 0.28, 0.36, 1.0]),
                ui.push_style_color(StyleColor::Text,          TEXT_PRIMARY),
            ]
        } else {
            vec![ui.push_style_color(StyleColor::Text, TEXT_SECONDARY)]
        };
        if ui.button(label) { current = *mode; }
        for t in tokens { t.pop(); }
        if i + 1 < 3 { ui.same_line(); }
    }
    if let Ok(mut g) = SUPPORT_MODE.lock() { *g = current; }
}

/// Renders a stack of value bars from `(id, value)` pairs, looking up
/// names via `resolve_skill_name`. Generalises the damage-skill bar
/// renderer for any non-negative numeric value.
fn render_value_bars(
    ui: &Ui,
    json: &EiJson,
    pairs: &[(i64, u64)],
    id_prefix: &str,
    bar_color: [f32; 4],
) {
    if pairs.is_empty() {
        ui.text_disabled("No skills recorded.");
        return;
    }
    let max = pairs.first().map(|p| p.1).unwrap_or(1).max(1);
    let total: u64 = pairs.iter().map(|p| p.1).sum();
    for (i, (id, value)) in pairs.iter().enumerate() {
        let frac = *value as f32 / max as f32;
        let pct = if total > 0 { *value as f64 / total as f64 * 100.0 } else { 0.0 };
        let name = resolve_skill_name(json, *id, "");
        draw_value_bar(ui, json, id_prefix, i, *id, &name, frac, pct, &format_damage(*value), bar_color);
    }
}

fn draw_value_bar(
    ui: &Ui,
    json: &EiJson,
    id_prefix: &str,
    row_idx: usize,
    id: i64,
    name: &str,
    frac: f32,
    pct: f64,
    value: &str,
    bar_color: [f32; 4],
) {
    use crate::ui::icons::{lookup, IconKey, IconKind};
    let avail = ui.content_region_avail()[0].max(120.0);
    let row_h = (ui.text_line_height() * 1.55).max(24.0);
    let cursor = ui.cursor_screen_pos();

    let icon = lookup(json, IconKey { kind: IconKind::Skill, id });

    {
        let draw = ui.get_window_draw_list();

        draw.add_rect([cursor[0], cursor[1]],
                      [cursor[0] + avail, cursor[1] + row_h], BG_CARD)
            .filled(true).rounding(5.0).build();
        let bar_w = avail * frac.clamp(0.0, 1.0);
        if bar_w > 0.5 {
            draw.add_rect([cursor[0], cursor[1]],
                          [cursor[0] + bar_w, cursor[1] + row_h], bar_color)
                .filled(true).rounding(5.0).build();
        }
        draw.add_rect([cursor[0], cursor[1]],
                      [cursor[0] + avail, cursor[1] + row_h], BG_CARD_BORDER)
            .rounding(5.0).build();

        let pad_left = 6.0;
        let mut text_x = cursor[0] + pad_left;
        if let Some(handle) = icon {
            let icon_h = row_h - 4.0;
            let icon_w = (icon_h * handle.aspect).max(1.0);
            let icon_y = cursor[1] + 2.0;
            draw.add_image(handle.tex, [text_x, icon_y], [text_x + icon_w, icon_y + icon_h]).build();
            text_x += icon_w + 6.0;
        }
        let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
        draw.add_text([text_x + 1.0, text_y + 1.0], [0.0, 0.0, 0.0, 0.55], name);
        draw.add_text([text_x, text_y], TEXT_PRIMARY, name);

        let pad_right = 10.0;
        let pct_label = if pct >= 0.1 { format!("{:.1}%", pct) } else { String::new() };
        let val_w = ui.calc_text_size(value)[0];
        let pct_w = ui.calc_text_size(&pct_label)[0];
        draw.add_text([cursor[0] + avail - pad_right - val_w + 1.0, text_y + 1.0],
                       [0.0, 0.0, 0.0, 0.55], value);
        draw.add_text([cursor[0] + avail - pad_right - val_w, text_y], TEXT_PRIMARY, value);
        if !pct_label.is_empty() {
            draw.add_text(
                [cursor[0] + avail - pad_right - val_w - 14.0 - pct_w, text_y],
                TEXT_SECONDARY, &pct_label,
            );
        }
    }

    ui.set_cursor_screen_pos(cursor);
    ui.invisible_button(format!("##{id_prefix}-{row_idx}-{id}"), [avail, row_h]);
}

fn render_defense(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::pulse_metrics::*;
    use crate::squad_rank::{rank_in_squad, RankMetric};

    let p = &json.players[idx];
    let dt = damage_taken(p);
    let deaths_n = deaths(p);
    let downs_n = downs(p);
    let dodges_n = dodges(p);
    let blocked_n = blocked(p);
    let evaded_n = evaded(p);
    let missed_n = missed(p);
    let invulned_n = invulned(p);
    let interrupted_n = interrupted(p);
    let cc_in = incoming_cc(p);
    let strips_in = incoming_strips(p);

    let dt_rank = rank_in_squad(json, idx, RankMetric::DamageTaken);
    hero_banner(ui,
        "DAMAGE TAKEN", ACCENT_DEFEND,
        &format_damage(dt), "",
        dt_rank.map(|r| format!("{} in squad", ordinal(r))).as_deref(),
    );
    ui.dummy([0.0, 2.0]);

    let alive_color = if deaths_n == 0 { ACCENT_SUCCESS } else { ACCENT_DANGER };
    let cells = [
        ("DEATHS / DOWNS",  alive_color,    format!("{deaths_n} / {downs_n}"), None),
        ("DODGES",          ACCENT_NEUTRAL, dodges_n.to_string(),              None),
        ("INCOMING CC",     ACCENT_DEFEND,  cc_in.to_string(),                 None),
        ("INCOMING STRIPS", ACCENT_DOWN,    strips_in.to_string(),             None),
    ];
    draw_2col_card_grid(ui, &cells);

    ui.dummy([0.0, 6.0]);
    section_label(ui, "MITIGATION");
    let mit_cells = [
        ("BLOCKED",     ACCENT_CLEANSE, blocked_n.to_string(),     None),
        ("EVADED",      ACCENT_SUPPORT, evaded_n.to_string(),      None),
        ("MISSED",      ACCENT_NEUTRAL, missed_n.to_string(),      None),
        ("INVULNED",    ACCENT_DEFEND,  invulned_n.to_string(),    None),
        ("INTERRUPTED", ACCENT_DAMAGE,  interrupted_n.to_string(), None),
    ];
    draw_2col_card_grid(ui, &mit_cells);
}

fn render_boons(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::boon_uptime::{collect_uptimes, BoonStacking};

    let p = &json.players[idx];
    let ups = collect_uptimes(p);
    if ups.is_empty() {
        ui.text_disabled("No boon uptimes recorded for this fight.");
        return;
    }
    section_label(ui, "BOON UPTIME");
    for boon in &ups {
        let (frac, label) = match boon.stacking {
            BoonStacking::Intensity => {
                let f = (boon.uptime / 25.0).clamp(0.0, 1.0) as f32;
                (f, format!("{:.1} stacks", boon.uptime))
            }
            BoonStacking::Duration => {
                let f = (boon.uptime / 100.0).clamp(0.0, 1.0) as f32;
                (f, format!("{:.1}%", boon.uptime))
            }
        };
        draw_boon_bar(ui, json, boon.id, boon.name, frac, &label, boon_color(boon.name));
    }
}

// --- helpers -------------------------------------------------------------

fn format_damage(d: u64) -> String {
    if d >= 1_000_000 { format!("{:.1}M", d as f64 / 1_000_000.0) }
    else if d >= 1_000 { format!("{:.1}k", d as f64 / 1_000.0) }
    else { format!("{d}") }
}

fn ordinal(n: u32) -> String {
    let s = ["th","st","nd","rd"];
    let v = (n % 100) as usize;
    let suffix = if v >= 20 { s.get(v % 10).copied().unwrap_or("th") }
                 else { s.get(v).copied().unwrap_or("th") };
    format!("{n}{suffix}")
}

/// Look up a skill's display name in the EI top-level `skill_map`.
/// EI emits `totalDamageDist[].name` empty in WvW. Falls back to
/// "Skill <id>" if the map is missing or only contains a numeric name.
fn resolve_skill_name(json: &EiJson, id: i64, fallback: &str) -> String {
    if !fallback.is_empty() && fallback.parse::<i64>().is_err() {
        return fallback.to_string();
    }
    if let Some(entry) = json.skill_map.get(&format!("s{id}")) {
        if !entry.name.is_empty() && entry.name.parse::<i64>().is_err() {
            return entry.name.clone();
        }
    }
    format!("Skill {id}")
}

fn section_label(ui: &Ui, label: &str) {
    ui.text_colored(TEXT_MUTED, label);
    ui.dummy([0.0, 1.0]);
}

/// Tall headline card with a big primary value, secondary metric, and
/// optional bottom-right rank footer.
fn hero_banner(
    ui: &Ui,
    label: &str,
    accent: [f32; 4],
    primary: &str,
    secondary: &str,
    rank: Option<&str>,
) {
    let avail = ui.content_region_avail()[0].max(120.0);
    let cursor = ui.cursor_screen_pos();
    let x = cursor[0];
    let y = cursor[1];
    let draw = ui.get_window_draw_list();

    draw.add_rect([x, y], [x + avail, y + HERO_H], BG_CARD)
        .filled(true).rounding(8.0).build();
    // Tint top half toward accent.
    let mut tint = accent; tint[3] = 0.08;
    draw.add_rect([x, y], [x + avail, y + HERO_H * 0.55], tint)
        .filled(true).rounding(8.0).build();
    // Subtle border with accent.
    let mut border = accent; border[3] = 0.22;
    draw.add_rect([x, y], [x + avail, y + HERO_H], border)
        .rounding(8.0).build();
    // Accent stripe on the left edge.
    draw.add_rect([x, y + 10.0], [x + 4.0, y + HERO_H - 10.0], accent)
        .filled(true).rounding(2.0).build();

    let pad_x = 16.0;
    let pad_y = 10.0;
    let line_h = ui.text_line_height();

    draw.add_text([x + pad_x, y + pad_y], accent, label);

    // Primary value, large-ish (drop shadow + bright fill).
    let primary_y = y + pad_y + line_h + 8.0;
    draw.add_text([x + pad_x + 1.0, primary_y + 1.0], [0.0, 0.0, 0.0, 0.5], primary);
    draw.add_text([x + pad_x, primary_y], TEXT_PRIMARY, primary);

    if !secondary.is_empty() {
        let sec_w = ui.calc_text_size(secondary)[0];
        draw.add_text([x + avail - pad_x - sec_w, primary_y + 2.0],
                       TEXT_SECONDARY, secondary);
    }
    if let Some(r) = rank {
        let rw = ui.calc_text_size(r)[0];
        draw.add_text(
            [x + avail - pad_x - rw, y + HERO_H - pad_y - line_h],
            TEXT_MUTED, r,
        );
    }

    ui.dummy([avail, HERO_H]);
}

/// Two-column grid of stat cards. Each tuple: (label, accent, value, rank).
fn draw_2col_card_grid(
    ui: &Ui,
    items: &[(&str, [f32; 4], String, Option<String>)],
) {
    let avail = ui.content_region_avail()[0].max(120.0);
    let col_w = (avail - GAP) / 2.0;
    let cursor = ui.cursor_screen_pos();
    let start_x = cursor[0];
    let start_y = cursor[1];
    let draw = ui.get_window_draw_list();
    let line_h = ui.text_line_height();

    for (i, (label, accent, value, rank)) in items.iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        let x = start_x + col as f32 * (col_w + GAP);
        let y = start_y + row as f32 * (CARD_H + GAP);

        draw.add_rect([x, y], [x + col_w, y + CARD_H], BG_CARD)
            .filled(true).rounding(6.0).build();
        draw.add_rect([x, y], [x + col_w, y + CARD_H], BG_CARD_BORDER)
            .rounding(6.0).build();
        // Left accent stripe
        draw.add_rect([x, y + 8.0], [x + 3.0, y + CARD_H - 8.0], *accent)
            .filled(true).rounding(2.0).build();

        let pad_x = 12.0;
        let pad_y = 8.0;
        draw.add_text([x + pad_x, y + pad_y], *accent, *label);
        draw.add_text(
            [x + pad_x + 1.0, y + pad_y + line_h + 5.0],
            [0.0, 0.0, 0.0, 0.5], value.as_str(),
        );
        draw.add_text(
            [x + pad_x, y + pad_y + line_h + 4.0],
            TEXT_PRIMARY, value.as_str(),
        );
        if let Some(r) = rank {
            let rw = ui.calc_text_size(r)[0];
            draw.add_text(
                [x + col_w - pad_x - rw, y + CARD_H - pad_y - line_h],
                TEXT_MUTED, r.as_str(),
            );
        }
    }

    let rows = (items.len() + 1) / 2;
    let total_h = rows as f32 * CARD_H + rows.saturating_sub(1) as f32 * GAP;
    ui.dummy([avail, total_h]);
}

/// Full-width row with a damage-coloured backing bar and skill label,
/// percentage, and right-aligned damage value.
fn draw_skill_bar(
    ui: &Ui,
    json: &EiJson,
    row_idx: usize,
    id: i64,
    name: &str,
    frac: f32,
    pct: f64,
    value: &str,
) {
    use crate::ui::icons::{lookup, IconKey, IconKind};
    let avail = ui.content_region_avail()[0].max(120.0);
    let row_h = (ui.text_line_height() * 1.55).max(24.0);
    let cursor = ui.cursor_screen_pos();
    let icon = lookup(json, IconKey { kind: IconKind::Skill, id });

    {
        let draw = ui.get_window_draw_list();

        draw.add_rect([cursor[0], cursor[1]],
                      [cursor[0] + avail, cursor[1] + row_h], BG_CARD)
            .filled(true).rounding(5.0).build();
        let bar_w = avail * frac.clamp(0.0, 1.0);
        if bar_w > 0.5 {
            let mut accent = ACCENT_DAMAGE; accent[3] = 0.55;
            draw.add_rect([cursor[0], cursor[1]],
                          [cursor[0] + bar_w, cursor[1] + row_h], accent)
                .filled(true).rounding(5.0).build();
        }
        draw.add_rect([cursor[0], cursor[1]],
                      [cursor[0] + avail, cursor[1] + row_h], BG_CARD_BORDER)
            .rounding(5.0).build();

        let pad_left = 6.0;
        let mut text_x = cursor[0] + pad_left;
        if let Some(handle) = icon {
            let icon_h = row_h - 4.0;
            let icon_w = (icon_h * handle.aspect).max(1.0);
            let icon_y = cursor[1] + 2.0;
            draw.add_image(handle.tex, [text_x, icon_y], [text_x + icon_w, icon_y + icon_h]).build();
            text_x += icon_w + 6.0;
        }
        let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
        draw.add_text([text_x + 1.0, text_y + 1.0], [0.0, 0.0, 0.0, 0.55], name);
        draw.add_text([text_x, text_y], TEXT_PRIMARY, name);

        let pad_right = 10.0;
        let pct_label = if pct >= 0.1 { format!("{:.1}%", pct) } else { String::new() };
        let val_w = ui.calc_text_size(value)[0];
        let pct_w = ui.calc_text_size(&pct_label)[0];
        draw.add_text([cursor[0] + avail - pad_right - val_w + 1.0, text_y + 1.0],
                       [0.0, 0.0, 0.0, 0.55], value);
        draw.add_text([cursor[0] + avail - pad_right - val_w, text_y], TEXT_PRIMARY, value);
        if !pct_label.is_empty() {
            draw.add_text(
                [cursor[0] + avail - pad_right - val_w - 14.0 - pct_w, text_y],
                TEXT_SECONDARY, &pct_label,
            );
        }
    }

    ui.set_cursor_screen_pos(cursor);
    ui.invisible_button(format!("##sk-{row_idx}-{id}"), [avail, row_h]);
}

fn boon_color(name: &str) -> [f32; 4] {
    match name {
        "Might"        => [0.91, 0.36, 0.23, 1.0],
        "Fury"         => [0.91, 0.60, 0.23, 1.0],
        "Quickness"    => [0.75, 0.42, 0.94, 1.0],
        "Alacrity"     => [0.94, 0.42, 0.74, 1.0],
        "Protection"   => [0.36, 0.61, 0.83, 1.0],
        "Regeneration" => [0.29, 0.86, 0.50, 1.0],
        "Vigor"        => [0.64, 0.90, 0.21, 1.0],
        "Swiftness"    => [0.98, 0.80, 0.08, 1.0],
        "Resistance"   => [0.77, 0.64, 0.35, 1.0],
        "Stability"    => [0.96, 0.62, 0.04, 1.0],
        "Aegis"        => [0.49, 0.83, 0.99, 1.0],
        "Resolution"   => [0.65, 0.51, 0.91, 1.0],
        "Retaliation"  => [0.98, 0.57, 0.20, 1.0],
        _              => [0.55, 0.55, 0.62, 1.0],
    }
}

fn draw_boon_bar(ui: &Ui, json: &EiJson, id: i64, name: &str, frac: f32, label: &str, color: [f32; 4]) {
    use crate::ui::icons::{lookup, IconKey, IconKind};
    let avail = ui.content_region_avail()[0].max(120.0);
    let row_h = (ui.text_line_height() * 1.55).max(24.0);
    let cursor = ui.cursor_screen_pos();
    let icon = lookup(json, IconKey { kind: IconKind::Buff, id });

    {
        let draw = ui.get_window_draw_list();

        draw.add_rect([cursor[0], cursor[1]],
                      [cursor[0] + avail, cursor[1] + row_h], BG_CARD)
            .filled(true).rounding(5.0).build();

        let bar_w = avail * frac.clamp(0.0, 1.0);
        if bar_w > 0.5 {
            let mut bc = color; bc[3] = 0.55;
            draw.add_rect([cursor[0], cursor[1]],
                          [cursor[0] + bar_w, cursor[1] + row_h], bc)
                .filled(true).rounding(5.0).build();
        }
        draw.add_rect([cursor[0], cursor[1]],
                      [cursor[0] + avail, cursor[1] + row_h], BG_CARD_BORDER)
            .rounding(5.0).build();

        let pad_left = 6.0;
        let mut text_x = cursor[0] + pad_left;
        if let Some(handle) = icon {
            let icon_h = row_h - 4.0;
            let icon_w = (icon_h * handle.aspect).max(1.0);
            let icon_y = cursor[1] + 2.0;
            draw.add_image(handle.tex, [text_x, icon_y], [text_x + icon_w, icon_y + icon_h]).build();
            text_x += icon_w + 6.0;
        } else {
            // No icon yet — keep the coloured accent stripe so the row
            // still has a visual identity for the boon.
            let stripe_w = 4.0;
            draw.add_rect([cursor[0] + 2.0, cursor[1] + 6.0],
                          [cursor[0] + 2.0 + stripe_w, cursor[1] + row_h - 6.0], color)
                .filled(true).rounding(2.0).build();
            text_x = cursor[0] + 14.0;
        }
        let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
        draw.add_text([text_x + 1.0, text_y + 1.0], [0.0, 0.0, 0.0, 0.55], name);
        draw.add_text([text_x, text_y], TEXT_PRIMARY, name);

        let pad_right = 14.0;
        let label_w = ui.calc_text_size(label)[0];
        draw.add_text([cursor[0] + avail - pad_right - label_w + 1.0, text_y + 1.0],
                       [0.0, 0.0, 0.0, 0.55], label);
        draw.add_text([cursor[0] + avail - pad_right - label_w, text_y],
                       TEXT_PRIMARY, label);
    }

    ui.set_cursor_screen_pos(cursor);
    ui.invisible_button(format!("##boon-{id}"), [avail, row_h]);
}

// --- fight composition card ----------------------------------------------

static COMP_SELECTED: Lazy<Mutex<Option<crate::fight_composition::GroupKey>>> =
    Lazy::new(|| Mutex::new(None));

fn render_fight_composition(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::fight_composition::compute;
    let groups = compute(json, idx);
    if groups.is_empty() { return; }
    let total: u32 = groups.iter().map(|g| g.count).sum();
    if total == 0 { return; }

    section_label(ui, "FIGHT COMPOSITION");

    let mut selected = COMP_SELECTED.lock().ok().and_then(|g| g.clone());

    // Segmented bar — proportional widths, one colour per group.
    let avail = ui.content_region_avail()[0].max(120.0);
    let pill_h = 22.0;
    let pad_x = 8.0;
    let pad_between = 6.0;
    {
        // Scope the draw list so it releases before downstream helpers
        // (draw_class_chips) re-acquire it — imgui-rs panics otherwise.
        let cursor = ui.cursor_screen_pos();
        let bar_h = 10.0;
        let draw = ui.get_window_draw_list();
        draw.add_rect([cursor[0], cursor[1]], [cursor[0] + avail, cursor[1] + bar_h], BG_CARD)
            .filled(true).rounding(5.0).build();
        let mut x = cursor[0];
        let seg_gap = 2.0;
        let total_w = avail - seg_gap * (groups.len() as f32 - 1.0).max(0.0);
        for g in &groups {
            let w = total_w * (g.count as f32 / total as f32);
            draw.add_rect([x, cursor[1]], [x + w, cursor[1] + bar_h], g.color)
                .filled(true).rounding(3.0).build();
            x += w + seg_gap;
        }
        ui.dummy([avail, bar_h + 4.0]);

        let cursor = ui.cursor_screen_pos();
        let mut px = cursor[0];
        let py = cursor[1];
        for (i, g) in groups.iter().enumerate() {
            let count_str = g.count.to_string();
            let pct = format!("{}%", (g.count as f32 / total as f32 * 100.0).round() as i32);
            let label_str = &g.label;
            let count_w = ui.calc_text_size(&count_str)[0];
            let label_w = ui.calc_text_size(label_str)[0];
            let pct_w = ui.calc_text_size(&pct)[0];
            let dot_w = 8.0;
            let token_gap = 6.0;
            let pill_w = pad_x + dot_w + token_gap + count_w + token_gap + label_w + token_gap + pct_w + pad_x;
            if i > 0 && (px + pill_w) > cursor[0] + avail {
                px = cursor[0];
            }
            let active = selected.as_ref() == Some(&g.key);
            let bg = if active { [0.18, 0.22, 0.28, 1.0] } else { [0.10, 0.12, 0.16, 1.0] };
            let border = if active { g.color } else { [1.0, 1.0, 1.0, 0.06] };
            draw.add_rect([px, py], [px + pill_w, py + pill_h], bg)
                .filled(true).rounding(11.0).build();
            draw.add_rect([px, py], [px + pill_w, py + pill_h], border)
                .rounding(11.0).build();
            let dot_y = py + (pill_h - dot_w) * 0.5;
            draw.add_rect([px + pad_x, dot_y], [px + pad_x + dot_w, dot_y + dot_w], g.color)
                .filled(true).rounding(2.0).build();
            let text_y = py + (pill_h - ui.text_line_height()) * 0.5;
            let mut tx = px + pad_x + dot_w + token_gap;
            draw.add_text([tx, text_y], TEXT_PRIMARY, &count_str);
            tx += count_w + token_gap;
            draw.add_text([tx, text_y], TEXT_SECONDARY, label_str);
            tx += label_w + token_gap;
            draw.add_text([tx, text_y], TEXT_MUTED, &pct);
            px += pill_w + pad_between;
        }
    }

    // Hit-test pass (no draw list acquisition; invisible_button is safe).
    let cursor = ui.cursor_screen_pos();
    let mut px = cursor[0];
    let py = cursor[1];
    for (i, g) in groups.iter().enumerate() {
        let count_str = g.count.to_string();
        let pct = format!("{}%", (g.count as f32 / total as f32 * 100.0).round() as i32);
        let count_w = ui.calc_text_size(&count_str)[0];
        let label_w = ui.calc_text_size(&g.label)[0];
        let pct_w = ui.calc_text_size(&pct)[0];
        let dot_w = 8.0;
        let token_gap = 6.0;
        let pill_w = pad_x + dot_w + token_gap + count_w + token_gap + label_w + token_gap + pct_w + pad_x;
        if i > 0 && (px + pill_w) > cursor[0] + avail {
            px = cursor[0];
        }
        let active = selected.as_ref() == Some(&g.key);
        ui.set_cursor_screen_pos([px, py]);
        if ui.invisible_button(format!("##comp-pill-{i}"), [pill_w, pill_h]) {
            selected = if active { None } else { Some(g.key.clone()) };
        }
        px += pill_w + pad_between;
    }
    if let Ok(mut g) = COMP_SELECTED.lock() { *g = selected.clone(); }
    ui.dummy([avail, pill_h]);

    // Expanded per-spec chips for the selected group.
    if let Some(key) = &selected {
        if let Some(g) = groups.iter().find(|g| &g.key == key) {
            if !g.class_counts.is_empty() {
                draw_class_chips(ui, &g.class_counts, g.color);
            }
        }
    }
}

fn draw_class_chips(ui: &Ui, chips: &[(String, u32)], accent: [f32; 4]) {
    let avail = ui.content_region_avail()[0].max(120.0);
    let cursor = ui.cursor_screen_pos();
    let chip_h = 20.0;
    let pad_x = 6.0;
    let gap = 4.0;
    let draw = ui.get_window_draw_list();
    let mut x = cursor[0];
    let mut y = cursor[1];
    let mut rows = 1u32;
    for (i, (spec, count)) in chips.iter().enumerate() {
        let count_str = count.to_string();
        let spec_w = ui.calc_text_size(spec.as_str())[0];
        let count_w = ui.calc_text_size(&count_str)[0];
        let chip_w = pad_x + spec_w + 6.0 + count_w + pad_x;
        if x + chip_w > cursor[0] + avail {
            x = cursor[0];
            y += chip_h + gap;
            rows += 1;
        }
        draw.add_rect([x, y], [x + chip_w, y + chip_h], [0.10, 0.12, 0.16, 1.0])
            .filled(true).rounding(4.0).build();
        let mut border = accent; border[3] = 0.30;
        draw.add_rect([x, y], [x + chip_w, y + chip_h], border)
            .rounding(4.0).build();
        let text_y = y + (chip_h - ui.text_line_height()) * 0.5;
        draw.add_text([x + pad_x, text_y], accent, spec.as_str());
        draw.add_text([x + pad_x + spec_w + 6.0, text_y], TEXT_PRIMARY, &count_str);
        // Invisible for layout
        ui.set_cursor_screen_pos([x, y]);
        let _ = ui.invisible_button(format!("##chip-{i}"), [chip_w, chip_h]);
        x += chip_w + gap;
    }
    let total_h = rows as f32 * chip_h + (rows.saturating_sub(1) as f32) * gap;
    ui.set_cursor_screen_pos([cursor[0], cursor[1]]);
    ui.dummy([avail, total_h]);
}
