#![cfg(windows)]
//! Notifier toast: a HUD surface, so it fills at `theme::ALPHA_HUD`
//! (times its own fade) while its ink draws unscaled.
//!
//! Shows "Parsing…" while a log is being parsed and "Parsed: <fight>"
//! briefly after one lands. Independent of the main AxiPulse window so
//! users can keep that hidden.

use arcdps::imgui::{Condition, StyleColor, StyleVar, Ui, WindowFlags};

use crate::config::Config;
use crate::plugin::ParsedToast;
use crate::ui::axi::{self, Rect};
use crate::ui::theme;

/// How long the "Parsed: …" toast lingers after a parse completes.
const PARSED_LINGER_SECS: f32 = 6.0;

/// The `Parsed` toast's peak surface alpha. Load-bearing in two places:
/// it is the peak the fade ramps to, and it is the reference the body
/// ink's fade is expressed relative to, so the ink reaches full opacity
/// exactly when the surface is at its peak. One constant, so the two
/// cannot drift apart.
const TOAST_PEAK_ALPHA: f32 = 0.75;

pub fn render(ui: &Ui, config: &mut Config) {
    if !config.show_notifications { return; }

    let parsing = crate::plugin::parsing_label();
    let last = crate::plugin::last_parsed();

    // Decide what (if anything) to show. Parsing wins over a fresh
    // "parsed" toast so the user sees the latest state.
    enum Msg<'a> { Parsing(&'a str), Parsed(&'a ParsedToast, f32), Placeholder }
    let parsed_age = last.as_ref().map(|(_, t)| t.elapsed().as_secs_f32());
    let msg = if let Some(label) = parsing.as_deref() {
        Some(Msg::Parsing(label))
    } else if let (Some((toast, _)), Some(age)) = (last.as_ref(), parsed_age) {
        if age <= PARSED_LINGER_SECS { Some(Msg::Parsed(toast, age)) } else { None }
    } else {
        None
    };
    // Force-show a placeholder card while the arcdps settings pane is
    // open so the user can drag the notifier into position even when
    // nothing is parsing right now.
    let msg = msg.or_else(|| {
        if crate::plugin::options_open_recently() { Some(Msg::Placeholder) } else { None }
    });
    let Some(msg) = msg else { return };

    // imgui paints WindowBg itself, so we take the fill away from it and
    // paint block, fill and outline ourselves, INWARD from the window
    // edge. That is what makes this surface's opacity one constant.
    let form_tokens = theme::push_form(ui);
    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([
            10.0 + theme::BORDER_PANEL,
            8.0 + theme::BORDER_PANEL,
        ])),
    ];
    // `body` is rendered as a series of (text, colour) segments laid
    // out left-to-right so the ally/enemy counts can be coloured.
    //
    // The match yields a `fade_scale` in [0, 1] rather than an absolute
    // background alpha: the surface fills at `ALPHA_HUD * fade_scale`,
    // so every pre-conversion timing is preserved while the HUD's
    // opacity stays a single constant.
    let accent = theme::accent(&config.accent);
    let (fade_scale, label_ink, label, body): (f32, [f32; 4], &str, Vec<(String, [f32; 4])>) = match msg {
        Msg::Parsing(name) => {
            // Same 3 rad/s pulse as before, expressed as a fraction of
            // the HUD alpha rather than as an absolute alpha.
            let t = ui.time() as f32;
            let pulse = 0.5 + 0.5 * ((t * 3.0).sin());
            let scale = (0.55 + 0.15 * pulse) / theme::ALPHA_HUD;
            (scale.min(1.0), accent, "Parsing...", vec![(name.to_string(), theme::TEXT)])
        }
        Msg::Parsed(toast, age) => {
            // Linear fade across the final 1.5s of the linger window,
            // unchanged.
            let remain = (PARSED_LINGER_SECS - age).max(0.0);
            let scale = (remain / 1.5).min(1.0) * (TOAST_PEAK_ALPHA / theme::ALPHA_HUD);
            let mut segs: Vec<(String, [f32; 4])> = Vec::new();
            // Per-team counts in the team bar's palette and order, so
            // the toast never disagrees with the widget.
            let teams = toast.counts.segments();
            if !toast.map.is_empty() {
                let sep = if teams.is_empty() { "" } else { " \u{00b7} " };
                segs.push((format!("{}{}", toast.map, sep), theme::TEXT));
            }
            for (i, (color, count)) in teams.iter().enumerate() {
                if i > 0 { segs.push((" v ".to_string(), theme::TEXT)); }
                segs.push((count.to_string(), color.rgba()));
            }
            // A completed parse is a status, not chrome — rule 5.
            (scale.min(1.0), theme::OK, "Parsed", segs)
        }
        Msg::Placeholder => (
            0.70 / theme::ALPHA_HUD,
            accent,
            "AxiPulse Notifier",
            vec![(
                "Drag to reposition. Hidden until a parse fires.".to_string(),
                theme::TEXT,
            )],
        ),
    };
    let bg = ui.push_style_color(StyleColor::WindowBg, theme::TRANSPARENT);

    let mut win = ui.window("##axipulse-notifier")
        .size([340.0, 0.0], Condition::Always)
        .flags(
            WindowFlags::NO_TITLE_BAR
                | WindowFlags::NO_RESIZE
                | WindowFlags::NO_SCROLLBAR
                | WindowFlags::NO_COLLAPSE
                | WindowFlags::NO_FOCUS_ON_APPEARING
                | WindowFlags::ALWAYS_AUTO_RESIZE,
        );
    if let Some(pos) = config.notifications_pos {
        win = win.position([pos.0, pos.1], Condition::FirstUseEver);
    } else {
        // Default anchor: top-right of the viewport.
        let [vw, _vh] = ui.io().display_size;
        win = win.position([vw - 360.0, 60.0], Condition::FirstUseEver);
    }
    let mut saved_pos: Option<(f32, f32)> = None;
    win.build(|| {
        // HUD surface. The toast's own fade multiplies ALPHA_HUD, so a
        // finishing toast dissolves without ever dimming its ink.
        let win_rect = Rect::at(ui.window_pos(), ui.window_size());
        axi::panel_inward(
            ui,
            win_rect,
            theme::with_alpha(theme::SURFACE, theme::ALPHA_HUD * fade_scale),
        );

        // AxiPulse heartbeat icon, sized to the two-line header. Pulses
        // size/alpha while parsing; static while showing the parsed
        // toast (alpha follows the bg fade).
        let icon = crate::ui::icons::lookup_bundled("__heartbeat__");
        let cursor = ui.cursor_screen_pos();
        let draw = ui.get_window_draw_list();
        let line_h = ui.text_line_height_with_spacing();
        let icon_box = line_h * 2.0;
        let (icon_size, _icon_alpha) = match msg {
            Msg::Parsing(_) => {
                let t = ui.time() as f32;
                let phase = (t / 1.1).fract();
                let beat = |c: f32, s: f32| { let d = phase - c; (-(d * d) / (2.0 * s * s)).exp() };
                let intensity = (beat(0.05, 0.05) + beat(0.22, 0.05)).clamp(0.0, 1.0);
                (icon_box * 0.7 + 4.0 * intensity, 0.7 + 0.3 * intensity)
            }
            Msg::Parsed(_, _) => (icon_box * 0.75, 0.95),
            Msg::Placeholder => (icon_box * 0.75, 0.95),
        };
        if let Some(handle) = icon {
            let cx = cursor[0] + icon_box * 0.5;
            let cy = cursor[1] + icon_box * 0.5;
            let half = icon_size * 0.5;
            // Square halo so the heartbeat reads on a busy backdrop;
            // the language has no soft round glow.
            let halo_r = icon_size * 0.65;
            let halo = theme::with_alpha(label_ink, 0.18);
            draw.add_rect(
                [cx - halo_r, cy - halo_r],
                [cx + halo_r, cy + halo_r],
                halo,
            ).filled(true).build();
            // No tint on the texture itself — the vendored imgui
            // binding's image-tint path appears to crash the host under
            // Wine when exercised.
            draw.add_image(
                handle.tex,
                [cx - half, cy - half],
                [cx + half, cy + half],
            ).build();
        }
        // Reserve the icon column and lay the two-line text block to
        // the right of it.
        ui.dummy([icon_box + 4.0, icon_box]);
        ui.same_line();
        let after_icon = ui.cursor_screen_pos();
        draw.add_text([after_icon[0], cursor[1]], label_ink, label);
        // Lay out body segments left-to-right at the second-line y.
        let body_y = cursor[1] + line_h;
        let mut bx = after_icon[0];
        // The ink carries the toast's own fade and is NOT scaled by the
        // surface's HUD alpha: `fade_scale` is pre-divided by ALPHA_HUD
        // so the surface lands on its exact pre-conversion alpha, and
        // the ALPHA_HUD factor below undoes that division. Without it a
        // fully-visible toast would draw dim text.
        let ink_fade = (theme::ALPHA_HUD * fade_scale / TOAST_PEAK_ALPHA).min(1.0);
        for (text, color) in &body {
            let c = theme::with_alpha(*color, color[3] * ink_fade);
            draw.add_text([bx, body_y], c, text);
            bx += ui.calc_text_size(text)[0];
        }
        let [px, py] = ui.window_pos();
        saved_pos = Some((px, py));
    });

    if let Some(pos) = saved_pos {
        if config.notifications_pos != Some(pos) {
            config.notifications_pos = Some(pos);
            config.save();
        }
    }

    bg.end();
    for tok in style_tokens { tok.end(); }
    for tok in form_tokens { tok.pop(); }
}
