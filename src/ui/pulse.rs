#![cfg(windows)]
//! Pulse window — five tabbed subviews showing the local player's
//! last-fight metrics.

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

/// Entry point called from `plugin::imgui`. Renders the Pulse window
/// when `config.show_pulse` is true.
pub fn render(ui: &Ui, state: &AppState, config: &mut Config) {
    if !config.show_pulse { return; }

    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([12.0, 10.0])),
        ui.push_style_var(StyleVar::WindowRounding(8.0)),
        ui.push_style_var(StyleVar::WindowBorderSize(0.0)),
        ui.push_style_var(StyleVar::FrameRounding(4.0)),
        ui.push_style_var(StyleVar::ItemSpacing([8.0, 6.0])),
    ];
    let color_tokens = [
        ui.push_style_color(StyleColor::WindowBg,      [0.06, 0.07, 0.09, 0.86]),
        ui.push_style_color(StyleColor::TitleBg,       [0.06, 0.07, 0.09, 0.95]),
        ui.push_style_color(StyleColor::TitleBgActive, [0.10, 0.11, 0.14, 0.95]),
        ui.push_style_color(StyleColor::Separator,     [1.0, 1.0, 1.0, 0.06]),
    ];

    let mut window = ui.window("Pulse").size([520.0, 480.0], Condition::FirstUseEver);
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
        ui.separator();

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
    for (label, sv) in [
        ("Overview", Subview::Overview),
        ("Damage",   Subview::Damage),
        ("Support",  Subview::Support),
        ("Defense",  Subview::Defense),
        ("Boons",    Subview::Boons),
    ] {
        if ui.radio_button_bool(label, current == sv) {
            current = sv;
        }
        ui.same_line();
    }
    ui.new_line();
    if let Ok(mut g) = SUBVIEW.lock() { *g = current; }
}

// Subview bodies — filled in across Tasks 7–11. Stubs print "TODO".
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

    ui.text_colored([0.92, 0.40, 0.40, 1.0], "DAMAGE DEALT");
    ui.text(format_damage(dmg));
    ui.same_line();
    ui.text_disabled(format!("({} DPS)", format_damage(dps_v)));
    if let Some(r) = rank_in_squad(json, idx, RankMetric::Damage) {
        ui.same_line();
        ui.text_disabled(format!("· {} in squad", ordinal(r)));
    }
    ui.separator();

    let cell = |ui: &Ui, label: &str, value: String, rank: Option<u32>| {
        ui.text_colored([0.65, 0.65, 0.72, 1.0], label);
        ui.text(value);
        if let Some(r) = rank {
            ui.same_line();
            ui.text_disabled(format!("· {} in squad", ordinal(r)));
        }
        ui.spacing();
    };

    cell(ui, "DOWN CONTRIBUTION", dc.to_string(),
         rank_in_squad(json, idx, RankMetric::DownContribution));
    cell(ui, "DEATHS / DOWNS", format!("{deaths_n} / {downs_n}"), None);
    cell(ui, "STRIPS", st.to_string(),
         rank_in_squad(json, idx, RankMetric::Strips));
    cell(ui, "CLEANSES", cl.to_string(),
         rank_in_squad(json, idx, RankMetric::Cleanses));
    cell(ui, "DAMAGE TAKEN", format_damage(dt),
         rank_in_squad(json, idx, RankMetric::DamageTaken));
    cell(ui, "DISTANCE TO TAG", if d_to_tag > 0.0 { format!("{:.0}", d_to_tag) } else { "—".into() }, None);
}
fn render_damage(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::pulse_metrics::*;
    use crate::top_skills::top_damage;

    let p = &json.players[idx];
    let dmg = damage(p);
    let dps_v = dps_value(p);
    let dc = down_contribution(p);

    ui.text_colored([0.92, 0.40, 0.40, 1.0], "TOTAL DAMAGE");
    ui.text(format_damage(dmg));
    ui.same_line();
    ui.text_disabled(format!("({} DPS)", format_damage(dps_v)));
    ui.spacing();

    ui.text_colored([0.97, 0.45, 0.45, 1.0], "DOWN CONTRIBUTION");
    ui.text(dc.to_string());
    ui.separator();

    let skills = top_damage(p, 8);
    if skills.is_empty() {
        ui.text_disabled("No skill damage recorded.");
        return;
    }
    let max = skills.first().map(|e| e.damage).unwrap_or(1).max(1);
    let total: u64 = skills.iter().map(|e| e.damage).sum();
    ui.text_disabled("TOP SKILLS");
    for entry in &skills {
        let frac = entry.damage as f32 / max as f32;
        let pct = if total > 0 { entry.damage as f64 / total as f64 * 100.0 } else { 0.0 };
        draw_skill_bar(ui, &entry.name, frac, pct, format_damage(entry.damage));
    }
}
fn render_support(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::pulse_metrics::*;
    use crate::squad_rank::{rank_in_squad, RankMetric};

    let p = &json.players[idx];
    let st = strips(p);
    let cl = cleanses(p);
    let cl_self = cleanse_self(p);

    ui.text_colored([0.45, 0.85, 0.65, 1.0], "BOON STRIPS");
    ui.text(st.to_string());
    if let Some(r) = rank_in_squad(json, idx, RankMetric::Strips) {
        ui.same_line();
        ui.text_disabled(format!("· {} in squad", ordinal(r)));
    }
    ui.spacing();

    ui.text_colored([0.45, 0.85, 0.65, 1.0], "CLEANSES");
    ui.text(format!("{cl} ({cl_self} self)"));
    if let Some(r) = rank_in_squad(json, idx, RankMetric::Cleanses) {
        ui.same_line();
        ui.text_disabled(format!("· {} in squad", ordinal(r)));
    }
    ui.separator();
    ui.text_disabled("Per-skill heal / barrier breakdowns require the");
    ui.text_disabled("arcdps healing addon — not wired in Pulse v1.");
}
fn render_defense (ui: &Ui, _json: &EiJson, _idx: usize) { ui.text("defense — Task 10"); }
fn render_boons   (ui: &Ui, _json: &EiJson, _idx: usize) { ui.text("boons — Task 11"); }

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

