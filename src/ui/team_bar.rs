#![cfg(windows)]
//! Persistent team-count bar: a small frameless window with one
//! stacked bar showing red/green/blue player counts from the latest
//! parsed fight. Independent of the main AxiPulse window, like the
//! notifier toast. Colors follow axibridge's WvW team palette.
//!
//! Two renderings behind `config.team_bar_compact`:
//!   full    — map-name header + total, glossy pill bar in a recessed
//!             track, color glow strip, legend with a "you" marker on
//!             the viewer's own team
//!   compact — just the glossy pill bar

use arcdps::imgui::{Condition, StyleColor, StyleVar, Ui, WindowFlags};

use crate::config::Config;
use crate::state::AppState;
use crate::wvw_teams::{count_teams, team_color, TeamColor, TeamCounts};

const BAR_WIDTH: f32 = 264.0;
const BAR_HEIGHT: f32 = 20.0;
/// Gap between segments; the dark track shows through.
const SEG_GAP: f32 = 2.0;
/// Corner rounding on the bar's outer ends. Softer than a full pill:
/// full-pill rounding turns near-empty segments into misshapen nubs.
const BAR_ROUNDING: f32 = 7.0;
/// Floor on segment width so a 1-player team stays a legible pip
/// instead of a 2px sliver.
const MIN_SEG_WIDTH: f32 = 14.0;
const GLOW_HEIGHT: f32 = 3.0;
const GLOW_GAP: f32 = 3.0;

const TRACK_BG: [f32; 4] = [0.0, 0.0, 0.0, 0.45];
const BAR_OUTLINE: [f32; 4] = [0.0, 0.0, 0.0, 0.55];
const SEG_TEXT: [f32; 4] = [0.03, 0.04, 0.06, 0.85];
const HEADER_DIM: [f32; 4] = [1.0, 1.0, 1.0, 0.40];
const HEADER_MID: [f32; 4] = [1.0, 1.0, 1.0, 0.55];
const HEADER_BRIGHT: [f32; 4] = [1.0, 1.0, 1.0, 0.85];

pub fn render(ui: &Ui, state: &AppState, config: &mut Config) {
    if !config.show_team_bar { return; }

    let fight = state.current();
    let counts = fight.map(|f| count_teams(&f.data));
    // The viewer's own team, for the legend "you" marker.
    let self_color = fight.and_then(|f| {
        let idx = f.derived.self_idx?;
        let tid = f.data.players.get(idx)?.team_id;
        Some(team_color(tid, f.data.wvw_map_data.as_ref()))
    });

    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([12.0, 9.0])),
        ui.push_style_var(StyleVar::WindowRounding(10.0)),
        ui.push_style_var(StyleVar::WindowBorderSize(1.0)),
    ];
    let color_tokens = [
        ui.push_style_color(StyleColor::WindowBg, [0.07, 0.08, 0.11, 0.82]),
        ui.push_style_color(StyleColor::Border, [1.0, 1.0, 1.0, 0.07]),
    ];

    let mut win = ui.window("##axipulse-team-bar")
        .size([BAR_WIDTH + 24.0, 0.0], Condition::Always)
        .flags(
            WindowFlags::NO_TITLE_BAR
                | WindowFlags::NO_RESIZE
                | WindowFlags::NO_SCROLLBAR
                | WindowFlags::NO_COLLAPSE
                | WindowFlags::NO_FOCUS_ON_APPEARING
                | WindowFlags::ALWAYS_AUTO_RESIZE,
        );
    if let Some(pos) = config.team_bar_pos {
        win = win.position([pos.0, pos.1], Condition::FirstUseEver);
    } else {
        let [vw, _vh] = ui.io().display_size;
        win = win.position([vw * 0.5 - BAR_WIDTH * 0.5, 30.0], Condition::FirstUseEver);
    }

    let compact = config.team_bar_compact;
    let mut saved_pos: Option<(f32, f32)> = None;
    win.build(|| {
        match counts {
            Some(c) if c.total() > 0 => {
                if !compact {
                    let map = fight
                        .map(|f| {
                            f.data.fight_name
                                .strip_prefix("Detailed WvW - ")
                                .unwrap_or(f.data.fight_name.as_str())
                                .to_uppercase()
                        })
                        .unwrap_or_default();
                    draw_header(ui, &map, c.total());
                }
                draw_bar(ui, c, compact);
                if !compact {
                    draw_legend(ui, c, self_color);
                }
            }
            _ => {
                let msg = "Waiting for a parsed fight...";
                let tw = ui.calc_text_size(msg)[0];
                let indent = ((BAR_WIDTH - tw) * 0.5).max(0.0);
                let [cx, cy] = ui.cursor_pos();
                ui.set_cursor_pos([cx + indent, cy]);
                ui.text_disabled(msg);
                ui.same_line();
                ui.dummy([indent, 1.0]);
            }
        }
        let [px, py] = ui.window_pos();
        saved_pos = Some((px, py));
    });

    if let Some(pos) = saved_pos {
        if config.team_bar_pos != Some(pos) {
            config.team_bar_pos = Some(pos);
            config.save();
        }
    }

    for tok in color_tokens { tok.end(); }
    for tok in style_tokens { tok.end(); }
}

