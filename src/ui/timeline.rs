#![cfg(windows)]
//! Timeline window — six stacked swim-lanes over the local player's
//! last fight. Lanes are independently toggleable via the layer panel.

use arcdps::imgui::{Condition, StyleColor, StyleVar, Ui};

use crate::config::Config;
use crate::ei_model::EiJson;
use crate::self_identify::find_self_index;
use crate::state::AppState;

const BG_WINDOW:     [f32; 4] = [0.055, 0.065, 0.085, 0.92];
const BG_CARD:       [f32; 4] = [0.085, 0.10,  0.13,  0.95];
const BG_CARD_BORDER:[f32; 4] = [1.0, 1.0, 1.0, 0.06];
const TEXT_MUTED:    [f32; 4] = [0.52, 0.54, 0.62, 1.0];

const COLOR_HEALTH: [f32; 4] = [0.29, 0.86, 0.50, 1.0];
const COLOR_DMG:    [f32; 4] = [0.95, 0.38, 0.38, 1.0];
const COLOR_TAKEN:  [f32; 4] = [0.97, 0.55, 0.42, 1.0];
const COLOR_DIST:   [f32; 4] = [0.95, 0.75, 0.40, 1.0];
const COLOR_OFF:    [f32; 4] = [0.42, 0.65, 0.94, 1.0];
const COLOR_DEF:    [f32; 4] = [0.32, 0.78, 0.92, 1.0];

const LANE_LABEL_W: f32 = 92.0;
const LANE_PAD_Y:   f32 = 2.0;
const AREA_LANE_H:  f32 = 48.0;
const BOON_ROW_H:   f32 = 12.0;
const BOON_GAP:     f32 = 2.0;

pub fn render(ui: &Ui, state: &AppState, config: &mut Config) {
    if !config.show_timeline { return; }

    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([12.0, 10.0])),
        ui.push_style_var(StyleVar::WindowRounding(10.0)),
        ui.push_style_var(StyleVar::WindowBorderSize(0.0)),
        ui.push_style_var(StyleVar::ItemSpacing([8.0, 8.0])),
    ];
    let color_tokens = [
        ui.push_style_color(StyleColor::WindowBg,      BG_WINDOW),
        ui.push_style_color(StyleColor::TitleBg,       [0.055, 0.065, 0.085, 0.95]),
        ui.push_style_color(StyleColor::TitleBgActive, [0.085, 0.10,  0.13,  0.95]),
        ui.push_style_color(StyleColor::Separator,     [1.0, 1.0, 1.0, 0.06]),
    ];

    let mut window = ui.window("Timeline").size([720.0, 480.0], Condition::FirstUseEver);
    if let Some(pos) = config.timeline_pos {
        window = window.position([pos.0, pos.1], Condition::FirstUseEver);
    }
    let mut open = true;
    window.opened(&mut open).build(|| {
        let current = state.current();
        if current.is_none() {
            ui.text_disabled("Waiting for the first parsed fight\u{2026}");
            return;
        }
        let record = current.unwrap();
        let json = &record.data;
        let Some(idx) = find_self_index(json) else {
            ui.text_disabled("Could not identify local player in this fight.");
            return;
        };

        render_layer_toggles(ui, &mut config.timeline_layers);
        ui.separator();
        render_time_axis(ui, json.duration_ms);
        render_lanes(ui, json, idx, &config.timeline_layers);
    });

    if !open {
        config.show_timeline = false;
        config.save();
    }

    for tok in color_tokens { tok.pop(); }
    for tok in style_tokens { tok.pop(); }
}

fn render_layer_toggles(ui: &Ui, layers: &mut crate::config::TimelineLayers) {
    let pairs: [(&str, &mut bool); 6] = [
        ("Health",     &mut layers.health),
        ("Dmg Dealt",  &mut layers.damage_dealt),
        ("Dmg Taken",  &mut layers.damage_taken),
        ("Dist Tag",   &mut layers.distance_to_tag),
        ("Off Boons",  &mut layers.offensive_boons),
        ("Def Boons",  &mut layers.defensive_boons),
    ];
    for (i, (label, value)) in pairs.into_iter().enumerate() {
        ui.checkbox(label, value);
        if i + 1 < 6 { ui.same_line(); }
    }
}

