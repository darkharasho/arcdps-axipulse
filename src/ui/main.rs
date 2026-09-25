#![cfg(windows)]
//! Single AxiPulse window. Hosts a fight picker, top-level tabs
//! (Pulse / Timeline), and dispatches to the corresponding content
//! renderer in `ui::pulse` or `ui::timeline`.

use std::sync::Mutex;

use arcdps::imgui::{Condition, StyleColor, StyleVar, Ui};
use once_cell::sync::Lazy;

use crate::config::Config;
use crate::state::AppState;
use crate::ui::axi::{self, Rect};
use crate::ui::theme;

/// The window's `WindowPadding`. Named rather than inlined because the
/// inward panel reconstructs the window's inner rect by re-expanding the
/// content region by exactly this, so the two must never drift. The
/// `+ BORDER_PANEL` is what keeps body text off our own 4px outline.
const WINDOW_PAD: [f32; 2] = [14.0 + theme::BORDER_PANEL, 12.0 + theme::BORDER_PANEL];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopTab { Pulse, Timeline, Map }

static TOP_TAB: Lazy<Mutex<TopTab>> = Lazy::new(|| Mutex::new(TopTab::Pulse));

/// Which fight is rendered in the body. `Latest` follows the most
/// recent parsed fight (auto-updates as new ones land); `History(i)`
/// pins to a specific past fight via `AppState::history(i)`.
#[derive(Debug, Clone, Copy)]
enum FightSel { Latest, History(usize) }

static FIGHT_SEL: Lazy<Mutex<FightSel>> = Lazy::new(|| Mutex::new(FightSel::Latest));

pub fn render(ui: &Ui, state: &AppState, config: &mut Config) {
    if !config.show_pulse { return; }

    // Resolved before the window closure borrows `config` mutably.
    let accent = crate::ui::theme::accent(&config.accent);

    // imgui paints WindowBg itself and clips the window draw list to
    // the window rect, so we take the fill away from imgui and paint
    // block, fill and outline ourselves, INWARD from the window edge.
    // Consequence: this surface's opacity is one constant, not two
    // code paths.
    let form_tokens = theme::push_form(ui);
    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding(WINDOW_PAD)),
        // timeline.rs's lane clearance depends on this spacing.
        ui.push_style_var(StyleVar::ItemSpacing([8.0, 8.0])),
    ];
    let color_tokens = [
        ui.push_style_color(StyleColor::WindowBg, theme::TRANSPARENT),
        ui.push_style_color(StyleColor::TitleBg, theme::SURFACE),
        ui.push_style_color(StyleColor::TitleBgActive, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::Separator, theme::RULE),
        ui.push_style_color(StyleColor::Text, theme::TEXT),
        ui.push_style_color(StyleColor::TextDisabled, theme::TEXT_FAINT),
        ui.push_style_color(StyleColor::Button, theme::SURFACE),
        ui.push_style_color(StyleColor::ButtonHovered, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::ButtonActive, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::FrameBg, theme::SURFACE),
        ui.push_style_color(StyleColor::FrameBgHovered, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::FrameBgActive, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::PopupBg, theme::SURFACE),
        ui.push_style_color(StyleColor::Header, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::HeaderHovered, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::HeaderActive, theme::SURFACE_RAISED),
    ];

    let mut window = ui.window("AxiPulse").size([720.0, 600.0], Condition::FirstUseEver);
    if let Some(pos) = config.pulse_pos {
        window = window.position([pos.0, pos.1], Condition::FirstUseEver);
    }
    let mut open = true;
    window.opened(&mut open).build(|| {
        // Our own fill, inside the window and inside the clip rect.
        // ALPHA_READING: this is a surface you open in order to read.
        //
        // NOT window_pos()/window_size(): that is the OUTER rect, and
        // Begin pushes a clip rect of the INNER rect (title bar removed
        // from the top, scrollbars removed from the right/bottom), so an
        // outer-rect panel loses its top outline always and its right
        // outline plus offset block the moment a scrollbar appears.
        //
        // The inner rect is reconstructed from three safe accessors,
        // taken here as the closure's first statement while the cursor
        // is still at its start position. Written as the substitution so
        // the next reader can check it rather than trust it — imgui.cpp
        // line numbers are the pinned fork's imgui-master copy:
        //
        //   cursor_screen_pos() = DC.CursorPos = DC.CursorStartPos
        //                       = Pos + Pad - Scroll + Deco1        (8003-8006)
        //                       = ContentRegionRect.Min             (7990-7991)
        //   content_region_avail() = ContentRegionRect.Max - CursorPos
        //                          = Size - 2*Pad - Deco1 - Deco2   (7992-7993)
        //
        // so, with Pad == WINDOW_PAD (the padding we pushed ourselves):
        //
        //   cursor - Pad          = Pos + Deco1 - Scroll
        //   cursor + avail + Pad  = Pos + Size - Deco2 - Scroll
        //   InnerRect             = [Pos + Deco1, Pos + Size - Deco2]
        //
        // i.e. the padding re-expansion alone yields InnerRect MINUS
        // Scroll. `- Scroll` appears with the SAME sign in both corners
        // (it is baked into ContentRegionRect.Min, which .Max is derived
        // from), so it does NOT cancel; it has to be added back. Adding
        // it lands on InnerRect exactly, whatever the scroll position.
        //
        // Deco1 is the title-bar height and Deco2 the scrollbar sizes,
        // both already inside the two accessors, so the scrollbar-present
        // and scrollbar-absent cases need no special casing.
        let content_min = ui.cursor_screen_pos();
        let avail = ui.content_region_avail();
        let scroll = [ui.scroll_x(), ui.scroll_y()];
        let inner = Rect::new(
            [
                content_min[0] - WINDOW_PAD[0] + scroll[0],
                content_min[1] - WINDOW_PAD[1] + scroll[1],
            ],
            [
                content_min[0] + avail[0] + WINDOW_PAD[0] + scroll[0],
                content_min[1] + avail[1] + WINDOW_PAD[1] + scroll[1],
            ],
        );
        axi::panel_inward(ui, inner, theme::with_alpha(theme::SURFACE, theme::ALPHA_READING));

        render_header(ui, state, accent);
        ui.dummy([0.0, 2.0]);

        // Resolve selected fight. If selection is stale (e.g. history is
        // shorter than the requested index), fall back to latest.
        let sel = FIGHT_SEL.lock().ok().map(|g| *g).unwrap_or(FightSel::Latest);
        let record = match sel {
            FightSel::Latest => state.current(),
            FightSel::History(i) => state.history(i).or_else(|| state.current()),
        };
        let Some(record) = record else {
            ui.text_disabled("Waiting for the first parsed fight...");
            return;
        };
        let fight = &record.data;
        // `self_idx` is resolved once at projection time from
        // `encounter.recorded_by`, not re-guessed here every frame.
        let Some(idx) = fight.self_idx else {
            ui.text_disabled("Could not identify local player in this fight.");
            return;
        };

        render_top_tabs(ui, accent);
        ui.dummy([0.0, 4.0]);

        let tab = TOP_TAB.lock().ok().map(|g| *g).unwrap_or(TopTab::Pulse);
        let derived = record.derived.as_ref();
        match tab {
            TopTab::Pulse    => crate::ui::pulse::render_content(ui, fight, idx, derived, accent),
            TopTab::Timeline => crate::ui::timeline::render_content(ui, fight, idx, derived, accent, &mut config.timeline_layers),
            TopTab::Map      => crate::ui::map::render_content(ui, fight, idx, derived, &record.log_path),
        }
    });

    if !open {
        config.show_pulse = false;
        config.save();
    }

    for tok in color_tokens { tok.pop(); }
    for tok in style_tokens { tok.pop(); }
    for tok in form_tokens { tok.pop(); }
}