fn draw_header(ui: &Ui, map: &str, total: u32) {
    let origin = ui.cursor_screen_pos();
    let draw = ui.get_window_draw_list();
    let line_h = ui.text_line_height();

    // Right-aligned "<n> players", bright count + dim suffix.
    let count_txt = total.to_string();
    let suffix = " players";
    let count_w = ui.calc_text_size(&count_txt)[0];
    let suffix_w = ui.calc_text_size(suffix)[0];
    let right = origin[0] + BAR_WIDTH;
    draw.add_text([right - suffix_w, origin[1]], HEADER_MID, suffix);
    draw.add_text([right - suffix_w - count_w, origin[1]], HEADER_BRIGHT, &count_txt);

    // Map name on the left, truncated with an ellipsis if it would
    // collide with the total.
    let avail = BAR_WIDTH - suffix_w - count_w - 10.0;
    let mut title = map.to_string();
    while !title.is_empty() && ui.calc_text_size(&title)[0] > avail {
        title.pop();
        while !title.is_char_boundary(title.len()) { title.pop(); }
        title = format!("{}…", title.trim_end());
    }
    draw.add_text(origin, HEADER_DIM, &title);

    ui.dummy([BAR_WIDTH, line_h + 4.0]);
}

/// Nonzero (color, count) segments in display order.
fn visible_segments(counts: TeamCounts) -> Vec<(TeamColor, u32)> {
    [
        (TeamColor::Red, counts.red),
        (TeamColor::Green, counts.green),
        (TeamColor::Blue, counts.blue),
        (TeamColor::Unknown, counts.unknown),
    ]
    .into_iter()
    .filter(|(_, c)| *c > 0)
    .collect()
}

/// Blend `c` toward white by `f`.
fn lighten(c: [f32; 4], f: f32) -> [f32; 4] {
    [c[0] + (1.0 - c[0]) * f, c[1] + (1.0 - c[1]) * f, c[2] + (1.0 - c[2]) * f, c[3]]
}

/// Blend `c` toward black by `f`.
fn darken(c: [f32; 4], f: f32) -> [f32; 4] {
    [c[0] * (1.0 - f), c[1] * (1.0 - f), c[2] * (1.0 - f), c[3]]
}

