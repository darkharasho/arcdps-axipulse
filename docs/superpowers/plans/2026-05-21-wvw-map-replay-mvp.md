# WvW Map Replay — MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a third "Map" tab to the AxiPulse window that shows a static top-down render of the current WvW fight: tiled background, landmark pins, and each squad member's final position with profession icon. No playback, no pan/zoom, no overlays — those are follow-up plans.

**Architecture:** Three new source files mirror upstream `axipulse`'s shared/map modules (ported TS → Rust): `src/map/wvw.rs` (data + zone resolver), `src/map/tiles.rs` (tile coord math + asset path resolution), `src/ui/map.rs` (render). A new build-time shell script `scripts/fetch_tiles.sh` downloads all WvW tiles (z0–z7, 4 maps) into `src/assets/tiles/`, gitignored. Tile textures are uploaded on demand via the existing `arcdps::d3d11_device()` pattern from `src/ui/icons.rs`, but cached in a new `MAP_TILES` map (textures live for plugin lifetime once loaded). Deploy script `scripts/deploy.sh` gains a sidecar copy step that ships the assets dir next to the DLL — runtime resolves it via `Globals::install_root`.

**Tech Stack:** Rust (existing), arcdps imgui, windows-rs D3D11, `image` crate (add `jpeg` feature), `ureq` (already in deps) for the dev-time tile fetch script — though we'll use `curl` from the shell script instead to avoid pulling network into the build.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `Cargo.toml` | Modify | Add `jpeg` to `image` features. |
| `.gitignore` | Modify | Ignore `src/assets/tiles/`. |
| `scripts/fetch_tiles.sh` | Create | One-time tile downloader → `src/assets/tiles/{z}/{x}/{y}.jpg`. |
| `scripts/deploy.sh` | Modify | After DLL install, rsync `src/assets/tiles/` next to the DLL as `axipulse-assets/tiles/`. |
| `src/map/mod.rs` | Create | `pub mod wvw; pub mod tiles;` |
| `src/map/wvw.rs` | Create | `WvwMap` enum, `WVW_LANDMARKS` table, `resolve_map_from_zone`. Direct port of upstream `wvwLandmarks.ts` + `mapUtils.ts`. |
| `src/map/tiles.rs` | Create | Tile coord math: `WvwTileData` table, `get_map_tiles(map, zoom) -> Vec<TileInfo>`. Direct port of upstream `wvwTiles.ts`. |
| `src/lib.rs` | Modify | Add `pub mod map;` |
| `src/config.rs` | Modify | No new field needed for MVP (Map is a tab on existing window). |
| `src/ui/main.rs` | Modify | Add `TopTab::Map` variant; extend tab strip and dispatcher. |
| `src/ui/mod.rs` | Modify | `pub mod map;` |
| `src/ui/map.rs` | Create | `render_content(ui, json, idx, derived)`. Loads tile textures via new `tile_cache`, draws background tiles + landmarks + player markers. |
| `src/ui/tile_cache.rs` | Create | D3D11 texture cache for JPEG tiles. Same upload pattern as `icons.rs`, but loads from disk (sidecar `axipulse-assets/tiles/`) on demand. |
| `src/plugin.rs` | Modify | Expose `install_root()` accessor so `tile_cache` can resolve the sidecar path. |
| `tests/wvw_zone_resolve_test.rs` | Create | Table tests for `resolve_map_from_zone`. |
| `tests/wvw_tiles_test.rs` | Create | Golden-number tests for `get_map_tiles` matching upstream TS output. |

---

## Task 1: Add `jpeg` feature to `image` crate

**Files:**
- Modify: `Cargo.toml:28`

- [ ] **Step 1: Edit Cargo.toml**

Change the `image` line from:
```toml
image = { version = "0.25", default-features = false, features = ["png"] }
```
to:
```toml
image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }
```

- [ ] **Step 2: Verify it still builds**

Run: `cargo dll-check`
Expected: clean exit, no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: enable jpeg decoding in image crate (for WvW map tiles)"
```

---

## Task 2: Create `src/map/` module skeleton

**Files:**
- Create: `src/map/mod.rs`
- Create: `src/map/wvw.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `src/map/mod.rs`**

```rust
//! WvW combat-replay map subsystem. Pure data + coord math (no I/O,
//! no D3D11). UI lives in `crate::ui::map`; texture cache lives in
//! `crate::ui::tile_cache`.

pub mod wvw;
pub mod tiles;
```

- [ ] **Step 2: Create `src/map/wvw.rs` with the enum + zone resolver**

