//! WvW combat-replay map view. MVP: static final-frame render of
//! squad positions on top of tile background + landmark pins.

#[cfg(windows)]
use std::sync::Mutex;
#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use once_cell::sync::Lazy;

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

/// Playback state for the Map tab. One instance lives for the plugin
/// lifetime; it resets to t=0 / paused whenever the rendered fight
/// changes (detected via `log_path`).
#[cfg(windows)]
struct MapPlayback {
    time_ms: u64,
    playing: bool,
    speed: f32,
    fight_key: Option<PathBuf>,
}

#[cfg(windows)]
impl MapPlayback {
    fn new() -> Self {
        Self { time_ms: 0, playing: false, speed: 1.0, fight_key: None }
    }
}

#[cfg(windows)]
static PLAYBACK: Lazy<Mutex<MapPlayback>> = Lazy::new(|| Mutex::new(MapPlayback::new()));

#[cfg(windows)]
const BG_DARK:   [f32; 4] = [0.04, 0.05, 0.07, 1.0];
#[cfg(windows)]
const TEXT_MUTED:[f32; 4] = [0.55, 0.58, 0.65, 1.0];

#[cfg(windows)]
const TRAIL_LENGTH_SAMPLES: usize = 15;
#[cfg(windows)]
const TRAIL_COLOR_HISTORY: [f32; 4] = [0.86, 0.86, 0.92, 0.18];
#[cfg(windows)]
const TRAIL_COLOR_RECENT_SELF: [f32; 4] = [0.06, 0.72, 0.51, 0.65];
#[cfg(windows)]
const TRAIL_COLOR_RECENT_PEER: [f32; 4] = [0.86, 0.86, 0.92, 0.55];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatus { Alive, Down, Dead }

/// Status of a player at time `t_ms`. Dead overrides Down.
pub fn status_at(dead_ranges: &[Vec<f64>], down_ranges: &[Vec<f64>], t_ms: u64) -> MemberStatus {
    let t = t_ms as f64;
    for r in dead_ranges {
        if r.len() >= 2 && t >= r[0] && t <= r[1] {
            return MemberStatus::Dead;
        }
    }
    for r in down_ranges {
        if r.len() >= 2 && t >= r[0] && t <= r[1] {
            return MemberStatus::Down;
        }
    }
    MemberStatus::Alive
}

/// Health percent at time `t_ms`. Each `samples` entry is `[time_ms, hp_percent]`.
/// Returns the most recent sample whose time is <= `t_ms`. Falls back to the
/// first sample if `t_ms` is before any sample. Returns 100.0 if no samples.
pub fn health_at(samples: &[Vec<f64>], t_ms: u64) -> f64 {
    if samples.is_empty() {
        return 100.0;
    }
    let t = t_ms as f64;
    let mut last = samples[0].get(1).copied().unwrap_or(100.0);
    for s in samples {
        if s.len() < 2 { continue; }
        if s[0] > t { break; }
        last = s[1];
    }
    last
}

/// Reset playback to t=0, paused, when the rendered fight changes.
/// Returns the (possibly updated) (time_ms, playing, speed) tuple.
#[cfg(windows)]
fn sync_fight_key(current: &PathBuf) -> (u64, bool, f32) {
    let mut guard = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    if guard.fight_key.as_ref() != Some(current) {
        guard.fight_key = Some(current.clone());
        guard.time_ms = 0;
        guard.playing = false;
    }
    (guard.time_ms, guard.playing, guard.speed)
}

/// Advance `time_ms` by the current frame delta while `playing` is true.
/// Auto-pauses at duration_ms. Returns the current time_ms after the tick.
#[cfg(windows)]
fn tick_playback(ui: &Ui, duration_ms: u64) -> u64 {
    let mut guard = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    if guard.playing && duration_ms > 0 {
        let delta_ms = (ui.io().delta_time * 1000.0 * guard.speed) as i64;
        let next = (guard.time_ms as i64).saturating_add(delta_ms).max(0) as u64;
        if next >= duration_ms {
            guard.time_ms = duration_ms;
            guard.playing = false;
        } else {
            guard.time_ms = next;
        }
    }
    guard.time_ms
}

