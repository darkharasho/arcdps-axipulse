#![cfg(windows)]
//! Single AxiPulse window. Hosts a fight picker, top-level tabs
//! (Pulse / Timeline), and dispatches to the corresponding content
//! renderer in `ui::pulse` or `ui::timeline`.

use std::sync::Mutex;

use arcdps::imgui::{Condition, StyleColor, StyleVar, Ui};
use once_cell::sync::Lazy;

use crate::config::Config;
use crate::self_identify::find_self_index;
use crate::state::AppState;

// --- palette (shared with pulse/timeline at a glance, kept local) -------

const BG_WINDOW:     [f32; 4] = [0.055, 0.065, 0.085, 0.92];
const TEXT_PRIMARY:  [f32; 4] = [0.97, 0.97, 1.00, 1.0];
const TEXT_SECONDARY:[f32; 4] = [0.78, 0.78, 0.85, 1.0];
const TEXT_MUTED:    [f32; 4] = [0.52, 0.54, 0.62, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopTab { Pulse, Timeline }

static TOP_TAB: Lazy<Mutex<TopTab>> = Lazy::new(|| Mutex::new(TopTab::Pulse));

/// Which fight is rendered in the body. `Latest` follows the most
/// recent parsed fight (auto-updates as new ones land); `History(i)`
/// pins to a specific past fight via `AppState::history(i)`.
#[derive(Debug, Clone, Copy)]
enum FightSel { Latest, History(usize) }

static FIGHT_SEL: Lazy<Mutex<FightSel>> = Lazy::new(|| Mutex::new(FightSel::Latest));

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
        ui.push_style_color(StyleColor::WindowBg,      BG_WINDOW),
        ui.push_style_color(StyleColor::TitleBg,       [0.055, 0.065, 0.085, 0.95]),
        ui.push_style_color(StyleColor::TitleBgActive, [0.085, 0.10,  0.13,  0.95]),
        ui.push_style_color(StyleColor::Separator,     [1.0, 1.0, 1.0, 0.06]),
        ui.push_style_color(StyleColor::Button,        [0.10, 0.12, 0.16, 1.0]),
        ui.push_style_color(StyleColor::ButtonHovered, [0.14, 0.17, 0.22, 1.0]),
        ui.push_style_color(StyleColor::ButtonActive,  [0.18, 0.22, 0.28, 1.0]),
    ];

    let mut window = ui.window("AxiPulse").size([720.0, 600.0], Condition::FirstUseEver);
    if let Some(pos) = config.pulse_pos {
        window = window.position([pos.0, pos.1], Condition::FirstUseEver);
    }
    let mut open = true;
    window.opened(&mut open).build(|| {
        render_header(ui, state);
        ui.dummy([0.0, 2.0]);

        // Resolve selected fight. If selection is stale (e.g. history is
        // shorter than the requested index), fall back to latest.
        let sel = FIGHT_SEL.lock().ok().map(|g| *g).unwrap_or(FightSel::Latest);
        let record = match sel {
            FightSel::Latest => state.current(),
            FightSel::History(i) => state.history(i).or_else(|| state.current()),
        };
        let Some(record) = record else {
            ui.text_disabled("Waiting for the first parsed fight\u{2026}");
            return;
        };
        let json = &record.data;
        let Some(idx) = find_self_index(json) else {
            ui.text_disabled("Could not identify local player in this fight.");
            return;
        };

        render_top_tabs(ui);
        ui.dummy([0.0, 4.0]);

        let tab = TOP_TAB.lock().ok().map(|g| *g).unwrap_or(TopTab::Pulse);
        let derived = record.derived.as_ref();
        match tab {
            TopTab::Pulse    => crate::ui::pulse::render_content(ui, json, idx, derived),
            TopTab::Timeline => crate::ui::timeline::render_content(ui, json, idx, derived, &mut config.timeline_layers),
        }
    });

    if !open {
        config.show_pulse = false;
        config.save();
    }

    for tok in color_tokens { tok.pop(); }
    for tok in style_tokens { tok.pop(); }
}

