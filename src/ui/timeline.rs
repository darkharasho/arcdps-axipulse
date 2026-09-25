#![cfg(windows)]
//! Timeline tab content — eight stacked swim-lanes + inspector cards.
//! Outer window lives in `ui::main`.
//!
//! No colour literal lives here: chrome comes from `ui::theme` and the
//! metric inks from `ui::series`. Every lane, tooltip and inspector card
//! is laid down by `ui::axi` so the block-fill-outline sequence exists
//! in one place.

use arcdps::imgui::Ui;

use crate::fight_data::FightData;
use crate::ui::axi::{self, Rect};
use crate::ui::series;
use crate::ui::theme;

const LANE_LABEL_W: f32 = 92.0;
/// Gap below a lane, on top of the window's own ItemSpacing — the
/// lanes are laid out as regular items, so the 8px spacing pushed in
/// `ui::main` already clears each lane's offset block.
const LANE_PAD_Y:   f32 = 2.0;
const AREA_LANE_H:  f32 = 48.0;
const BOON_ROW_H:   f32 = 12.0;
const BOON_GAP:     f32 = 2.0;
/// Symmetric padding on the plate behind a boon-row name.
const PLATE_PAD_X:  f32 = 2.0;
const PLATE_PAD_Y:  f32 = 1.0;

