//! The axi-design draw layer: one place where a raised element's
//! block, fill and outline are laid down, so five surfaces do not
//! hand-roll the sequence five ways.
//!
//! The geometry is pure and host-compilable — `Rect` is a plain pair of
//! corners, not an imgui type — because the two structural traps in
//! this conversion are arithmetic:
//!
//!   * imgui's `add_rect` stroke is neither flush nor centred on the
//!     path it is given, so outlines are drawn as four filled bands
//!     (`ring_parts`) instead of stroked;
//!   * the window draw list is clipped to the window rect, so the
//!     shell's offset block is drawn inward.
//!
//! Both are wrong-everywhere-and-subtle if mis-derived, which is why
//! they are unit-tested on the host rather than eyeballed in-game.

use super::theme;

/// An axis-aligned rectangle by its two corners. Deliberately not an
/// imgui type: this compiles and tests on Linux.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Rect {
    pub fn new(min: [f32; 2], max: [f32; 2]) -> Self {
        Self { min, max }
    }

    /// From a top-left corner and a size.
    pub fn at(pos: [f32; 2], size: [f32; 2]) -> Self {
        Self { min: pos, max: [pos[0] + size[0], pos[1] + size[1]] }
    }

    pub fn w(&self) -> f32 { self.max[0] - self.min[0] }
    pub fn h(&self) -> f32 { self.max[1] - self.min[1] }

    /// Nothing worth drawing: no area, or a coordinate imgui cannot
    /// use. Every draw helper below early-returns on this rather than
    /// emitting an inverted or NaN rect.
    pub fn is_degenerate(&self) -> bool {
        let finite = self.min.iter().chain(self.max.iter()).all(|v| v.is_finite());
        !finite || self.w() <= 0.0 || self.h() <= 0.0
    }

    /// Shrink by `d` on all four sides. Collapses to a zero-area rect
    /// at its own centre rather than inverting, so an over-inset rect
    /// is degenerate instead of drawn inside out.
    pub fn inset(&self, d: f32) -> Rect {
        let min = [self.min[0] + d, self.min[1] + d];
        let max = [self.max[0] - d, self.max[1] - d];
        Rect {
            min,
            max: [max[0].max(min[0]), max[1].max(min[1])],
        }
    }

    /// Translate without resizing.
    pub fn shifted(&self, dx: f32, dy: f32) -> Rect {
        Rect {
            min: [self.min[0] + dx, self.min[1] + dy],
            max: [self.max[0] + dx, self.max[1] + dy],
        }
    }
}

/// The four filled bands that make an outline of `thickness` sitting
/// flush inside `r`. Order: top, bottom, left, right.
///
/// Outlines are drawn as fills, not as a stroke, because imgui's
/// `AddRect` does not stroke the path it is handed: it insets it half a
/// pixel first (`imgui_draw.cpp`, `AddRect`: `p_min + 0.50`,
/// `p_max - 0.50`), on top of the half-thickness a path-centred stroke
/// already needs. Compensating for both leaves the outer edge half a
/// pixel inside the rect, so a pale hairline of the fill showed all the
/// way around every raised element — and on the right and bottom it cut
/// between the outline and the block the outline should meet.
/// `AddRectFilled` carries no such fudge: four filled bands land
/// exactly where they are asked to.
///
/// `thickness` is clamped to half the shorter side, so an outline
/// heavier than the rect it encloses fills the rect instead of
/// inverting. The side bands stop short of the top and bottom ones, so
/// no two bands overlap at a corner.
pub fn ring_parts(r: Rect, thickness: f32) -> [Rect; 4] {
    // NaN loses to `max`, so a non-finite thickness reads as zero and
    // every band below comes out degenerate rather than NaN.
    let t = thickness.max(0.0).min(r.w().min(r.h()) * 0.5);
    let [x0, y0] = r.min;
    let [x1, y1] = r.max;
    [
        Rect::new([x0, y0], [x1, y0 + t]),
        Rect::new([x0, y1 - t], [x1, y1]),
        Rect::new([x0, y0 + t], [x0 + t, y1 - t]),
        Rect::new([x1 - t, y0 + t], [x1, y1 - t]),
    ]
}