```rust
//! WvW map identity, landmark data, and zone-name resolution.
//! Ported from `axipulse/src/shared/wvwLandmarks.ts` and `mapUtils.ts`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WvwMap {
    EternalBattlegrounds,
    GreenBorderlands,
    BlueBorderlands,
    RedBorderlands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandmarkType { Keep, Tower, Camp, Ruins, Named }

#[derive(Debug, Clone, Copy)]
pub struct Landmark {
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
    pub kind: LandmarkType,
}

const ZONE_PREFIXES: &[&str] = &[
    "Detailed WvW - ",
    "World vs World - ",
    "WvW - ",
];

fn strip_prefix(zone: &str) -> &str {
    for p in ZONE_PREFIXES {
        if let Some(rest) = zone.strip_prefix(p) {
            return rest;
        }
    }
    zone
}

pub fn resolve_map_from_zone(zone: &str) -> Option<WvwMap> {
    let clean = strip_prefix(zone).to_lowercase();
    if clean.contains("eternal") || clean == "ebg" {
        Some(WvwMap::EternalBattlegrounds)
    } else if clean.contains("green") {
        Some(WvwMap::GreenBorderlands)
    } else if clean.contains("blue") {
        Some(WvwMap::BlueBorderlands)
    } else if clean.contains("red") {
        Some(WvwMap::RedBorderlands)
    } else {
        None
    }
}

pub fn landmarks(map: WvwMap) -> &'static [Landmark] {
    match map {
        WvwMap::EternalBattlegrounds => EBG,
        WvwMap::GreenBorderlands => GREEN_ALPINE,
        WvwMap::BlueBorderlands => BLUE_ALPINE,
        WvwMap::RedBorderlands => RED_DESERT,
    }
}

const EBG: &[Landmark] = &[
    Landmark { name: "Stonemist Castle",  x: 370.0, y: 435.0, kind: LandmarkType::Keep },
    Landmark { name: "Overlook",          x: 400.0, y: 230.0, kind: LandmarkType::Keep },
    Landmark { name: "Lowlands",          x: 151.0, y: 569.0, kind: LandmarkType::Keep },
    Landmark { name: "Valley",            x: 592.0, y: 567.0, kind: LandmarkType::Keep },
    Landmark { name: "Mendon's Gap",      x: 290.0, y: 175.0, kind: LandmarkType::Tower },
    Landmark { name: "Veloka Slope",      x: 470.0, y: 200.0, kind: LandmarkType::Tower },
    Landmark { name: "Speldan Clearcut",  x: 206.0, y: 200.0, kind: LandmarkType::Tower },
    Landmark { name: "Wildcreek Run",     x: 221.0, y: 446.0, kind: LandmarkType::Tower },
    Landmark { name: "Aldon's Ledge",     x: 106.0, y: 487.0, kind: LandmarkType::Tower },
    Landmark { name: "Klovan Gully",      x: 283.0, y: 557.0, kind: LandmarkType::Tower },
    Landmark { name: "Jerrifer's Slough", x: 198.0, y: 636.0, kind: LandmarkType::Tower },
    Landmark { name: "Quentin Lake",      x: 441.0, y: 592.0, kind: LandmarkType::Tower },
    Landmark { name: "Langor Gulch",      x: 581.0, y: 657.0, kind: LandmarkType::Tower },
    Landmark { name: "Bravost Escarpment",x: 635.0, y: 487.0, kind: LandmarkType::Tower },
    Landmark { name: "Durios Gulch",      x: 512.0, y: 445.0, kind: LandmarkType::Tower },
    Landmark { name: "Ogrewatch Cut",     x: 468.0, y: 307.0, kind: LandmarkType::Tower },
    Landmark { name: "Anzalias Pass",     x: 287.0, y: 314.0, kind: LandmarkType::Tower },
    Landmark { name: "Pangloss Rise",     x: 541.0, y: 229.0, kind: LandmarkType::Camp },
    Landmark { name: "Danelon Passage",   x: 485.0, y: 673.0, kind: LandmarkType::Camp },
    Landmark { name: "Golanta Clearing",  x: 290.0, y: 644.0, kind: LandmarkType::Camp },
    Landmark { name: "Umberglade Woods",  x: 595.0, y: 402.0, kind: LandmarkType::Camp },
    Landmark { name: "Rogue's Quarry",    x: 143.0, y: 397.0, kind: LandmarkType::Camp },
];

const GREEN_ALPINE: &[Landmark] = &[
    Landmark { name: "Dreadfall Bay",         x: 48.0,  y: 435.0, kind: LandmarkType::Keep },
    Landmark { name: "Shadaran Hills",        x: 501.0, y: 419.0, kind: LandmarkType::Keep },
    Landmark { name: "Garrison",              x: 257.0, y: 325.0, kind: LandmarkType::Keep },
    Landmark { name: "Bluebriar",             x: 182.0, y: 515.0, kind: LandmarkType::Tower },
    Landmark { name: "Sunnyhill",             x: 132.0, y: 251.0, kind: LandmarkType::Tower },
    Landmark { name: "Redlake",               x: 364.0, y: 530.0, kind: LandmarkType::Tower },
    Landmark { name: "Cragtop",               x: 385.0, y: 241.0, kind: LandmarkType::Tower },
    Landmark { name: "Titanpaw",              x: 262.0, y: 73.0,  kind: LandmarkType::Camp },
    Landmark { name: "Bluevale Refuge",       x: 95.0,  y: 540.0, kind: LandmarkType::Camp },
    Landmark { name: "Faithleap",             x: 85.0,  y: 276.0, kind: LandmarkType::Camp },
    Landmark { name: "Foghaven",              x: 455.0, y: 270.0, kind: LandmarkType::Camp },
    Landmark { name: "Hero's Lodge",          x: 263.0, y: 660.0, kind: LandmarkType::Camp },
    Landmark { name: "Redwater Lowlands",     x: 453.0, y: 549.0, kind: LandmarkType::Camp },
    Landmark { name: "Temple of the Fallen",  x: 259.0, y: 515.0, kind: LandmarkType::Ruins },
    Landmark { name: "Cohen's Overlook",      x: 312.0, y: 393.0, kind: LandmarkType::Ruins },
    Landmark { name: "Gertzz's Estate",       x: 217.0, y: 382.0, kind: LandmarkType::Ruins },
    Landmark { name: "Patrick's Ascent",      x: 320.0, y: 468.0, kind: LandmarkType::Ruins },
    Landmark { name: "Norfolk's Hollow",      x: 197.0, y: 460.0, kind: LandmarkType::Ruins },
];

const BLUE_ALPINE: &[Landmark] = &[
    Landmark { name: "Ascension Bay",         x: 48.0,  y: 435.0, kind: LandmarkType::Keep },
    Landmark { name: "Askalion Hills",        x: 501.0, y: 419.0, kind: LandmarkType::Keep },
    Landmark { name: "Garrison",              x: 257.0, y: 325.0, kind: LandmarkType::Keep },
    Landmark { name: "Redbriar",              x: 182.0, y: 515.0, kind: LandmarkType::Tower },
    Landmark { name: "Woodhaven",             x: 132.0, y: 251.0, kind: LandmarkType::Tower },
    Landmark { name: "Greenlake",             x: 364.0, y: 530.0, kind: LandmarkType::Tower },
    Landmark { name: "Dawn's Eyrie",          x: 385.0, y: 241.0, kind: LandmarkType::Tower },
    Landmark { name: "Spiritholme",           x: 262.0, y: 73.0,  kind: LandmarkType::Camp },
    Landmark { name: "Redvale Refuge",        x: 95.0,  y: 540.0, kind: LandmarkType::Camp },
    Landmark { name: "Godslore",              x: 85.0,  y: 276.0, kind: LandmarkType::Camp },
    Landmark { name: "Stargrove",             x: 455.0, y: 270.0, kind: LandmarkType::Camp },
    Landmark { name: "Champion's Demesne",    x: 263.0, y: 660.0, kind: LandmarkType::Camp },
    Landmark { name: "Greenwater Lowlands",   x: 453.0, y: 549.0, kind: LandmarkType::Camp },
    Landmark { name: "Temple of Lost Prayers",x: 259.0, y: 515.0, kind: LandmarkType::Ruins },
    Landmark { name: "Orchard Overlook",      x: 312.0, y: 393.0, kind: LandmarkType::Ruins },
    Landmark { name: "Bauer's Estate",        x: 217.0, y: 382.0, kind: LandmarkType::Ruins },
    Landmark { name: "Carver's Ascent",       x: 320.0, y: 468.0, kind: LandmarkType::Ruins },
    Landmark { name: "Battle's Hollow",       x: 197.0, y: 460.0, kind: LandmarkType::Ruins },
];

const RED_DESERT: &[Landmark] = &[
    Landmark { name: "Blistering Undercroft", x: 28.0,  y: 409.0, kind: LandmarkType::Keep },
    Landmark { name: "Stoic Rampart",         x: 370.0, y: 272.0, kind: LandmarkType::Keep },
    Landmark { name: "Osprey's Palace",       x: 700.0, y: 427.0, kind: LandmarkType::Keep },
    Landmark { name: "O'del Academy",         x: 151.0, y: 134.0, kind: LandmarkType::Tower },
    Landmark { name: "Eternal Necropolis",    x: 590.0, y: 155.0, kind: LandmarkType::Tower },
    Landmark { name: "Crankshaft Depot",      x: 485.0, y: 610.0, kind: LandmarkType::Tower },
    Landmark { name: "Parched Outpost",       x: 251.0, y: 579.0, kind: LandmarkType::Tower },
    Landmark { name: "Hamm's Lab",            x: 367.0, y: 130.0, kind: LandmarkType::Camp },
    Landmark { name: "Bauer Farmstead",       x: 654.0, y: 569.0, kind: LandmarkType::Camp },
    Landmark { name: "McLain's Encampment",   x: 90.0,  y: 576.0, kind: LandmarkType::Camp },
    Landmark { name: "Roy's Refuge",          x: 704.0, y: 259.0, kind: LandmarkType::Camp },
    Landmark { name: "Boettiger's Hideaway",  x: 23.0,  y: 256.0, kind: LandmarkType::Camp },
    Landmark { name: "Dustwhisper Well",      x: 376.0, y: 707.0, kind: LandmarkType::Camp },
    Landmark { name: "Higgins's Ascent",      x: 415.0, y: 547.0, kind: LandmarkType::Ruins },
    Landmark { name: "Bearce's Dwelling",     x: 301.0, y: 440.0, kind: LandmarkType::Ruins },
    Landmark { name: "Zak's Overlook",        x: 433.0, y: 444.0, kind: LandmarkType::Ruins },
    Landmark { name: "Darra's Maze",          x: 289.0, y: 513.0, kind: LandmarkType::Ruins },
    Landmark { name: "Tilly's Encampment",    x: 369.0, y: 365.0, kind: LandmarkType::Ruins },
];
```

