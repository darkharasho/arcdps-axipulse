#![cfg(windows)]
//! Pulse tab content — five subviews (Overview/Damage/Support/Defense/
//! Boons) rendered inside the unified AxiPulse window. The outer
//! window + fight picker live in `ui::main`.
//!
//! No colour literal lives here: chrome comes from `ui::theme`, and the
//! metric and boon inks from `ui::series`. Every raised surface, bar and
//! chip is laid down by `ui::axi` so the block-fill-outline sequence
//! exists in one place.

use std::sync::Mutex;

use arcdps::imgui::Ui;
use once_cell::sync::Lazy;

use crate::derived::Derived;
use crate::fight_data::FightData;
use crate::ui::axi::{self, Rect};
use crate::ui::series;
use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Subview { Overview, Damage, Support, Defense, Boons }

static SUBVIEW: Lazy<Mutex<Subview>> = Lazy::new(|| Mutex::new(Subview::Overview));

const HERO_H: f32 = 76.0;
const CARD_H: f32 = 58.0;
/// Gap between grid cells. Must exceed OFFSET_CONTROL or a cell's
/// offset block lands under its neighbour.
const GAP: f32 = 8.0 + theme::OFFSET_CONTROL;

// --- entry point ---------------------------------------------------------

/// Render the Pulse tab contents (no window — caller owns that).
pub fn render_content(ui: &Ui, fight: &FightData, idx: usize, derived: &Derived, accent: [f32; 4]) {
    render_tab_strip(ui, accent);
    ui.dummy([0.0, 4.0]);

    let subview = SUBVIEW.lock().ok().map(|g| *g).unwrap_or(Subview::Overview);
    match subview {
        Subview::Overview => render_overview(ui, fight, idx, derived),
        Subview::Damage   => render_damage(ui, fight, idx, derived),
        Subview::Support  => render_support(ui, fight, idx, derived, accent),
        Subview::Defense  => render_defense(ui, fight, idx),
        Subview::Boons    => render_boons(ui, fight, idx, derived),
    }
}

fn render_tab_strip(ui: &Ui, accent: [f32; 4]) {
    let mut current = SUBVIEW.lock().ok().map(|g| *g).unwrap_or(Subview::Overview);
    let tabs = [
        ("Overview", Subview::Overview),
        ("Damage",   Subview::Damage),
        ("Support",  Subview::Support),
        ("Defense",  Subview::Defense),
        ("Boons",    Subview::Boons),
    ];
    let pad = [12.0_f32, 5.0_f32];
    let h = ui.text_line_height() + pad[1] * 2.0;

    let origin = ui.cursor_screen_pos();
    let mut x = origin[0];
    for (label, sv) in tabs.iter() {
        let (clicked, w) = axi::chip(
            ui,
            [x, origin[1]],
            &format!("pulse-tab-{label}"),
            label,
            current == *sv,
            accent,
            pad,
        );
        if clicked { current = *sv; }
        // Clear the neighbour's offset block before the next chip.
        x += w + theme::OFFSET_CONTROL + 5.0;
    }
    // Reserve the strip's span with a regular item rather than an
    // absolute cursor, so the block below it is not overdrawn.
    ui.set_cursor_screen_pos(origin);
    ui.dummy([x - origin[0], h + theme::OFFSET_CONTROL]);

    if let Ok(mut g) = SUBVIEW.lock() { *g = current; }
}

// --- subviews ------------------------------------------------------------

