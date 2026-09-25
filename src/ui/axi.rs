//! The axi-design draw layer: one place where a raised element's
//! block, fill and outline are laid down, so five surfaces do not
//! hand-roll the sequence five ways.
//!
//! The geometry is pure and host-compilable — `Rect` is a plain pair of
//! corners, not an imgui type — because the two structural traps in
//! this conversion are arithmetic:
//!
//!   * imgui centres `add_rect` stroke ON the path, so an outline flush
//!     with its fill must sit `thickness / 2` inside it;
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

/// The rect to hand `add_rect(..).thickness(t)` so the stroke's OUTER
/// edge lands exactly on `r`'s edge.
pub fn outline_path(r: Rect, thickness: f32) -> Rect {
    r.inset(thickness / 2.0)
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

/// The filled length of a bar track — rule 7: a quantity is drawn as
/// length, never intensity.
pub fn bar_fill(track: Rect, frac: f32) -> Rect {
    Rect {
        min: track.min,
        max: [track.min[0] + track.w() * clamp_frac(frac), track.max[1]],
    }
}

// --- draw helpers -------------------------------------------------------

#[cfg(windows)]
use arcdps::imgui::Ui;

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

    let path = outline_path(r, border);
    if !path.is_degenerate() {
        draw.add_rect(path.min, path.max, theme::INK_LINE)
            .thickness(border).build();
    }
}

/// A panel: the heavier of the two form steps. `hovered` deepens the
/// offset — pass `false` for anything the user cannot click, because a
/// hover state on a non-interactive thing is a lie about affordance.
#[cfg(windows)]
pub fn panel(ui: &Ui, r: Rect, fill: [f32; 4], hovered: bool) {
    let offset = if hovered { theme::OFFSET_PANEL_HOVER } else { theme::OFFSET_PANEL };
    blocked(ui, r, fill, theme::BORDER_PANEL, offset);
}

/// A card or chip: the lighter of the two form steps.
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
#[cfg(windows)]
pub fn panel_inward(ui: &Ui, window: Rect, fill: [f32; 4]) -> Rect {
    let body = inward_body(window, theme::OFFSET_PANEL);
    blocked(ui, body, fill, theme::BORDER_PANEL, theme::OFFSET_PANEL);
    body
}

/// A quantity as length: an outlined track with a filled bar in it.
#[cfg(windows)]
pub fn bar(ui: &Ui, track: Rect, frac: f32, ink: [f32; 4]) {
    if track.is_degenerate() { return; }
    let draw = ui.get_window_draw_list();
    draw.add_rect(track.min, track.max, theme::GROUND).filled(true).build();
    let fill = bar_fill(track, frac);
    if !fill.is_degenerate() {
        draw.add_rect(fill.min, fill.max, ink).filled(true).build();
    }
    let path = outline_path(track, theme::BORDER_CONTROL);
    if !path.is_degenerate() {
        draw.add_rect(path.min, path.max, theme::INK_LINE)
            .thickness(theme::BORDER_CONTROL).build();
    }
}

/// The family motif, as two filled triangles meeting on the
/// horizontal. `size` is the full diagonal.
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
#[cfg(windows)]
pub fn rule(ui: &Ui, from: [f32; 2], to: [f32; 2]) {
    if !from.iter().chain(to.iter()).all(|v| v.is_finite()) { return; }
    ui.get_window_draw_list()
        .add_line(from, to, theme::RULE)
        .thickness(theme::BORDER_HAIRLINE)
        .build();
}
