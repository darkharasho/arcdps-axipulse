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
