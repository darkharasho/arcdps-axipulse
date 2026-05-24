#![cfg(windows)]
//! Transparent notifier toast. Shows "Parsing…" while a log is being
//! parsed and "Parsed: <fight>" briefly after one lands. Independent
//! of the main AxiPulse window so users can keep that hidden.

use arcdps::imgui::{Condition, StyleColor, StyleVar, Ui, WindowFlags};

use crate::config::Config;
use crate::plugin::ParsedToast;

/// How long the "Parsed: …" toast lingers after a parse completes.
const PARSED_LINGER_SECS: f32 = 6.0;

const ALLY_GREEN: [f32; 4] = [0.40, 0.92, 0.55, 1.0];
const ENEMY_RED:  [f32; 4] = [1.00, 0.40, 0.40, 1.0];

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

    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([10.0, 8.0])),
        ui.push_style_var(StyleVar::WindowRounding(8.0)),
        ui.push_style_var(StyleVar::WindowBorderSize(0.0)),
    ];
    // Translucent dark panel; alpha pulses for "Parsing" and fades out
    // for the tail of "Parsed".
    // `body` is rendered as a series of (text, colour) segments laid
    // out left-to-right so the ally/enemy counts can be coloured.
    let (bg_alpha, accent, label, body): (f32, [f32; 4], &str, Vec<(String, [f32; 4])>) = match msg {
        Msg::Parsing(name) => {
            let t = ui.time() as f32;
            let pulse = 0.5 + 0.5 * ((t * 3.0).sin());
            let alpha = 0.55 + 0.15 * pulse;
            (alpha, [0.31, 0.86, 0.61, 1.0], "Parsing...",
             vec![(name.to_string(), [0.97, 0.97, 1.0, 1.0])])
        }
        Msg::Parsed(toast, age) => {
            // Linear fade across the final 1.5s of the linger window.
            let fade_in = 1.0_f32;
            let remain = (PARSED_LINGER_SECS - age).max(0.0);
            let alpha = (remain / 1.5).min(fade_in) * 0.75;
            let neutral = [0.97, 0.97, 1.0, 1.0];
            let mut segs: Vec<(String, [f32; 4])> = Vec::new();
            if !toast.map.is_empty() {
                segs.push((format!("{} \u{00b7} ", toast.map), neutral));
            }
            segs.push((format!("{}", toast.allies), ALLY_GREEN));
            segs.push((" v ".to_string(), neutral));
            segs.push((format!("{}", toast.enemies), ENEMY_RED));
            (alpha, [0.50, 0.78, 1.0, 1.0], "Parsed", segs)
        }
        Msg::Placeholder => (
            0.70,
            [0.31, 0.86, 0.61, 1.0],
            "AxiPulse Notifier",
            vec![("Drag to reposition. Hidden until a parse fires.".to_string(),
                  [0.97, 0.97, 1.0, 1.0])],
        ),
    };
    let bg = ui.push_style_color(StyleColor::WindowBg, [0.06, 0.07, 0.09, bg_alpha]);

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
            // Soft halo so the heartbeat reads on transparent bgs.
            let halo_r = icon_size * 0.65;
            let mut halo = accent; halo[3] = 0.18;
            draw.add_rect(
                [cx - halo_r, cy - halo_r],
                [cx + halo_r, cy + halo_r],
                halo,
            ).filled(true).rounding(halo_r).build();
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
        draw.add_text([after_icon[0], cursor[1]], accent, label);
        // Lay out body segments left-to-right at the second-line y.
        let body_y = cursor[1] + line_h;
        let mut bx = after_icon[0];
        // Honour the window's overall fade by scaling segment alpha by
        // bg_alpha so colours stay readable during the fade-out.
        let fade = (bg_alpha / 0.75).clamp(0.0, 1.0);
        for (text, color) in &body {
            let mut c = *color;
            c[3] *= fade;
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
}
