//! The domain palettes. These are the DATA's colours, not the
//! system's: the accent never touches them, and this conversion must
//! not change a single value — recolouring a WvW team would make the
//! team bar lie about which side is which.

use arcdps_axipulse::ui::series;
use arcdps_axipulse::wvw_teams::TeamColor;

#[test]
fn wvw_team_colours_are_byte_identical_to_v0_4_4() {
    // axibridge's hex palette: #f87171 / #4ade80 / #60a5fa / #9ca3af,
    // as they were spelled before the move to series.rs.
    assert_eq!(series::TEAM_RED, [0.973, 0.443, 0.443, 1.0]);
    assert_eq!(series::TEAM_GREEN, [0.290, 0.871, 0.502, 1.0]);
    assert_eq!(series::TEAM_BLUE, [0.376, 0.647, 0.980, 1.0]);
    assert_eq!(series::TEAM_UNKNOWN, [0.612, 0.639, 0.686, 1.0]);
    assert_eq!(series::NO_TEAM, [0.29, 0.86, 0.50, 1.0]);
}

#[test]
fn team_color_rgba_still_answers_from_series() {
    assert_eq!(TeamColor::Red.rgba(), series::TEAM_RED);
    assert_eq!(TeamColor::Green.rgba(), series::TEAM_GREEN);
    assert_eq!(TeamColor::Blue.rgba(), series::TEAM_BLUE);
    assert_eq!(TeamColor::Unknown.rgba(), series::TEAM_UNKNOWN);
}

#[test]
fn known_boons_keep_their_pre_conversion_inks() {
    let expected: [(&str, [f32; 4]); 13] = [
        ("Might",        [0.91, 0.36, 0.23, 1.0]),
        ("Fury",         [0.91, 0.60, 0.23, 1.0]),
        ("Quickness",    [0.75, 0.42, 0.94, 1.0]),
        ("Alacrity",     [0.94, 0.42, 0.74, 1.0]),
        ("Protection",   [0.36, 0.61, 0.83, 1.0]),
        ("Regeneration", [0.29, 0.86, 0.50, 1.0]),
        ("Vigor",        [0.64, 0.90, 0.21, 1.0]),
        ("Swiftness",    [0.98, 0.80, 0.08, 1.0]),
        ("Resistance",   [0.77, 0.64, 0.35, 1.0]),
        ("Stability",    [0.96, 0.62, 0.04, 1.0]),
        ("Aegis",        [0.49, 0.83, 0.99, 1.0]),
        ("Resolution",   [0.65, 0.51, 0.91, 1.0]),
        ("Retaliation",  [0.98, 0.57, 0.20, 1.0]),
    ];
    for (name, ink) in expected {
        assert_eq!(series::boon(name), ink, "boon {name}");
    }
}

#[test]
fn an_unknown_boon_name_resolves_to_neutral_without_panicking() {
    // A new GW2 boon, a renamed one, or a log from a build we have
    // never seen. This runs inside GW2's render callback.
    assert_eq!(series::BOON_NEUTRAL, [0.55, 0.55, 0.62, 1.0]);
    for name in ["", "Alacrity ", "alacrity", "Sharpened Edges", "\u{1f600}"] {
        assert_eq!(series::boon(name), series::BOON_NEUTRAL, "name {name:?}");
    }
}

#[test]
fn metric_inks_are_byte_identical_to_v0_4_4() {
    // Pulse and Timeline shared five of these values already; those
    // five are now one constant each. The four that differed keep
    // their own values — unifying them would change what the plugin
    // draws, and this is a reskin.
    assert_eq!(series::METRIC_DAMAGE, [0.95, 0.38, 0.38, 1.0]);       // was ACCENT_DAMAGE / COLOR_DMG
    assert_eq!(series::METRIC_DOWN, [0.97, 0.55, 0.42, 1.0]);         // was ACCENT_DOWN / COLOR_TAKEN
    assert_eq!(series::METRIC_DAMAGE_TAKEN, series::METRIC_DOWN);
    assert_eq!(series::METRIC_SUPPORT, [0.40, 0.85, 0.65, 1.0]);      // was ACCENT_SUPPORT
    assert_eq!(series::METRIC_CLEANSE, [0.32, 0.78, 0.92, 1.0]);      // was ACCENT_CLEANSE / COLOR_DEF
    assert_eq!(series::METRIC_DEF_BOONS, series::METRIC_CLEANSE);
    assert_eq!(series::METRIC_DEFEND, [0.95, 0.62, 0.30, 1.0]);       // was ACCENT_DEFEND
    assert_eq!(series::METRIC_SUCCESS, [0.40, 0.85, 0.55, 1.0]);      // was ACCENT_SUCCESS
    assert_eq!(series::METRIC_NEUTRAL, [0.55, 0.62, 0.78, 1.0]);      // was ACCENT_NEUTRAL
    assert_eq!(series::METRIC_HEALTH, [0.29, 0.86, 0.50, 1.0]);       // was COLOR_HEALTH
    assert_eq!(series::METRIC_DISTANCE, [0.95, 0.75, 0.40, 1.0]);     // was COLOR_DIST
    assert_eq!(series::METRIC_OFF_BOONS, [0.42, 0.65, 0.94, 1.0]);    // was COLOR_OFF
    assert_eq!(series::METRIC_HEAL_IN, [0.35, 0.88, 0.62, 1.0]);      // was COLOR_HEAL_IN
    assert_eq!(series::METRIC_BARRIER_IN, [0.85, 0.72, 0.32, 1.0]);   // was COLOR_BARRIER_IN
    assert_eq!(series::METRIC_BARRIER, [0.65, 0.51, 0.91, 1.0]);      // was pulse.rs:218 inline
}

#[test]
fn every_domain_ink_is_fully_opaque() {
    // Domain inks are drawn at full strength or not at all (rule 2).
    // Surfaces apply their own fill alpha; they never dim the data.
    let all = [
        series::TEAM_RED, series::TEAM_GREEN, series::TEAM_BLUE,
        series::TEAM_UNKNOWN, series::NO_TEAM, series::BOON_NEUTRAL,
        series::METRIC_DAMAGE, series::METRIC_DOWN, series::METRIC_DAMAGE_TAKEN,
        series::METRIC_SUPPORT, series::METRIC_CLEANSE, series::METRIC_DEFEND,
        series::METRIC_SUCCESS, series::METRIC_NEUTRAL, series::METRIC_HEALTH,
        series::METRIC_DISTANCE, series::METRIC_OFF_BOONS, series::METRIC_DEF_BOONS,
        series::METRIC_HEAL_IN, series::METRIC_BARRIER_IN, series::METRIC_BARRIER,
    ];
    for ink in all {
        assert_eq!(ink[3], 1.0);
    }
    assert_eq!(series::boon("Might")[3], 1.0);
}