fn draw_bar(ui: &Ui, counts: TeamCounts, compact: bool) {
    let total = counts.total() as f32;
    let segments = visible_segments(counts);
    let gaps = SEG_GAP * (segments.len().saturating_sub(1)) as f32;
    let usable = BAR_WIDTH - gaps;

    let origin = ui.cursor_screen_pos();
    let draw = ui.get_window_draw_list();
    let y0 = origin[1];
    let y1 = y0 + BAR_HEIGHT;

    // Recessed track behind the segments; shows through the gaps.
    draw.add_rect([origin[0], y0], [origin[0] + BAR_WIDTH, y1], TRACK_BG)
        .filled(true)
        .rounding(BAR_ROUNDING)
        .build();

    // Proportional widths with a per-segment floor wide enough for the
    // count label, so every segment always shows its count. The deficit
    // from floored segments is taken from the flexible ones pro rata.
    let labels: Vec<String> = segments.iter().map(|(_, c)| c.to_string()).collect();
    let raw: Vec<f32> = segments
        .iter()
        .map(|(_, c)| usable * (*c as f32 / total))
        .collect();
    let mins: Vec<f32> = labels
        .iter()
        .map(|l| (ui.calc_text_size(l)[0] + 8.0).max(MIN_SEG_WIDTH))
        .collect();
    let mut deficit = 0.0;
    let mut flexible = 0.0;
    for (w, m) in raw.iter().zip(&mins) {
        if w < m { deficit += m - w; } else { flexible += w; }
    }
    let widths: Vec<f32> = raw
        .iter()
        .zip(&mins)
        .map(|(w, m)| {
            if *w < *m { *m } else if flexible > 0.0 { w - (w / flexible) * deficit } else { *w }
        })
        .collect();

    let mut x = origin[0];
    let last = segments.len() - 1;
    let mut glow_spans: Vec<(f32, f32, TeamColor)> = Vec::with_capacity(segments.len());
    for (i, (color, _count)) in segments.iter().copied().enumerate() {
        let w = widths[i];
        let (x0, x1) = (x, x + w);
        let (first, is_last) = (i == 0, i == last);
        // Middle segments must use zero rounding rather than clearing
        // all four corner flags: imgui promotes an empty corner mask
        // back to RoundCornersAll, which would round their edges.
        let rounding = if first || is_last { BAR_ROUNDING.min(w * 0.5) } else { 0.0 };

        // Base fill, then a full-height gradient overlay: light at the
        // top fading through the base color to a shaded bottom. The
        // overlay is a plain quad, so inset it past the rounded corners
        // and let the base fill own the corner arcs.
        let base = color.rgba();
        draw.add_rect([x0, y0], [x1, y1], base)
            .filled(true)
            .rounding(rounding)
            .round_top_left(first)
            .round_bot_left(first)
            .round_top_right(is_last)
            .round_bot_right(is_last)
            .build();
        let gx0 = if first { x0 + rounding } else { x0 };
        let gx1 = if is_last { x1 - rounding } else { x1 };
        if gx1 > gx0 {
            let mid = y0 + BAR_HEIGHT * 0.55;
            let top = lighten(base, 0.22);
            let bot = darken(base, 0.14);
            draw.add_rect_filled_multicolor([gx0, y0], [gx1, mid], top, top, base, base);
            draw.add_rect_filled_multicolor([gx0, mid], [gx1, y1], base, base, bot, bot);
            // 1px inner top highlight.
            draw.add_line([gx0 + 1.0, y0 + 1.0], [gx1 - 1.0, y0 + 1.0], [1.0, 1.0, 1.0, 0.28])
                .build();
        }

        // Count label centered in the segment; the width floor above
        // guarantees it fits. Faint light offset underneath gives the
        // dark text a crisper edge on the bright fill.
        let label = &labels[i];
        let ts = ui.calc_text_size(label);
        let tx = x0 + (w - ts[0]) * 0.5;
        let ty = y0 + (BAR_HEIGHT - ts[1]) * 0.5;
        draw.add_text([tx, ty + 1.0], [1.0, 1.0, 1.0, 0.25], label);
        draw.add_text([tx, ty], SEG_TEXT, label);

        glow_spans.push((x0, x1, color));
        x = x1 + SEG_GAP;
    }

    // Crisp dark outline around the whole bar, over the segments.
    draw.add_rect(
        [origin[0] - 0.5, y0 - 0.5],
        [origin[0] + BAR_WIDTH + 0.5, y1 + 0.5],
        BAR_OUTLINE,
    )
    .rounding(BAR_ROUNDING)
    .thickness(1.0)
    .build();

    let mut height = BAR_HEIGHT;
    if !compact {
        // Thin color-matched glow strip under the bar.
        let gy0 = y1 + GLOW_GAP;
        let gy1 = gy0 + GLOW_HEIGHT;
        for (x0, x1, color) in glow_spans {
            let mut c = color.rgba();
            c[3] = 0.55;
            draw.add_rect([x0, gy0], [x1, gy1], c)
                .filled(true)
                .rounding(1.5)
                .build();
        }
        height += GLOW_GAP + GLOW_HEIGHT;
    }
    ui.dummy([BAR_WIDTH, height]);
}

fn draw_legend(ui: &Ui, counts: TeamCounts, self_color: Option<TeamColor>) {
    const DOT_R: f32 = 3.5;
    const DOT_TEXT_GAP: f32 = 5.0;
    const ITEM_GAP: f32 = 12.0;

    let segments = visible_segments(counts);
    let items: Vec<(TeamColor, String, bool)> = segments
        .into_iter()
        .map(|(color, count)| {
            let mine = self_color == Some(color);
            let label = if mine { format!("{count} · you") } else { count.to_string() };
            (color, label, mine)
        })
        .collect();

    let total_w: f32 = items
        .iter()
        .map(|(_, label, _)| DOT_R * 2.0 + DOT_TEXT_GAP + ui.calc_text_size(label)[0])
        .sum::<f32>()
        + ITEM_GAP * items.len().saturating_sub(1) as f32;

    ui.dummy([BAR_WIDTH, 4.0]);
    let origin = ui.cursor_screen_pos();
    let draw = ui.get_window_draw_list();
    let line_h = ui.text_line_height();
    let mut x = origin[0] + ((BAR_WIDTH - total_w) * 0.5).max(0.0);
    let cy = origin[1] + line_h * 0.5;

    for (color, label, mine) in items {
        let cx = x + DOT_R;
        draw.add_circle([cx, cy], DOT_R, color.rgba()).filled(true).build();
        if mine {
            let mut ring = color.rgba();
            ring[3] = 0.30;
            draw.add_circle([cx, cy], DOT_R + 2.0, ring).thickness(2.0).build();
        }
        let text_color = if mine { [1.0, 1.0, 1.0, 0.92] } else { [1.0, 1.0, 1.0, 0.60] };
        let tx = x + DOT_R * 2.0 + DOT_TEXT_GAP;
        draw.add_text([tx, origin[1]], text_color, &label);
        x = tx + ui.calc_text_size(&label)[0] + ITEM_GAP;
    }
    ui.dummy([BAR_WIDTH, line_h]);
}
