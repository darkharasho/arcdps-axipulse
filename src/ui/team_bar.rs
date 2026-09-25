#![cfg(windows)]
//! Persistent team-count bar: a small frameless window with one
//! stacked bar showing red/green/blue player counts from the latest
//! parsed fight. Independent of the main AxiPulse window, like the
//! notifier toast. Colors follow axibridge's WvW team palette
//! (`ui::series`), which the accent never recolours — rule 10.
//!
//! A HUD surface: small, always-on, and sitting over gameplay rather
//! than being opened and read, so it fills at `theme::ALPHA_HUD` while
//! the three reading surfaces went opaque. Its ink is NOT scaled.
//!
//! Two renderings behind `config.team_bar_compact`:
//!   full    — map-name header + total, segmented bar in a GROUND
//!             track, legend with a "you" marker on the viewer's own
//!             team
//!   compact — just the bar

use arcdps::imgui::{Condition, StyleColor, StyleVar, Ui, WindowFlags};

use crate::config::Config;
use crate::state::AppState;
use crate::ui::axi::{self, Rect};
use crate::ui::theme;
use crate::wvw_teams::{count_teams, team_color, TeamColor, TeamCounts};

const BAR_WIDTH: f32 = 264.0;
/// Window padding. `WIN_WIDTH` below carries the same x term, so the two
/// MUST move together: the content region is the window minus twice this,
/// and `draw_bar` / `draw_header` lay out against `BAR_WIDTH` assuming
/// they are exactly equal.
const PAD_X: f32 = 12.0 + theme::BORDER_PANEL;
const PAD_Y: f32 = 9.0 + theme::BORDER_PANEL;
/// Sized so the content region is exactly `BAR_WIDTH` by construction.
/// `SetNextWindowSize` with `Condition::Always` overrides
/// `ALWAYS_AUTO_RESIZE` on x, so imgui will not absorb a mismatch.
const WIN_WIDTH: f32 = BAR_WIDTH + PAD_X * 2.0;
const BAR_HEIGHT: f32 = 20.0;
/// Gap between segments; the dark track shows through.
const SEG_GAP: f32 = 2.0;
/// Floor on segment width so a 1-player team stays a legible pip
/// instead of a 2px sliver.
const MIN_SEG_WIDTH: f32 = 14.0;