/// Full-width row: text label, percent overlay, damage right-aligned,
/// behind a coloured backing bar.
fn draw_skill_bar(ui: &Ui, name: &str, frac: f32, pct: f64, value: String) {
    let avail = ui.content_region_avail()[0].max(120.0);
    let row_h = (ui.text_line_height() * 1.5).max(22.0);
    let cursor = ui.cursor_screen_pos();
    let draw = ui.get_window_draw_list();

    draw.add_rect([cursor[0], cursor[1]],
                  [cursor[0] + avail, cursor[1] + row_h],
                  [0.10, 0.12, 0.15, 1.0])
        .filled(true).rounding(4.0).build();

    let bar_w = avail * frac.clamp(0.0, 1.0);
    if bar_w > 0.5 {
        draw.add_rect([cursor[0], cursor[1]],
                      [cursor[0] + bar_w, cursor[1] + row_h],
                      [0.85, 0.30, 0.30, 0.55])
            .filled(true).rounding(4.0).build();
    }

    let pad = 8.0;
    let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
    let label = if name.is_empty() { "(unnamed skill)" } else { name };
    draw.add_text([cursor[0] + pad + 1.0, text_y + 1.0], [0.0, 0.0, 0.0, 0.55], label);
    draw.add_text([cursor[0] + pad, text_y], [1.0, 1.0, 1.0, 0.97], label);

    let pct_label = if pct >= 0.1 { format!("{:.1}%", pct) } else { String::new() };
    let val_w = ui.calc_text_size(&value)[0];
    let pct_w = ui.calc_text_size(&pct_label)[0];
    draw.add_text([cursor[0] + avail - pad - val_w, text_y], [1.0, 1.0, 1.0, 0.95], &value);
    if !pct_label.is_empty() {
        draw.add_text(
            [cursor[0] + avail - pad - val_w - 12.0 - pct_w, text_y],
            [0.85, 0.85, 0.85, 0.85], &pct_label,
        );
    }

    ui.set_cursor_screen_pos(cursor);
    ui.invisible_button(format!("##sk-{}", name), [avail, row_h]);
    ui.spacing();
}