/// Render the Timeline tab contents (no window — caller owns that).
pub fn render_content(
    ui: &Ui,
    fight: &FightData,
    idx: usize,
    derived: &crate::derived::Derived,
    accent: [f32; 4],
    layers: &mut crate::config::TimelineLayers,
) {
    render_layer_toggles(ui, layers, accent);
    // Our own divider rather than imgui's: rule 5, at hairline weight.
    let sep = ui.cursor_screen_pos();
    let sep_w = ui.content_region_avail()[0].max(0.0);
    axi::rule(ui, [sep[0], sep[1]], [sep[0] + sep_w, sep[1]]);
    ui.dummy([sep_w, theme::BORDER_HAIRLINE + 4.0]);
    render_time_axis(ui, fight.duration_ms);

    // All heavy data was pre-computed once when the fight landed.
    let dur = fight.duration_ms;
    let health    = if layers.health           { derived.health_samples.as_slice() }    else { &[] };
    let dmg_dealt = if layers.damage_dealt     { derived.dmg_dealt_samples.as_slice() } else { &[] };
    let dmg_taken = if layers.damage_taken     { derived.dmg_taken_samples.as_slice() } else { &[] };
    let distance  = if layers.distance_to_tag  { derived.distance_samples.as_slice() }  else { &[] };
    let off: &[_] = if layers.offensive_boons  { derived.off_boons.as_slice() }         else { &[] };
    let def: &[_] = if layers.defensive_boons  { derived.def_boons.as_slice() }         else { &[] };
    let heal_in: &[u64]    = if layers.incoming_healing { derived.incoming_heal_samples.as_slice() }    else { &[] };
    let barrier_in: &[u64] = if layers.incoming_barrier { derived.incoming_barrier_samples.as_slice() } else { &[] };

    let avail = ui.content_region_avail()[0].max(LANE_LABEL_W + 60.0);
    let lanes_origin = ui.cursor_screen_pos();
    let data_x = lanes_origin[0] + LANE_LABEL_W;
    let data_w = avail - LANE_LABEL_W;
    let lanes_top_y = lanes_origin[1];

    // Every lane draws `Option<f32>` per second so a lane CAN have gaps;
    // health, damage dealt and damage taken simply never do -- once
    // present they are defined for every second of the fight -- so they
    // wrap in `Some` at the call site rather than each carrying an
    // Option they would never populate. Health can still be absent for
    // the WHOLE lane, which is the empty-slice case below.
    if layers.health {
        // Empty means the health pass never saw this entity. That is an
        // absence, not 100% -- see `timeline_health::
        // sample_health_per_second`.
        if health.is_empty() {
            draw_empty_lane(ui, "Health", series::METRIC_HEALTH, "no health data");
        } else {
            let v: Vec<Option<f32>> = health.iter().map(|x| Some(*x as f32)).collect();
            draw_area_lane(ui, "Health", series::METRIC_HEALTH, &v, 100.0);
        }
    }
    if layers.damage_dealt {
        let v: Vec<Option<f32>> = dmg_dealt.iter().map(|x| Some(*x as f32)).collect();
        draw_area_lane_auto(ui, "Dmg Dealt", series::METRIC_DAMAGE, &v);
    }
    if layers.damage_taken {
        let v: Vec<Option<f32>> = dmg_taken.iter().map(|x| Some(*x as f32)).collect();
        draw_area_lane_auto(ui, "Dmg Taken", series::METRIC_DAMAGE_TAKEN, &v);
    }
    if layers.distance_to_tag {
        // `all(is_none)` is true for an empty slice too, so this covers
        // both "no commander at all" and "a lane that never resolved a
        // single second".
        if distance.iter().all(Option::is_none) {
            draw_empty_lane(ui, "Dist Tag", series::METRIC_DISTANCE, "no commander tagged");
        } else {
            let v: Vec<Option<f32>> = distance.iter().map(|d| d.map(|x| x as f32)).collect();
            draw_area_lane_auto(ui, "Dist Tag", series::METRIC_DISTANCE, &v);
        }
    }
    if layers.offensive_boons {
        draw_boon_lane(ui, "Off Boons", series::METRIC_OFF_BOONS, &off, dur);
    }
    if layers.defensive_boons {
        draw_boon_lane(ui, "Def Boons", series::METRIC_DEF_BOONS, &def, dur);
    }
    // Absent for the WHOLE lane (not one gap at a time) when the log has
    // no healing addon data or this player has no series row -- see
    // `Derived::incoming_heal_samples`'s doc comment. An empty slice is
    // that absence; a non-empty slice of zeros is a real "received
    // nothing" measurement and draws as a flat lane, same as any other
    // area lane would.
    if layers.incoming_healing {
        if heal_in.is_empty() {
            let reason = if fight.healing_available { "no data" } else { "no healing addon" };
            draw_empty_lane(ui, "Heal In", series::METRIC_HEAL_IN, reason);
        } else {
            let v: Vec<Option<f32>> = heal_in.iter().map(|x| Some(*x as f32)).collect();
            draw_area_lane_auto(ui, "Heal In", series::METRIC_HEAL_IN, &v);
        }
    }
    if layers.incoming_barrier {
        if barrier_in.is_empty() {
            let reason = if fight.healing_available { "no data" } else { "no healing addon" };
            draw_empty_lane(ui, "Barrier In", series::METRIC_BARRIER_IN, reason);
        } else {
            let v: Vec<Option<f32>> = barrier_in.iter().map(|x| Some(*x as f32)).collect();
            draw_area_lane_auto(ui, "Barrier In", series::METRIC_BARRIER_IN, &v);
        }
    }

    let lanes_bottom_y = ui.cursor_screen_pos()[1];
    draw_hover_crosshair(
        ui, data_x, data_w, lanes_top_y, lanes_bottom_y, dur,
        layers, &health, &dmg_dealt, &dmg_taken, &distance, &off, &def,
        heal_in, barrier_in,
    );

    ui.dummy([0.0, 6.0]);
    render_inspector(ui, fight, idx, derived);
}

