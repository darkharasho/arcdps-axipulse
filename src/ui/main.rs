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
        match tab {
            TopTab::Pulse    => crate::ui::pulse::render_content(ui, json, idx),
            TopTab::Timeline => crate::ui::timeline::render_content(ui, json, idx, &mut config.timeline_layers),
        }
    });

    if !open {
        config.show_pulse = false;
        config.save();
    }

    for tok in color_tokens { tok.pop(); }
    for tok in style_tokens { tok.pop(); }
}

/// Header row: AxiPulse logo + brand label + fight picker dropdown.
fn render_header(ui: &Ui, state: &AppState) {
    let cursor = ui.cursor_screen_pos();
    let row_h = 28.0;
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
    {
        let draw = ui.get_window_draw_list();
        draw.add_text([x, brand_text_y], TEXT_PRIMARY, "Axi");
        let axi_w = ui.calc_text_size("Axi")[0];
        draw.add_text([x + axi_w, brand_text_y], [0.31, 0.86, 0.61, 1.0], "Pulse");
    }
    // Advance ImGui's layout past the header decorations so the
    // fight-picker combo lays out cleanly below.
    ui.dummy([0.0, row_h]);
    render_fight_picker(ui, state);
}

/// Combo dropdown listing "Latest" + each entry in `AppState.history`,
/// newest-history-first. Selecting an entry pins the view to that fight.
fn render_fight_picker(ui: &Ui, state: &AppState) {
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
        // Newest history entry is at index `history_len - 1`; iterate
        // backwards so the combo lists most-recent-historical first.
        let i = history_len - 1 - offset;
        if let Some(rec) = state.history(i) {
            labels.push(format!(
                "-{}  \u{00b7} {} \u{00b7} {} \u{00b7} {} players",
                offset + 1,
                mmss(rec.data.duration_ms),
                short_fight_name(&rec.data.fight_name),
                rec.data.players.len(),
            ));
        }
    }

    let mut current_idx: usize = match sel {
        FightSel::Latest => 0,
        FightSel::History(i) if i < history_len => history_len - i, // combo offset
        _ => 0,
    };

    ui.text_colored(TEXT_SECONDARY, "Fight");
    ui.same_line();
    let avail = ui.content_region_avail()[0].max(120.0);
    ui.set_next_item_width(avail);
    let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    if ui.combo_simple_string("##fight-picker", &mut current_idx, &label_refs) {
        sel = if current_idx == 0 {
            FightSel::Latest
        } else {
            // combo offset 1 → newest historical = history_len - 1
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