fn render_time_axis(ui: &Ui, duration_ms: u64) {
    let avail = ui.content_region_avail()[0].max(LANE_LABEL_W + 60.0);
    let cursor = ui.cursor_screen_pos();
    let data_x = cursor[0] + LANE_LABEL_W;
    let data_w = avail - LANE_LABEL_W;
    let draw = ui.get_window_draw_list();

    let tick_count = 5usize;
    for i in 0..tick_count {
        let frac = i as f32 / (tick_count - 1) as f32;
        let t_ms = ((i as u64) * duration_ms) / ((tick_count - 1) as u64).max(1);
        let x = data_x + data_w * frac;
        let label = format_mmss(t_ms);
        let w = ui.calc_text_size(&label)[0];
        let lx = if i == 0 { x } else if i + 1 == tick_count { x - w } else { x - w * 0.5 };
        draw.add_text([lx, cursor[1]], TEXT_MUTED, &label);
    }
    ui.dummy([avail, ui.text_line_height() + 4.0]);
}

fn render_lanes(ui: &Ui, json: &EiJson, idx: usize, layers: &crate::config::TimelineLayers) {
    use crate::timeline_buckets::{extract_damage_dealt, extract_damage_taken};
    use crate::timeline_health::sample_health_per_second;
    use crate::timeline_distance::distance_to_commander_per_second;
    use crate::timeline_boons::{offensive_boons, defensive_boons};

    let p = &json.players[idx];
    let dur = json.duration_ms;

    if layers.health {
        let samples: Vec<f32> = sample_health_per_second(p, dur).into_iter().map(|x| x as f32).collect();
        draw_area_lane(ui, "Health", COLOR_HEALTH, &samples, 100.0);
    }
    if layers.damage_dealt {
        let v: Vec<f32> = extract_damage_dealt(p).into_iter().map(|x| x as f32).collect();
        draw_area_lane_auto(ui, "Dmg Dealt", COLOR_DMG, &v);
    }
    if layers.damage_taken {
        let v: Vec<f32> = extract_damage_taken(p).into_iter().map(|x| x as f32).collect();
        draw_area_lane_auto(ui, "Dmg Taken", COLOR_TAKEN, &v);
    }
    if layers.distance_to_tag {
        let samples = distance_to_commander_per_second(json, idx, dur);
        if samples.is_empty() {
            draw_empty_lane(ui, "Dist Tag", COLOR_DIST, "no commander tagged");
        } else {
            let v: Vec<f32> = samples.into_iter().map(|x| x as f32).collect();
            draw_area_lane_auto(ui, "Dist Tag", COLOR_DIST, &v);
        }
    }
    if layers.offensive_boons {
        let series = offensive_boons(p, dur);
        draw_boon_lane(ui, "Off Boons", COLOR_OFF, &series, dur);
    }
    if layers.defensive_boons {
        let series = defensive_boons(p, dur);
        draw_boon_lane(ui, "Def Boons", COLOR_DEF, &series, dur);
    }
}

fn draw_area_lane_auto(ui: &Ui, label: &str, accent: [f32; 4], samples: &[f32]) {
    let max = samples.iter().copied().fold(1.0_f32, f32::max);
    draw_area_lane(ui, label, accent, samples, max);
}

fn draw_area_lane(ui: &Ui, label: &str, accent: [f32; 4], samples: &[f32], max: f32) {
    let avail = ui.content_region_avail()[0].max(LANE_LABEL_W + 60.0);
    let cursor = ui.cursor_screen_pos();
    let data_x = cursor[0] + LANE_LABEL_W;
    let data_w = avail - LANE_LABEL_W;
    let y = cursor[1];
    let h = AREA_LANE_H;
    let draw = ui.get_window_draw_list();

    let label_w = ui.calc_text_size(label)[0];
    draw.add_text([cursor[0] + LANE_LABEL_W - label_w - 6.0, y + (h - ui.text_line_height()) * 0.5], accent, label);
    draw.add_rect([data_x, y], [data_x + data_w, y + h], BG_CARD).filled(true).rounding(4.0).build();
    draw.add_rect([data_x, y], [data_x + data_w, y + h], BG_CARD_BORDER).rounding(4.0).build();

    if samples.len() >= 2 && max > 0.0 {
        let n = samples.len();
        let mut fill = accent; fill[3] = 0.30;
        let mut prev_x = data_x;
        let mut prev_y = y + h - (samples[0] / max).clamp(0.0, 1.0) * (h - 4.0) - 2.0;
        for i in 1..n {
            let x = data_x + data_w * (i as f32 / (n - 1) as f32);
            let v = (samples[i] / max).clamp(0.0, 1.0);
            let yv = y + h - v * (h - 4.0) - 2.0;
            // Fill trapezoid below curve via two triangles.
            draw.add_triangle([prev_x, y + h], [x, y + h], [x, yv], fill).filled(true).build();
            draw.add_triangle([prev_x, y + h], [x, yv], [prev_x, prev_y], fill).filled(true).build();
            draw.add_line([prev_x, prev_y], [x, yv], accent).thickness(1.2).build();
            prev_x = x;
            prev_y = yv;
        }
    }
    ui.dummy([avail, h + LANE_PAD_Y]);
}