pub fn render(ui: &Ui, state: &AppState, config: &mut Config) {
    if !config.show_team_bar { return; }

    let fight = state.current();
    let counts = fight.map(|f| count_teams(&f.data));
    // The viewer's own team, for the legend "you" marker.
    let self_color = fight.and_then(|f| {
        let idx = f.derived.self_idx?;
        Some(team_color(&f.data.players.get(idx)?.team))
    });

    // imgui paints WindowBg itself and clips the window draw list to the
    // window rect, so we take the fill away from imgui and paint block,
    // fill and outline ourselves, INWARD from the window edge. That is
    // what makes this surface's opacity one constant rather than two
    // code paths.
    let form_tokens = theme::push_form(ui);
    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([PAD_X, PAD_Y])),
    ];
    let color_tokens = [
        ui.push_style_color(StyleColor::WindowBg, theme::TRANSPARENT),
        ui.push_style_color(StyleColor::Text, theme::TEXT),
        ui.push_style_color(StyleColor::TextDisabled, theme::TEXT_FAINT),
    ];

    let mut win = ui.window("##axipulse-team-bar")
        .size([WIN_WIDTH, 0.0], Condition::Always)
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
        // HUD surface: translucent fill, fully opaque ink. A see-through
        // offset block with the battlefield moving inside it reads as a
        // rendering fault, not as a hard block.
        let win_rect = Rect::at(ui.window_pos(), ui.window_size());
        axi::panel_inward(ui, win_rect, theme::with_alpha(theme::SURFACE, theme::ALPHA_HUD));

        match counts {
            Some(c) if c.total() > 0 => {
                if !compact {
                    // `encounter.map` carries no "Detailed WvW - "
                    // prefix, unlike EI's `fightName`.
                    let map = fight.map(|f| f.data.map_name.to_uppercase()).unwrap_or_default();
                    draw_header(ui, &map, c.total());
                }
                draw_bar(ui, c);
                if !compact {
                    draw_legend(ui, c, self_color);
                }
            }
            _ => {
                let msg = "Waiting for a parsed fight...";
                let tw = ui.calc_text_size(msg)[0];
                // Centre on the panel FACE, not the window. `panel_inward`
                // takes `OFFSET_PANEL` off the right edge for the block, so
                // the face is that much narrower than the content region
                // this text is laid out in.
                let indent = ((BAR_WIDTH - theme::OFFSET_PANEL - tw) * 0.5).max(0.0);
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
    for tok in form_tokens { tok.pop(); }
}

fn draw_header(ui: &Ui, map: &str, total: u32) {
    let origin = ui.cursor_screen_pos();
    let line_h = ui.text_line_height();

    // Right-aligned "<n> players", bright count + dim suffix.
    let count_txt = total.to_string();
    let suffix = " players";
    let count_w = ui.calc_text_size(&count_txt)[0];
    let suffix_w = ui.calc_text_size(suffix)[0];
    let right = origin[0] + BAR_WIDTH;

    // Map name on the left, truncated with an ellipsis if it would
    // collide with the total.
    let avail = BAR_WIDTH - suffix_w - count_w - 10.0;
    // Pure, and host-tested: the loop this replaces never terminated.
    let title = axi::truncate_to_width(map, avail, |s| ui.calc_text_size(s)[0]);

    // One draw list, confined to this block: `get_window_draw_list`
    // takes a process-global single-instance lock and panics if a
    // second list is acquired while this one is alive.
    {
        let draw = ui.get_window_draw_list();
        draw.add_text([right - suffix_w, origin[1]], theme::TEXT_DIM, suffix);
        draw.add_text([right - suffix_w - count_w, origin[1]], theme::TEXT, &count_txt);
        draw.add_text(origin, theme::TEXT_FAINT, &title);
    }

    ui.dummy([BAR_WIDTH, line_h + 4.0]);
}

fn draw_bar(ui: &Ui, counts: TeamCounts) {
    let total = counts.total() as f32;
    let segments = counts.segments();
    let gaps = SEG_GAP * (segments.len().saturating_sub(1)) as f32;
    let usable = BAR_WIDTH - gaps;

    let origin = ui.cursor_screen_pos();

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

    // Label metrics up front so nothing inside the draw-list block
    // needs a second list.
    let label_sizes: Vec<[f32; 2]> = labels.iter().map(|l| ui.calc_text_size(l)).collect();

    let track = Rect::at(origin, [BAR_WIDTH, BAR_HEIGHT]);
    {
        // One list for the whole bar; the block closes before `dummy`.
        let draw = ui.get_window_draw_list();

        // The track behind the segments; shows through the gaps. Not
        // `axi::bar`: the segments are three fractions, not one.
        draw.add_rect(track.min, track.max, theme::GROUND).filled(true).build();

        let mut x = origin[0];
        for (i, (color, _count)) in segments.iter().copied().enumerate() {
            let w = widths[i];
            let seg = Rect::new([x, track.min[1]], [x + w, track.max[1]]);
            if !seg.is_degenerate() {
                draw.add_rect(seg.min, seg.max, color.rgba()).filled(true).build();
            }

            // Count label centered in the segment; the width floor above
            // guarantees it fits. Near-black on the team ink, the same
            // reason `--axi-accent-ink` is near-black.
            let ts = label_sizes[i];
            let tx = x + (w - ts[0]) * 0.5;
            let ty = track.min[1] + (BAR_HEIGHT - ts[1]) * 0.5;
            draw.add_text([tx, ty], theme::ACCENT_INK, &labels[i]);

            x += w + SEG_GAP;
        }

        // One near-black outline around the whole bar, over the
        // segments, its stroke inside the track's edge.
        let path = axi::outline_path(track, theme::BORDER_CONTROL);
        if !path.is_degenerate() {
            draw.add_rect(path.min, path.max, theme::INK_LINE)
                .thickness(theme::BORDER_CONTROL)
                .build();
        }
    }

    ui.dummy([BAR_WIDTH, BAR_HEIGHT]);
}

fn draw_legend(ui: &Ui, counts: TeamCounts, self_color: Option<TeamColor>) {
    const DOT_R: f32 = 3.5;
    const DOT_TEXT_GAP: f32 = 5.0;
    const ITEM_GAP: f32 = 12.0;

    let segments = counts.segments();
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
    let line_h = ui.text_line_height();
    let mut x = origin[0] + ((BAR_WIDTH - total_w) * 0.5).max(0.0);
    let cy = origin[1] + line_h * 0.5;

    // Lay out first, draw second: `axi::diamond` takes its own draw
    // list, so it must not be called while one is held here.
    struct Item { center: [f32; 2], size: f32, ink: [f32; 4], tx: f32, label: String }
    let mut laid: Vec<Item> = Vec::with_capacity(items.len());
    for (color, label, mine) in items {
        let tx = x + DOT_R * 2.0 + DOT_TEXT_GAP;
        // The viewer's own team reads as a larger diamond; the old
        // translucent ring is gone — rule 2 forbids colour at partial
        // opacity over the ground.
        let size = if mine { DOT_R * 2.0 + 3.0 } else { DOT_R * 2.0 };
        laid.push(Item {
            center: [x + DOT_R, cy],
            size,
            ink: color.rgba(),
            tx,
            label: label.clone(),
        });
        x = tx + ui.calc_text_size(&label)[0] + ITEM_GAP;
    }

    {
        let draw = ui.get_window_draw_list();
        for it in &laid {
            // A legend label that identifies a coloured series wears
            // that series' ink (ruling 7), not a ramp step. "You" is
            // carried by the larger diamond and the "· you"
            // suffix instead.
            draw.add_text([it.tx, origin[1]], it.ink, &it.label);
        }
    }
    for it in &laid {
        axi::diamond(ui, it.center, it.size, it.ink);
    }

    ui.dummy([BAR_WIDTH, line_h]);
}