/// The hard offset block for `r`: same size, shifted down and right.
pub fn block_path(r: Rect, offset: f32) -> Rect {
    r.shifted(offset, offset)
}

/// The body rect for a window whose block must be drawn INWARD: the
/// window shrunk on its right and bottom by `offset`, so
/// `block_path(body, offset)` lands flush with the window's own edge
/// and inside the draw list's clip rect.
pub fn inward_body(window: Rect, offset: f32) -> Rect {
    let max = [
        (window.max[0] - offset).max(window.min[0]),
        (window.max[1] - offset).max(window.min[1]),
    ];
    Rect { min: window.min, max }
}

/// A fraction fit to draw with: clamped to `[0, 1]`, with anything
/// non-finite reading as empty. A zero-duration fight divides by zero
/// somewhere upstream, and a bar spanning the screen is worse than no
/// bar at all.
pub fn clamp_frac(frac: f32) -> f32 {
    if frac.is_finite() { frac.clamp(0.0, 1.0) } else { 0.0 }
}

/// A bar fraction with a sub-pixel fill suppressed. Pre-conversion every
/// value bar guarded its fill with `if bar_w > 0.5`, so a fraction too
/// small to cover half a pixel drew nothing at all; without it a sliver
/// appears where there used to be blank track, which reads as a stray
/// mark rather than as a quantity. `bar` applies this to every fill it
/// draws.
pub fn fill_frac(track_w: f32, frac: f32) -> f32 {
    let f = clamp_frac(frac);
    if track_w.is_finite() && track_w * f > 0.5 { f } else { 0.0 }
}

/// The filled length of a bar track — rule 7: a quantity is drawn as
/// length, never intensity.
pub fn bar_fill(track: Rect, frac: f32) -> Rect {
    Rect {
        min: track.min,
        max: [track.min[0] + track.w() * clamp_frac(frac), track.max[1]],
    }
}

/// Fit `text` into `avail` pixels, appending a single ellipsis when it
/// does not. `width_of` measures a string in the caller's font —
/// `|s| ui.calc_text_size(s)[0]` in the overlay, a stub on the host,
/// which is what makes this testable off Windows.
///
/// Each iteration shortens the BASE string and the ellipsis is appended
/// to a candidate, never to the string being shortened. The previous
/// form appended the ellipsis to `title` itself, so the next iteration
/// popped the ellipsis it had just added and re-appended it: the string
/// stopped changing and the condition never cleared — an infinite loop
/// inside GW2's render callback, i.e. a hang of the game.
///
/// `String::pop` removes a whole `char`, so every intermediate value is
/// valid UTF-8 and a multi-byte name can never be cut mid-character.
///
/// Returns an empty string if not even the ellipsis fits, rather than
/// drawing a glyph past the space it was given.
pub fn truncate_to_width(text: &str, avail: f32, width_of: impl Fn(&str) -> f32) -> String {
    if width_of(text) <= avail {
        return text.to_string();
    }
    let mut base = text.to_string();
    while !base.is_empty() {
        base.pop();
        let candidate = format!("{}\u{2026}", base.trim_end());
        if width_of(&candidate) <= avail {
            return candidate;
        }
    }
    String::new()
}

// --- draw helpers -------------------------------------------------------
//
// Every helper below acquires its own window draw list.
// `ui.get_window_draw_list()` takes a process-global single-instance
// lock that is released only when the list drops, and PANICS if a second
// list is acquired while the first is live — a panic that crosses the
// arcdps FFI boundary inside GW2's render callback. So none of these may
// be called while the caller holds a draw list of its own: confine each
// list to a bare block, or `drop` it, before calling one.

#[cfg(windows)]
use arcdps::imgui::{DrawListMut, Ui};