- [ ] **Step 3: Wire module into `src/lib.rs`**

Add `pub mod map;` somewhere alongside the other top-level `pub mod` lines.

- [ ] **Step 4: Build host-side**

Run: `cargo test --no-run`
Expected: builds clean, no warnings about the new module.

- [ ] **Step 5: Commit**

```bash
git add src/map/ src/lib.rs
git commit -m "feat(map): add WvW landmark table + zone resolver

Port of axipulse upstream src/shared/wvwLandmarks.ts and mapUtils.ts.
Pure data, no I/O — render layer comes in a later commit."
```

---

## Task 3: TDD the zone resolver

**Files:**
- Test: `tests/wvw_zone_resolve_test.rs`

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Run tests**

Run: `cargo test --test wvw_zone_resolve_test`
Expected: all 7 pass (the implementation from Task 2 already covers these).

- [ ] **Step 3: Commit**

```bash
git add tests/wvw_zone_resolve_test.rs
git commit -m "test(map): cover all WvW zone-name variants in resolve_map_from_zone"
```

---

## Task 4: Port tile coord math (TDD)

**Files:**
- Create: `src/map/tiles.rs`
- Test: `tests/wvw_tiles_test.rs`

- [ ] **Step 1: Write the failing test FIRST**

```rust
// tests/wvw_tiles_test.rs
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

// Coord math: at z4 the first EBG tile's top-left in pixel space should
// be slightly negative-x (continentRect starts at 8958, tile x=4 starts
// at 8192, so the tile extends 766 continent units left of the map edge).
#[test]
fn ebg_zoom4_first_tile_negative_x() {
    let tiles = get_map_tiles(WvwMap::EternalBattlegrounds, 4);
    assert!(tiles[0].x < 0.0, "first EBG z4 tile should overlap left edge, got x={}", tiles[0].x);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test wvw_tiles_test`
Expected: FAIL — module `tiles` doesn't exist yet.

- [ ] **Step 3: Implement `src/map/tiles.rs`**

