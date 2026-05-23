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
    show_party_panel: bool,
}

#[cfg(windows)]
impl MapPlayback {
    fn new() -> Self {
        Self {
            time_ms: 0,
            playing: false,
            speed: 1.0,
            fight_key: None,
            show_party_panel: false,
        }
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

/// Boon stack count at time `t_ms`. Each `states` entry is `[time_ms, stacks]`.
/// Returns the value of the last sample at or before `t_ms`, else 0.
pub fn boon_stacks_at(states: &[Vec<f64>], t_ms: u64) -> i32 {
    if states.is_empty() {
        return 0;
    }
    let t = t_ms as f64;
    let mut last = 0_i32;
    for s in states {
        if s.len() < 2 { continue; }
        if s[0] > t { break; }
        last = s[1] as i32;
    }
    last
}

/// Up to `max_results` most recent skill casts at or before `t_ms`, newest
/// first. Negative cast times (pre-fight) are filtered out. Returns
/// `Vec<(skill_id, cast_time_ms)>`.
pub fn recent_skill_casts(
    rotation: &[crate::ei_model::RotationEntry],
    t_ms: u64,
    max_results: usize,
) -> Vec<(i64, i64)> {
    let t = t_ms as i64;
    let mut all: Vec<(i64, i64)> = Vec::new();
    for entry in rotation {
        for cast in &entry.skills {
            if cast.cast_time < 0 { continue; }
            if cast.cast_time > t { continue; }
            all.push((entry.id, cast.cast_time));
        }
    }
    all.sort_by(|a, b| b.1.cmp(&a.1));
    all.truncate(max_results);
    all
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
    let (cur_time, playing, speed, panel_open) = {
        let g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
        (g.time_ms, g.playing, g.speed, g.show_party_panel)
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

    let party_label = if panel_open { "Party*" } else { "Party " };
    if ui.button(party_label) {
        let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
        g.show_party_panel = !g.show_party_panel;
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
fn render_party_panel(
    ui: &Ui,
    json: &EiJson,
    self_idx: usize,
    time_ms: u64,
    panel_origin: [f32; 2],
    panel_size: [f32; 2],
) {
    let draw = ui.get_window_draw_list();

    // Panel background (overlays on top of the map below).
    let bg = [0.08, 0.10, 0.13, 0.92];
    draw.add_rect(
        panel_origin,
        [panel_origin[0] + panel_size[0], panel_origin[1] + panel_size[1]],
        bg,
    ).filled(true).rounding(6.0).build();

    let local_group = json.players.get(self_idx).map(|p| p.group).unwrap_or(-1);
    let commander_pos: Option<(f64, f64)> = find_commander_position(json, time_ms);
    let inch_to_pixel = json
        .combat_replay_meta_data
        .as_ref()
        .and_then(|m| m.inch_to_pixel)
        .unwrap_or(1.0);
    let polling_rate = json
        .combat_replay_meta_data
        .as_ref()
        .and_then(|m| m.polling_rate)
        .unwrap_or(150);

    let pad = 10.0_f32;
    let mut y = panel_origin[1] + pad;
    draw.add_text(
        [panel_origin[0] + pad, y],
        [0.55, 0.58, 0.65, 1.0],
        "PARTY",
    );
    y += 18.0;
    let row_h = 108.0_f32;

    for (i, p) in json.players.iter().enumerate() {
        if p.group != local_group { continue; }
        if p.not_in_squad { continue; }

        let rd_pos = p.combat_replay_data.as_ref()
            .and_then(|rd| lerp_position(&rd.positions, time_ms, polling_rate));
        let status = p.combat_replay_data.as_ref()
            .map(|rd| status_at(&rd.dead, &rd.down, time_ms))
            .unwrap_or(MemberStatus::Alive);
        let hp = health_at(&p.health_percents, time_ms);

        let row_y0 = y;
        let row_y1 = y + row_h;
        draw.add_rect(
            [panel_origin[0] + 4.0, row_y0],
            [panel_origin[0] + panel_size[0] - 4.0, row_y1],
            [1.0, 1.0, 1.0, 0.04],
        ).filled(true).rounding(4.0).build();

        let icon_size = 20.0_f32;
        let icon_x = panel_origin[0] + pad;
        let icon_y = row_y0 + 6.0;
        if let Some(icon) = crate::ui::icons::lookup_bundled(p.profession.as_str()) {
            draw.add_image(icon.tex, [icon_x, icon_y], [icon_x + icon_size, icon_y + icon_size]).build();
        }

        let name_x = icon_x + icon_size + 8.0;
        let name_color = if i == self_idx { [0.06, 0.72, 0.51, 1.0] }
            else if p.has_commander_tag { [0.96, 0.62, 0.04, 1.0] }
            else { [0.97, 0.97, 1.00, 1.0] };
        draw.add_text([name_x, icon_y + 2.0], name_color, p.name.as_str());

        if let (Some(cp), Some((px, py))) = (commander_pos, rd_pos) {
            if !p.has_commander_tag {
                let dx = (px - cp.0) as f32;
                let dy = (py - cp.1) as f32;
                let pixels = (dx * dx + dy * dy).sqrt();
                let inches = (pixels / inch_to_pixel as f32) as i32;
                let dist_color = if inches > 600 { [0.93, 0.27, 0.27, 1.0] }
                    else if inches > 300 { [0.96, 0.62, 0.04, 1.0] }
                    else { [0.13, 0.77, 0.37, 1.0] };
                draw.add_text(
                    [panel_origin[0] + panel_size[0] - 50.0, icon_y + 2.0],
                    dist_color,
                    format!("{}", inches),
                );
            }
        }

        let bar_x0 = name_x;
        let bar_y0 = row_y0 + 26.0;
        let bar_w = panel_size[0] - (name_x - panel_origin[0]) - pad;
        let bar_h = 8.0;
        draw.add_rect([bar_x0, bar_y0], [bar_x0 + bar_w, bar_y0 + bar_h], [1.0, 1.0, 1.0, 0.08])
            .filled(true).rounding(2.0).build();
        let (fill_color, fill_frac, label): ([f32; 4], f32, String) = match status {
            MemberStatus::Dead => ([0.55, 0.13, 0.13, 1.0], 1.0, "Dead".to_string()),
            MemberStatus::Down => ([0.23, 0.51, 0.96, 1.0], 1.0, "Down".to_string()),
            MemberStatus::Alive => {
                let c = if hp > 50.0 { [0.13, 0.77, 0.37, 1.0] }
                    else if hp > 25.0 { [0.96, 0.62, 0.04, 1.0] }
                    else { [0.93, 0.27, 0.27, 1.0] };
                (c, (hp / 100.0) as f32, format!("{}%", hp.round() as i32))
            }
        };
        let fill_w = (bar_w * fill_frac).max(0.0);
        if fill_w > 0.0 {
            draw.add_rect([bar_x0, bar_y0], [bar_x0 + fill_w, bar_y0 + bar_h], fill_color)
                .filled(true).rounding(2.0).build();
        }
        draw.add_text([bar_x0 + 4.0, bar_y0 + bar_h + 2.0], [0.78, 0.78, 0.85, 1.0], &label);

        // Boon stack tiles.
        let boon_px = 18.0_f32;
        let mut bx = name_x;
        let by = bar_y0 + bar_h + 18.0;
        for boon_id in crate::map::boon_panel::PANEL_BOON_ORDER {
            let stacks = p.buff_uptimes.iter()
                .find(|b| b.id == *boon_id)
                .map(|b| boon_stacks_at(&b.states, time_ms))
                .unwrap_or(0);
            if stacks == 0 { continue; }
            let icon = crate::ui::icons::lookup(
                json,
                crate::ui::icons::IconKey { kind: crate::ui::icons::IconKind::Buff, id: *boon_id },
            );
            if let Some(handle) = icon {
                draw.add_image(handle.tex, [bx, by], [bx + boon_px, by + boon_px]).build();
            } else {
                draw.add_rect([bx, by], [bx + boon_px, by + boon_px], [1.0, 1.0, 1.0, 0.15])
                    .filled(true).rounding(3.0).build();
            }
            if stacks > 1 {
                draw.add_text(
                    [bx + boon_px - 8.0, by + boon_px - 10.0],
                    [0.97, 0.97, 1.0, 1.0],
                    format!("{stacks}"),
                );
            }
            bx += boon_px + 2.0;
            if bx + boon_px > panel_origin[0] + panel_size[0] - pad { break; }
        }

        // Recent skill casts.
        let skills = recent_skill_casts(&p.rotation, time_ms, 4);
        if !skills.is_empty() {
            let skill_px = 18.0_f32;
            let mut sx = name_x;
            let sy = by + boon_px + 4.0;
            let latest_hold_ms: i64 = 1200;
            let latest_fade_ms: i64 = 2500;
            let fade_ms: i64 = 1500;
            let t = time_ms as i64;
            for (idx_s, (id, cast_t)) in skills.iter().enumerate() {
                let age = t - cast_t;
                let opacity: f32 = if idx_s == 0 {
                    if age <= latest_hold_ms { 1.0 }
                    else if age <= latest_hold_ms + latest_fade_ms {
                        1.0 - (age - latest_hold_ms) as f32 / latest_fade_ms as f32
                    } else { 0.0 }
                } else {
                    if age >= fade_ms { 0.0 } else { 1.0 - age as f32 / fade_ms as f32 }
                };
                if opacity <= 0.0 { continue; }
                let icon = crate::ui::icons::lookup(
                    json,
                    crate::ui::icons::IconKey { kind: crate::ui::icons::IconKind::Skill, id: *id },
                );
                if let Some(handle) = icon {
                    draw.add_image(handle.tex, [sx, sy], [sx + skill_px, sy + skill_px])
                        .col([1.0, 1.0, 1.0, opacity])
                        .build();
                }
                sx += skill_px + 2.0;
                if sx + skill_px > panel_origin[0] + panel_size[0] - pad { break; }
            }
        }

        y += row_h + 4.0;
        if y > panel_origin[1] + panel_size[1] - row_h { break; }
    }
}


#[cfg(windows)]
fn find_commander_position(json: &EiJson, time_ms: u64) -> Option<(f64, f64)> {
    let polling_rate = json
        .combat_replay_meta_data
        .as_ref()
        .and_then(|m| m.polling_rate)
        .unwrap_or(150);
    for p in &json.players {
        if !p.has_commander_tag { continue; }
        if let Some(rd) = p.combat_replay_data.as_ref() {
            if let Some(pos) = lerp_position(&rd.positions, time_ms, polling_rate) {
                return Some(pos);
            }
        }
    }
    None
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
    account: &'a str,
    profession: &'a str,
    x: f32,
    y: f32,
    is_self: bool,
    is_commander: bool,
    group: i32,
    status: MemberStatus,
    health_pct: f64,
    /// Index of the most recent sample at or before time_ms.
    sample_idx: usize,
    /// The full positions vec, borrowed for the duration of this frame.
    positions: &'a [Vec<f64>],
    player_index: usize,
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
            account: p.account.as_str(),
            profession: p.profession.as_str(),
            x: x as f32,
            y: y as f32,
            is_self: i == self_idx,
            is_commander: p.has_commander_tag,
            group: p.group as i32,
            status: status_at(&rd.dead, &rd.down, time_ms),
            health_pct: health_at(&p.health_percents, time_ms),
            sample_idx,
            positions: &rd.positions,
            player_index: i,
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
            let panel_open = PLAYBACK.lock().ok().map(|g| g.show_party_panel).unwrap_or(false);
            let inner = ui.content_region_avail();

            // Map gets the full child-window area. The party panel, when
            // open, overlays the left 260 px on top of the map.
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
                let sz_alive = if dot.is_self { 18.0 } else { 14.0 };

                match dot.status {
                    MemberStatus::Dead => {
                        let r: f32 = 7.0;
                        draw.add_circle([cx, cy], r, [0.93, 0.27, 0.27, 0.95]).filled(true).build();
                        draw.add_circle([cx, cy], r, [0.55, 0.10, 0.10, 1.0]).thickness(1.5).build();
                        let h = r * 0.55;
                        draw.add_line([cx - h, cy - h], [cx + h, cy + h], [1.0, 1.0, 1.0, 0.95]).thickness(1.8).build();
                        draw.add_line([cx + h, cy - h], [cx - h, cy + h], [1.0, 1.0, 1.0, 0.95]).thickness(1.8).build();
                    }
                    MemberStatus::Down => {
                        let r: f32 = 6.5;
                        draw.add_circle([cx, cy], r, [0.23, 0.51, 0.96, 0.85]).filled(true).build();
                        draw.add_circle([cx, cy], r, [0.10, 0.30, 0.70, 1.0]).thickness(1.5).build();
                        let h = r * 0.55;
                        draw.add_triangle(
                            [cx - h, cy - h * 0.6],
                            [cx + h, cy - h * 0.6],
                            [cx, cy + h * 0.7],
                            [1.0, 1.0, 1.0, 0.95],
                        ).filled(true).build();
                    }
                    MemberStatus::Alive => {
                        if let Some(icon) = crate::ui::icons::lookup_bundled(dot.profession) {
                            let half = sz_alive * 0.5;
                            if dot.is_self {
                                draw.add_circle([cx, cy], half + 2.5, [0.06, 0.72, 0.51, 0.85])
                                    .thickness(2.0)
                                    .build();
                            } else if dot.is_commander {
                                draw.add_circle([cx, cy], half + 2.5, [0.96, 0.62, 0.04, 0.90])
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
                }
            }

            // Drop the outer draw_list so render_party_panel can acquire
            // its own (imgui-rs forbids two live DrawListMut at once).
            drop(draw);

            // Party panel overlays the left 260 px of the map area.
            if panel_open {
                let panel_w: f32 = 260.0_f32.min(inner[0]);
                render_party_panel(
                    ui,
                    json,
                    idx,
                    time_ms,
                    [origin[0], origin[1]],
                    [panel_w, inner[1]],
                );
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