fn render_overview(ui: &Ui, fight: &FightData, idx: usize, derived: &Derived) {
    use crate::pulse_metrics::*;

    let p = &fight.players[idx];
    let dmg = damage(p);
    let dps_v = dps_value(p);
    let dc = down_contribution(p);
    let cl = cleanses(p);
    let st = strips(p);
    let dt = damage_taken(p);
    let d_to_tag = dist_to_tag(p);
    let deaths_n = deaths(p);
    let downs_n = downs(p);

    hero_banner(ui,
        "DAMAGE DEALT", series::METRIC_DAMAGE,
        &format_damage(dmg),
        &format!("{} DPS", format_damage(dps_v)),
        derived.rank_damage.map(|r| format!("{} in squad", ordinal(r))).as_deref(),
    );
    ui.dummy([0.0, 2.0]);

    let cells = [
        ("DOWN CONTRIBUTION", series::METRIC_DOWN,
            format_damage(dc),
            derived.rank_down_contribution.map(ordinal)),
        ("DEATHS / DOWNS", if deaths_n == 0 { series::METRIC_SUCCESS } else { series::METRIC_DAMAGE },
            format!("{deaths_n} / {downs_n}"), None),
        ("STRIPS", series::METRIC_SUPPORT,
            st.to_string(),
            derived.rank_strips.map(ordinal)),
        ("CLEANSES", series::METRIC_CLEANSE,
            cl.to_string(),
            derived.rank_cleanses.map(ordinal)),
        ("DAMAGE TAKEN", series::METRIC_DEFEND,
            format_damage(dt),
            derived.rank_damage_taken.map(ordinal)),
        // `None` means no distance was measured at all -- never a 0,
        // which would read as "stacked on the tag".
        ("DISTANCE TO TAG", series::METRIC_NEUTRAL,
            d_to_tag.map(|d| format!("{d:.0}")).unwrap_or_else(|| "—".into()),
            None),
    ];
    draw_2col_card_grid(ui, &cells);
    ui.dummy([0.0, 8.0]);
    render_fight_composition(ui, derived);
}