```rust
//! Tile coordinate math for the GW2 official tile service.
//! Ported from `axipulse/src/shared/wvwTiles.ts`. Pure functions —
//! no I/O.

use crate::map::wvw::WvwMap;

const CONTINENT_ID: u32 = 2;
const FLOOR_ID: u32 = 3;
const MAX_TILE_ZOOM: u32 = 7;
const TILE_SIZE: u32 = 256;

#[derive(Debug, Clone, Copy)]
struct WvwTileData {
    /// [[cx1, cy1], [cx2, cy2]] continent-pixel rectangle covering the map.
    continent_rect: [[f32; 2]; 2],
    /// EI combat-replay pixel-space size of the map image.
    pixel_size: [f32; 2],
    /// Shift applied to tile positions to align with EI pixel space.
    pixel_offset: [f32; 2],
}

fn tile_data(map: WvwMap) -> WvwTileData {
    match map {
        WvwMap::EternalBattlegrounds => WvwTileData {
            continent_rect: [[8958.0, 12798.0], [12030.0, 15870.0]],
            pixel_size: [716.0, 750.0],
            pixel_offset: [-14.0, 20.0],
        },
        WvwMap::GreenBorderlands => WvwTileData {
            continent_rect: [[5630.0, 11518.0], [8190.0, 15102.0]],
            pixel_size: [523.0, 750.0],
            pixel_offset: [0.0, 0.0],
        },
        WvwMap::BlueBorderlands => WvwTileData {
            continent_rect: [[12798.0, 10878.0], [15358.0, 14462.0]],
            pixel_size: [523.0, 750.0],
            pixel_offset: [0.0, 0.0],
        },
        WvwMap::RedBorderlands => WvwTileData {
            continent_rect: [[9214.0, 8958.0], [12286.0, 12030.0]],
            pixel_size: [750.0, 750.0],
            pixel_offset: [0.0, 0.0],
        },
    }
}

/// Pixel-space size of a WvW map in EI combat-replay coordinates.
pub fn map_pixel_size(map: WvwMap) -> (f32, f32) {
    let d = tile_data(map);
    (d.pixel_size[0], d.pixel_size[1])
}

#[derive(Debug, Clone)]
pub struct TileInfo {
    pub url: String,
    /// Tile X coord (used to build sidecar path: assets/tiles/{z}/{x}/{y}.jpg).
    pub tx: u32,
    pub ty: u32,
    pub zoom: u32,
    /// Top-left in EI pixel space.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn get_map_tiles(map: WvwMap, tile_zoom: u32) -> Vec<TileInfo> {
    let data = tile_data(map);
    let [[cx1, cy1], [cx2, cy2]] = data.continent_rect;
    let [pw, ph] = data.pixel_size;
    let [ox, oy] = data.pixel_offset;
    let cw = cx2 - cx1;
    let ch = cy2 - cy1;

    let tile_span = (TILE_SIZE as f32) * 2f32.powi((MAX_TILE_ZOOM - tile_zoom) as i32);

    let tx_min = (cx1 / tile_span).floor() as i32;
    let ty_min = (cy1 / tile_span).floor() as i32;
    let tx_max = ((cx2 - 1.0) / tile_span).floor() as i32;
    let ty_max = ((cy2 - 1.0) / tile_span).floor() as i32;

    let mut out = Vec::new();
    for ty in ty_min..=ty_max {
        for tx in tx_min..=tx_max {
            let tile_cx = (tx as f32) * tile_span;
            let tile_cy = (ty as f32) * tile_span;
            let px = (tile_cx - cx1) / cw * pw + ox;
            let py = (tile_cy - cy1) / ch * ph + oy;
            let tw = tile_span / cw * pw;
            let th = tile_span / ch * ph;
            out.push(TileInfo {
                url: format!(
                    "https://tiles.guildwars2.com/{}/{}/{}/{}/{}.jpg",
                    CONTINENT_ID, FLOOR_ID, tile_zoom, tx, ty,
                ),
                tx: tx as u32,
                ty: ty as u32,
                zoom: tile_zoom,
                x: px,
                y: py,
                width: tw,
                height: th,
            });
        }
    }
    out
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test wvw_tiles_test`
Expected: all 4 PASS. If `ebg_zoom4_first_tile_negative_x` fails, the pixel_offset sign is inverted — re-check against `wvwTiles.ts:21`.

- [ ] **Step 5: Commit**

```bash
git add src/map/tiles.rs tests/wvw_tiles_test.rs
git commit -m "feat(map): port tile coord math from axipulse wvwTiles.ts

Pure function get_map_tiles(map, zoom) -> Vec<TileInfo>. Covers all
four WvW maps and produces URLs against tiles.guildwars2.com plus
EI-pixel-space rectangles for rendering. TDD against golden numbers
from the TS reference."
```

---

## Task 5: Pre-fetch script for all WvW tiles (z0–z7)

**Files:**
- Create: `scripts/fetch_tiles.sh`
- Modify: `.gitignore`

- [ ] **Step 1: Add the gitignore entry**

Append to `.gitignore`:
```
# WvW map tiles downloaded by scripts/fetch_tiles.sh (sidecar asset, not in source)
src/assets/tiles/
```

- [ ] **Step 2: Write the script**

```bash
#!/usr/bin/env bash
# scripts/fetch_tiles.sh
# Download every WvW map tile (z0..z7, all 4 maps) into src/assets/tiles/
# so the plugin can render the WvW combat replay without runtime network.
# Idempotent: skips tiles that already exist on disk.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$REPO_ROOT/src/assets/tiles"
mkdir -p "$OUT"

# Continent rectangles in continent-pixel space, mirrored from
# src/map/tiles.rs / axipulse src/shared/wvwTiles.ts. Format: name cx1 cy1 cx2 cy2
MAPS=(
    "ebg    8958  12798 12030 15870"
    "green  5630  11518 8190  15102"
    "blue   12798 10878 15358 14462"
    "red    9214  8958  12286 12030"
)
CONT=2
FLOOR=3
TILE=256
MAX_Z=7

# Aggregate the union tile set across maps so we don't double-download.
declare -A SEEN
for z in 0 1 2 3 4 5 6 7; do
    span=$(( TILE * (1 << (MAX_Z - z)) ))
    for m in "${MAPS[@]}"; do
        read -r _ cx1 cy1 cx2 cy2 <<< "$m"
        tx_min=$(( cx1 / span ))
        ty_min=$(( cy1 / span ))
        tx_max=$(( (cx2 - 1) / span ))
        ty_max=$(( (cy2 - 1) / span ))
        for (( ty=ty_min; ty<=ty_max; ty++ )); do
            for (( tx=tx_min; tx<=tx_max; tx++ )); do
                SEEN["$z/$tx/$ty"]=1
            done
        done
    done
done

total=${#SEEN[@]}
echo "fetching $total WvW tiles → $OUT" >&2
i=0
for key in "${!SEEN[@]}"; do
    IFS=/ read -r z tx ty <<< "$key"
    dst="$OUT/$z/$tx/$ty.jpg"
    i=$((i+1))
    if [[ -s "$dst" ]]; then
        continue
    fi
    mkdir -p "$(dirname "$dst")"
    url="https://tiles.guildwars2.com/$CONT/$FLOOR/$z/$tx/$ty.jpg"
    if ! curl -fsSL --retry 3 --retry-delay 1 -o "$dst.tmp" "$url"; then
        echo "  [$i/$total] FAIL $url" >&2
        rm -f "$dst.tmp"
        continue
    fi
    mv "$dst.tmp" "$dst"
    if (( i % 25 == 0 )); then
        echo "  [$i/$total] $z/$tx/$ty" >&2
    fi
done
echo "done." >&2
```

