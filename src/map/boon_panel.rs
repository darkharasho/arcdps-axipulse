//! Boon ordering for the WvW Map party side panel. Ported from
//! axipulse upstream MovementView.tsx (PANEL_BOON_ORDER).
//!
//! Order is intentional: defensive boons first (Aegis, Prot), then
//! damage/quickness/alacrity, then utility. Limited list keeps the
//! row visually scannable.

pub const PANEL_BOON_ORDER: &[i64] = &[
    740,   // Might
    725,   // Fury
    717,   // Protection
    718,   // Regeneration
    726,   // Vigor
    1122,  // Stability
    719,   // Swiftness
    743,   // Aegis
    873,   // Resolution
    1187,  // Quickness
    30328, // Alacrity
    26980, // Resistance
];

/// Short display name (tooltip text).
pub fn boon_name(id: i64) -> &'static str {
    match id {
        740 => "Might",
        725 => "Fury",
        717 => "Protection",
        718 => "Regeneration",
        726 => "Vigor",
        1122 => "Stability",
        719 => "Swiftness",
        743 => "Aegis",
        873 => "Resolution",
        1187 => "Quickness",
        30328 => "Alacrity",
        26980 => "Resistance",
        _ => "Boon",
    }
}