/// Lay `ink` down as an outline flush inside `r`. Takes the CALLER's
/// draw list rather than acquiring one, so a caller already holding a
/// list can outline without tripping the single-instance lock above.
#[cfg(windows)]
pub fn outline_on(draw: &DrawListMut, r: Rect, thickness: f32, ink: [f32; 4]) {
    if r.is_degenerate() { return; }
    for band in ring_parts(r, thickness) {
        if !band.is_degenerate() {
            draw.add_rect(band.min, band.max, ink).filled(true).build();
        }
    }
}

/// Lay down one raised element: block, then fill, then outline, in
/// that order. Order matters — the block must be painted before the
/// fill or it covers it.
#[cfg(windows)]
fn blocked(ui: &Ui, r: Rect, fill: [f32; 4], border: f32, offset: f32) {
    if r.is_degenerate() { return; }
    let draw = ui.get_window_draw_list();

    let block = block_path(r, offset);
    if !block.is_degenerate() {
        draw.add_rect(block.min, block.max, theme::INK_LINE)
            .filled(true).build();
    }
    draw.add_rect(r.min, r.max, fill).filled(true).build();

    outline_on(&draw, r, border, theme::INK_LINE);
}

/// A panel: the heavier of the two form steps. `hovered` deepens the
/// offset — pass `false` for anything the user cannot click, because a
/// hover state on a non-interactive thing is a lie about affordance.
/// Acquires its own draw list: never call this while one is live.
#[cfg(windows)]
pub fn panel(ui: &Ui, r: Rect, fill: [f32; 4], hovered: bool) {
    let offset = if hovered { theme::OFFSET_PANEL_HOVER } else { theme::OFFSET_PANEL };
    blocked(ui, r, fill, theme::BORDER_PANEL, offset);
}

/// A card or chip: the lighter of the two form steps.
/// Acquires its own draw list: never call this while one is live.
#[cfg(windows)]
pub fn card(ui: &Ui, r: Rect, fill: [f32; 4], hovered: bool) {
    let offset = if hovered { theme::OFFSET_CONTROL_HOVER } else { theme::OFFSET_CONTROL };
    blocked(ui, r, fill, theme::BORDER_CONTROL, offset);
}

/// A blocked, clickable chip: a tab, a toggle, a filter. Returns
/// `(clicked, width)` — the width so the caller can advance its own
/// cursor, since chips are laid out in rows by hand.
///
/// `id` must be unique within the window; it is suffixed onto the
/// invisible button's label, not drawn.
///
/// Selected chips wear the accent with near-black ink; the rest wear
/// SURFACE with dim text. Both lift on hover, because a chip is
/// genuinely clickable — rule 4 forbids the lift only on things that
/// are not.
/// Acquires its own draw list: never call this while one is live.
#[cfg(windows)]
pub fn chip(
    ui: &Ui,
    origin: [f32; 2],
    id: &str,
    label: &str,
    selected: bool,
    accent: [f32; 4],
    pad: [f32; 2],
) -> (bool, f32) {
    let w = ui.calc_text_size(label)[0] + pad[0] * 2.0;
    let h = ui.text_line_height() + pad[1] * 2.0;

    // Hit-test before drawing so the lift lands on the frame the
    // cursor arrives, not one frame later.
    ui.set_cursor_screen_pos(origin);
    let clicked = ui.invisible_button(format!("##{id}"), [w, h]);
    let hovered = ui.is_item_hovered();

    let fill = if selected { accent } else { theme::SURFACE };
    card(ui, Rect::at(origin, [w, h]), fill, hovered);
    let ink = if selected { theme::ACCENT_INK } else { theme::TEXT_DIM };
    ui.get_window_draw_list()
        .add_text([origin[0] + pad[0], origin[1] + pad[1]], ink, label);

    (clicked, w)
}

/// A panel for a window whose block cannot go outside it. Takes the
/// WINDOW rect and derives the body; returns the body so the caller
/// knows how much room the content has.
/// Acquires its own draw list: never call this while one is live.
#[cfg(windows)]
pub fn panel_inward(ui: &Ui, window: Rect, fill: [f32; 4]) -> Rect {
    let body = inward_body(window, theme::OFFSET_PANEL);
    blocked(ui, body, fill, theme::BORDER_PANEL, theme::OFFSET_PANEL);
    body
}