fn render_damage(ui: &Ui, fight: &FightData, idx: usize, derived: &Derived) {
    use crate::pulse_metrics::*;
    let p = &fight.players[idx];
    let dmg = damage(p);
    let dps_v = dps_value(p);
    let dc = down_contribution(p);

    hero_banner(ui,
        "TOTAL DAMAGE", series::METRIC_DAMAGE,
        &format_damage(dmg),
        &format!("{} DPS", format_damage(dps_v)),
        derived.rank_damage.map(|r| format!("{} in squad", ordinal(r))).as_deref(),
    );
    ui.dummy([0.0, 2.0]);

    let cells = [
        ("DOWN CONTRIBUTION", series::METRIC_DOWN, format_damage(dc),
            derived.rank_down_contribution.map(ordinal)),
    ];
    draw_2col_card_grid(ui, &cells);

    ui.dummy([0.0, 6.0]);
    let skills = &derived.top_damage;
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
        // The name is resolved from `catalogs.skills` at projection
        // time (`top_skills::skill_label`), so there is no per-frame
        // skill-map lookup left to do here.
        draw_skill_bar(ui, fight, i, entry.id, &entry.name, frac, pct, &format_damage(entry.damage));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupportMode { Healing, Downed, Barrier }

static SUPPORT_MODE: Lazy<Mutex<SupportMode>> = Lazy::new(|| Mutex::new(SupportMode::Healing));

fn render_support(ui: &Ui, fight: &FightData, idx: usize, derived: &Derived, accent: [f32; 4]) {
    use crate::pulse_metrics::*;

    let p = &fight.players[idx];
    let st = strips(p);
    let cl = cleanses(p);
    let cl_self = cleanse_self(p);
    // Log-wide, not per-player: `healing_available` is false when this
    // LOG was recorded without the arcdps healing addon, in which case
    // `healing_out`/`barrier_out`/`downed_healing_out` are a structural
    // zero standing in for "unknown". Showing that zero would claim a
    // measurement nobody took, so the not-available layout runs instead.
    let has_heal = fight.healing_available;
    let heal = healing(p);
    let heal_hps = hps(p, fight.duration_ms);
    let heal_downed = healing_downed(p);
    let barr = barrier(p);
    let inc_heal = incoming_healing(p);

    if has_heal && heal > 0 {
        hero_banner(ui,
            "HEALING OUTPUT", series::METRIC_SUPPORT,
            &format_damage(heal),
            &format!("{} HPS", format_damage(heal_hps)),
            None,
        );
    } else {
        hero_banner(ui,
            "BOON STRIPS", series::METRIC_SUPPORT,
            &st.to_string(), "",
            derived.rank_strips.map(|r| format!("{} in squad", ordinal(r))).as_deref(),
        );
    }
    ui.dummy([0.0, 2.0]);

    let cells: Vec<(&str, [f32; 4], String, Option<String>)> = if has_heal {
        vec![
            ("BARRIER",       series::METRIC_BARRIER, format_damage(barr),        None),
            ("DOWNED HEALING",series::METRIC_DOWN,    format_damage(heal_downed), None),
            ("STRIPS",        series::METRIC_SUPPORT, st.to_string(),             derived.rank_strips.map(ordinal)),
            ("CLEANSES",      series::METRIC_CLEANSE, cl.to_string(),             derived.rank_cleanses.map(ordinal)),
            ("SELF CLEANSE",  series::METRIC_CLEANSE, cl_self.to_string(),        None),
            ("INCOMING HEAL", series::METRIC_SUCCESS, format_damage(inc_heal),    None),
        ]
    } else {
        vec![
            ("CLEANSES",      series::METRIC_CLEANSE, cl.to_string(),      derived.rank_cleanses.map(ordinal)),
            ("SELF CLEANSE",  series::METRIC_CLEANSE, cl_self.to_string(), None),
            ("STRIPS / SEC",  series::METRIC_SUPPORT,
                format!("{:.2}", st as f64 / (fight.duration_ms.max(1) as f64 / 1000.0)),
                None),
            ("INCOMING HEAL", series::METRIC_SUCCESS,
                "no addon".to_string(), None),
        ]
    };
    draw_2col_card_grid(ui, &cells);

    if !has_heal {
        ui.dummy([0.0, 4.0]);
        ui.text_disabled("Per-skill heal / barrier breakdowns require the arcdps");
        ui.text_disabled("healing addon (arcdps_healing_stats.dll).");
        return;
    }

    ui.dummy([0.0, 6.0]);
    render_support_mode_toggle(ui, accent);

    let mode = SUPPORT_MODE.lock().ok().map(|g| *g).unwrap_or(SupportMode::Healing);
    section_label(ui, match mode {
        SupportMode::Healing => "TOP HEALING SKILLS",
        SupportMode::Downed  => "TOP DOWNED-HEALING SKILLS",
        SupportMode::Barrier => "TOP BARRIER SKILLS",
    });

    match mode {
        SupportMode::Healing => {
            render_value_bars(ui, fight,
                &derived.top_healing.iter().map(|s| (s.id, s.name.as_str(), s.healing)).collect::<Vec<_>>(),
                "heal", series::METRIC_SUPPORT);
        }
        SupportMode::Downed => {
            render_value_bars(ui, fight,
                &derived.top_downed_healing.iter().map(|s| (s.id, s.name.as_str(), s.downed_healing)).collect::<Vec<_>>(),
                "down", series::METRIC_DOWN);
        }
        SupportMode::Barrier => {
            render_value_bars(ui, fight,
                &derived.top_barrier.iter().map(|s| (s.id, s.name.as_str(), s.barrier)).collect::<Vec<_>>(),
                "barr", series::METRIC_BARRIER);
        }
    }
}

fn render_support_mode_toggle(ui: &Ui, accent: [f32; 4]) {
    let mut current = SUPPORT_MODE.lock().ok().map(|g| *g).unwrap_or(SupportMode::Healing);
    let pad = [12.0_f32, 5.0_f32];
    let h = ui.text_line_height() + pad[1] * 2.0;

    let origin = ui.cursor_screen_pos();
    let mut x = origin[0];
    for (label, mode) in [
        ("Healing", SupportMode::Healing),
        ("Downed",  SupportMode::Downed),
        ("Barrier", SupportMode::Barrier),
    ].iter() {
        let (clicked, w) = axi::chip(
            ui,
            [x, origin[1]],
            &format!("support-mode-{label}"),
            label,
            current == *mode,
            accent,
            pad,
        );
        if clicked { current = *mode; }
        x += w + theme::OFFSET_CONTROL + 5.0;
    }
    ui.set_cursor_screen_pos(origin);
    ui.dummy([x - origin[0], h + theme::OFFSET_CONTROL]);

    if let Ok(mut g) = SUPPORT_MODE.lock() { *g = current; }
}

/// Renders a stack of value bars from `(id, value)` pairs, looking up
/// names via `resolve_skill_name`. Generalises the damage-skill bar
/// renderer for any non-negative numeric value.
fn render_value_bars(
    ui: &Ui,
    fight: &FightData,
    rows: &[(u32, &str, u64)],
    id_prefix: &str,
    bar_color: [f32; 4],
) {
    if rows.is_empty() {
        ui.text_disabled("No skills recorded.");
        return;
    }
    let max = rows.first().map(|r| r.2).unwrap_or(1).max(1);
    let total: u64 = rows.iter().map(|r| r.2).sum();
    for (i, (id, name, value)) in rows.iter().enumerate() {
        let frac = *value as f32 / max as f32;
        let pct = if total > 0 { *value as f64 / total as f64 * 100.0 } else { 0.0 };
        draw_value_bar(ui, fight, id_prefix, i, *id, name, frac, pct, &format_damage(*value), bar_color);
    }
}

fn draw_value_bar(
    ui: &Ui,
    fight: &FightData,
    id_prefix: &str,
    row_idx: usize,
    id: u32,
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

    let icon = lookup(fight, IconKey { kind: IconKind::Skill, id });

    {
        let track = Rect::at(cursor, [avail, row_h]);
        axi::bar(ui, track, frac, bar_color);

        let draw = ui.get_window_draw_list();
        let pad_left = 6.0 + theme::BORDER_CONTROL;
        let mut text_x = cursor[0] + pad_left;
        if let Some(handle) = icon {
            // Full-size artwork: the outline stays where it is, and the
            // image is NOT inset by its thickness. Insetting all four
            // sides took a 20px icon to 14px in a 24px row, and these
            // are identified at a glance mid-fight.
            let icon_h = row_h - 4.0;
            let icon_w = (icon_h * handle.aspect).max(1.0);
            let icon_y = cursor[1] + 2.0;
            draw.add_image(handle.tex, [text_x, icon_y], [text_x + icon_w, icon_y + icon_h]).build();
            text_x += icon_w + 6.0;
        }
        let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
        draw.add_text([text_x, text_y], theme::TEXT, name);

        let pad_right = 10.0 + theme::BORDER_CONTROL;
        let pct_label = if pct >= 0.1 { format!("{:.1}%", pct) } else { String::new() };
        let val_w = ui.calc_text_size(value)[0];
        let pct_w = ui.calc_text_size(&pct_label)[0];
        draw.add_text([cursor[0] + avail - pad_right - val_w, text_y], theme::TEXT, value);
        if !pct_label.is_empty() {
            draw.add_text(
                [cursor[0] + avail - pad_right - val_w - 14.0 - pct_w, text_y],
                theme::TEXT_DIM, &pct_label,
            );
        }
    }

    ui.set_cursor_screen_pos(cursor);
    ui.invisible_button(format!("##{id_prefix}-{row_idx}-{id}"), [avail, row_h]);
}

fn render_defense(ui: &Ui, fight: &FightData, idx: usize) {
    use crate::pulse_metrics::*;
    use crate::squad_rank::{rank_in_squad, RankMetric};

    let p = &fight.players[idx];
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

    let dt_rank = rank_in_squad(fight, idx, RankMetric::DamageTaken);
    hero_banner(ui,
        "DAMAGE TAKEN", series::METRIC_DEFEND,
        &format_damage(dt), "",
        dt_rank.map(|r| format!("{} in squad", ordinal(r))).as_deref(),
    );
    ui.dummy([0.0, 2.0]);

    let alive_color = if deaths_n == 0 { series::METRIC_SUCCESS } else { series::METRIC_DAMAGE };
    let cells = [
        ("DEATHS / DOWNS",  alive_color,            format!("{deaths_n} / {downs_n}"), None),
        ("DODGES",          series::METRIC_NEUTRAL, dodges_n.to_string(),              None),
        ("INCOMING CC",     series::METRIC_DEFEND,  cc_in.to_string(),                 None),
        ("INCOMING STRIPS", series::METRIC_DOWN,    strips_in.to_string(),             None),
    ];
    draw_2col_card_grid(ui, &cells);

    ui.dummy([0.0, 6.0]);
    section_label(ui, "MITIGATION");
    let mit_cells = [
        ("BLOCKED",     series::METRIC_CLEANSE, blocked_n.to_string(),     None),
        ("EVADED",      series::METRIC_SUPPORT, evaded_n.to_string(),      None),
        ("MISSED",      series::METRIC_NEUTRAL, missed_n.to_string(),      None),
        ("INVULNED",    series::METRIC_DEFEND,  invulned_n.to_string(),    None),
        ("INTERRUPTED", series::METRIC_DAMAGE,  interrupted_n.to_string(), None),
    ];
    draw_2col_card_grid(ui, &mit_cells);
}

fn render_boons(ui: &Ui, fight: &FightData, _idx: usize, derived: &Derived) {
    use crate::boon_uptime::BoonStacking;

    if derived.boon_uptimes.is_empty() {
        ui.text_disabled("No boon uptimes recorded for this fight.");
        return;
    }
    section_label(ui, "BOON UPTIME");
    for boon in &derived.boon_uptimes {
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
        draw_boon_bar(ui, fight, boon.id, boon.name, frac, &label, series::boon(boon.name));
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

fn section_label(ui: &Ui, label: &str) {
    axi::label(ui, label);
    ui.dummy([0.0, 1.0]);
}

/// Tall headline card with a big primary value, secondary metric, and
/// optional bottom-right rank footer. `ink` is the metric's own colour:
/// a domain palette entry, never the chrome accent.
fn hero_banner(
    ui: &Ui,
    label: &str,
    ink: [f32; 4],
    primary: &str,
    secondary: &str,
    rank: Option<&str>,
) {
    let avail = ui.content_region_avail()[0].max(120.0);
    let cursor = ui.cursor_screen_pos();
    let x = cursor[0];
    let y = cursor[1];

    let r = Rect::at([x, y], [avail, HERO_H]);
    // A display card: no hover lift. A hover state on something you
    // cannot click is a lie about affordance.
    axi::card(ui, r, theme::SURFACE_RAISED, false);

    let draw = ui.get_window_draw_list();
    // Accent stripe on the left edge, inside our own outline.
    let stripe_x = x + theme::BORDER_CONTROL;
    draw.add_rect(
        [stripe_x, y + theme::BORDER_CONTROL],
        [stripe_x + 4.0, y + HERO_H - theme::BORDER_CONTROL],
        ink,
    ).filled(true).build();

    let pad_x = 16.0 + theme::BORDER_CONTROL;
    let pad_y = 10.0 + theme::BORDER_CONTROL;
    let line_h = ui.text_line_height();

    // Uppercase eyebrow: with one font face at one size, case and
    // colour are the whole weight hierarchy available.
    draw.add_text([x + pad_x, y + pad_y], ink, label.to_uppercase());

    let primary_y = y + pad_y + line_h + 8.0;
    draw.add_text([x + pad_x, primary_y], theme::TEXT, primary);

    if !secondary.is_empty() {
        let sec_w = ui.calc_text_size(secondary)[0];
        draw.add_text([x + avail - pad_x - sec_w, primary_y + 2.0], theme::TEXT_DIM, secondary);
    }
    if let Some(rk) = rank {
        let rw = ui.calc_text_size(rk)[0];
        draw.add_text(
            [x + avail - pad_x - rw, y + HERO_H - pad_y - line_h],
            theme::TEXT_FAINT, rk,
        );
    }

    ui.dummy([avail, HERO_H + theme::OFFSET_CONTROL]);
}

/// Two-column grid of stat cards. Each tuple: (label, ink, value, rank),
/// where `ink` is the metric's own colour.
fn draw_2col_card_grid(
    ui: &Ui,
    items: &[(&str, [f32; 4], String, Option<String>)],
) {
    let avail = ui.content_region_avail()[0].max(120.0);
    let col_w = (avail - GAP) / 2.0;
    let cursor = ui.cursor_screen_pos();
    let start_x = cursor[0];
    let start_y = cursor[1];
    let line_h = ui.text_line_height();

    for (i, (label, ink, value, rank)) in items.iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        let x = start_x + col as f32 * (col_w + GAP);
        let y = start_y + row as f32 * (CARD_H + GAP);

        // A display card: not clickable, so no hover lift.
        axi::card(ui, Rect::at([x, y], [col_w, CARD_H]), theme::SURFACE_RAISED, false);

        let draw = ui.get_window_draw_list();
        // Left metric stripe, inside our own outline.
        let stripe_x = x + theme::BORDER_CONTROL;
        draw.add_rect(
            [stripe_x, y + theme::BORDER_CONTROL],
            [stripe_x + 3.0, y + CARD_H - theme::BORDER_CONTROL],
            *ink,
        ).filled(true).build();

        let pad_x = 12.0 + theme::BORDER_CONTROL;
        let pad_y = 8.0 + theme::BORDER_CONTROL;
        draw.add_text([x + pad_x, y + pad_y], *ink, *label);
        draw.add_text(
            [x + pad_x, y + pad_y + line_h + 4.0],
            theme::TEXT, value.as_str(),
        );
        if let Some(r) = rank {
            let rw = ui.calc_text_size(r)[0];
            draw.add_text(
                [x + col_w - pad_x - rw, y + CARD_H - pad_y - line_h],
                theme::TEXT_FAINT, r.as_str(),
            );
        }
    }

    let rows = (items.len() + 1) / 2;
    let total_h = rows as f32 * CARD_H + rows.saturating_sub(1) as f32 * GAP;
    ui.dummy([avail, total_h + theme::OFFSET_CONTROL]);
}

/// Full-width row with a damage-coloured backing bar and skill label,
/// percentage, and right-aligned damage value.
fn draw_skill_bar(
    ui: &Ui,
    fight: &FightData,
    row_idx: usize,
    id: u32,
    name: &str,
    frac: f32,
    pct: f64,
    value: &str,
) {
    use crate::ui::icons::{lookup, IconKey, IconKind};
    let avail = ui.content_region_avail()[0].max(120.0);
    let row_h = (ui.text_line_height() * 1.55).max(24.0);
    let cursor = ui.cursor_screen_pos();
    let icon = lookup(fight, IconKey { kind: IconKind::Skill, id });

    {
        let track = Rect::at(cursor, [avail, row_h]);
        axi::bar(ui, track, frac, series::METRIC_DAMAGE);

        let draw = ui.get_window_draw_list();
        let pad_left = 6.0 + theme::BORDER_CONTROL;
        let mut text_x = cursor[0] + pad_left;
        if let Some(handle) = icon {
            // Full-size artwork: the outline stays where it is, and the
            // image is NOT inset by its thickness. Insetting all four
            // sides took a 20px icon to 14px in a 24px row, and these
            // are identified at a glance mid-fight.
            let icon_h = row_h - 4.0;
            let icon_w = (icon_h * handle.aspect).max(1.0);
            let icon_y = cursor[1] + 2.0;
            draw.add_image(handle.tex, [text_x, icon_y], [text_x + icon_w, icon_y + icon_h]).build();
            text_x += icon_w + 6.0;
        }
        let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
        draw.add_text([text_x, text_y], theme::TEXT, name);

        let pad_right = 10.0 + theme::BORDER_CONTROL;
        let pct_label = if pct >= 0.1 { format!("{:.1}%", pct) } else { String::new() };
        let val_w = ui.calc_text_size(value)[0];
        let pct_w = ui.calc_text_size(&pct_label)[0];
        draw.add_text([cursor[0] + avail - pad_right - val_w, text_y], theme::TEXT, value);
        if !pct_label.is_empty() {
            draw.add_text(
                [cursor[0] + avail - pad_right - val_w - 14.0 - pct_w, text_y],
                theme::TEXT_DIM, &pct_label,
            );
        }
    }

    ui.set_cursor_screen_pos(cursor);
    ui.invisible_button(format!("##sk-{row_idx}-{id}"), [avail, row_h]);
}

fn draw_boon_bar(ui: &Ui, fight: &FightData, id: u32, name: &str, frac: f32, label: &str, color: [f32; 4]) {
    use crate::ui::icons::{lookup, IconKey, IconKind};
    let avail = ui.content_region_avail()[0].max(120.0);
    let row_h = (ui.text_line_height() * 1.55).max(24.0);
    let cursor = ui.cursor_screen_pos();
    let icon = lookup(fight, IconKey { kind: IconKind::Buff, id });

    {
        let track = Rect::at(cursor, [avail, row_h]);
        axi::bar(ui, track, frac, color);

        let draw = ui.get_window_draw_list();
        let pad_left = 6.0 + theme::BORDER_CONTROL;
        let mut text_x = cursor[0] + pad_left;
        if let Some(handle) = icon {
            // Full-size artwork: the outline stays where it is, and the
            // image is NOT inset by its thickness. Insetting all four
            // sides took a 20px icon to 14px in a 24px row, and these
            // are identified at a glance mid-fight.
            let icon_h = row_h - 4.0;
            let icon_w = (icon_h * handle.aspect).max(1.0);
            let icon_y = cursor[1] + 2.0;
            draw.add_image(handle.tex, [text_x, icon_y], [text_x + icon_w, icon_y + icon_h]).build();
            text_x += icon_w + 6.0;
        } else {
            // No icon yet — keep the coloured stripe so the row still
            // has a visual identity for the boon. Inside our outline.
            let stripe_w = 4.0;
            let stripe_x = cursor[0] + 2.0 + theme::BORDER_CONTROL;
            draw.add_rect([stripe_x, cursor[1] + 6.0],
                          [stripe_x + stripe_w, cursor[1] + row_h - 6.0], color)
                .filled(true).build();
            text_x = cursor[0] + 14.0 + theme::BORDER_CONTROL;
        }
        let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
        draw.add_text([text_x, text_y], theme::TEXT, name);

        let pad_right = 14.0 + theme::BORDER_CONTROL;
        let label_w = ui.calc_text_size(label)[0];
        draw.add_text([cursor[0] + avail - pad_right - label_w, text_y],
                       theme::TEXT, label);
    }

    ui.set_cursor_screen_pos(cursor);
    ui.invisible_button(format!("##boon-{id}"), [avail, row_h]);
}

// --- fight composition card ----------------------------------------------

static COMP_SELECTED: Lazy<Mutex<Option<crate::fight_composition::GroupKey>>> =
    Lazy::new(|| Mutex::new(None));

fn render_fight_composition(ui: &Ui, derived: &Derived) {
    let groups = &derived.composition;
    if groups.is_empty() { return; }
    let total: u32 = groups.iter().map(|g| g.count).sum();
    if total == 0 { return; }

    section_label(ui, "FIGHT COMPOSITION");

    let mut selected = COMP_SELECTED.lock().ok().and_then(|g| g.clone());

    let avail = ui.content_region_avail()[0].max(120.0);
    let pill_h = 22.0;
    let pad_x = 8.0;
    let pad_between = 6.0;
    {
        let cursor = ui.cursor_screen_pos();
        let bar_h = 10.0;
        // A track with one segment per group drawn inside its outline.
        let track = Rect::at(cursor, [avail, bar_h]);
        {
            // Track only: this bar's quantity is carried by the segments
            // below, not by one fill, so it does not go through
            // `axi::bar` — a zero fraction there is an argument waiting
            // to be "corrected" into a full-width wash.
            let draw = ui.get_window_draw_list();
            draw.add_rect(track.min, track.max, theme::GROUND).filled(true).build();
            let path = axi::outline_path(track, theme::BORDER_CONTROL);
            if !path.is_degenerate() {
                draw.add_rect(path.min, path.max, theme::INK_LINE)
                    .thickness(theme::BORDER_CONTROL).build();
            }
            let inner = track.inset(theme::BORDER_CONTROL);
            let mut x = inner.min[0];
            let seg_gap = 2.0;
            let total_w = inner.w() - seg_gap * (groups.len() as f32 - 1.0).max(0.0);
            for g in groups.iter() {
                let w = total_w * (g.count as f32 / total as f32);
                draw.add_rect([x, inner.min[1]], [x + w, inner.max[1]], g.color)
                    .filled(true).build();
                x += w + seg_gap;
            }
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
            let bg = if active { theme::SURFACE_RAISED } else { theme::SURFACE };
            // The hit-test pass below lays an invisible button over this
            // exact rect, so the lift is honest: the pill is clickable.
            // `is_mouse_hovering_rect` clips but ignores window z-order,
            // so it is gated on THIS window being the hovered one — a
            // pill under the options window must not light up.
            let hovered = ui.is_window_hovered()
                && ui.is_mouse_hovering_rect([px, py], [px + pill_w, py + pill_h]);
            axi::card(ui, Rect::at([px, py], [pill_w, pill_h]), bg, hovered);
            axi::diamond(
                ui,
                [px + theme::BORDER_CONTROL + 7.0, py + pill_h * 0.5],
                8.0,
                g.color,
            );
            let draw = ui.get_window_draw_list();
            let text_y = py + (pill_h - ui.text_line_height()) * 0.5;
            let mut tx = px + pad_x + dot_w + token_gap;
            draw.add_text([tx, text_y], theme::TEXT, &count_str);
            tx += count_w + token_gap;
            draw.add_text([tx, text_y], theme::TEXT_DIM, label_str);
            tx += label_w + token_gap;
            draw.add_text([tx, text_y], theme::TEXT_FAINT, &pct);
            px += pill_w + pad_between;
        }
    }

    // Hit-test pass — uses real ImGui items (not set_cursor_screen_pos)
    // so the cursor flows naturally and End() can finalise the window.
    // The pills row's vertical span is reserved by ui.dummy below.
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
    // Reserve the pill row's vertical span via a regular item rather than
    // set_cursor_screen_pos. Manually-set absolute cursors after a series
    // of invisible_buttons crashed the host's ImGui End() on big fights.
    ui.set_cursor_screen_pos([cursor[0], py]);
    ui.dummy([avail, pill_h + 2.0 + theme::OFFSET_CONTROL]);

    // Expanded per-spec chips for the selected group.
    if let Some(key) = &selected {
        if let Some(g) = groups.iter().find(|g| &g.key == key) {
            if !g.class_counts.is_empty() {
                draw_class_chips(ui, &g.class_counts, g.color);
            }
        }
    }
}

fn draw_class_chips(ui: &Ui, chips: &[(String, u32)], ink: [f32; 4]) {
    use crate::ui::icons::lookup_bundled;
    let avail = ui.content_region_avail()[0].max(120.0);
    let cursor = ui.cursor_screen_pos();
    let chip_h = 22.0;
    let pad_x = 6.0;
    let gap = 4.0;
    let icon_gap = 4.0;
    let mut x = cursor[0];
    let mut y = cursor[1];
    let mut rows = 1u32;

    // Resolve icons first so we can measure widths accurately.
    struct Chip<'a> {
        spec: &'a str,
        count_str: String,
        spec_w: f32,
        icon: Option<crate::ui::icons::IconHandle>,
        chip_w: f32,
    }
    let icon_h = chip_h - 6.0;
    let mut prepared: Vec<Chip> = Vec::with_capacity(chips.len());
    for (spec, count) in chips {
        let icon = lookup_bundled(spec.as_str());
        let count_str = count.to_string();
        let spec_w = ui.calc_text_size(spec.as_str())[0];
        let count_w = ui.calc_text_size(&count_str)[0];
        let icon_w = icon.map(|h| (icon_h * h.aspect).max(1.0) + icon_gap).unwrap_or(0.0);
        let chip_w = pad_x + icon_w + spec_w + 6.0 + count_w + pad_x;
        prepared.push(Chip { spec: spec.as_str(), count_str, spec_w, icon, chip_w });
    }

    for (i, chip) in prepared.iter().enumerate() {
        if x + chip.chip_w > cursor[0] + avail {
            x = cursor[0];
            y += chip_h + gap;
            rows += 1;
        }
        axi::card(ui, Rect::at([x, y], [chip.chip_w, chip_h]), theme::SURFACE, false);
        {
            let draw = ui.get_window_draw_list();
            let mut text_x = x + pad_x;
            if let Some(handle) = chip.icon {
                let icon_w = (icon_h * handle.aspect).max(1.0);
                let icon_y = y + (chip_h - icon_h) * 0.5;
                draw.add_image(handle.tex, [text_x, icon_y], [text_x + icon_w, icon_y + icon_h]).build();
                text_x += icon_w + icon_gap;
            }
            let text_y = y + (chip_h - ui.text_line_height()) * 0.5;
            draw.add_text([text_x, text_y], ink, chip.spec);
            draw.add_text([text_x + chip.spec_w + 6.0, text_y], theme::TEXT, &chip.count_str);
        }
        ui.set_cursor_screen_pos([x, y]);
        let _ = ui.invisible_button(format!("##chip-{i}"), [chip.chip_w, chip_h]);
        x += chip.chip_w + gap;
    }
    let total_h = rows as f32 * chip_h + (rows.saturating_sub(1) as f32) * gap;
    ui.set_cursor_screen_pos([cursor[0], cursor[1]]);
    ui.dummy([avail, total_h + theme::OFFSET_CONTROL]);
}