/// Header row: AxiPulse logo + brand label + (when parsing) a pulsing
/// indicator on the left, and the fight-picker combo right-aligned on
/// the same line.
fn render_header(ui: &Ui, state: &AppState, accent: [f32; 4]) {
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
        draw.add_text([x, brand_text_y], theme::TEXT, "Axi");
        draw.add_text([x + axi_w, brand_text_y], accent, "Pulse");
    }
    let brand_end_x = x + axi_w + pulse_w;
    let is_parsing = crate::plugin::is_parsing();
    if is_parsing {
        render_parsing_pulse(ui, brand_end_x + 14.0, cursor[1] + row_h * 0.5, brand_text_y, accent);
    }

    // --- Right-aligned fight picker on the same row ---
    let combo_x = cursor[0] + avail - combo_w;
    let combo_y = cursor[1] + (row_h - ui.frame_height_with_spacing()).max(0.0) * 0.5;
    ui.set_cursor_screen_pos([combo_x, combo_y]);
    ui.set_next_item_width(combo_w);
    render_fight_picker_combo(ui, state);

    // Park the cursor at the bottom of the row for downstream layout.
    ui.set_cursor_screen_pos([cursor[0], cursor[1] + row_h]);
    render_update_pill(ui);
}

fn render_update_pill(ui: &arcdps::imgui::Ui) {
    use crate::updater::{snapshot, start_install, dismiss_error, UpdateState};
    let st = snapshot();
    // Rule 6: META is the cool ink reserved for meta, and an update's
    // download progress is exactly that — information about the
    // software rather than about the fight.
    let (label, color) = match &st {
        UpdateState::Available { tag, .. } =>
            (format!("Update available \u{00b7} {tag}"), theme::OK),
        UpdateState::Downloading { pct, .. } if pct.is_finite() =>
            (format!("Downloading... {:.0}%", pct), theme::META),
        UpdateState::Downloading { .. } =>
            ("Downloading...".to_string(), theme::META),
        UpdateState::Installed { tag } =>
            (format!("Restart GW2 to load {tag}"), theme::WARN),
        UpdateState::Failed { msg } =>
            (format!("Update failed: {msg}"), theme::DANGER),
        _ => return,
    };
    ui.text_colored(color, &label);
    if let UpdateState::Available { .. } = &st {
        ui.same_line();
        if ui.small_button("Install") {
            match crate::plugin::dll_dir() {
                Some(dir) => start_install(dir),
                None => crate::updater::set_failed("could not locate DLL directory"),
            }
        }
    }
    if let UpdateState::Failed { .. } = &st {
        ui.same_line();
        if ui.small_button("\u{00d7}##dismiss-update") { dismiss_error(); }
    }
}

