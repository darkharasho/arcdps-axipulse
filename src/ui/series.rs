//! The domain palettes: the data's own colours, not the system's.
//!
//! This is the only file besides `super::theme` permitted a colour
//! literal, and it is the only one permitted a DOMAIN colour. Nothing
//! here is ever recoloured by the accent: red, green and blue are the
//! game's team identities, and recolouring them would make the team bar
//! lie about which side is which. The desktop app leaves its profession
//! colours alone for the same reason
//! (`src/renderer/themes/series.css`).
//!
//! `theme.rs` holds no domain colour and this file holds no chrome
//! colour. That split is what makes "the accent drives chrome only" a
//! checkable property.
//!
//! Professions need no entry here: they are drawn as PNG icons from
//! `src/assets/classes/`, not as colours.

// --- WvW teams ----------------------------------------------------------

/// axibridge's hex palette (#f87171 / #4ade80 / #60a5fa / #9ca3af) as
/// normalized RGBA. `wvw_teams::TeamColor::rgba` reads these.
pub const TEAM_RED: [f32; 4]     = [0.973, 0.443, 0.443, 1.0];
pub const TEAM_GREEN: [f32; 4]   = [0.290, 0.871, 0.502, 1.0];
pub const TEAM_BLUE: [f32; 4]    = [0.376, 0.647, 0.980, 1.0];
pub const TEAM_UNKNOWN: [f32; 4] = [0.612, 0.639, 0.686, 1.0];

/// Fallback for Squad/Allies when the log never resolved our own team
/// colour (PvE, or a WvW log with no team data) — the green the card
/// used to hard-code for everyone.
pub const NO_TEAM: [f32; 4] = [0.29, 0.86, 0.50, 1.0];

// --- metric inks --------------------------------------------------------

/// One palette for both tabs, so Pulse's "damage" and Timeline's
/// "damage dealt" cannot disagree about what damage looks like.
///
/// Where a Pulse ink and its Timeline counterpart already held the same
/// value they are one constant with an alias. Where they differed they
/// stay two constants at their pre-conversion values: unifying them
/// would change what the plugin draws, and this is a reskin.
pub const METRIC_DAMAGE: [f32; 4]      = [0.95, 0.38, 0.38, 1.0];
pub const METRIC_DOWN: [f32; 4]        = [0.97, 0.55, 0.42, 1.0];
/// Damage taken shares the down-contribution ink; they were already
/// the same value in Pulse and Timeline respectively.
pub const METRIC_DAMAGE_TAKEN: [f32; 4] = METRIC_DOWN;
pub const METRIC_SUPPORT: [f32; 4]     = [0.40, 0.85, 0.65, 1.0];
pub const METRIC_CLEANSE: [f32; 4]     = [0.32, 0.78, 0.92, 1.0];
/// Defensive boons share the cleanse ink; already the same value.
pub const METRIC_DEF_BOONS: [f32; 4]   = METRIC_CLEANSE;
pub const METRIC_DEFEND: [f32; 4]      = [0.95, 0.62, 0.30, 1.0];
pub const METRIC_SUCCESS: [f32; 4]     = [0.40, 0.85, 0.55, 1.0];
pub const METRIC_HEALTH: [f32; 4]      = [0.29, 0.86, 0.50, 1.0];
pub const METRIC_DISTANCE: [f32; 4]    = [0.95, 0.75, 0.40, 1.0];
pub const METRIC_OFF_BOONS: [f32; 4]   = [0.42, 0.65, 0.94, 1.0];
pub const METRIC_HEAL_IN: [f32; 4]     = [0.35, 0.88, 0.62, 1.0];
pub const METRIC_BARRIER_IN: [f32; 4]  = [0.85, 0.72, 0.32, 1.0];
pub const METRIC_BARRIER: [f32; 4]     = [0.65, 0.51, 0.91, 1.0];

/// For a metric with no ink of its own.
pub const METRIC_NEUTRAL: [f32; 4] = [0.55, 0.62, 0.78, 1.0];

// --- boons --------------------------------------------------------------

/// For a boon name we do not know: a new one, a renamed one, or a log
/// from a build we have never seen.
pub const BOON_NEUTRAL: [f32; 4] = [0.55, 0.55, 0.62, 1.0];

/// The boon's own ink. Matching is exact on axilog's boon name; an
/// unrecognised name resolves to `BOON_NEUTRAL` rather than panicking,
/// because this runs inside GW2's render callback.
pub fn boon(name: &str) -> [f32; 4] {
    match name {
        "Might"        => [0.91, 0.36, 0.23, 1.0],
        "Fury"         => [0.91, 0.60, 0.23, 1.0],
        "Quickness"    => [0.75, 0.42, 0.94, 1.0],
        "Alacrity"     => [0.94, 0.42, 0.74, 1.0],
        "Protection"   => [0.36, 0.61, 0.83, 1.0],
        "Regeneration" => [0.29, 0.86, 0.50, 1.0],
        "Vigor"        => [0.64, 0.90, 0.21, 1.0],
        "Swiftness"    => [0.98, 0.80, 0.08, 1.0],
        "Resistance"   => [0.77, 0.64, 0.35, 1.0],
        "Stability"    => [0.96, 0.62, 0.04, 1.0],
        "Aegis"        => [0.49, 0.83, 0.99, 1.0],
        "Resolution"   => [0.65, 0.51, 0.91, 1.0],
        "Retaliation"  => [0.98, 0.57, 0.20, 1.0],
        _              => BOON_NEUTRAL,
    }
}