/// Header row: AxiPulse logo + brand label + (when parsing) a pulsing
/// indicator on the left, and the fight-picker combo right-aligned on
/// the same line.
fn render_header(ui: &Ui, state: &AppState) {
    let cursor = ui.cursor_screen_pos();
    let row_h = 28.0;
    let avail = ui.content_region_avail()[0].max(200.0);
    let combo_w = 380.0_f32.min(avail * 0.65);

    // --- Left content (logo + wordmark + optional parsing indicator) ---
    let logo = crate::ui::icons::lookup_bundled("__logo__");
    let mut x = cursor[0];
    if let Some(handle) = logo {
        let icon_h = row_h - 6.0;
        let icon_w = (icon_h * handle.aspect).max(1.0);
        let y = cursor[1] + 3.0;
        let draw = ui.get_window_draw_list();
        draw.add_image(handle.tex, [x, y], [x + icon_w, y + icon_h]).build();
        x += icon_w + 8.0;
    }
    let brand_text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
    let axi_w = ui.calc_text_size("Axi")[0];
    let pulse_w = ui.calc_text_size("Pulse")[0];
    {
        let draw = ui.get_window_draw_list();
        draw.add_text([x, brand_text_y], TEXT_PRIMARY, "Axi");
        draw.add_text([x + axi_w, brand_text_y], [0.31, 0.86, 0.61, 1.0], "Pulse");
    }
    let brand_end_x = x + axi_w + pulse_w;
    let is_parsing = crate::plugin::is_parsing();
    if is_parsing {
        render_parsing_pulse(ui, brand_end_x + 14.0, cursor[1] + row_h * 0.5, brand_text_y);
    }

    // --- Right-aligned fight picker on the same row ---
    let combo_x = cursor[0] + avail - combo_w;
    let combo_y = cursor[1] + (row_h - ui.frame_height_with_spacing()).max(0.0) * 0.5;
    ui.set_cursor_screen_pos([combo_x, combo_y]);
    ui.set_next_item_width(combo_w);
    render_fight_picker_combo(ui, state);

    // Park the cursor at the bottom of the row for downstream layout.
    ui.set_cursor_screen_pos([cursor[0], cursor[1] + row_h]);
}

/// Heartbeat icon (lucide Activity) pulsed in scale + alpha, mirroring
/// the `heartbeat-pulse` animation AxiPulse's web UI uses.
fn render_parsing_pulse(ui: &Ui, cx: f32, cy: f32, label_y: f32) {
    use std::time::Instant;
    static START: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(Instant::now);
    let t = START.elapsed().as_secs_f32();
    // Two quick blips per 1.1s cycle: one at phase=0.05, one at 0.22.
    let phase = (t / 1.1).fract();
    let beat = |centre: f32, sigma: f32| {
        let d = phase - centre;
        (-(d * d) / (2.0 * sigma * sigma)).exp()
    };
    let intensity = (beat(0.05, 0.05) + beat(0.22, 0.05)).clamp(0.0, 1.0);

    let base_size = 16.0_f32;
    let icon_size = base_size + 4.0 * intensity;
    let alpha = 0.55 + 0.45 * intensity;

    let icon = crate::ui::icons::lookup_bundled("__heartbeat__");

    let draw = ui.get_window_draw_list();
    if let Some(handle) = icon {
        let half = icon_size * 0.5;
        let x0 = cx - half;
        let y0 = cy - half;
        // Soft halo behind so the beat reads even on a busy backdrop.
        // Alpha-driven so it shrinks/expands with the beat.
        let halo_r = icon_size * 0.65 + 2.0 * intensity;
        let halo_color = [0.31, 0.86, 0.61, 0.10 + 0.25 * intensity * alpha];
        draw.add_rect(
            [cx - halo_r, cy - halo_r],
            [cx + halo_r, cy + halo_r],
            halo_color,
        ).filled(true).rounding(halo_r).build();
        // No tint on the texture itself — the vendored imgui binding's
        // image-tint path appears to crash the host under Wine when
        // exercised. The icon was rasterised already coloured #50dba0
        // so untinted is fine.
        draw.add_image(
            handle.tex,
            [x0, y0],
            [x0 + icon_size, y0 + icon_size],
        ).build();
    } else {
        // Bundled icon not loaded yet (D3D11 device unavailable on the
        // first frame). Fall back to the simple dot so we still show
        // *some* parsing indicator.
        let r = 5.0 + 2.0 * intensity;
        let dot_color = [0.31, 0.86, 0.61, alpha];
        draw.add_rect([cx - r, cy - r], [cx + r, cy + r], dot_color)
            .filled(true).rounding(r).build();
    }

    // "parsing…" label to the right of the icon, slightly muted, alpha
    // pulses with the beat.
    let label = "parsing\u{2026}";
    let mut text_color = TEXT_MUTED;
    text_color[3] = 0.60 + 0.35 * intensity;
    draw.add_text([cx + base_size * 0.6 + 8.0, label_y], text_color, label);
}