- [ ] **Step 3: Make executable**

Run: `chmod +x scripts/fetch_tiles.sh`

- [ ] **Step 4: Dry-run the script**

Run: `./scripts/fetch_tiles.sh`
Expected: prints "fetching N WvW tiles" with N somewhere in the 700–1200 range, then progress lines, ends with "done." Verify `ls src/assets/tiles/7/ | wc -l` shows multiple `tx` subdirs and total disk usage is a few tens of MB at most: `du -sh src/assets/tiles/`.

- [ ] **Step 5: Commit**

```bash
git add scripts/fetch_tiles.sh .gitignore
git commit -m "build: add WvW tile pre-fetch script

scripts/fetch_tiles.sh downloads every tile covering the 4 WvW maps
across zooms 0-7 into src/assets/tiles/{z}/{x}/{y}.jpg. Idempotent
so re-runs are cheap. Tile dir is gitignored — ship via deploy.sh
sidecar copy (next task)."
```

---

## Task 6: Wire sidecar tile assets into `deploy.sh`

**Files:**
- Modify: `scripts/deploy.sh`

- [ ] **Step 1: Read current `scripts/deploy.sh` to confirm exact shape**

Run: `cat scripts/deploy.sh`
Expected output matches what's already in the survey above (cp src→tmp→mv to DEST).

- [ ] **Step 2: Extend it**

Replace the file contents with:

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$REPO_ROOT/target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll"
DEST="${AXIPULSE_DEPLOY_DEST:-/var/mnt/data/SteamLibrary/steamapps/common/Guild Wars 2/addons/arcdps_axipulse.dll}"

if [[ ! -f "$SRC" ]]; then
    echo "build artifact missing: $SRC — run 'cargo dll' first" >&2
    exit 1
fi

# Atomic DLL install (tmp + rename so a live GW2 doesn't see a truncated inode).
TMP="${DEST}.new"
cp "$SRC" "$TMP"
mv "$TMP" "$DEST"
ls -lh "$DEST"

# Sidecar tile assets. Placed in axipulse-assets/ next to the DLL; the
# plugin resolves them at runtime via Globals::install_root. Only sync
# if the source tile dir exists (engineers without the WvW map feature
# enabled can skip running scripts/fetch_tiles.sh).
TILES_SRC="$REPO_ROOT/src/assets/tiles"
TILES_DEST="${DEST%/*}/axipulse-assets/tiles"
if [[ -d "$TILES_SRC" ]]; then
    mkdir -p "$TILES_DEST"
    # rsync gives us atomic-ish per-file replacement + delete-on-source-removed.
    rsync -a --delete "$TILES_SRC/" "$TILES_DEST/"
    echo "synced tiles → $TILES_DEST"
else
    echo "no tile assets at $TILES_SRC (skip); run scripts/fetch_tiles.sh to populate" >&2
fi
```

- [ ] **Step 3: Verify it runs**

Run: `./scripts/deploy.sh` (after a prior `cargo dll`)
Expected: prints the DLL ls line, then either "synced tiles → ..." or the skip message. Verify the tiles dir landed:
Run: `ls "${AXIPULSE_DEPLOY_DEST%/*}/axipulse-assets/tiles/" 2>&1 | head -3` (use the same path your env points at).

- [ ] **Step 4: Commit**

```bash
git add scripts/deploy.sh
git commit -m "build(deploy): rsync WvW tile assets next to the DLL

After the DLL atomic-install, mirror src/assets/tiles/ into
<addons>/axipulse-assets/tiles/ so the plugin can load tile JPEGs
at runtime. Skip cleanly when the source dir is empty."
```

---

## Task 7: Expose `install_root` accessor from plugin globals

**Files:**
- Modify: `src/plugin.rs`

- [ ] **Step 1: Find the `install_root` field in `Globals`**

Run: `grep -n install_root src/plugin.rs`
Expected: at least one hit on the struct declaration, plus wherever it's set.

- [ ] **Step 2: Add a public accessor**

Add this function in `src/plugin.rs` next to other `pub fn` accessors (e.g. near `is_parsing()`):

```rust
/// Resolved directory the DLL was loaded from. Used by the tile cache
/// to locate sidecar assets at `<install_root>/axipulse-assets/tiles/`.
/// Returns `None` until arcdps has told us the install location.
pub fn install_root() -> Option<std::path::PathBuf> {
    G.install_root.lock().ok().and_then(|g| g.clone())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo dll-check`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/plugin.rs
git commit -m "feat(plugin): expose install_root() for sidecar asset lookup"
```

---

## Task 8: Tile texture cache

**Files:**
- Create: `src/ui/tile_cache.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add module declaration**

In `src/ui/mod.rs`, add `pub mod tile_cache;` alongside the existing module decls.

- [ ] **Step 2: Create `src/ui/tile_cache.rs`**

```rust
#![cfg(windows)]
//! Sidecar JPEG tile texture cache. On first lookup of a (zoom, tx, ty)
//! tuple we load the JPEG from `<install_root>/axipulse-assets/tiles/
//! {z}/{tx}/{ty}.jpg`, decode to RGBA8, upload to a D3D11 texture, and
//! cache the SRV pointer for ImGui to consume as a `TextureId`.
//!
//! Mirrors the lifetime model of `crate::ui::icons`: textures live for
//! the lifetime of the plugin. Frame-drain to avoid CreateTexture2D
//! bursts: at most `MAX_UPLOADS_PER_FRAME` uploads per `drain_pending`
//! call.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use arcdps::imgui::TextureId;
use once_cell::sync::Lazy;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_SHADER_RESOURCE,
    D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC};

const MAX_UPLOADS_PER_FRAME: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey { pub zoom: u32, pub tx: u32, pub ty: u32 }

#[derive(Clone, Copy)]
pub struct TileHandle { pub tex: TextureId }

