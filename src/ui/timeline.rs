#![cfg(windows)]
//! Timeline tab content — six stacked swim-lanes + inspector cards.
//! Outer window lives in `ui::main`.

use arcdps::imgui::Ui;

use crate::ei_model::EiJson;

const BG_CARD:       [f32; 4] = [0.085, 0.10,  0.13,  0.95];
const BG_CARD_BORDER:[f32; 4] = [1.0, 1.0, 1.0, 0.06];
const TEXT_PRIMARY:  [f32; 4] = [0.97, 0.97, 1.00, 1.0];
const TEXT_SECONDARY:[f32; 4] = [0.78, 0.78, 0.85, 1.0];
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

/// Render the Timeline tab contents (no window — caller owns that).
pub fn render_content(
    ui: &Ui,
    json: &EiJson,
    idx: usize,
    layers: &mut crate::config::TimelineLayers,
) {
    use crate::timeline_boons::{defensive_boons, offensive_boons};
    use crate::timeline_buckets::{extract_damage_dealt, extract_damage_taken};
    use crate::timeline_distance::distance_to_commander_per_second;
    use crate::timeline_health::sample_health_per_second;

    render_layer_toggles(ui, layers);
    ui.separator();
    render_time_axis(ui, json.duration_ms);

    // Compute lane data once so the crosshair pass can re-sample without
    // doubling EI traversal.
    let p = &json.players[idx];
    let dur = json.duration_ms;
    let health = if layers.health { sample_health_per_second(p, dur) } else { Vec::new() };
    let dmg_dealt = if layers.damage_dealt { extract_damage_dealt(p) } else { Vec::new() };
    let dmg_taken = if layers.damage_taken { extract_damage_taken(p) } else { Vec::new() };
    let distance = if layers.distance_to_tag {
        distance_to_commander_per_second(json, idx, dur)
    } else { Vec::new() };
    let off = if layers.offensive_boons { offensive_boons(p, dur) } else { Vec::new() };
    let def = if layers.defensive_boons { defensive_boons(p, dur) } else { Vec::new() };

    let avail = ui.content_region_avail()[0].max(LANE_LABEL_W + 60.0);
    let lanes_origin = ui.cursor_screen_pos();
    let data_x = lanes_origin[0] + LANE_LABEL_W;
    let data_w = avail - LANE_LABEL_W;
    let lanes_top_y = lanes_origin[1];

    if layers.health {
        let v: Vec<f32> = health.iter().map(|x| *x as f32).collect();
        draw_area_lane(ui, "Health", COLOR_HEALTH, &v, 100.0);
    }
    if layers.damage_dealt {
        let v: Vec<f32> = dmg_dealt.iter().map(|x| *x as f32).collect();
        draw_area_lane_auto(ui, "Dmg Dealt", COLOR_DMG, &v);
    }
    if layers.damage_taken {
        let v: Vec<f32> = dmg_taken.iter().map(|x| *x as f32).collect();
        draw_area_lane_auto(ui, "Dmg Taken", COLOR_TAKEN, &v);
    }
    if layers.distance_to_tag {
        if distance.is_empty() {
            draw_empty_lane(ui, "Dist Tag", COLOR_DIST, "no commander tagged");
        } else {
            let v: Vec<f32> = distance.iter().map(|x| *x as f32).collect();
            draw_area_lane_auto(ui, "Dist Tag", COLOR_DIST, &v);
        }
    }
    if layers.offensive_boons {
        draw_boon_lane(ui, "Off Boons", COLOR_OFF, &off, dur);
    }
    if layers.defensive_boons {
        draw_boon_lane(ui, "Def Boons", COLOR_DEF, &def, dur);
    }

    let lanes_bottom_y = ui.cursor_screen_pos()[1];
    draw_hover_crosshair(
        ui, data_x, data_w, lanes_top_y, lanes_bottom_y, dur,
        layers, &health, &dmg_dealt, &dmg_taken, &distance, &off, &def,
    );

    ui.dummy([0.0, 6.0]);
    render_inspector(ui, json, idx);
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

#[allow(clippy::too_many_arguments)]
fn draw_hover_crosshair(
    ui: &Ui,
    data_x: f32,
    data_w: f32,
    top_y: f32,
    bottom_y: f32,
    duration_ms: u64,
    layers: &crate::config::TimelineLayers,
    health: &[f64],
    dmg_dealt: &[u64],
    dmg_taken: &[u64],
    distance: &[f64],
    off: &[crate::timeline_boons::BoonSeries],
    def: &[crate::timeline_boons::BoonSeries],
) {
    if !ui.is_mouse_hovering_rect([data_x, top_y], [data_x + data_w, bottom_y]) {
        return;
    }
    let mouse = ui.io().mouse_pos;
    if !mouse[0].is_finite() || data_w <= 0.0 { return; }
    let pct = ((mouse[0] - data_x) / data_w).clamp(0.0, 1.0);
    let t_ms = (pct as f64 * duration_ms as f64) as u64;

    {
        // Scope the draw list so it is released before draw_tooltip
        // (which re-acquires it) — imgui-rs panics if two are alive.
        let draw = ui.get_window_draw_list();
        draw.add_line([mouse[0], top_y], [mouse[0], bottom_y], [1.0, 1.0, 1.0, 0.35])
            .thickness(1.0).build();
    }

    // Build tooltip rows.
    let sample_idx = |arr_len: usize| -> Option<usize> {
        if arr_len == 0 { None } else { Some(((pct * (arr_len - 1).max(0) as f32) as usize).min(arr_len - 1)) }
    };
    let mut rows: Vec<(&'static str, [f32; 4], String)> = Vec::new();
    if layers.health {
        if let Some(i) = sample_idx(health.len()) {
            rows.push(("Health", COLOR_HEALTH, format!("{:.0}%", health[i])));
        }
    }
    if layers.damage_dealt {
        if let Some(i) = sample_idx(dmg_dealt.len()) {
            rows.push(("Dmg Dealt", COLOR_DMG, short_value(dmg_dealt[i])));
        }
    }
    if layers.damage_taken {
        if let Some(i) = sample_idx(dmg_taken.len()) {
            rows.push(("Dmg Taken", COLOR_TAKEN, short_value(dmg_taken[i])));
        }
    }
    if layers.distance_to_tag && !distance.is_empty() {
        if let Some(i) = sample_idx(distance.len()) {
            rows.push(("Dist Tag", COLOR_DIST, format!("{:.0}", distance[i])));
        }
    }
    if layers.offensive_boons {
        let active: Vec<&str> = off.iter()
            .filter(|s| s.segments.iter().any(|seg| seg.start_ms <= t_ms && t_ms < seg.end_ms))
            .map(|s| s.name).collect();
        let label = if active.is_empty() { "none".to_string() } else { active.join(", ") };
        rows.push(("Off Boons", COLOR_OFF, label));
    }
    if layers.defensive_boons {
        let active: Vec<&str> = def.iter()
            .filter(|s| s.segments.iter().any(|seg| seg.start_ms <= t_ms && t_ms < seg.end_ms))
            .map(|s| s.name).collect();
        let label = if active.is_empty() { "none".to_string() } else { active.join(", ") };
        rows.push(("Def Boons", COLOR_DEF, label));
    }

    draw_tooltip(ui, mouse, t_ms, &rows, data_x, data_w);
}

fn draw_tooltip(
    ui: &Ui,
    mouse: [f32; 2],
    t_ms: u64,
    rows: &[(&'static str, [f32; 4], String)],
    data_x: f32,
    data_w: f32,
) {
    let line_h = ui.text_line_height();
    let pad = 6.0;
    let time_label = format_mmss(t_ms);
    let mut max_value_w: f32 = 0.0;
    let mut max_label_w: f32 = 0.0;
    for (label, _, value) in rows {
        max_label_w = max_label_w.max(ui.calc_text_size(*label)[0]);
        max_value_w = max_value_w.max(ui.calc_text_size(value)[0]);
    }
    let dot_w = 6.0;
    let row_gap = 4.0;
    let inner_w = dot_w + 6.0 + max_label_w + 12.0 + max_value_w;
    let header_w = ui.calc_text_size(&time_label)[0];
    let w = (inner_w.max(header_w)) + pad * 2.0;
    let h = pad + line_h + 4.0 + (rows.len() as f32) * (line_h + row_gap) + pad - row_gap;

    // Position: 12px right of cursor by default; flip left if it would overflow.
    let mut tx = mouse[0] + 12.0;
    if tx + w > data_x + data_w { tx = mouse[0] - 12.0 - w; }
    let ty = (mouse[1] - h * 0.5).max(0.0);

    let draw = ui.get_window_draw_list();
    draw.add_rect([tx, ty], [tx + w, ty + h], [0.05, 0.06, 0.08, 0.95])
        .filled(true).rounding(6.0).build();
    draw.add_rect([tx, ty], [tx + w, ty + h], [1.0, 1.0, 1.0, 0.10])
        .rounding(6.0).build();

    let mut y = ty + pad;
    draw.add_text([tx + pad, y], TEXT_MUTED, &time_label);
    y += line_h + 4.0;
    for (label, color, value) in rows {
        let dot_y = y + (line_h - dot_w) * 0.5;
        draw.add_rect([tx + pad, dot_y], [tx + pad + dot_w, dot_y + dot_w], *color)
            .filled(true).rounding(2.0).build();
        draw.add_text([tx + pad + dot_w + 6.0, y], TEXT_SECONDARY, *label);
        let vw = ui.calc_text_size(value)[0];
        draw.add_text([tx + w - pad - vw, y], TEXT_PRIMARY, value.as_str());
        y += line_h + row_gap;
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

    if samples.len() >= 1 && max > 0.0 {
        // Render each sample as a single filled rectangle from the
        // baseline up to the sample height. Adjacent rects share a
        // vertical edge so there are no diagonal seams, eliminating
        // the zigzag artifacts the two-triangle trapezoid fill caused
        // under ImGui's anti-aliasing.
        let n = samples.len();
        let mut fill = accent; fill[3] = 0.50;
        let baseline = y + h - 2.0;
        let step = data_w / n as f32;
        for i in 0..n {
            let x0 = data_x + step * i as f32;
            let x1 = data_x + step * (i + 1) as f32;
            let v = (samples[i] / max).clamp(0.0, 1.0);
            let top = y + h - v * (h - 4.0) - 2.0;
            if baseline - top < 0.5 || x1 - x0 < 0.5 { continue; }
            draw.add_rect([x0, top], [x1, baseline], fill).filled(true).build();
        }
        // Outline along the top of the column stack — draws as line
        // segments between adjacent sample tops (no extra fill, no AA seams).
        if n >= 2 {
            for i in 1..n {
                let xa = data_x + step * (i - 1) as f32 + step * 0.5;
                let xb = data_x + step * i as f32 + step * 0.5;
                let va = (samples[i - 1] / max).clamp(0.0, 1.0);
                let vb = (samples[i]     / max).clamp(0.0, 1.0);
                let ya = y + h - va * (h - 4.0) - 2.0;
                let yb = y + h - vb * (h - 4.0) - 2.0;
                draw.add_line([xa, ya], [xb, yb], accent).thickness(1.1).build();
            }
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
        // Drop-shadow + white label so the name reads on any backing colour.
        draw.add_text(
            [data_x + data_w - name_w - 4.0 + 1.0, row_y + nudge_y + 1.0],
            [0.0, 0.0, 0.0, 0.55], s.name,
        );
        draw.add_text(
            [data_x + data_w - name_w - 4.0, row_y + nudge_y],
            TEXT_PRIMARY, s.name,
        );
    }
    ui.dummy([avail, h + LANE_PAD_Y]);
}

// --- inspector cards under the timeline ---------------------------------

fn render_inspector(ui: &Ui, json: &EiJson, idx: usize) {
    use crate::boon_uptime::collect_uptimes;
    use crate::pulse_metrics::*;
    use crate::timeline_distance::distance_to_commander_per_second;

    let p = &json.players[idx];
    let ending_hp = p.health_percents.last()
        .and_then(|pair| pair.get(1).copied())
        .unwrap_or(100.0);
    let deaths_n = deaths(p);
    let downs_n = downs(p);
    let dmg_taken = damage_taken(p);

    let boons = collect_uptimes(p);
    let dist_samples = distance_to_commander_per_second(json, idx, json.duration_ms);
    let (dist_avg, dist_max) = if dist_samples.is_empty() {
        (None, None)
    } else {
        let sum: f64 = dist_samples.iter().sum();
        let avg = sum / dist_samples.len() as f64;
        let max = dist_samples.iter().copied().fold(0.0_f64, f64::max);
        (Some(avg), Some(max))
    };

    section_label(ui, "INSPECTOR");

    // 3-card row: Health & Survival, Boon Uptime, Position.
    let avail = ui.content_region_avail()[0].max(300.0);
    let gap = 8.0;
    let col_w = (avail - gap * 2.0) / 3.0;
    let card_h = 110.0;
    let cursor = ui.cursor_screen_pos();
    let start_x = cursor[0];
    let start_y = cursor[1];

    let health_lines = vec![
        ("Ending HP", format!("{:.0}%", ending_hp), if ending_hp <= 0.0 { COLOR_DMG } else { COLOR_HEALTH }),
        ("Deaths",    deaths_n.to_string(),         if deaths_n == 0 { COLOR_HEALTH } else { COLOR_DMG }),
        ("Downs",     downs_n.to_string(),          if downs_n  == 0 { COLOR_HEALTH } else { COLOR_TAKEN }),
        ("Dmg Taken", short_value(dmg_taken),       COLOR_TAKEN),
    ];
    draw_inspector_card(ui, start_x, start_y, col_w, card_h, "Health & Survival", COLOR_HEALTH, &health_lines);

    let mut boon_lines: Vec<(&str, String, [f32; 4])> = Vec::new();
    for b in &boons {
        let label = match b.stacking {
            crate::boon_uptime::BoonStacking::Intensity => format!("{:.1} st", b.uptime),
            crate::boon_uptime::BoonStacking::Duration  => format!("{:.0}%", b.uptime),
        };
        boon_lines.push((b.name, label, COLOR_OFF));
        if boon_lines.len() >= 4 { break; }
    }
    if boon_lines.is_empty() {
        boon_lines.push(("(no boons)", "—".to_string(), TEXT_MUTED));
    }
    draw_inspector_card(ui, start_x + col_w + gap, start_y, col_w, card_h, "Boon Uptime", COLOR_OFF, &boon_lines);

    let pos_lines = match (dist_avg, dist_max) {
        (Some(a), Some(m)) => vec![
            ("Avg distance", format!("{:.0}", a), COLOR_DIST),
            ("Max distance", format!("{:.0}", m), COLOR_DIST),
        ],
        _ => vec![("Distance", "no tag".to_string(), TEXT_MUTED)],
    };
    draw_inspector_card(ui, start_x + (col_w + gap) * 2.0, start_y, col_w, card_h, "Position", COLOR_DIST, &pos_lines);

    ui.dummy([avail, card_h]);
}

fn draw_inspector_card(
    ui: &Ui,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    title: &str,
    accent: [f32; 4],
    lines: &[(&str, String, [f32; 4])],
) {
    let draw = ui.get_window_draw_list();
    draw.add_rect([x, y], [x + w, y + h], BG_CARD).filled(true).rounding(6.0).build();
    draw.add_rect([x, y], [x + w, y + h], BG_CARD_BORDER).rounding(6.0).build();
    // Left accent stripe.
    draw.add_rect([x, y + 8.0], [x + 3.0, y + h - 8.0], accent).filled(true).rounding(2.0).build();

    let pad_x = 12.0;
    let pad_y = 8.0;
    let line_h = ui.text_line_height();
    draw.add_text([x + pad_x, y + pad_y], accent, title);

    let body_y0 = y + pad_y + line_h + 6.0;
    let row_step = (h - (body_y0 - y) - pad_y) / (lines.len().max(1) as f32);
    for (i, (label, value, color)) in lines.iter().enumerate() {
        let row_y = body_y0 + (i as f32) * row_step;
        draw.add_text([x + pad_x, row_y], TEXT_SECONDARY, *label);
        let vw = ui.calc_text_size(value)[0];
        draw.add_text([x + w - pad_x - vw, row_y], *color, value.as_str());
    }
}

fn short_value(n: u64) -> String {
    if n >= 1_000_000 { format!("{:.1}M", n as f64 / 1_000_000.0) }
    else if n >= 1_000 { format!("{:.1}k", n as f64 / 1_000.0) }
    else { format!("{n}") }
}

fn section_label(ui: &Ui, label: &str) {
    ui.text_colored(TEXT_MUTED, label);
}

fn format_mmss(ms: u64) -> String {
    let sec = ms / 1000;
    let m = sec / 60;
    let s = sec % 60;
    format!("{m}:{s:02}")
}