/// Combo dropdown listing "Latest" + each entry in `AppState.history`,
/// newest-history-first. Selecting an entry pins the view to that
/// fight. Caller positions and sizes the combo via `set_cursor_screen_pos`
/// + `set_next_item_width` before calling.
fn render_fight_picker_combo(ui: &Ui, state: &AppState) {
    let mut sel = FIGHT_SEL.lock().ok().map(|g| *g).unwrap_or(FightSel::Latest);
    let history_len = state.history_len();

    // Build labels: "Latest", then history newest→oldest.
    let mut labels: Vec<String> = Vec::with_capacity(history_len + 1);
    let latest_label = match state.current() {
        Some(rec) => format!(
            "Latest \u{00b7} {} \u{00b7} {} \u{00b7} {} players",
            mmss(rec.data.duration_ms),
            short_fight_name(&rec.data.fight_name),
            rec.data.players.len(),
        ),
        None => "Latest".to_string(),
    };
    labels.push(latest_label);
    for offset in 0..history_len {
        let i = history_len - 1 - offset;
        if let Some(rec) = state.history(i) {
            // F1 = oldest fight in history, FN = most recent past fight.
            let fight_no = i + 1;
            labels.push(format!(
                "F{}  \u{00b7} {} \u{00b7} {} \u{00b7} {} players",
                fight_no,
                mmss(rec.data.duration_ms),
                short_fight_name(&rec.data.fight_name),
                rec.data.players.len(),
            ));
        }
    }

    let mut current_idx: usize = match sel {
        FightSel::Latest => 0,
        FightSel::History(i) if i < history_len => history_len - i,
        _ => 0,
    };

    let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    if ui.combo_simple_string("##fight-picker", &mut current_idx, &label_refs) {
        sel = if current_idx == 0 {
            FightSel::Latest
        } else {
            let offset_from_newest = current_idx - 1;
            FightSel::History(history_len.saturating_sub(1).saturating_sub(offset_from_newest))
        };
    }

    if let Ok(mut g) = FIGHT_SEL.lock() { *g = sel; }
}

fn render_top_tabs(ui: &Ui) {
    let mut current = TOP_TAB.lock().ok().map(|g| *g).unwrap_or(TopTab::Pulse);
    for (i, (label, tab)) in [("Pulse", TopTab::Pulse), ("Timeline", TopTab::Timeline)].iter().enumerate() {
        let selected = current == *tab;
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
        if ui.button(label) { current = *tab; }
        for t in tokens { t.pop(); }
        if i + 1 < 2 { ui.same_line(); }
    }
    if let Ok(mut g) = TOP_TAB.lock() { *g = current; }
}

fn mmss(ms: u64) -> String {
    let sec = ms / 1000;
    format!("{}:{:02}", sec / 60, sec % 60)
}

/// Strip the "Detailed WvW - " prefix EI puts on WvW logs.
fn short_fight_name(name: &str) -> String {
    name.strip_prefix("Detailed WvW - ").unwrap_or(name).to_string()
}