enum TileState {
    Pending(Vec<u8>), // raw JPEG bytes loaded from disk, awaiting upload
    Ready { ptr: usize },
    Failed,
}

static TILES: Lazy<Mutex<HashMap<TileKey, TileState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
// Hold SRVs alive for plugin lifetime (TextureId is just the raw ptr).
static SRVS: Lazy<Mutex<Vec<ID3D11ShaderResourceView>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

fn assets_root() -> Option<PathBuf> {
    crate::plugin::install_root()
        .map(|p| {
            // install_root is the addons/ directory; sidecar is right beside the DLL.
            let mut buf = p;
            // If install_root points at the DLL file itself, parent it.
            if buf.is_file() {
                if let Some(parent) = buf.parent() { buf = parent.to_path_buf(); }
            }
            buf.push("axipulse-assets");
            buf.push("tiles");
            buf
        })
        .filter(|p| p.exists())
}

/// Look up a tile. If not yet seen, schedule a disk load + queue upload.
/// Returns `None` until the texture is uploaded.
pub fn lookup(key: TileKey) -> Option<TileHandle> {
    let mut guard = TILES.lock().ok()?;
    match guard.get(&key) {
        Some(TileState::Ready { ptr }) => Some(TileHandle { tex: TextureId::new(*ptr) }),
        Some(TileState::Failed) => None,
        Some(TileState::Pending(_)) => None,
        None => {
            // Load JPEG bytes from disk (cheap; bounded by tile size ~20KB).
            let Some(root) = assets_root() else {
                guard.insert(key, TileState::Failed);
                return None;
            };
            let path = root.join(format!("{}/{}/{}.jpg", key.zoom, key.tx, key.ty));
            match std::fs::read(&path) {
                Ok(bytes) => { guard.insert(key, TileState::Pending(bytes)); }
                Err(_) => { guard.insert(key, TileState::Failed); }
            }
            None
        }
    }
}

/// Call once per imgui frame. Uploads up to `MAX_UPLOADS_PER_FRAME`
/// pending tiles to D3D11. Safe to call when no device is available
/// (no-op until arcdps has handed us one).
pub fn drain_pending() {
    let Some(device) = arcdps::d3d11_device() else { return; };
    let mut guard = match TILES.lock() { Ok(g) => g, Err(_) => return };
    let mut to_upload: Vec<(TileKey, Vec<u8>)> = Vec::new();
    for (key, state) in guard.iter_mut() {
        if to_upload.len() >= MAX_UPLOADS_PER_FRAME { break; }
        if let TileState::Pending(_) = state {
            let TileState::Pending(bytes) = std::mem::replace(state, TileState::Failed) else { unreachable!() };
            to_upload.push((*key, bytes));
        }
    }
    drop(guard);

    for (key, bytes) in to_upload {
        let result = decode_and_upload(&device, &bytes);
        let mut guard = match TILES.lock() { Ok(g) => g, Err(_) => continue };
        match result {
            Some(ptr) => { guard.insert(key, TileState::Ready { ptr }); }
            None      => { guard.insert(key, TileState::Failed); }
        }
    }
}

fn decode_and_upload(device: &ID3D11Device, jpeg_bytes: &[u8]) -> Option<usize> {
    let img = image::load_from_memory(jpeg_bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let pixels = img.into_raw();
    unsafe { upload_rgba(device, w, h, &pixels).ok() }
}

unsafe fn upload_rgba(device: &ID3D11Device, w: u32, h: u32, rgba: &[u8]) -> windows::core::Result<usize> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: w,
        Height: h,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let init_data = D3D11_SUBRESOURCE_DATA {
        pSysMem: rgba.as_ptr() as *const _,
        SysMemPitch: w * 4,
        SysMemSlicePitch: 0,
    };
    let mut tex: Option<ID3D11Texture2D> = None;
    device.CreateTexture2D(&desc, Some(&init_data), Some(&mut tex))?;
    let tex = tex.ok_or_else(|| windows::core::Error::from_win32())?;
    let mut srv: Option<ID3D11ShaderResourceView> = None;
    device.CreateShaderResourceView(&tex, None, Some(&mut srv))?;
    let srv = srv.ok_or_else(|| windows::core::Error::from_win32())?;
    let ptr = srv.as_raw() as usize;
    SRVS.lock().ok().map(|mut v| v.push(srv));
    Ok(ptr)
}
```

- [ ] **Step 3: Build**

Run: `cargo dll-check`
Expected: clean. Common gotchas: missing `Interface` import for `as_raw()`; missing `windows::core::Result` import.

- [ ] **Step 4: Commit**

```bash
git add src/ui/tile_cache.rs src/ui/mod.rs
git commit -m "feat(ui): D3D11 tile cache for sidecar WvW JPEG tiles

Loads from <install_root>/axipulse-assets/tiles/{z}/{tx}/{ty}.jpg on
first lookup, decodes via image crate, uploads via the same D3D11
path icons.rs uses. Frame-drained at 2 uploads per frame to avoid
CreateTexture2D bursts."
```

---

## Task 9: Add Map tab to main window

**Files:**
- Modify: `src/ui/main.rs`
- Modify: `src/ui/mod.rs`
- Create: `src/ui/map.rs` (stub for this task)

- [ ] **Step 1: Add stub `src/ui/map.rs`**

```rust
#![cfg(windows)]
//! WvW combat-replay map view. MVP: static final-frame render of
//! squad positions on top of tile background + landmark pins.

use arcdps::imgui::Ui;

use crate::derived::Derived;
use crate::ei_model::EiJson;