/// The eight lanes as blocked toggle chips — same labels, same order,
/// same `TimelineLayers` fields the checkboxes wrote. Chips are
/// clickable, so they lift on hover.
fn render_layer_toggles(ui: &Ui, layers: &mut crate::config::TimelineLayers, accent: [f32; 4]) {
    let pad = [10.0_f32, 4.0_f32];
    let h = ui.text_line_height() + pad[1] * 2.0;
    let origin = ui.cursor_screen_pos();
    let row_x0 = origin[0];
    let right = row_x0 + ui.content_region_avail()[0].max(120.0);
    // A wrapped row must clear the row above it by more than the chips'
    // offset blocks.
    let row_step = h + theme::OFFSET_CONTROL + 3.0;

    let mut pairs: [(&str, &mut bool); 8] = [
        ("Health",     &mut layers.health),
        ("Dmg Dealt",  &mut layers.damage_dealt),
        ("Dmg Taken",  &mut layers.damage_taken),
        ("Dist Tag",   &mut layers.distance_to_tag),
        ("Off Boons",  &mut layers.offensive_boons),
        ("Def Boons",  &mut layers.defensive_boons),
        ("Heal In",    &mut layers.incoming_healing),
        ("Barrier In", &mut layers.incoming_barrier),
    ];

    let mut pos = origin;
    let mut rows = 1usize;
    for (label, value) in pairs.iter_mut() {
        // `axi::chip` sizes itself the same way; measured here too so
        // the wrap decision is made before anything is drawn.
        let want_w = ui.calc_text_size(*label)[0] + pad[0] * 2.0;
        if pos[0] > row_x0 && pos[0] + want_w + theme::OFFSET_CONTROL > right {
            pos = [row_x0, pos[1] + row_step];
            rows += 1;
        }
        let (clicked, w) = axi::chip(
            ui,
            pos,
            &format!("tl-layer-{label}"),
            label,
            **value,
            accent,
            pad,
        );
        if clicked { **value = !**value; }
        // Clear the neighbour's offset block before the next chip.
        pos = [pos[0] + w + theme::OFFSET_CONTROL + 5.0, pos[1]];
    }

    // Reserve the strip's span with a regular item rather than an
    // absolute cursor, so the content below it is not overdrawn.
    let total_h = (rows - 1) as f32 * row_step + h + theme::OFFSET_CONTROL;
    ui.set_cursor_screen_pos(origin);
    ui.dummy([right - row_x0, total_h]);
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
        draw.add_text([lx, cursor[1]], theme::TEXT_FAINT, &label);
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
    distance: &[Option<f64>],
    off: &[crate::timeline_boons::BoonSeries],
    def: &[crate::timeline_boons::BoonSeries],
    heal_in: &[u64],
    barrier_in: &[u64],
) {
    if !ui.is_mouse_hovering_rect([data_x, top_y], [data_x + data_w, bottom_y]) {
        return;
    }
    let mouse = ui.io().mouse_pos;
    if !mouse[0].is_finite() || data_w <= 0.0 { return; }
    let pct = ((mouse[0] - data_x) / data_w).clamp(0.0, 1.0);
    let t_ms = (pct as f64 * duration_ms as f64) as u64;

    // The crosshair is an annotation (rule 5): theme::RULE at hairline
    // weight, at full strength (rule 2) rather than washed-out white.
    // `axi::rule` acquires and releases the draw list inside the call,
    // so nothing is still alive when draw_tooltip re-acquires it —
    // imgui-rs panics if two are.
    axi::rule(ui, [mouse[0], top_y], [mouse[0], bottom_y]);

    // Build tooltip rows.
    let sample_idx = |arr_len: usize| -> Option<usize> {
        if arr_len == 0 { None } else { Some(((pct * (arr_len - 1).max(0) as f32) as usize).min(arr_len - 1)) }
    };
    let mut rows: Vec<(&'static str, [f32; 4], String)> = Vec::new();
    if layers.health {
        if let Some(i) = sample_idx(health.len()) {
            rows.push(("Health", series::METRIC_HEALTH, format!("{:.0}%", health[i])));
        }
    }
    if layers.damage_dealt {
        if let Some(i) = sample_idx(dmg_dealt.len()) {
            rows.push(("Dmg Dealt", series::METRIC_DAMAGE, short_value(dmg_dealt[i])));
        }
    }
    if layers.damage_taken {
        if let Some(i) = sample_idx(dmg_taken.len()) {
            rows.push(("Dmg Taken", series::METRIC_DAMAGE_TAKEN, short_value(dmg_taken[i])));
        }
    }
    if layers.distance_to_tag && !distance.iter().all(Option::is_none) {
        if let Some(i) = sample_idx(distance.len()) {
            // An unmeasured second reads as absent, not as a number
            // carried over from a second that was measured.
            let label = match distance[i] {
                Some(d) => format!("{d:.0}"),
                None => "—".to_string(),
            };
            rows.push(("Dist Tag", series::METRIC_DISTANCE, label));
        }
    }
    if layers.offensive_boons {
        let active: Vec<&str> = off.iter()
            .filter(|s| s.segments.iter().any(|seg| seg.start_ms <= t_ms && t_ms < seg.end_ms))
            .map(|s| s.name).collect();
        let label = if active.is_empty() { "none".to_string() } else { active.join(", ") };
        rows.push(("Off Boons", series::METRIC_OFF_BOONS, label));
    }
    if layers.defensive_boons {
        let active: Vec<&str> = def.iter()
            .filter(|s| s.segments.iter().any(|seg| seg.start_ms <= t_ms && t_ms < seg.end_ms))
            .map(|s| s.name).collect();
        let label = if active.is_empty() { "none".to_string() } else { active.join(", ") };
        rows.push(("Def Boons", series::METRIC_DEF_BOONS, label));
    }
    if layers.incoming_healing && !heal_in.is_empty() {
        if let Some(i) = sample_idx(heal_in.len()) {
            rows.push(("Heal In", series::METRIC_HEAL_IN, short_value(heal_in[i])));
        }
    }
    if layers.incoming_barrier && !barrier_in.is_empty() {
        if let Some(i) = sample_idx(barrier_in.len()) {
            rows.push(("Barrier In", series::METRIC_BARRIER_IN, short_value(barrier_in[i])));
        }
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
    // The card's outline sits ON its edge, so the text clears it by
    // BORDER_CONTROL before its own breathing room.
    let pad = 6.0 + theme::BORDER_CONTROL;
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

    // Position: 12px right of cursor by default; flip left if it (or its
    // offset block) would overflow.
    let mut tx = mouse[0] + 12.0;
    if tx + w + theme::OFFSET_CONTROL > data_x + data_w { tx = mouse[0] - 12.0 - w; }
    let ty = (mouse[1] - h * 0.5).max(0.0);

    // A tooltip is not clickable, so it never lifts.
    axi::card(ui, Rect::at([tx, ty], [w, h]), theme::SURFACE_RAISED, false);

    let draw = ui.get_window_draw_list();
    let mut y = ty + pad;
    draw.add_text([tx + pad, y], theme::TEXT_FAINT, &time_label);
    y += line_h + 4.0;
    for (label, ink, value) in rows {
        let dot_y = y + (line_h - dot_w) * 0.5;
        draw.add_rect([tx + pad, dot_y], [tx + pad + dot_w, dot_y + dot_w], *ink)
            .filled(true).build();
        draw.add_text([tx + pad + dot_w + 6.0, y], theme::TEXT_DIM, *label);
        let vw = ui.calc_text_size(value)[0];
        draw.add_text([tx + w - pad - vw, y], theme::TEXT, value.as_str());
        y += line_h + row_gap;
    }
}

/// A lane's name, right-aligned in the label gutter and vertically
/// centred on the lane. Uppercase so it reads as an eyebrow, but NOT
/// `axi::label`, and the reason is the ink: this is the lane's legend
/// entry, so it wears its own series' colour under the domain-palette
/// carve-out. It is the only lane-to-colour mapping in the tab, and
/// `axi::label`'s `TEXT_FAINT` would delete it. Rule 5 governs shapes
/// (filled = status, outlined = annotation), not label text, and rule
/// 9's "a chart's ink is the accent" is about the chart body, which
/// here is the domain palette by design.
///
/// Secondary, practical: `axi::label` draws at the cursor, whereas the
/// gutter is positioned off the lane rect.
fn lane_label(ui: &Ui, gutter_right: f32, lane_y: f32, lane_h: f32, label: &str, ink: [f32; 4]) {
    let text = label.to_uppercase();
    let w = ui.calc_text_size(&text)[0];
    ui.get_window_draw_list().add_text(
        [gutter_right - w - 6.0, lane_y + (lane_h - ui.text_line_height()) * 0.5],
        ink,
        &text,
    );
}

fn draw_area_lane_auto(ui: &Ui, label: &str, ink: [f32; 4], samples: &[Option<f32>]) {
    // Scale off the measured values only; an unmeasured second must not
    // influence the axis any more than it influences the curve.
    let max = samples.iter().flatten().copied().fold(1.0_f32, f32::max);
    draw_area_lane(ui, label, ink, samples, max);
}

/// `samples[i] == None` is a second with NO value -- the lane leaves a
/// gap there rather than drawing a baseline zero or bridging the hole
/// with a straight line between its neighbours. Both would render an
/// invented measurement; see
/// `timeline_distance::distance_to_commander_per_second`.
fn draw_area_lane(ui: &Ui, label: &str, ink: [f32; 4], samples: &[Option<f32>], max: f32) {
    let avail = ui.content_region_avail()[0].max(LANE_LABEL_W + 60.0);
    let cursor = ui.cursor_screen_pos();
    let data_x = cursor[0] + LANE_LABEL_W;
    let data_w = avail - LANE_LABEL_W;
    let y = cursor[1];
    let h = AREA_LANE_H;

    lane_label(ui, cursor[0] + LANE_LABEL_W, y, h, label, ink);
    // A lane is read, not clicked, so its card never lifts.
    axi::card(ui, Rect::at([data_x, y], [data_w, h]), theme::SURFACE_RAISED, false);

    let draw = ui.get_window_draw_list();
    if samples.len() >= 1 && max > 0.0 {
        // Rasterise the area in 1-px-wide vertical columns, linearly
        // interpolating between samples. Avoids the blocky look of
        // one-rect-per-sample and the diagonal AA seams that the
        // two-triangle trapezoid fill produces under ImGui's AA.
        //
        // The fill is the metric ink at full strength: rule 2, and rule
        // 7 — the area's height already carries the quantity, so the
        // alpha has no work left to do.
        let n = samples.len();
        let baseline = y + h - 2.0;
        let usable_h = h - 4.0;
        let norm = |v: Option<f32>| -> Option<f32> { v.map(|x| (x / max).clamp(0.0, 1.0)) };
        // `None` when either bracketing sample is absent: a column
        // straddling the edge of a gap has no honest value to show, so
        // it is left empty rather than half-interpolated.
        let sample_at = |x_frac: f32| -> Option<f32> {
            if n == 1 { return norm(samples[0]); }
            let f = x_frac * (n - 1) as f32;
            let i0 = (f as usize).min(n - 1);
            let i1 = (i0 + 1).min(n - 1);
            let t = f - i0 as f32;
            let v0 = norm(samples[i0])?;
            let v1 = norm(samples[i1])?;
            Some(v0 + (v1 - v0) * t)
        };
        let cols = data_w.floor() as i32;
        for c in 0..cols {
            let x0 = data_x + c as f32;
            let x1 = x0 + 1.0;
            let Some(v) = sample_at((c as f32 + 0.5) / cols as f32) else { continue };
            let top = y + h - v * usable_h - 2.0;
            if baseline - top < 0.5 { continue; }
            // Overlap by 0.5px to prevent hairline gaps between columns
            // under ImGui's edge AA.
            draw.add_rect([x0, top], [x1 + 0.5, baseline], ink).filled(true).build();
        }
        // Outline traces the actual samples so the curve reads as a line.
        // A segment with an absent endpoint is skipped, so the line
        // breaks at a gap instead of leaping across it.
        if n >= 2 {
            let step = data_w / (n - 1) as f32;
            for i in 1..n {
                let (Some(va), Some(vb)) = (norm(samples[i - 1]), norm(samples[i])) else {
                    continue;
                };
                let xa = data_x + step * (i - 1) as f32;
                let xb = data_x + step * i as f32;
                let ya = y + h - va * usable_h - 2.0;
                let yb = y + h - vb * usable_h - 2.0;
                draw.add_line([xa, ya], [xb, yb], ink).thickness(1.1).build();
            }
        }
    }
    drop(draw);
    ui.dummy([avail, h + LANE_PAD_Y]);
}

fn draw_empty_lane(ui: &Ui, label: &str, ink: [f32; 4], reason: &str) {
    let avail = ui.content_region_avail()[0].max(LANE_LABEL_W + 60.0);
    let cursor = ui.cursor_screen_pos();
    let data_x = cursor[0] + LANE_LABEL_W;
    let data_w = avail - LANE_LABEL_W;
    let y = cursor[1];
    let h = AREA_LANE_H;

    lane_label(ui, cursor[0] + LANE_LABEL_W, y, h, label, ink);
    axi::card(ui, Rect::at([data_x, y], [data_w, h]), theme::SURFACE_RAISED, false);

    let rw = ui.calc_text_size(reason)[0];
    ui.get_window_draw_list().add_text(
        [data_x + (data_w - rw) * 0.5, y + (h - ui.text_line_height()) * 0.5],
        theme::TEXT_FAINT,
        reason,
    );
    ui.dummy([avail, h + LANE_PAD_Y]);
}

fn draw_boon_lane(
    ui: &Ui,
    label: &str,
    ink: [f32; 4],
    series_rows: &[crate::timeline_boons::BoonSeries],
    duration_ms: u64,
) {
    let avail = ui.content_region_avail()[0].max(LANE_LABEL_W + 60.0);
    let cursor = ui.cursor_screen_pos();
    let data_x = cursor[0] + LANE_LABEL_W;
    let data_w = avail - LANE_LABEL_W;
    let y = cursor[1];
    let h_calc = (series_rows.len() as f32) * (BOON_ROW_H + BOON_GAP) + 4.0;
    let h = h_calc.max(AREA_LANE_H);

    lane_label(ui, cursor[0] + LANE_LABEL_W, y, h, label, ink);
    axi::card(ui, Rect::at([data_x, y], [data_w, h]), theme::SURFACE_RAISED, false);

    if duration_ms == 0 {
        ui.dummy([avail, h + LANE_PAD_Y]);
        return;
    }

    let draw = ui.get_window_draw_list();
    for (row, s) in series_rows.iter().enumerate() {
        let row_y = y + 2.0 + row as f32 * (BOON_ROW_H + BOON_GAP);
        for seg in &s.segments {
            let sx = data_x + data_w * (seg.start_ms.min(duration_ms) as f32 / duration_ms as f32);
            let ex = data_x + data_w * (seg.end_ms.min(duration_ms) as f32 / duration_ms as f32);
            if ex - sx < 1.0 { continue; }
            // Full strength: a segment's LENGTH is the uptime (rule 7),
            // so its alpha carries nothing (rule 2).
            draw.add_rect([sx, row_y], [ex, row_y + BOON_ROW_H], ink).filled(true).build();
        }
        // The name sits at a fixed offset from the lane's right edge, so
        // depending on uptime it lands on a full-strength segment or on
        // the bare card. A hard INK_LINE plate behind it makes TEXT
        // legible on both — this language separates a label from a
        // bright fill with a block, never with a blur.
        let name_w = ui.calc_text_size(s.name)[0];
        let line_h = ui.text_line_height();
        let nudge_y = (BOON_ROW_H - line_h).max(0.0) * 0.5;
        let tx = data_x + data_w - name_w - 4.0;
        let ty = row_y + nudge_y;
        // Clamped to this boon's own row band and to the lane's left
        // edge, so a tall glyph line or an over-long name cannot paint
        // the plate over a neighbouring row or outside the card.
        let plate = Rect::new(
            [(tx - PLATE_PAD_X).max(data_x), (ty - PLATE_PAD_Y).max(row_y)],
            [
                tx + name_w + PLATE_PAD_X,
                (ty + line_h + PLATE_PAD_Y).min(row_y + BOON_ROW_H),
            ],
        );
        if !plate.is_degenerate() {
            draw.add_rect(plate.min, plate.max, theme::INK_LINE).filled(true).build();
        }
        draw.add_text([tx, ty], theme::TEXT, s.name);
    }
    drop(draw);
    ui.dummy([avail, h + LANE_PAD_Y]);
}

// --- inspector cards under the timeline ---------------------------------

fn render_inspector(ui: &Ui, fight: &FightData, idx: usize, derived: &crate::derived::Derived) {
    use crate::pulse_metrics::*;

    let p = &fight.players[idx];
    // `None` when the health pass never saw this entity: the card reads
    // "—" rather than claiming a measured 100%.
    let ending_hp: Option<f64> = p.health_percents.last().map(|(_, hp)| *hp);
    let deaths_n = deaths(p);
    let downs_n = downs(p);
    let dmg_taken = damage_taken(p);

    let boons = &derived.boon_uptimes;
    // Averages the MEASURED seconds only -- an unmeasured second is not
    // in the numerator and, crucially, not in the denominator either.
    let dist = crate::timeline_distance::summarize(&derived.distance_samples);

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
        (
            "Ending HP",
            ending_hp.map_or_else(|| "—".to_string(), |hp| format!("{hp:.0}%")),
            if ending_hp.is_some_and(|hp| hp <= 0.0) { series::METRIC_DAMAGE } else { series::METRIC_HEALTH },
        ),
        ("Deaths",    deaths_n.to_string(),         if deaths_n == 0 { series::METRIC_HEALTH } else { series::METRIC_DAMAGE }),
        ("Downs",     downs_n.to_string(),          if downs_n  == 0 { series::METRIC_HEALTH } else { series::METRIC_DAMAGE_TAKEN }),
        ("Dmg Taken", short_value(dmg_taken),       series::METRIC_DAMAGE_TAKEN),
    ];
    draw_inspector_card(ui, start_x, start_y, col_w, card_h, "Health & Survival", series::METRIC_HEALTH, &health_lines);

    let mut boon_lines: Vec<(&str, String, [f32; 4])> = Vec::new();
    for b in boons.iter() {
        let label = match b.stacking {
            crate::boon_uptime::BoonStacking::Intensity => format!("{:.1} st", b.uptime),
            crate::boon_uptime::BoonStacking::Duration  => format!("{:.0}%", b.uptime),
        };
        boon_lines.push((b.name, label, series::METRIC_OFF_BOONS));
        if boon_lines.len() >= 4 { break; }
    }
    if boon_lines.is_empty() {
        boon_lines.push(("(no boons)", "—".to_string(), theme::TEXT_FAINT));
    }
    draw_inspector_card(ui, start_x + col_w + gap, start_y, col_w, card_h, "Boon Uptime", series::METRIC_OFF_BOONS, &boon_lines);

    let pos_lines = match dist {
        Some(d) => {
            let mut lines = vec![
                ("Avg distance", format!("{:.0}", d.avg), series::METRIC_DISTANCE),
                ("Max distance", format!("{:.0}", d.max), series::METRIC_DISTANCE),
            ];
            if d.is_partial() {
                // Say so on the card. An average over two thirds of a
                // fight looks identical to an average over all of it
                // unless the coverage is on screen next to it.
                // Raw second counts, not m:ss. A lane missing its final
                // second would render as "2:18 of 2:18" once m:ss
                // rounds, which reads as full coverage -- the exact
                // impression this note exists to prevent.
                lines.push((
                    "Measured",
                    format!("{}s of {}s", d.measured_secs, d.total_secs),
                    theme::TEXT_FAINT,
                ));
            }
            lines
        }
        None => vec![("Distance", "no tag".to_string(), theme::TEXT_FAINT)],
    };
    draw_inspector_card(ui, start_x + (col_w + gap) * 2.0, start_y, col_w, card_h, "Position", series::METRIC_DISTANCE, &pos_lines);

    // Room for the cards' offset blocks below the row.
    ui.dummy([avail, card_h + theme::OFFSET_CONTROL]);
}

fn draw_inspector_card(
    ui: &Ui,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    title: &str,
    ink: [f32; 4],
    lines: &[(&str, String, [f32; 4])],
) {
    // A card that is read, not clicked: no hover lift.
    axi::card(ui, Rect::at([x, y], [w, h]), theme::SURFACE_RAISED, false);

    let draw = ui.get_window_draw_list();
    // Left stripe, inside the card's own outline, naming the metric.
    let stripe_x = x + theme::BORDER_CONTROL;
    draw.add_rect([stripe_x, y + 8.0], [stripe_x + 3.0, y + h - 8.0], ink)
        .filled(true).build();

    // Text clears the outline before its own breathing room.
    let pad_x = 12.0 + theme::BORDER_CONTROL;
    let pad_y = 8.0 + theme::BORDER_CONTROL;
    let line_h = ui.text_line_height();
    draw.add_text([x + pad_x, y + pad_y], ink, title);

    let body_y0 = y + pad_y + line_h + 6.0;
    let row_step = (h - (body_y0 - y) - pad_y) / (lines.len().max(1) as f32);
    for (i, (label, value, color)) in lines.iter().enumerate() {
        let row_y = body_y0 + (i as f32) * row_step;
        draw.add_text([x + pad_x, row_y], theme::TEXT_DIM, *label);
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
    axi::label(ui, label);
}

fn format_mmss(ms: u64) -> String {
    let sec = ms / 1000;
    let m = sec / 60;
    let s = sec % 60;
    format!("{m}:{s:02}")
}
