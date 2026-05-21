#![cfg(windows)]
//! Pulse window — five tabbed subviews showing the local player's
//! last-fight metrics, styled as backing-card "stat cards" inspired by
//! the AxiPulse desktop UI.

use std::sync::Mutex;

use arcdps::imgui::{Condition, StyleColor, StyleVar, Ui};
use once_cell::sync::Lazy;

use crate::config::Config;
use crate::ei_model::EiJson;
use crate::self_identify::find_self_index;
use crate::state::AppState;

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

pub fn render(ui: &Ui, state: &AppState, config: &mut Config) {
    if !config.show_pulse { return; }

    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([14.0, 12.0])),
        ui.push_style_var(StyleVar::WindowRounding(10.0)),
        ui.push_style_var(StyleVar::WindowBorderSize(0.0)),
        ui.push_style_var(StyleVar::FrameRounding(6.0)),
        ui.push_style_var(StyleVar::ItemSpacing([8.0, 8.0])),
    ];
    let color_tokens = [
        ui.push_style_color(StyleColor::WindowBg,      [0.055, 0.065, 0.085, 0.92]),
        ui.push_style_color(StyleColor::TitleBg,       [0.055, 0.065, 0.085, 0.95]),
        ui.push_style_color(StyleColor::TitleBgActive, [0.085, 0.10,  0.13,  0.95]),
        ui.push_style_color(StyleColor::Separator,     [1.0, 1.0, 1.0, 0.06]),
        ui.push_style_color(StyleColor::Button,        [0.10, 0.12, 0.16, 1.0]),
        ui.push_style_color(StyleColor::ButtonHovered, [0.14, 0.17, 0.22, 1.0]),
        ui.push_style_color(StyleColor::ButtonActive,  [0.18, 0.22, 0.28, 1.0]),
    ];

    let mut window = ui.window("Pulse").size([560.0, 540.0], Condition::FirstUseEver);
    if let Some(pos) = config.pulse_pos {
        window = window.position([pos.0, pos.1], Condition::FirstUseEver);
    }
    let mut open = true;
    window.opened(&mut open).build(|| {
        let current = state.current();
        if current.is_none() {
            ui.text_disabled("Waiting for the first parsed fight…");
            return;
        }
        let record = current.unwrap();
        let json = &record.data;

        let Some(idx) = find_self_index(json) else {
            ui.text_disabled("Could not identify local player in this fight.");
            return;
        };

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
    });

    if !open {
        config.show_pulse = false;
        config.save();
    }

    for tok in color_tokens { tok.pop(); }
    for tok in style_tokens { tok.pop(); }
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
        draw_skill_bar(ui, i, entry.id, &name, frac, pct, &format_damage(entry.damage));
    }
}

fn render_support(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::pulse_metrics::*;
    use crate::squad_rank::{rank_in_squad, RankMetric};

    let p = &json.players[idx];
    let st = strips(p);
    let cl = cleanses(p);
    let cl_self = cleanse_self(p);
    let cc_in = incoming_cc(p);

    let strips_rank = rank_in_squad(json, idx, RankMetric::Strips);
    hero_banner(ui,
        "BOON STRIPS", ACCENT_SUPPORT,
        &st.to_string(), "",
        strips_rank.map(|r| format!("{} in squad", ordinal(r))).as_deref(),
    );
    ui.dummy([0.0, 2.0]);

    let cl_rank = rank_in_squad(json, idx, RankMetric::Cleanses);
    let cells = [
        ("CLEANSES",      ACCENT_CLEANSE, cl.to_string(),      cl_rank.map(ordinal)),
        ("SELF CLEANSE",  ACCENT_CLEANSE, cl_self.to_string(), None),
        ("INCOMING CC",   ACCENT_DEFEND,  cc_in.to_string(),   None),
        ("STRIPS / SEC",  ACCENT_SUPPORT,
            format!("{:.2}", st as f64 / (json.duration_ms.max(1) as f64 / 1000.0)),
            None),
    ];
    draw_2col_card_grid(ui, &cells);

    ui.dummy([0.0, 4.0]);
    ui.text_disabled("Per-skill heal / barrier breakdowns require the arcdps");
    ui.text_disabled("healing addon and aren't wired in Pulse v1.");
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
        draw_boon_bar(ui, boon.id, boon.name, frac, &label, boon_color(boon.name));
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
    row_idx: usize,
    id: i64,
    name: &str,
    frac: f32,
    pct: f64,
    value: &str,
) {
    let avail = ui.content_region_avail()[0].max(120.0);
    let row_h = (ui.text_line_height() * 1.55).max(24.0);
    let cursor = ui.cursor_screen_pos();
    let draw = ui.get_window_draw_list();

    draw.add_rect([cursor[0], cursor[1]],
                  [cursor[0] + avail, cursor[1] + row_h],
                  BG_CARD)
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

    let pad = 10.0;
    let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
    draw.add_text([cursor[0] + pad + 1.0, text_y + 1.0], [0.0, 0.0, 0.0, 0.55], name);
    draw.add_text([cursor[0] + pad, text_y], TEXT_PRIMARY, name);

    let pct_label = if pct >= 0.1 { format!("{:.1}%", pct) } else { String::new() };
    let val_w = ui.calc_text_size(value)[0];
    let pct_w = ui.calc_text_size(&pct_label)[0];
    draw.add_text([cursor[0] + avail - pad - val_w + 1.0, text_y + 1.0],
                   [0.0, 0.0, 0.0, 0.55], value);
    draw.add_text([cursor[0] + avail - pad - val_w, text_y], TEXT_PRIMARY, value);
    if !pct_label.is_empty() {
        draw.add_text(
            [cursor[0] + avail - pad - val_w - 14.0 - pct_w, text_y],
            TEXT_SECONDARY, &pct_label,
        );
    }

    // Use row index + id for a stable, unique ImGui ID even when names are blank.
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

fn draw_boon_bar(ui: &Ui, id: i64, name: &str, frac: f32, label: &str, color: [f32; 4]) {
    let avail = ui.content_region_avail()[0].max(120.0);
    let row_h = (ui.text_line_height() * 1.55).max(24.0);
    let cursor = ui.cursor_screen_pos();
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

    // Coloured accent stripe on the left so the boon's identity colour stays present.
    let stripe_w = 4.0;
    draw.add_rect([cursor[0] + 2.0, cursor[1] + 6.0],
                  [cursor[0] + 2.0 + stripe_w, cursor[1] + row_h - 6.0], color)
        .filled(true).rounding(2.0).build();

    let pad = 14.0;
    let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
    // Name in white for legibility regardless of boon colour.
    draw.add_text([cursor[0] + pad + 1.0, text_y + 1.0], [0.0, 0.0, 0.0, 0.55], name);
    draw.add_text([cursor[0] + pad, text_y], TEXT_PRIMARY, name);

    let label_w = ui.calc_text_size(label)[0];
    draw.add_text([cursor[0] + avail - pad - label_w + 1.0, text_y + 1.0],
                   [0.0, 0.0, 0.0, 0.55], label);
    draw.add_text([cursor[0] + avail - pad - label_w, text_y],
                   TEXT_PRIMARY, label);

    ui.set_cursor_screen_pos(cursor);
    ui.invisible_button(format!("##boon-{id}"), [avail, row_h]);
}