fn draw_empty_lane(ui: &Ui, label: &str, accent: [f32; 4], reason: &str) {
    let avail = ui.content_region_avail()[0].max(LANE_LABEL_W + 60.0);
    let cursor = ui.cursor_screen_pos();
    let data_x = cursor[0] + LANE_LABEL_W;
    let data_w = avail - LANE_LABEL_W;
    let y = cursor[1];
    let h = AREA_LANE_H;
    let draw = ui.get_window_draw_list();

    let label_w = ui.calc_text_size(label)[0];
    draw.add_text([cursor[0] + LANE_LABEL_W - label_w - 6.0, y + (h - ui.text_line_height()) * 0.5], accent, label);
    draw.add_rect([data_x, y], [data_x + data_w, y + h], BG_CARD).filled(true).rounding(4.0).build();
    draw.add_rect([data_x, y], [data_x + data_w, y + h], BG_CARD_BORDER).rounding(4.0).build();
    let rw = ui.calc_text_size(reason)[0];
    draw.add_text([data_x + (data_w - rw) * 0.5, y + (h - ui.text_line_height()) * 0.5], TEXT_MUTED, reason);
    ui.dummy([avail, h + LANE_PAD_Y]);
}

fn draw_boon_lane(
    ui: &Ui,
    label: &str,
    accent: [f32; 4],
    series: &[crate::timeline_boons::BoonSeries],
    duration_ms: u64,
) {
    let avail = ui.content_region_avail()[0].max(LANE_LABEL_W + 60.0);
    let cursor = ui.cursor_screen_pos();
    let data_x = cursor[0] + LANE_LABEL_W;
    let data_w = avail - LANE_LABEL_W;
    let y = cursor[1];
    let h_calc = (series.len() as f32) * (BOON_ROW_H + BOON_GAP) + 4.0;
    let h = h_calc.max(AREA_LANE_H);
    let draw = ui.get_window_draw_list();

    let label_w = ui.calc_text_size(label)[0];
    draw.add_text([cursor[0] + LANE_LABEL_W - label_w - 6.0, y + (h - ui.text_line_height()) * 0.5], accent, label);
    draw.add_rect([data_x, y], [data_x + data_w, y + h], BG_CARD).filled(true).rounding(4.0).build();
    draw.add_rect([data_x, y], [data_x + data_w, y + h], BG_CARD_BORDER).rounding(4.0).build();

    if duration_ms == 0 {
        ui.dummy([avail, h + LANE_PAD_Y]);
        return;
    }

    let mut fill = accent; fill[3] = 0.55;
    for (row, s) in series.iter().enumerate() {
        let row_y = y + 2.0 + row as f32 * (BOON_ROW_H + BOON_GAP);
        for seg in &s.segments {
            let sx = data_x + data_w * (seg.start_ms.min(duration_ms) as f32 / duration_ms as f32);
            let ex = data_x + data_w * (seg.end_ms.min(duration_ms) as f32 / duration_ms as f32);
            if ex - sx < 1.0 { continue; }
            draw.add_rect([sx, row_y], [ex, row_y + BOON_ROW_H], fill).filled(true).rounding(2.0).build();
        }
        let name_w = ui.calc_text_size(s.name)[0];
        let nudge_y = (BOON_ROW_H - ui.text_line_height()).max(0.0) * 0.5;
        draw.add_text(
            [data_x + data_w - name_w - 4.0, row_y + nudge_y],
            accent, s.name,
        );
    }
    ui.dummy([avail, h + LANE_PAD_Y]);
}

fn format_mmss(ms: u64) -> String {
    let sec = ms / 1000;
    let m = sec / 60;
    let s = sec % 60;
    format!("{m}:{s:02}")
}
