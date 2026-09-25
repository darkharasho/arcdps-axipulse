//! Host tests for the axi chrome tokens and the pure draw geometry.
//! These run under plain `cargo test` on Linux: nothing here touches
//! `arcdps::imgui`, which is Windows-only.

use arcdps_axipulse::ui::theme;

#[test]
fn rgb_converts_eight_bit_channels_at_full_alpha() {
    assert_eq!(theme::rgb(0x00, 0x00, 0x00), [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(theme::rgb(0xff, 0xff, 0xff), [1.0, 1.0, 1.0, 1.0]);
    let mid = theme::rgb(0x80, 0x40, 0x20);
    assert!((mid[0] - 128.0 / 255.0).abs() < 1e-6);
    assert!((mid[1] - 64.0 / 255.0).abs() < 1e-6);
    assert!((mid[2] - 32.0 / 255.0).abs() < 1e-6);
    assert_eq!(mid[3], 1.0);
}

#[test]
fn tokens_match_the_axi_design_values() {
    // Verbatim from @axiapps/axi-design dist/axi.css. If a token
    // changes upstream, this test is where it surfaces.
    assert_eq!(theme::GROUND, theme::rgb(0x15, 0x18, 0x1d));
    assert_eq!(theme::SURFACE, theme::rgb(0x22, 0x27, 0x31));
    assert_eq!(theme::SURFACE_RAISED, theme::rgb(0x2b, 0x31, 0x3d));
    assert_eq!(theme::INK_LINE, theme::rgb(0x0c, 0x0e, 0x12));
    assert_eq!(theme::RULE, theme::rgb(0x3a, 0x42, 0x50));
    assert_eq!(theme::TEXT, theme::rgb(0xf4, 0xf6, 0xf9));
    assert_eq!(theme::TEXT_DIM, theme::rgb(0xa7, 0xb0, 0xbe));
    assert_eq!(theme::TEXT_FAINT, theme::rgb(0x7c, 0x86, 0x95));
    assert_eq!(theme::META, theme::rgb(0x4e, 0xc3, 0xff));
    assert_eq!(theme::OK, theme::rgb(0x2f, 0xd3, 0x8a));
    assert_eq!(theme::WARN, theme::rgb(0xff, 0x7a, 0x2f));
    assert_eq!(theme::DANGER, theme::rgb(0xff, 0x52, 0x52));
}

#[test]
fn all_eleven_accents_resolve_to_their_own_value() {
    let expected: [(&str, [f32; 4]); 11] = [
        ("axi-gold", theme::rgb(0xff, 0xc5, 0x3d)),
        ("electric-blue", theme::rgb(0x3b, 0x82, 0xf6)),
        ("refined-cyan", theme::rgb(0x5e, 0xad, 0xd5)),
        ("amber-warm", theme::rgb(0xf5, 0x9e, 0x0b)),
        ("emerald-mint", theme::rgb(0x34, 0xd3, 0x99)),
        ("rose-pink", theme::rgb(0xf4, 0x3f, 0x5e)),
        ("violet-purple", theme::rgb(0x8b, 0x5c, 0xf6)),
        ("crimson-red", theme::rgb(0xef, 0x44, 0x44)),
        ("slate-silver", theme::rgb(0x94, 0xa3, 0xb8)),
        ("teal-ocean", theme::rgb(0x14, 0xb8, 0xa6)),
        ("gold-bronze", theme::rgb(0xd4, 0xa0, 0x17)),
    ];
    assert_eq!(expected.len(), theme::ACCENTS.len());
    for (id, value) in expected {
        assert_eq!(theme::accent(id), value, "accent {id}");
    }
}

#[test]
fn unknown_accent_ids_fall_back_to_the_default_without_panicking() {
    let default = theme::accent(theme::DEFAULT_ACCENT_ID);
    assert_eq!(default, theme::rgb(0x34, 0xd3, 0x99));
    // A hand-edited config, a downgrade, a locale-cased id, and a
    // partial match must all land on the default rather than panic:
    // this runs inside GW2's render callback.
    for id in ["", "Emerald-Mint", "EMERALD-MINT", "emerald", "nope", "  emerald-mint  "] {
        assert_eq!(theme::accent(id), default, "id {id:?}");
    }
}

#[test]
fn accent_index_is_the_combo_position_and_defaults_in_range() {
    assert_eq!(theme::accent_index("axi-gold"), 0);
    assert_eq!(theme::accent_index("emerald-mint"), 4);
    assert_eq!(theme::accent_index("gold-bronze"), 10);
    let fallback = theme::accent_index("nope");
    assert_eq!(fallback, theme::accent_index(theme::DEFAULT_ACCENT_ID));
    assert!(fallback < theme::ACCENTS.len());
}

#[test]
fn accent_labels_line_up_with_accents_positionally() {
    let labels = theme::accent_labels();
    assert_eq!(labels.len(), theme::ACCENTS.len());
    for (i, label) in labels.iter().enumerate() {
        assert_eq!(*label, theme::ACCENTS[i].1);
    }
    assert_eq!(labels[4], "Emerald Mint");
}

#[test]
fn the_two_form_steps_stay_ordered_at_every_positive_scale() {
    // The point of two weight steps is that a panel outweighs a
    // control. Tuning SCALE must never break that, and tuning the two
    // steps independently is what would.
    for scale in [0.5_f32, 0.75, 1.0, 1.5, 2.0, 3.0] {
        let (bp, bc) = (theme::TOKEN_BORDERS.0 * scale, theme::TOKEN_BORDERS.1 * scale);
        let (op, oc) = (theme::TOKEN_OFFSETS.0 * scale, theme::TOKEN_OFFSETS.1 * scale);
        let (ph, ch) = (theme::TOKEN_OFFSETS_HOVER.0 * scale, theme::TOKEN_OFFSETS_HOVER.1 * scale);
        assert!(bp > bc, "panel border must outweigh control at scale {scale}");
        assert!(op > oc, "panel offset must outweigh control at scale {scale}");
        assert!(ph > op, "panel hover must lift above rest at scale {scale}");
        assert!(ch > oc, "control hover must lift above rest at scale {scale}");
        assert!(ph > ch, "panel hover must outweigh control hover at scale {scale}");
        assert!(bc > 0.0 && oc > 0.0, "no weight vanishes at scale {scale}");
    }
}

#[test]
fn shipped_geometry_consts_are_the_tokens_times_scale() {
    assert_eq!(theme::SCALE, 1.0);
    assert_eq!(theme::BORDER_PANEL, theme::TOKEN_BORDERS.0 * theme::SCALE);
    assert_eq!(theme::BORDER_CONTROL, theme::TOKEN_BORDERS.1 * theme::SCALE);
    assert_eq!(theme::OFFSET_PANEL, theme::TOKEN_OFFSETS.0 * theme::SCALE);
    assert_eq!(theme::OFFSET_CONTROL, theme::TOKEN_OFFSETS.1 * theme::SCALE);
    assert_eq!(theme::OFFSET_PANEL_HOVER, theme::TOKEN_OFFSETS_HOVER.0 * theme::SCALE);
    assert_eq!(theme::OFFSET_CONTROL_HOVER, theme::TOKEN_OFFSETS_HOVER.1 * theme::SCALE);
    assert_eq!(theme::BORDER_PANEL, 4.0);
    assert_eq!(theme::OFFSET_PANEL, 6.0);
    assert_eq!(theme::BORDER_CONTROL, 3.0);
    assert_eq!(theme::OFFSET_CONTROL, 3.0);
    assert_eq!(theme::OFFSET_PANEL_HOVER, 10.0);
    assert_eq!(theme::OFFSET_CONTROL_HOVER, 6.0);
    assert_eq!(theme::BORDER_HAIRLINE, 2.0);
}

#[test]
fn alpha_policy_splits_by_surface_role_and_never_touches_ink() {
    assert_eq!(theme::ALPHA_READING, 1.0);
    assert_eq!(theme::ALPHA_HUD, 0.82);
    // `with_alpha` is for fills. Ink consts stay fully opaque, so a
    // translucent offset block can never happen by accident.
    assert_eq!(theme::with_alpha(theme::SURFACE, theme::ALPHA_HUD)[3], 0.82);
    assert_eq!(theme::with_alpha(theme::SURFACE, theme::ALPHA_READING), theme::SURFACE);
    assert_eq!(theme::INK_LINE[3], 1.0);
    assert_eq!(theme::TEXT[3], 1.0);
    assert_eq!(theme::TRANSPARENT, [0.0, 0.0, 0.0, 0.0]);
}

use arcdps_axipulse::ui::axi::{self, Rect};

fn r(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::new([x0, y0], [x1, y1])
}

#[test]
fn rect_reports_its_own_size() {
    let a = r(10.0, 20.0, 110.0, 70.0);
    assert_eq!(a.w(), 100.0);
    assert_eq!(a.h(), 50.0);
    let b = Rect::at([10.0, 20.0], [100.0, 50.0]);
    assert_eq!(b, a);
}

#[test]
fn ring_parts_cover_the_rect_edge_exactly_and_never_overlap() {
    // Outlines are four filled bands, not a stroke: imgui's AddRect
    // insets the path it is given by half a pixel before stroking it,
    // which left a pale hairline of fill outside every outline — and
    // on the right and bottom, a gap between the outline and its
    // block. Filled bands have no such fudge.
    let r0 = r(100.0, 100.0, 300.0, 200.0);
    let [top, bottom, left, right] = axi::ring_parts(r0, 4.0);
    assert_eq!(top, r(100.0, 100.0, 300.0, 104.0));
    assert_eq!(bottom, r(100.0, 196.0, 300.0, 200.0));
    assert_eq!(left, r(100.0, 104.0, 104.0, 196.0));
    assert_eq!(right, r(296.0, 104.0, 300.0, 196.0));
    // Every band's outer edge is ON the rect's edge — nothing of the
    // fill shows outside the outline.
    assert_eq!(top.min, r0.min);
    assert_eq!(bottom.max, r0.max);
    assert_eq!(left.min[0], r0.min[0]);
    assert_eq!(right.max[0], r0.max[0]);
    // The side bands stop short of the top and bottom ones.
    assert_eq!(left.min[1], top.max[1]);
    assert_eq!(left.max[1], bottom.min[1]);
}

#[test]
fn ring_parts_fill_a_rect_too_thin_to_outline_rather_than_inverting() {
    // A window dragged to 3px wide, outlined at 4px, would otherwise
    // hand imgui bands whose min is past their max.
    let thin = r(0.0, 0.0, 3.0, 40.0);
    let bands = axi::ring_parts(thin, 4.0);
    for b in bands {
        assert!(b.max[0] >= b.min[0] && b.max[1] >= b.min[1], "clamped, not inverted");
        assert!(b.min[0] >= thin.min[0] && b.max[0] <= thin.max[0], "stays inside");
    }
    // Clamped to half the shorter side, the left and right bands meet:
    // the rect reads as solid outline rather than as a rect with a hole.
    assert_eq!(bands[2].max[0], bands[3].min[0]);
}

#[test]
fn ring_parts_draw_nothing_for_a_non_finite_or_zero_thickness() {
    let r0 = r(0.0, 0.0, 100.0, 50.0);
    for t in [0.0, -3.0, f32::NAN] {
        for b in axi::ring_parts(r0, t) {
            assert!(b.is_degenerate(), "thickness {t} must draw no band");
        }
    }
}

#[test]
fn block_path_shifts_down_right_without_resizing() {
    let fill = r(100.0, 100.0, 300.0, 200.0);
    let block = axi::block_path(fill, 6.0);
    assert_eq!(block, r(106.0, 106.0, 306.0, 206.0));
    assert_eq!(block.w(), fill.w());
    assert_eq!(block.h(), fill.h());
}

#[test]
fn block_parts_cover_the_shifted_rect_outside_the_face_and_nothing_inside_it() {
    // A translucent face must sit on one backdrop, so no part of the
    // block may be painted where the face will land on top of it.
    let face = r(100.0, 100.0, 300.0, 200.0);
    let [right, bottom] = axi::block_parts(face, 6.0);
    assert_eq!(right, r(300.0, 106.0, 306.0, 206.0));
    assert_eq!(bottom, r(106.0, 200.0, 300.0, 206.0));
    for band in [right, bottom] {
        assert!(
            band.min[0] >= face.max[0] || band.min[1] >= face.max[1],
            "{band:?} overlaps the face it sits behind",
        );
    }
    // The two bands tile the visible block exactly: their areas sum to
    // the shifted rect's area minus the part hidden under the face.
    let hidden = (face.w() - 6.0) * (face.h() - 6.0);
    let drawn = right.w() * right.h() + bottom.w() * bottom.h();
    assert_eq!(drawn, face.w() * face.h() - hidden);
}

#[test]
fn block_parts_keep_the_whole_block_when_the_offset_clears_the_face() {
    // An offset bigger than the rect leaves the block fully detached.
    // The right band must then take all of it rather than the two
    // bands together dropping the corner.
    let tiny = r(0.0, 0.0, 4.0, 3.0);
    let [right, bottom] = axi::block_parts(tiny, 6.0);
    assert_eq!(right, axi::block_path(tiny, 6.0));
    assert!(bottom.is_degenerate());
}

#[test]
fn block_parts_draw_nothing_for_a_non_finite_or_zero_offset() {
    let face = r(10.0, 10.0, 50.0, 40.0);
    for o in [0.0, -6.0, f32::NAN] {
        for band in axi::block_parts(face, o) {
            assert!(band.is_degenerate(), "offset {o} must draw no block");
        }
    }
}

#[test]
fn inward_body_leaves_exactly_the_offset_for_the_block() {
    // The window draw list is clipped to the window rect, so a block
    // drawn outside the window is cut off. The body shrinks instead,
    // and its block lands flush with the window's own edge — the same
    // trick `.axi-window` uses in the desktop app.
    let window = r(0.0, 0.0, 300.0, 400.0);
    let body = axi::inward_body(window, 6.0);
    assert_eq!(body, r(0.0, 0.0, 294.0, 394.0));
    let block = axi::block_path(body, 6.0);
    assert_eq!(block.max, window.max);
    assert!(block.max[0] <= window.max[0] && block.max[1] <= window.max[1]);
}

#[test]
fn inward_block_is_a_square_l_that_tiles_the_window_with_the_body() {
    // The desktop app draws this block as `inset -6px -6px 0`, whose L
    // reaches both corners. A shifted-rect block would notch the
    // top-right and bottom-left by the offset.
    let window = r(0.0, 0.0, 300.0, 400.0);
    let body = axi::inward_body(window, 6.0);
    let [right, bottom] = axi::inward_block_parts(window, 6.0);
    assert_eq!(right, r(294.0, 0.0, 300.0, 400.0), "the column runs the full height");
    assert_eq!(bottom, r(0.0, 394.0, 294.0, 400.0), "the row meets it square");
    // Body plus block tile the window exactly, with no overlap.
    let area = |x: axi::Rect| x.w() * x.h();
    assert_eq!(area(body) + area(right) + area(bottom), area(window));
    assert!(right.min[0] >= body.max[0] && bottom.min[1] >= body.max[1]);
}

#[test]
fn inward_block_never_inverts_on_a_window_smaller_than_the_offset() {
    let tiny = r(0.0, 0.0, 4.0, 3.0);
    for band in axi::inward_block_parts(tiny, 6.0) {
        assert!(band.max[0] >= band.min[0] && band.max[1] >= band.min[1]);
    }
    for o in [0.0, -6.0, f32::NAN] {
        for band in axi::inward_block_parts(r(0.0, 0.0, 50.0, 50.0), o) {
            assert!(band.is_degenerate(), "offset {o} must draw no block");
        }
    }
}

#[test]
fn inward_body_never_inverts_on_a_window_smaller_than_the_offset() {
    let tiny = r(0.0, 0.0, 4.0, 3.0);
    let body = axi::inward_body(tiny, 6.0);
    assert!(body.is_degenerate(), "a window narrower than its own block has no body");
    assert!(body.max[0] >= body.min[0] && body.max[1] >= body.min[1]);
}

#[test]
fn degenerate_rects_are_recognised_rather_than_drawn() {
    assert!(!r(0.0, 0.0, 10.0, 10.0).is_degenerate());
    assert!(r(0.0, 0.0, 0.0, 10.0).is_degenerate(), "zero width");
    assert!(r(0.0, 0.0, 10.0, 0.0).is_degenerate(), "zero height");
    assert!(r(10.0, 0.0, 0.0, 10.0).is_degenerate(), "inverted");
    assert!(r(f32::NAN, 0.0, 10.0, 10.0).is_degenerate(), "NaN corner");
    assert!(r(0.0, 0.0, f32::INFINITY, 10.0).is_degenerate(), "infinite corner");
}

#[test]
fn bar_fill_draws_a_quantity_as_length_clamped_to_the_track() {
    let track = r(0.0, 0.0, 200.0, 20.0);
    assert_eq!(axi::bar_fill(track, 0.5), r(0.0, 0.0, 100.0, 20.0));
    assert_eq!(axi::bar_fill(track, 1.0), track);
    assert!(axi::bar_fill(track, 0.0).is_degenerate());
    // Over- and under-range fractions clamp rather than overrun the
    // track or reach back past its left edge.
    assert_eq!(axi::bar_fill(track, 4.0), track);
    assert!(axi::bar_fill(track, -3.0).is_degenerate());
}

#[test]
fn a_non_finite_fraction_reads_as_empty() {
    // A zero-duration fight divides by zero. A bar spanning the screen
    // is worse than an empty one, and a NaN rect is worse than both.
    let track = r(0.0, 0.0, 200.0, 20.0);
    for frac in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(axi::clamp_frac(frac), 0.0, "frac {frac}");
        let fill = axi::bar_fill(track, frac);
        assert!(fill.is_degenerate());
        assert!(fill.max[0].is_finite() && fill.max[1].is_finite());
    }
}

#[test]
fn a_sub_pixel_fill_draws_nothing_at_all() {
    // Pre-conversion guard: `if bar_w > 0.5`. Below half a pixel a fill
    // is a stray mark rather than a quantity, and the track was left
    // blank. Strictly greater than 0.5, as it was.
    assert_eq!(axi::fill_frac(200.0, 0.0), 0.0);
    assert_eq!(axi::fill_frac(200.0, 0.001), 0.0); // 0.2px
    assert_eq!(axi::fill_frac(200.0, 0.0025), 0.0); // exactly 0.5px
    assert_eq!(axi::fill_frac(200.0, 0.01), 0.01); // 2px
    assert_eq!(axi::fill_frac(200.0, 1.0), 1.0);
    // Still clamped and still finite-safe.
    assert_eq!(axi::fill_frac(200.0, 4.0), 1.0);
    assert_eq!(axi::fill_frac(200.0, -1.0), 0.0);
    assert_eq!(axi::fill_frac(f32::NAN, 0.5), 0.0);
    assert_eq!(axi::fill_frac(200.0, f32::NAN), 0.0);
}

// --- text truncation ----------------------------------------------------
//
// The loop these replace appended the ellipsis to the string it was
// shortening, so after the first iteration it popped the ellipsis it had
// just added and re-appended it: the string stopped changing and the
// width condition never cleared. Inside GW2's render callback that is a
// hang of the game, not a glitch. Every test here would spin forever
// against that version.

/// One pixel per `char`, so the expected results are countable by hand.
fn one_px_per_char(s: &str) -> f32 {
    s.chars().count() as f32
}

#[test]
fn a_name_that_fits_is_returned_untouched() {
    assert_eq!(axi::truncate_to_width("Eternal", 7.0, one_px_per_char), "Eternal");
    assert_eq!(axi::truncate_to_width("Eternal", 99.0, one_px_per_char), "Eternal");
    assert_eq!(axi::truncate_to_width("", 0.0, one_px_per_char), "");
}

#[test]
fn a_name_one_glyph_too_long_loses_two_glyphs_to_the_ellipsis() {
    // 8 chars into 7px: pop one (7 chars) and the ellipsis makes 8 —
    // still too wide — pop again (6 chars) and the ellipsis makes 7.
    assert_eq!(axi::truncate_to_width("Eternals", 7.0, one_px_per_char), "Eterna\u{2026}");
}

#[test]
fn a_name_far_too_long_terminates_and_fits() {
    let long = "Eternal Battlegrounds of the Mists and Beyond";
    let out = axi::truncate_to_width(long, 10.0, one_px_per_char);
    assert_eq!(out, "Eternal B\u{2026}");
    assert!(one_px_per_char(&out) <= 10.0);
    assert!(out.ends_with('\u{2026}'));
}

#[test]
fn trailing_space_before_the_ellipsis_is_trimmed() {
    // The cut lands mid-space; `trim_end` keeps "Eternal…" rather than
    // "Eternal …", exactly as the pre-fix code intended.
    assert_eq!(axi::truncate_to_width("Eternal Bg", 8.0, one_px_per_char), "Eternal\u{2026}");
}

#[test]
fn a_multi_byte_name_is_never_cut_mid_character() {
    // Every intermediate value must be valid UTF-8; `String::pop`
    // removes a whole `char`, so a 2-byte or 4-byte glyph goes whole.
    let name = "Rotmühle Ödland \u{1f409}\u{1f409}\u{1f409}";
    for avail in 0..20 {
        let out = axi::truncate_to_width(name, avail as f32, one_px_per_char);
        // Round-tripping through str proves it is valid UTF-8; if a
        // boundary were ever split the pop itself would have panicked.
        assert_eq!(out, String::from_utf8(out.clone().into_bytes()).unwrap());
        assert!(one_px_per_char(&out) <= avail as f32, "avail {avail}: {out:?}");
    }
    assert_eq!(axi::truncate_to_width(name, 6.0, one_px_per_char), "Rotmü\u{2026}");
}

#[test]
fn nothing_is_drawn_when_not_even_the_ellipsis_fits() {
    assert_eq!(axi::truncate_to_width("Eternal", 0.0, one_px_per_char), "");
    assert_eq!(axi::truncate_to_width("Eternal", 0.5, one_px_per_char), "");
}
