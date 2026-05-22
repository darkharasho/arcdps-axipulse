use arcdps_axipulse::map::tiles::{get_map_tiles, TileInfo};
use arcdps_axipulse::map::wvw::WvwMap;

// Golden numbers captured by running upstream's wvwTiles.ts
// getMapTiles(WvwMap.EternalBattlegrounds, 4) and dumping the array.
#[test]
fn ebg_zoom4_count_and_first_tile() {
    let tiles = get_map_tiles(WvwMap::EternalBattlegrounds, 4);
    // EBG continentRect width = 12030-8958 = 3072 ; height = 15870-12798 = 3072
    // At zoom 4 the tileSpan = 256 * 2^(7-4) = 2048
    // Expected tile grid: tx range floor(8958/2048)..floor(12029/2048) = 4..5  (2 tiles wide)
    //                    ty range floor(12798/2048)..floor(15869/2048) = 6..7  (2 tiles tall)
    // -> 4 tiles total.
    assert_eq!(tiles.len(), 4, "expected 2x2 grid at z4 for EBG");
    let first = &tiles[0];
    assert_eq!(first.url, "https://tiles.guildwars2.com/2/3/4/4/6.jpg");
}

#[test]
fn green_bl_zoom5_nonempty() {
    let tiles = get_map_tiles(WvwMap::GreenBorderlands, 5);
    assert!(!tiles.is_empty());
}

#[test]
fn red_bl_at_max_zoom_has_more_tiles_than_low_zoom() {
    let lo = get_map_tiles(WvwMap::RedBorderlands, 3).len();
    let hi = get_map_tiles(WvwMap::RedBorderlands, 7).len();
    assert!(hi > lo, "max-zoom tile count ({}) must exceed low-zoom ({})", hi, lo);
}

#[test]
fn ebg_zoom4_first_tile_negative_x() {
    let tiles = get_map_tiles(WvwMap::EternalBattlegrounds, 4);
    assert!(tiles[0].x < 0.0, "first EBG z4 tile should overlap left edge, got x={}", tiles[0].x);
}