#[cfg(windows)]
fn render_controls(ui: &Ui, duration_ms: u64) {
    // Snapshot state up front so we don't hold the lock across imgui calls.
    let (cur_time, playing, speed) = {
        let g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
        (g.time_ms, g.playing, g.speed)
    };

    // Play / Pause button.
    let play_label = if playing { "Pause" } else { "Play" };
    if ui.button(play_label) {
        let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
        if !g.playing && g.time_ms >= duration_ms && duration_ms > 0 {
            g.time_ms = 0;
        }
        g.playing = !g.playing;
    }
    ui.same_line();

    // Speed cycle button.
    let speed_label = format!("{:.1}x", speed);
    if ui.button(&speed_label) {
        let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
        g.speed = match g.speed {
            x if x < 0.75 => 1.0,
            x if x < 1.25 => 1.5,
            x if x < 1.75 => 2.0,
            _             => 0.5,
        };
    }
    ui.same_line();

    // M:SS / M:SS time label.
    let label = format!("{} / {}", mmss(cur_time), mmss(duration_ms));
    ui.text(&label);
    ui.same_line();

    // Scrubber — fills remaining width.
    let avail = ui.content_region_avail()[0].max(80.0);
    ui.set_next_item_width(avail);
    let mut slider_val: i32 = cur_time.min(i32::MAX as u64) as i32;
    let max_val = duration_ms.min(i32::MAX as u64) as i32;
    if ui.slider_config("##map-scrubber", 0_i32, max_val)
        .display_format("")
        .build(&mut slider_val)
    {
        let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
        g.time_ms = slider_val.max(0) as u64;
        g.playing = false;
    }
}

#[cfg(windows)]
fn mmss(ms: u64) -> String {
    let s = ms / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(windows)]
#[allow(dead_code)]
struct PlayerDot<'a> {
    name: &'a str,
    profession: &'a str,
    x: f32,
    y: f32,
    is_self: bool,
    /// Index of the most recent sample at or before time_ms.
    sample_idx: usize,
    /// The full positions vec, borrowed for the duration of this frame.
    positions: &'a [Vec<f64>],
}

#[cfg(windows)]
fn collect_positions_at_time<'a>(
    json: &'a EiJson,
    self_idx: usize,
    time_ms: u64,
) -> Vec<PlayerDot<'a>> {
    let polling_rate = json
        .combat_replay_meta_data
        .as_ref()
        .and_then(|m| m.polling_rate)
        .unwrap_or(150);
    let mut out = Vec::new();
    for (i, p) in json.players.iter().enumerate() {
        let Some(rd) = p.combat_replay_data.as_ref() else { continue };
        let Some((x, y)) = lerp_position(&rd.positions, time_ms, polling_rate) else { continue };
        let sample_idx = if polling_rate == 0 || rd.positions.is_empty() {
            0
        } else {
            ((time_ms / polling_rate) as usize).min(rd.positions.len().saturating_sub(1))
        };
        out.push(PlayerDot {
            name: p.name.as_str(),
            profession: p.profession.as_str(),
            x: x as f32,
            y: y as f32,
            is_self: i == self_idx,
            sample_idx,
            positions: &rd.positions,
        });
    }
    out
}

#[cfg(windows)]
pub fn render_content(ui: &Ui, json: &EiJson, idx: usize, _derived: &Derived, log_path: &std::path::PathBuf) {
    // Drain a couple of pending tile uploads per frame.
    tile_cache::drain_pending();
    let _ = sync_fight_key(log_path);
    let duration_ms = json.duration_ms;
    let time_ms = tick_playback(ui, duration_ms);

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

            // Time-indexed player positions.
            let dots = collect_positions_at_time(json, idx, time_ms);

            // Trails (drawn before markers so dots sit on top).
            for dot in &dots {
                let recent_start = dot.sample_idx.saturating_sub(TRAIL_LENGTH_SAMPLES);
                // Historical: every other sample, faded.
                if recent_start > 1 {
                    let mut prev: Option<[f32; 2]> = None;
                    for sample in dot.positions[..recent_start].iter().step_by(2) {
                        if sample.len() < 2 { continue; }
                        let p = [ox + (sample[0] as f32) * scale, oy + (sample[1] as f32) * scale];
                        if let Some(q) = prev {
                            draw.add_line(q, p, TRAIL_COLOR_HISTORY).thickness(1.0).build();
                        }
                        prev = Some(p);
                    }
                }
                // Recent: last TRAIL_LENGTH_SAMPLES segments, brighter.
                let recent_end = (dot.sample_idx + 1).min(dot.positions.len());
                if recent_end > recent_start + 1 {
                    let recent_slice = &dot.positions[recent_start..recent_end];
                    let color = if dot.is_self { TRAIL_COLOR_RECENT_SELF } else { TRAIL_COLOR_RECENT_PEER };
                    let thick = if dot.is_self { 2.0 } else { 1.5 };
                    let mut prev: Option<[f32; 2]> = None;
                    for sample in recent_slice {
                        if sample.len() < 2 { continue; }
                        let p = [ox + (sample[0] as f32) * scale, oy + (sample[1] as f32) * scale];
                        if let Some(q) = prev {
                            draw.add_line(q, p, color).thickness(thick).build();
                        }
                        prev = Some(p);
                    }
                }
            }

            // Player markers.
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

            // Pin the controls row to the bottom of the child window.
            let remaining = ui.content_region_avail();
            let row_h = ui.frame_height_with_spacing();
            if remaining[1] > row_h {
                ui.dummy([0.0, remaining[1] - row_h]);
            }
            render_controls(ui, duration_ms);
        });
}
