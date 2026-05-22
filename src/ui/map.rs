//! WvW combat-replay map view. MVP: static final-frame render of
//! squad positions on top of tile background + landmark pins.

#[cfg(windows)]
use arcdps::imgui::Ui;

#[cfg(windows)]
use crate::derived::Derived;
#[cfg(windows)]
use crate::ei_model::EiJson;
#[cfg(windows)]
use crate::map::tiles::{get_map_tiles, map_pixel_size};
#[cfg(windows)]
use crate::map::wvw::{landmarks, resolve_map_from_zone, LandmarkType};
#[cfg(windows)]
use crate::ui::tile_cache::{self, TileKey};

#[cfg(windows)]
const MVP_TILE_ZOOM: u32 = 5;

#[cfg(windows)]
const BG_DARK:   [f32; 4] = [0.04, 0.05, 0.07, 1.0];
#[cfg(windows)]
const TEXT_MUTED:[f32; 4] = [0.55, 0.58, 0.65, 1.0];

/// Linearly interpolate between two adjacent position samples.
///
/// `samples` is the raw `combat_replay_data.positions` vec: each entry
/// is `[x, y]` (or longer; we only read indices 0 and 1).
/// `t_ms` is elapsed time since fight start. `polling_rate_ms` is the
/// EI sample spacing.
///
/// Returns `None` if `samples` is empty or the resolved sample is
/// malformed (fewer than 2 components). Clamps to the last sample for
/// times past the end. A zero polling rate returns the first sample.
pub fn lerp_position(samples: &[Vec<f64>], t_ms: u64, polling_rate_ms: u64) -> Option<(f64, f64)> {
    if samples.is_empty() {
        return None;
    }
    if polling_rate_ms == 0 || samples.len() == 1 {
        let s = &samples[0];
        if s.len() < 2 { return None; }
        return Some((s[0], s[1]));
    }
    let last_idx = samples.len() - 1;
    let f_idx = (t_ms as f64) / (polling_rate_ms as f64);
    let idx = (f_idx.floor() as usize).min(last_idx);
    let frac = (f_idx - (idx as f64)).clamp(0.0, 1.0);
    let a = &samples[idx];
    if a.len() < 2 { return None; }
    if idx >= last_idx {
        return Some((a[0], a[1]));
    }
    let b = &samples[idx + 1];
    if b.len() < 2 { return None; }
    Some((
        a[0] + (b[0] - a[0]) * frac,
        a[1] + (b[1] - a[1]) * frac,
    ))
}

#[cfg(windows)]
#[allow(dead_code)]
struct PlayerDot<'a> {
    name: &'a str,
    profession: &'a str,
    x: f32,
    y: f32,
    is_self: bool,
}

#[cfg(windows)]
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

#[cfg(windows)]
pub fn render_content(ui: &Ui, json: &EiJson, idx: usize, _derived: &Derived) {
    // Drain a couple of pending tile uploads per frame.
    tile_cache::drain_pending();

    // Resolve which WvW map this fight took place on. EI populates
    // `zone`/`map_name` for some encounters but leaves them empty for
    // WvW logs — fight_name ("Blue Alpine Borderlands", etc.) is the
    // reliable source there.
    let zone = [json.zone.as_deref(), json.map_name.as_deref(), Some(json.fight_name.as_str())]
        .into_iter()
        .flatten()
        .find(|s| !s.is_empty())
        .unwrap_or("");
    let Some(map) = resolve_map_from_zone(zone) else {
        ui.text_colored(TEXT_MUTED, format!("Not a WvW fight (zone: \"{}\")", zone));
        return;
    };

    // Render inside a child window so the map's draw_list output is
    // clipped to this area and its allocated size can't push the parent
    // window into scroll mode (which would hide the tab strip above).
    let avail = ui.content_region_avail();
    let (mw, mh) = map_pixel_size(map);
    ui.child_window("axipulse-map-canvas")
        .size([avail[0], avail[1]])
        .build(|| {
            let inner = ui.content_region_avail();
            let scale = (inner[0] / mw).min(inner[1] / mh).max(0.01);
            let render_w = mw * scale;
            let render_h = mh * scale;
            let origin = ui.cursor_screen_pos();
            let ox = origin[0] + (inner[0] - render_w) * 0.5;
            let oy = origin[1] + (inner[1] - render_h) * 0.5;

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
                let (r, color): (f32, [f32; 4]) = match lm.kind {
                    LandmarkType::Keep  => (6.0, [0.93, 0.27, 0.27, 0.85]),
                    LandmarkType::Tower => (5.0, [0.96, 0.62, 0.04, 0.85]),
                    LandmarkType::Camp  => (4.0, [0.13, 0.77, 0.37, 0.85]),
                    LandmarkType::Ruins => (4.0, [0.55, 0.36, 0.96, 0.85]),
                    LandmarkType::Named => (3.5, [0.42, 0.45, 0.50, 0.85]),
                };
                draw.add_circle([cx, cy], r, color).filled(true).build();
                draw.add_text([cx + r + 2.0, cy - 6.0], TEXT_MUTED, lm.name);
            }

            // Final-frame player positions.
            let dots = collect_final_positions(json, idx);
            for dot in &dots {
                let cx = ox + dot.x * scale;
                let cy = oy + dot.y * scale;
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
                    let r: f32 = if dot.is_self { 5.5 } else { 4.0 };
                    let color: [f32; 4] = if dot.is_self { [0.06, 0.72, 0.51, 0.95] } else { [0.86, 0.86, 0.92, 0.85] };
                    draw.add_circle([cx, cy], r, color).filled(true).build();
                }
            }
        });
}
