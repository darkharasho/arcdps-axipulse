use arcdps_axipulse::map::wvw::{resolve_map_from_zone, WvwMap};

#[test]
fn ebg_full_prefix() {
    assert_eq!(resolve_map_from_zone("Detailed WvW - Eternal Battlegrounds"), Some(WvwMap::EternalBattlegrounds));
}

#[test]
fn ebg_short() {
    assert_eq!(resolve_map_from_zone("EBG"), Some(WvwMap::EternalBattlegrounds));
}

#[test]
fn green_borderlands() {
    assert_eq!(resolve_map_from_zone("Detailed WvW - Green Borderlands"), Some(WvwMap::GreenBorderlands));
}

#[test]
fn blue_borderlands_alt_prefix() {
    assert_eq!(resolve_map_from_zone("WvW - Blue Borderlands"), Some(WvwMap::BlueBorderlands));
}

#[test]
fn red_desert_borderlands() {
    assert_eq!(resolve_map_from_zone("World vs World - Red Desert Borderlands"), Some(WvwMap::RedBorderlands));
}

#[test]
fn pve_map_returns_none() {
    assert_eq!(resolve_map_from_zone("Crystal Oasis"), None);
}

#[test]
fn empty_returns_none() {
    assert_eq!(resolve_map_from_zone(""), None);
}

// EI populates `fight_name` for WvW logs with names like
// "Blue Alpine Borderlands" / "Red Desert Borderlands" — no prefix.
// These are the strings the resolver actually sees in practice.
#[test]
fn raw_blue_alpine_borderlands() {
    assert_eq!(resolve_map_from_zone("Blue Alpine Borderlands"), Some(WvwMap::BlueBorderlands));
}

#[test]
fn raw_green_alpine_borderlands() {
    assert_eq!(resolve_map_from_zone("Green Alpine Borderlands"), Some(WvwMap::GreenBorderlands));
}

#[test]
fn raw_red_desert_borderlands() {
    assert_eq!(resolve_map_from_zone("Red Desert Borderlands"), Some(WvwMap::RedBorderlands));
}

#[test]
fn raw_eternal_battlegrounds() {
    assert_eq!(resolve_map_from_zone("Eternal Battlegrounds"), Some(WvwMap::EternalBattlegrounds));
}

#[test]
fn raw_edge_of_the_mists() {
    assert_eq!(resolve_map_from_zone("Edge of the Mists"), Some(WvwMap::EdgeOfTheMists));
}

#[test]
fn eotm_short_form() {
    assert_eq!(resolve_map_from_zone("EotM"), Some(WvwMap::EdgeOfTheMists));
}

#[test]
fn prefixed_edge_of_the_mists() {
    assert_eq!(resolve_map_from_zone("WvW - Edge of the Mists"), Some(WvwMap::EdgeOfTheMists));
}