/// A quantity as length: an outlined track with a filled bar in it.
///
/// Returns the x of the fill's right edge — the boundary between the
/// filled part of the track and the empty part. `bar_text` needs it to
/// colour a label that crosses it, and deriving it here rather than in
/// each caller keeps the boundary the text splits on identical to the
/// one actually drawn.
/// Acquires its own draw list: never call this while one is live.
#[cfg(windows)]
pub fn bar(ui: &Ui, track: Rect, frac: f32, ink: [f32; 4]) -> f32 {
    if track.is_degenerate() { return track.min[0]; }
    let draw = ui.get_window_draw_list();
    draw.add_rect(track.min, track.max, theme::GROUND).filled(true).build();
    let fill = bar_fill(track, fill_frac(track.w(), frac));
    if !fill.is_degenerate() {
        draw.add_rect(fill.min, fill.max, ink).filled(true).build();
    }
    outline_on(&draw, track, theme::BORDER_CONTROL, theme::INK_LINE);
    fill.max[0]
}

/// Draw a label lying across a bar, in two inks split at `fill_x`.
///
/// A row label sits ON the bar rather than beside it, so part of it
/// lands on the fill and part on the empty track — and no single ink
/// reads on both. White on a saturated fill is the unreadable half.
/// So the string is drawn twice, each pass clipped to one side of the
/// boundary: near-black over the fill, the caller's ink over the track.
/// The two passes place the text at the same origin, so a word split by
/// the boundary is still one word, just two-toned.
///
/// This is the team bar's rule generalised — a count on a team fill is
/// already near-black, for the same reason `--axi-accent-ink` is.
///
/// Takes the caller's draw list: the callers here are mid-row with one
/// already open.
#[cfg(windows)]
pub fn bar_text(
    draw: &DrawListMut,
    track: Rect,
    fill_x: f32,
    pos: [f32; 2],
    over_track: [f32; 4],
    text: &str,
) {
    if text.is_empty() || track.is_degenerate() { return; }
    let x = if fill_x.is_finite() {
        fill_x.clamp(track.min[0], track.max[0])
    } else {
        track.min[0]
    };
    if x > track.min[0] {
        draw.with_clip_rect_intersect(track.min, [x, track.max[1]], || {
            draw.add_text(pos, theme::ACCENT_INK, text);
        });
    }
    if x < track.max[0] {
        draw.with_clip_rect_intersect([x, track.min[1]], track.max, || {
            draw.add_text(pos, over_track, text);
        });
    }
}

/// The family motif, as two filled triangles meeting on the
/// horizontal. `size` is the full diagonal.
/// Acquires its own draw list: never call this while one is live.
#[cfg(windows)]
pub fn diamond(ui: &Ui, center: [f32; 2], size: f32, ink: [f32; 4]) {
    if !size.is_finite() || size <= 0.0 { return; }
    if !center.iter().all(|v| v.is_finite()) { return; }
    let h = size * 0.5;
    let [cx, cy] = center;
    let draw = ui.get_window_draw_list();
    draw.add_triangle([cx, cy - h], [cx + h, cy], [cx - h, cy], ink)
        .filled(true).build();
    draw.add_triangle([cx - h, cy], [cx + h, cy], [cx, cy + h], ink)
        .filled(true).build();
}

/// An eyebrow or label. Uppercase and faint: with one font face at one
/// size, case and colour are the only weight hierarchy reachable.
#[cfg(windows)]
pub fn label(ui: &Ui, text: &str) {
    ui.text_colored(theme::TEXT_FAINT, text.to_uppercase());
}

/// An internal rule at hairline weight, for dividing content inside an
/// already-outlined panel. Rule 5: outlined means annotation.
/// Acquires its own draw list: never call this while one is live.
#[cfg(windows)]
pub fn rule(ui: &Ui, from: [f32; 2], to: [f32; 2]) {
    if !from.iter().chain(to.iter()).all(|v| v.is_finite()) { return; }
    ui.get_window_draw_list()
        .add_line(from, to, theme::RULE)
        .thickness(theme::BORDER_HAIRLINE)
        .build();
}
