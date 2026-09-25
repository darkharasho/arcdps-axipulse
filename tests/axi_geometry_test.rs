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