pub fn render_content(ui: &Ui, _json: &EiJson, _idx: usize, _derived: &Derived) {
    ui.text_disabled("Map view — coming up.");
}
```

- [ ] **Step 2: Register module**

In `src/ui/mod.rs`, add `pub mod map;` alongside the existing decls.

- [ ] **Step 3: Add `TopTab::Map` to `src/ui/main.rs`**

Change line 23 from:
```rust
enum TopTab { Pulse, Timeline }
```
to:
```rust
enum TopTab { Pulse, Timeline, Map }
```

In `render_top_tabs` (around line 263), change the tab iter array from:
```rust
for (i, (label, tab)) in [("Pulse", TopTab::Pulse), ("Timeline", TopTab::Timeline)].iter().enumerate() {
```
to:
```rust
let tabs = [("Pulse", TopTab::Pulse), ("Timeline", TopTab::Timeline), ("Map", TopTab::Map)];
let n = tabs.len();
for (i, (label, tab)) in tabs.iter().enumerate() {
```
and replace the `if i + 1 < 2 { ui.same_line(); }` with:
```rust
if i + 1 < n { ui.same_line(); }
```

In the dispatch `match tab` (around line 86), add the Map arm:
```rust
match tab {
    TopTab::Pulse    => crate::ui::pulse::render_content(ui, json, idx, derived),
    TopTab::Timeline => crate::ui::timeline::render_content(ui, json, idx, derived, &mut config.timeline_layers),
    TopTab::Map      => crate::ui::map::render_content(ui, json, idx, derived),
}
```

- [ ] **Step 4: Build & deploy**

```bash
cargo dll
./scripts/deploy.sh
```

- [ ] **Step 5: Verify in GW2**

Boot GW2 with the plugin loaded. Open the AxiPulse window. Confirm the third "Map" tab exists; clicking it shows "Map view — coming up."

- [ ] **Step 6: Commit**

```bash
git add src/ui/map.rs src/ui/mod.rs src/ui/main.rs
git commit -m "feat(ui): add Map tab to AxiPulse window (stub)

Third top-level tab alongside Pulse and Timeline. Renders a placeholder
for now; real content lands next."
```

---

## Task 10: Map view — tile background + landmarks (static, no positions yet)

**Files:**
- Modify: `src/ui/map.rs`

- [ ] **Step 1: Replace `src/ui/map.rs` content**

```rust
#![cfg(windows)]
//! WvW combat-replay map view. MVP: static final-frame render of
//! squad positions on top of tile background + landmark pins.

use arcdps::imgui::{ImColor32, Ui};

use crate::derived::Derived;
use crate::ei_model::EiJson;
use crate::map::tiles::{get_map_tiles, map_pixel_size};
use crate::map::wvw::{landmarks, resolve_map_from_zone, LandmarkType, WvwMap};
use crate::ui::tile_cache::{self, TileKey};

const MVP_TILE_ZOOM: u32 = 5;

const BG_DARK:   [f32; 4] = [0.04, 0.05, 0.07, 1.0];
const TEXT_MUTED:[f32; 4] = [0.55, 0.58, 0.65, 1.0];

pub fn render_content(ui: &Ui, json: &EiJson, _idx: usize, _derived: &Derived) {
    // Drain a couple of pending tile uploads per frame.
    tile_cache::drain_pending();

    // Resolve which WvW map this fight took place on.
    let zone = json.zone.as_deref().or(json.map_name.as_deref()).unwrap_or("");
    let Some(map) = resolve_map_from_zone(zone) else {
        ui.text_colored(TEXT_MUTED, format!("Not a WvW fight (zone: \"{}\")", zone));
        return;
    };

    // Compute the on-screen rect: fit the map's pixel-space aspect into
    // the remaining content region.
    let (mw, mh) = map_pixel_size(map);
    let avail = ui.content_region_avail();
    let scale = (avail[0] / mw).min(avail[1] / mh).max(0.05);
    let render_w = mw * scale;
    let render_h = mh * scale;
    let origin = ui.cursor_screen_pos();
    let ox = origin[0] + (avail[0] - render_w) * 0.5;
    let oy = origin[1];

    let draw = ui.get_window_draw_list();

    // Background panel.
    draw.add_rect([ox, oy], [ox + render_w, oy + render_h], BG_DARK)
        .filled(true)
        .build();

    // Tile background.
    let tiles = get_map_tiles(map, MVP_TILE_ZOOM);
    for tile in &tiles {
        if let Some(h) = tile_cache::lookup(TileKey { zoom: tile.zoom, tx: tile.tx, ty: tile.ty }) {
            let x0 = ox + tile.x * scale;
            let y0 = oy + tile.y * scale;
            let x1 = x0 + tile.width * scale;
            let y1 = y0 + tile.height * scale;
            draw.add_image(h.tex, [x0, y0], [x1, y1]).build();
        }
    }

    // Landmark pins.
    for lm in landmarks(map) {
        let cx = ox + lm.x * scale;
        let cy = oy + lm.y * scale;
        let (r, color) = match lm.kind {
            LandmarkType::Keep  => (6.0, [0.93, 0.27, 0.27, 0.85]),
            LandmarkType::Tower => (5.0, [0.96, 0.62, 0.04, 0.85]),
            LandmarkType::Camp  => (4.0, [0.13, 0.77, 0.37, 0.85]),
            LandmarkType::Ruins => (4.0, [0.55, 0.36, 0.96, 0.85]),
            LandmarkType::Named => (3.5, [0.42, 0.45, 0.50, 0.85]),
        };
        draw.add_circle([cx, cy], r, ImColor32::from_rgba_f32s(color[0], color[1], color[2], color[3]))
            .filled(true)
            .build();
        // Name text just to the right of the dot.
        draw.add_text([cx + r + 2.0, cy - 6.0], TEXT_MUTED, lm.name);
    }

    // Advance imgui's cursor past the map so subsequent items (if any)
    // don't overlap. Use dummy() since set_cursor_screen_pos crashed
    // historically — see commit 6430e2f.
    ui.dummy([avail[0], render_h]);
}
```

> **Note for the engineer:** `ImColor32::from_rgba_f32s` may not exist in your bound version of the imgui crate. If `cargo dll-check` complains, swap the call to `ImColor32::from([(color[0]*255.0) as u8, (color[1]*255.0) as u8, (color[2]*255.0) as u8, (color[3]*255.0) as u8])` or use `draw.add_circle(..., color)` directly with the `[f32;4]` if the binding accepts it (check `src/ui/main.rs:175` for an `add_rect` color usage as a reference).

- [ ] **Step 2: Build + deploy**

```bash
cargo dll
./scripts/deploy.sh
```

If `cargo dll-check` errors on `ImColor32`: simplest fix is to call `draw.add_circle([cx, cy], r, color).filled(true).build();` and let the binding's `Into<ImColor32>` handle the `[f32; 4]`.

- [ ] **Step 3: Trigger a test log**

Using the memorised process (`memory/project_trigger_test_log.md`):
```bash
DIR="/var/mnt/data/SteamLibrary/steamapps/compatdata/1284210/pfx/drive_c/users/steamuser/Documents/Guild Wars 2/addons/arcdps/arcdps.cbtlogs/1"
SRC=$(ls "$DIR"/*.zevtc | grep -v -- '-sim\.zevtc$' | shuf -n1)
NEW="$DIR/$(date +%Y%m%d-%H%M%S)-sim.zevtc"
cp "$SRC" "$NEW.tmp"
sleep 1
mv "$NEW.tmp" "$NEW"
```

Then in GW2, open AxiPulse → Map tab. Expected:
- A dark panel sized to the map's aspect ratio.
- Tile JPEGs gradually fill in (2 per frame, so a brief moment of blank → tiled background).
- Coloured landmark dots overlaid with names to the right.

If tiles never appear: check the arcdps log
```bash
tail -50 "/var/mnt/data/SteamLibrary/steamapps/common/Guild Wars 2/addons/arcdps/arcdps.log"
```
and verify `axipulse-assets/tiles/5/...` actually exists next to the DLL.

- [ ] **Step 4: Commit**

```bash
git add src/ui/map.rs
git commit -m "feat(ui/map): render WvW tile background + landmark pins

Resolves WvW map from fight zone name, fits the map's pixel-space
aspect to the tab area, draws tile JPEGs from the sidecar cache, and
overlays coloured landmark dots. No player positions yet (next task)."
```

---

## Task 11: Render final-frame squad positions

**Files:**
- Modify: `src/ui/map.rs`

- [ ] **Step 1: Add a helper to pull the last known position per player**

Add this inside `src/ui/map.rs` above `render_content`:

```rust
struct PlayerDot<'a> {
    name: &'a str,
    profession: &'a str,
    x: f32,
    y: f32,
    is_self: bool,
}

fn collect_final_positions<'a>(json: &'a EiJson, self_idx: usize) -> Vec<PlayerDot<'a>> {
    let mut out = Vec::new();
    for (i, p) in json.players.iter().enumerate() {
        let Some(rd) = p.combat_replay_data.as_ref() else { continue };
        let Some(last) = rd.positions.last() else { continue };
        if last.len() < 2 { continue; }
        out.push(PlayerDot {
            name: p.name.as_str(),
            profession: p.profession.as_str(),
            x: last[0] as f32,
            y: last[1] as f32,
            is_self: i == self_idx,
        });
    }
    out
}
```

> **Engineer note:** `EiPlayer` has fields `name: String` and `profession: String` — confirm with `grep -n 'pub struct EiPlayer' src/ei_model.rs` and the lines after. If `profession` is on a sibling struct, adjust the accessor.

- [ ] **Step 2: Use `idx` and the helper in `render_content`**

Replace `_idx: usize` in the signature with `idx: usize`. After the landmark loop in `render_content`, before the trailing `ui.dummy(...)` call, add:

```rust
// Final-frame player positions.
let dots = collect_final_positions(json, idx);
for dot in &dots {
    let cx = ox + dot.x * scale;
    let cy = oy + dot.y * scale;
    // Profession icon if available, else coloured dot.
    if let Some(icon) = crate::ui::icons::lookup_bundled(dot.profession) {
        let sz = if dot.is_self { 18.0 } else { 14.0 };
        let half = sz * 0.5;
        if dot.is_self {
            draw.add_circle([cx, cy], half + 2.5, [0.06, 0.72, 0.51, 0.85])
                .thickness(2.0)
                .build();
        }
        draw.add_image(icon.tex, [cx - half, cy - half], [cx + half, cy + half]).build();
    } else {
        let r = if dot.is_self { 5.5 } else { 4.0 };
        let color = if dot.is_self { [0.06, 0.72, 0.51, 0.95] } else { [0.86, 0.86, 0.92, 0.85] };
        draw.add_circle([cx, cy], r, color).filled(true).build();
    }
}
```

- [ ] **Step 3: Build + deploy**

```bash
cargo dll
./scripts/deploy.sh
```

- [ ] **Step 4: Trigger a log + verify**

Re-run the test-log trigger from Task 10 Step 3. Open AxiPulse → Map tab. Expected:
- Tile background + landmarks as before.
- One profession icon per squad member dropped at the player's final position; your own marker has a green ring around it.

Sanity check: if all dots cluster at the top-left (0, 0), the `combat_replay_data.positions` field probably needs `polling_rate` interpretation or the deserializer isn't pulling positions. Confirm with:
```bash
cargo test --test ei_model_test
```
and add a one-off `dbg!(dots.len(), dots.first().map(|d| (d.x, d.y)))` if needed.

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs
git commit -m "feat(ui/map): render final-frame squad positions

For each player with combat_replay_data.positions, drop a profession
icon (or coloured dot fallback) at the last known location. Local
player marker gets a green ring."
```

---

## Task 12: README mention + plan archival

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add a Map section to the README**

Open `README.md`, find the existing feature list or "Usage" section, and append:

```markdown
### WvW Combat Replay (Map tab) — MVP

Renders a static top-down view of the fight on the matching WvW map:
- Tile background sourced from official GW2 tiles (pre-cached on disk).
- Landmark pins (keeps, towers, camps, ruins).
- Each squad member's final position with profession icon.

**One-time setup:** run `./scripts/fetch_tiles.sh` to populate
`src/assets/tiles/` (~tens of MB). Re-run `./scripts/deploy.sh` so the
sidecar `axipulse-assets/tiles/` is synced next to the DLL.

Time playback, pan/zoom, and state overlays (down/dead, boons, skill
casts) ship in follow-up plans.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: note WvW Map tab MVP + tile pre-cache setup"
```

---

## Self-review checklist (engineer should re-read before starting)

- [ ] All file paths are absolute or relative-from-repo-root.
- [ ] Every task's "verify" step has a concrete command + expected output.
- [ ] No `TODO`, `tbd`, or "fill in" — every code block is complete.
- [ ] TDD tasks (3 and 4) write the test first, run it failing, then implement.
- [ ] No tasks depend on a future plan's work (this is shippable on its own).