/// Heartbeat icon (lucide Activity) pulsed in scale + alpha, mirroring
/// the `heartbeat-pulse` animation AxiPulse's web UI uses.
fn render_parsing_pulse(ui: &Ui, cx: f32, cy: f32, label_y: f32, accent: [f32; 4]) {
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

    // `get_window_draw_list` takes a GLOBAL single-instance lock and
    // panics if a second list is acquired while the first is alive
    // (arcdps-imgui `draw_list.rs::lock_draw_list`). A panic here
    // crosses the arcdps FFI boundary, so the lifetime is structural
    // rather than incidental: the list below is confined to a block
    // that ends before anything else needs one, and `axi::diamond`
    // takes its own. Do not flatten this block, and do not acquire a
    // second list inside it.
    let mut fallback = false;
    {
        let draw = ui.get_window_draw_list();
        if let Some(handle) = icon {
            let half = icon_size * 0.5;
            let x0 = cx - half;
            let y0 = cy - half;
            // Square halo: the language has no soft round glow. Scaled
            // with the beat so the pulse still reads on a busy backdrop.
            let halo_r = icon_size * 0.65 + 2.0 * intensity;
            let halo = theme::with_alpha(accent, 0.10 + 0.25 * intensity * alpha);
            draw.add_rect([cx - halo_r, cy - halo_r], [cx + halo_r, cy + halo_r], halo)
                .filled(true)
                .build();
            // No tint on the texture itself — the vendored imgui
            // binding's image-tint path appears to crash the host under
            // Wine when exercised. The icon was rasterised already
            // coloured so untinted is fine.
            draw.add_image(handle.tex, [x0, y0], [x0 + icon_size, y0 + icon_size]).build();
        } else {
            // Bundled icon not loaded yet (D3D11 device unavailable on
            // the first frame). Drawn after this block, not here, so it
            // is not holding the list open.
            fallback = true;
        }
    }
    if fallback {
        // Fall back to the family motif so we still show *some* parsing
        // indicator.
        axi::diamond(ui, [cx, cy], 10.0 + 4.0 * intensity, theme::with_alpha(accent, alpha));
    }

    // "parsing..." label to the right of the icon, faint, alpha pulses
    // with the beat. Painted last, as it was before the conversion.
    let label = "parsing...";
    let text_color = theme::with_alpha(theme::TEXT_FAINT, 0.60 + 0.35 * intensity);
    ui.get_window_draw_list()
        .add_text([cx + base_size * 0.6 + 8.0, label_y], text_color, label);
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
            rec.data.map_name.as_str(),
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
                rec.data.map_name.as_str(),
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

/// The top tabs as blocked controls: the selected one wears the accent
/// with near-black ink, the rest surface with dim text, and only the
/// hovered one lifts — they are buttons, so the lift is honest.
/// `axi::chip` lays all of that down, the same way Pulse's and
/// Timeline's own strips do.
fn render_top_tabs(ui: &Ui, accent: [f32; 4]) {
    let mut current = TOP_TAB.lock().ok().map(|g| *g).unwrap_or(TopTab::Pulse);
    let tabs = [("Pulse", TopTab::Pulse), ("Timeline", TopTab::Timeline), ("Map", TopTab::Map)];
    let pad = [14.0_f32, 6.0_f32];
    let h = ui.text_line_height() + pad[1] * 2.0;

    let origin = ui.cursor_screen_pos();
    let mut x = origin[0];
    for (label, tab) in tabs.iter() {
        let (clicked, w) = axi::chip(
            ui,
            [x, origin[1]],
            &format!("top-tab-{label}"),
            label,
            current == *tab,
            accent,
            pad,
        );
        if clicked { current = *tab; }
        // Clear the neighbour's offset block before the next chip.
        x += w + theme::OFFSET_CONTROL + 6.0;
    }
    // Reserve the strip's span with a regular item rather than parking
    // the cursor absolutely, so the strip stays in the layout and in the
    // window's content-size calculation and the chips' offset blocks are
    // not overdrawn. This leaves the cursor one ItemSpacing.y (8px)
    // BELOW where an absolute park would have left it; the extra 8px is
    // deliberate and matches pulse.rs's and timeline.rs's strips.
    ui.set_cursor_screen_pos(origin);
    ui.dummy([x - origin[0], h + theme::OFFSET_CONTROL]);

    if let Ok(mut g) = TOP_TAB.lock() { *g = current; }
}

fn mmss(ms: u64) -> String {
    let sec = ms / 1000;
    format!("{}:{:02}", sec / 60, sec % 60)
}
