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
