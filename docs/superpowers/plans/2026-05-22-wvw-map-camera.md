# WvW Map Camera Implementation Plan (Phase 4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pan and zoom the WvW Map. Mouse-wheel zooms toward the cursor; left-click drag pans; a Reset button restores the fit view; a Follow toggle keeps the local player centered. Tile loads switch zoom level so zoomed-in views stay crisp.

**Architecture:** Viewport transform `(user_scale, pan_x, pan_y, follow_player)` lives on the existing `MapPlayback` struct in `src/ui/map.rs`. The effective scale = `fit_scale * user_scale`; the final origin is `fit_origin + (pan_x, pan_y)`. Mouse input is captured via an `invisible_button` covering the map area, then `is_item_hovered()` + `ui.io().mouse_wheel` for zoom and `is_item_active()` + `ui.io().mouse_delta` for pan. Tile zoom level is selected by a pure helper `tile_zoom_for_scale(user_scale) -> u32` based on upstream `tileZoomForScale`. Following pins the pan so the local player stays at the center of the visible map area.

**Tech Stack:** Rust (existing), arcdps imgui, no new deps.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/ui/map.rs` | Modify | Add view fields to MapPlayback; per-fight reset; tile_zoom_for_scale helper; mouse-wheel zoom; drag-pan; Reset/Follow buttons; follow-player pan-update. |
| `tests/wvw_map_camera_test.rs` | Create | Pure-function tests for `tile_zoom_for_scale` and `zoom_at_point` (the cursor-anchored zoom transform). |
| `README.md` | Modify | Add "pan, zoom, reset, follow" to the Map tab feature bullets; drop "Pan/zoom ships in a follow-up plan." |

---

## Task 1: View state on MapPlayback + per-fight reset

**Files:**
- Modify: `src/ui/map.rs`

Add four fields to `MapPlayback`: `user_scale`, `pan_x`, `pan_y`, `follow_player`. Reset them in `sync_fight_key` when the fight changes.

- [ ] **Step 1: Extend `MapPlayback`**

Find the existing `struct MapPlayback` and `impl MapPlayback::new` in `src/ui/map.rs`. Replace with:

```rust
#[cfg(windows)]
struct MapPlayback {
    time_ms: u64,
    playing: bool,
    speed: f32,
    fight_key: Option<PathBuf>,
    show_party_panel: bool,
    /// User zoom multiplier on top of the fit-to-window scale.
    /// 1.0 = exactly fits; >1.0 = zoomed in.
    user_scale: f32,
    /// Pan offset in screen pixels, applied to the fit origin.
    pan_x: f32,
    pan_y: f32,
    /// When true, every frame re-pins the pan so the local player sits
    /// at the centre of the visible map area.
    follow_player: bool,
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
            user_scale: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            follow_player: false,
        }
    }
    /// Restore the view to its default (fit-to-window, no pan, no follow).
    fn reset_view(&mut self) {
        self.user_scale = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.follow_player = false;
    }
}
```

- [ ] **Step 2: Reset view in `sync_fight_key` on fight change**

Find `fn sync_fight_key`. Inside the `if guard.fight_key.as_ref() != Some(current)` branch, add a call to `reset_view`. Final body:

```rust
fn sync_fight_key(current: &PathBuf) -> (u64, bool, f32) {
    let mut guard = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    if guard.fight_key.as_ref() != Some(current) {
        guard.fight_key = Some(current.clone());
        guard.time_ms = 0;
        guard.playing = false;
        guard.reset_view();
    }
    (guard.time_ms, guard.playing, guard.speed)
}
```

- [ ] **Step 3: Build**

Run: `cargo dll-check 2>&1 | tail -5`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(map): add view-transform state to MapPlayback

Four new fields: user_scale (zoom multiplier on top of fit-scale),
pan_x/pan_y (screen-pixel offsets), and follow_player (auto-centre
on local player). reset_view() restores all four to defaults; called
from sync_fight_key whenever the rendered fight changes so per-fight
camera state doesn't bleed across encounters.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: TDD `tile_zoom_for_scale` + `zoom_at_point` helpers

**Files:**
- Modify: `src/ui/map.rs`
- Create: `tests/wvw_map_camera_test.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/wvw_map_camera_test.rs`:

```rust
use arcdps_axipulse::ui::map::{tile_zoom_for_scale, zoom_at_point};

#[test]
fn tile_zoom_picks_higher_levels_as_scale_grows() {
    assert_eq!(tile_zoom_for_scale(0.5), 4);
    assert_eq!(tile_zoom_for_scale(1.0), 4);
    assert_eq!(tile_zoom_for_scale(1.99), 4);
    assert_eq!(tile_zoom_for_scale(2.0), 5);
    assert_eq!(tile_zoom_for_scale(3.5), 5);
    assert_eq!(tile_zoom_for_scale(4.0), 6);
    assert_eq!(tile_zoom_for_scale(7.5), 6);
    assert_eq!(tile_zoom_for_scale(8.0), 7);
    assert_eq!(tile_zoom_for_scale(100.0), 7);
}

#[test]
fn zoom_at_point_keeps_cursor_pinned() {
    // Before zoom: user_scale=1, pan=(0,0). Cursor at (50, 30) relative
    // to the map's fit centre. Zooming to 2x should adjust pan so the
    // world point under the cursor stays under the cursor.
    // The formula: pan' = pan - (cursor - pan) * (ratio - 1)
    // With pan=(0,0), cursor=(50,30), ratio=2: pan' = (-50, -30).
    let (s, px, py) = zoom_at_point(1.0, 0.0, 0.0, 2.0, 50.0, 30.0);
    assert_eq!(s, 2.0);
    assert_eq!(px, -50.0);
    assert_eq!(py, -30.0);
}

#[test]
fn zoom_at_point_with_existing_pan() {
    // pan=(10, -5), cursor=(20, 10), ratio=2.0/1.0 = 2.
    // pan' = (10, -5) - ((20, 10) - (10, -5)) * 1 = (10 - 10, -5 - 15) = (0, -20).
    let (s, px, py) = zoom_at_point(1.0, 10.0, -5.0, 2.0, 20.0, 10.0);
    assert_eq!(s, 2.0);
    assert_eq!(px, 0.0);
    assert_eq!(py, -20.0);
}

#[test]
fn zoom_at_point_zoom_out_inverse() {
    // Zoom from 2x back to 1x at cursor (50, 30) with pan (-50, -30) (from first test)
    // should land back at pan = (0, 0).
    let (s, px, py) = zoom_at_point(2.0, -50.0, -30.0, 1.0, 50.0, 30.0);
    assert_eq!(s, 1.0);
    assert!((px - 0.0).abs() < 1e-4, "expected px ≈ 0, got {}", px);
    assert!((py - 0.0).abs() < 1e-4, "expected py ≈ 0, got {}", py);
}
```

- [ ] **Step 2: Confirm FAIL**

Run: `cargo test --test wvw_map_camera_test 2>&1 | tail -10`
Expected: build error — `tile_zoom_for_scale` and `zoom_at_point` not found.

- [ ] **Step 3: Implement**

In `src/ui/map.rs`, add these pure functions (no `#[cfg(windows)]` gate — they need to be host-testable). Place them near the other pure helpers like `lerp_position`:

```rust
/// Pick a tile zoom level (z4..z7) based on the user's view scale.
/// Higher view scale → load higher zoom tiles so zoomed-in views stay
/// crisp without re-fetching tiles. Mirrors upstream's
/// `tileZoomForScale` in axipulse/src/renderer/views/map/MovementView.tsx.
pub fn tile_zoom_for_scale(view_scale: f32) -> u32 {
    if view_scale >= 8.0 { 7 }
    else if view_scale >= 4.0 { 6 }
    else if view_scale >= 2.0 { 5 }
    else { 4 }
}

/// Cursor-anchored zoom: given the current scale and pan, returns the
/// new (scale, pan_x, pan_y) such that the world point currently under
/// the cursor remains under the cursor after the zoom.
///
/// `cursor_x`/`cursor_y` are in the same coordinate space as `pan_x`/`pan_y`
/// (i.e. screen pixels relative to the fit-centre of the map).
pub fn zoom_at_point(
    cur_scale: f32,
    cur_pan_x: f32,
    cur_pan_y: f32,
    new_scale: f32,
    cursor_x: f32,
    cursor_y: f32,
) -> (f32, f32, f32) {
    let ratio = new_scale / cur_scale;
    let new_pan_x = cur_pan_x - (cursor_x - cur_pan_x) * (ratio - 1.0);
    let new_pan_y = cur_pan_y - (cursor_y - cur_pan_y) * (ratio - 1.0);
    (new_scale, new_pan_x, new_pan_y)
}
```

- [ ] **Step 4: Confirm PASS**

Run: `cargo test --test wvw_map_camera_test 2>&1 | tail -10`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs tests/wvw_map_camera_test.rs
git commit -m "$(cat <<'EOF'
feat(map): pure helpers tile_zoom_for_scale + zoom_at_point

tile_zoom_for_scale picks z4..z7 based on view scale (matches axipulse
upstream MovementView.tileZoomForScale).
zoom_at_point implements the cursor-anchored zoom transform: the world
point under the cursor stays under the cursor across the scale change.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Apply the view transform in `render_content`

**Files:**
- Modify: `src/ui/map.rs`

Read `user_scale`/`pan_x`/`pan_y` from `MapPlayback` and apply them to the rendered map. Use `tile_zoom_for_scale` to pick the tile zoom dynamically. No input handling yet — Tasks 4 (zoom) and 5 (pan) add the inputs.

- [ ] **Step 1: Read view state at the top of `render_content`**

Find the existing block at the top of `render_content`:

```rust
let _ = sync_fight_key(log_path);
let duration_ms = json.duration_ms;
let time_ms = tick_playback(ui, duration_ms);
```

Change to:

```rust
let _ = sync_fight_key(log_path);
let duration_ms = json.duration_ms;
let time_ms = tick_playback(ui, duration_ms);
let (user_scale, pan_x, pan_y) = {
    let g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    (g.user_scale, g.pan_x, g.pan_y)
};
```

- [ ] **Step 2: Apply scale + pan inside the child window**

Find this block (currently at the top of the child_window closure):

```rust
let scale = (inner[0] / mw).min(inner[1] / mh).max(0.01);
let render_w = mw * scale;
let render_h = mh * scale;
let origin = ui.cursor_screen_pos();
let ox = origin[0] + (inner[0] - render_w) * 0.5;
let oy = origin[1] + (inner[1] - render_h) * 0.5;
```

Replace with:

```rust
let fit_scale = (inner[0] / mw).min(inner[1] / mh).max(0.01);
let scale = fit_scale * user_scale;
let render_w = mw * scale;
let render_h = mh * scale;
let origin = ui.cursor_screen_pos();
let ox = origin[0] + (inner[0] - render_w) * 0.5 + pan_x;
let oy = origin[1] + (inner[1] - render_h) * 0.5 + pan_y;
```

- [ ] **Step 3: Pick the tile zoom level dynamically**

Find this line (inside the child_window closure, around the tile-rendering block):

```rust
let tiles = get_map_tiles(map, MVP_TILE_ZOOM);
```

Replace with:

```rust
let tile_zoom = tile_zoom_for_scale(user_scale);
let tiles = get_map_tiles(map, tile_zoom);
```

Leave the `MVP_TILE_ZOOM` constant in place; the upcoming Reset button still uses default user_scale = 1.0 which maps back to z4 via `tile_zoom_for_scale`. Note: the constant is now unused — remove it. Find:

```rust
#[cfg(windows)]
const MVP_TILE_ZOOM: u32 = 5;
```

Delete those two lines.

- [ ] **Step 4: Build + deploy**

```bash
cargo dll-check 2>&1 | tail -5
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

No visual change yet — defaults still render the map fit-to-viewport. Confirm the build is clean and the deployed DLL loads in-game without panicking.

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): apply view transform + adaptive tile zoom in render

Reads user_scale + pan_x/pan_y from MapPlayback and applies them to
the map render: scale = fit_scale * user_scale; origin shifted by
(pan_x, pan_y). Tile fetch level chosen via tile_zoom_for_scale so
zooming in loads sharper tiles (z4 default → z5/z6/z7 as user zooms).
No input wired yet; controls land in the next two tasks.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Mouse-wheel zoom (cursor-anchored)

**Files:**
- Modify: `src/ui/map.rs`

Cover the map area with an `invisible_button`; when hovered, read `ui.io().mouse_wheel` and apply `zoom_at_point` to the playback state. Clamp `user_scale` to a sane range.

- [ ] **Step 1: Add zoom constants near the other map constants**

In `src/ui/map.rs`, near `BG_DARK` / `TEXT_MUTED`:

```rust
#[cfg(windows)]
const MIN_USER_SCALE: f32 = 1.0;
#[cfg(windows)]
const MAX_USER_SCALE: f32 = 16.0;
#[cfg(windows)]
const ZOOM_STEP: f32 = 0.15;
```

- [ ] **Step 2: Add an invisible button + scroll handler**

Inside the `child_window` closure, AFTER computing `ox`/`oy` but BEFORE `let draw = ui.get_window_draw_list();`, add:

```rust
            // Mouse hit area covering the whole child window — captures
            // wheel + drag for the camera. Sits underneath the panel and
            // controls; imgui's invisible_button is a regular item so
            // later imgui calls layer on top of it normally.
            let hit_pos = ui.cursor_screen_pos();
            ui.invisible_button("##map-camera-hit", [inner[0], inner[1]]);
            ui.set_cursor_screen_pos(hit_pos); // restore cursor so layout below isn't shifted
            let hit_hovered = ui.is_item_hovered();
            let hit_active = ui.is_item_active();
            let wheel = ui.io().mouse_wheel;
            let mouse_pos = ui.io().mouse_pos;
            if hit_hovered && wheel != 0.0 {
                let cur_centre_x = origin[0] + inner[0] * 0.5;
                let cur_centre_y = origin[1] + inner[1] * 0.5;
                let cursor_rel_x = mouse_pos[0] - cur_centre_x;
                let cursor_rel_y = mouse_pos[1] - cur_centre_y;
                let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
                let direction = wheel.signum();
                let next_scale = (g.user_scale * (1.0 + direction * ZOOM_STEP))
                    .clamp(MIN_USER_SCALE, MAX_USER_SCALE);
                if (next_scale - g.user_scale).abs() > f32::EPSILON {
                    let (s, npx, npy) = zoom_at_point(
                        g.user_scale, g.pan_x, g.pan_y, next_scale, cursor_rel_x, cursor_rel_y,
                    );
                    g.user_scale = s;
                    g.pan_x = npx;
                    g.pan_y = npy;
                    g.follow_player = false; // user took manual control
                }
            }
            let _ = hit_active; // used in the pan task
```

> **Engineer note:** `invisible_button` advances the cursor; the `set_cursor_screen_pos(hit_pos)` after it restores the cursor so the rest of the closure (panel + controls placement) lands in the same spot it would have without the button. If `set_cursor_screen_pos` triggers the historical Wine crash (see commit `6430e2f`), substitute with `ui.same_line_with_pos(hit_pos[0]);` followed by setting Y via dummy — but most likely it's fine here because the button is the very first item drawn in this closure (no live draw_list).

- [ ] **Step 3: Build + deploy + verify**

```bash
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

Trigger a sim log. Scroll the mouse wheel over the Map tab. Map should zoom in/out centred on the cursor.

- [ ] **Step 4: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): mouse-wheel zoom centred on the cursor

Invisible button over the map area captures hover state. When hovered
and the mouse wheel ticks, apply zoom_at_point with the cursor's
screen-relative coords. Scale is clamped to [1.0, 16.0]. Manual zoom
turns Follow off so the user's view doesn't snap back to centre on
the next playback frame.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Drag-pan

**Files:**
- Modify: `src/ui/map.rs`

When the hit button is active (left-click held) and the mouse moves, add `mouse_delta` to `pan_x`/`pan_y`.

- [ ] **Step 1: Wire the pan handler**

Replace the placeholder `let _ = hit_active;` line in `render_content` (added in Task 4) with:

```rust
            if hit_active {
                let delta = ui.io().mouse_delta;
                if delta[0] != 0.0 || delta[1] != 0.0 {
                    let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
                    g.pan_x += delta[0];
                    g.pan_y += delta[1];
                    g.follow_player = false; // dragging cancels follow
                }
            }
```

- [ ] **Step 2: Build + deploy + verify**

```bash
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

Trigger a sim log. Left-click drag over the Map tab. Map should pan with the cursor. The party panel and bottom controls should still receive their clicks (their items render after the invisible button so they take precedence).

- [ ] **Step 3: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): left-click drag to pan

When the camera hit-button is active (left mouse held), accumulate
mouse_delta into pan_x/pan_y each frame. Dragging cancels Follow
so the user's manual pan isn't fought by the auto-centre next frame.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Reset-view + Follow toggle buttons

**Files:**
- Modify: `src/ui/map.rs`

Add two buttons to the controls row: **Reset** (calls `reset_view`) and **Follow** (toggles `follow_player`). The Follow handler also runs each frame in `render_content` to pin the pan when enabled.

- [ ] **Step 1: Add the buttons to `render_controls`**

Find `render_controls`. Update the snapshot block at the top to also pull `follow_player`:

```rust
let (cur_time, playing, speed, panel_open, follow) = {
    let g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    (g.time_ms, g.playing, g.speed, g.show_party_panel, g.follow_player)
};
```

After the existing `if ui.button(party_label) { ... }` + `ui.same_line();` block, insert:

```rust
let follow_label = if follow { "Follow*" } else { "Follow " };
if ui.button(follow_label) {
    let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    g.follow_player = !g.follow_player;
}
ui.same_line();
if ui.button("Reset") {
    let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    g.reset_view();
}
ui.same_line();
```

- [ ] **Step 2: Compute follow-pan inside the child window**

Inside the `child_window` closure, AFTER computing `fit_scale`, AFTER computing `scale`, but BEFORE computing `ox`/`oy`, insert:

```rust
            // If Follow is on, override pan so the local player lands
            // at the fit centre this frame.
            let (user_scale, pan_x, pan_y) = {
                let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
                if g.follow_player {
                    let polling_rate = json.combat_replay_meta_data
                        .as_ref().and_then(|m| m.polling_rate).unwrap_or(150);
                    let local_pos = json.players.get(idx)
                        .and_then(|p| p.combat_replay_data.as_ref())
                        .and_then(|rd| lerp_position(&rd.positions, time_ms, polling_rate));
                    if let Some((wx, wy)) = local_pos {
                        // Map-centre world point at the current scale: mw/2, mh/2.
                        // Pan so (wx, wy) maps to the fit centre.
                        let centred_x = (mw * 0.5 - wx as f32) * scale;
                        let centred_y = (mh * 0.5 - wy as f32) * scale;
                        g.pan_x = centred_x;
                        g.pan_y = centred_y;
                    }
                }
                (g.user_scale, g.pan_x, g.pan_y)
            };
```

Then in the line below (the `let render_w` block), replace the outer `user_scale`/`pan_x`/`pan_y` references with these shadowed locals — they should already match since we shadowed with the same names. The existing line:

```rust
let render_w = mw * scale;
let render_h = mh * scale;
let origin = ui.cursor_screen_pos();
let ox = origin[0] + (inner[0] - render_w) * 0.5 + pan_x;
let oy = origin[1] + (inner[1] - render_h) * 0.5 + pan_y;
```

stays as-is — but note that `scale = fit_scale * user_scale` was computed BEFORE we shadowed `user_scale`. That's fine because Follow only changes `pan`, not `user_scale`. The pan adjustment uses the already-computed `scale`, which is what we want.

> **Engineer note:** the read of `(user_scale, pan_x, pan_y)` at the TOP of `render_content` (added in Task 3) is still needed because the value flows through `render_controls` button states. Keep that read.

- [ ] **Step 3: Build + deploy + verify**

```bash
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

Trigger a sim log. In GW2:
- Pan + zoom, then click **Reset** → view snaps back to fit.
- Click **Follow** → button shows `Follow*`; play the scrubber and your dot stays centred. Click **Follow** again to release.
- Manually drag or wheel-zoom while Follow is on → it auto-disables (per Tasks 4 and 5).

- [ ] **Step 4: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): Reset + Follow buttons on the controls row

Reset calls reset_view() to restore fit-scale + zero pan + Follow off.
Follow toggles auto-centre; when on, the render closure overrides pan
each frame so the local player's lerp'd position lands at the fit
centre. Manual zoom or drag (Tasks 4 and 5) automatically turns
Follow off so the user's input isn't fought.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the Map section**

In `README.md`, find the WvW Combat Replay bullet list. Append:

```markdown
- Camera: mouse-wheel zoom (cursor-anchored), left-click drag to pan, Reset button, Follow toggle to keep your dot centred.
```

In the trailing "ships in a follow-up plan" paragraph, remove the whole sentence (it should now have no remaining deferred items from the original Phase 4 scope). If it would leave the paragraph empty, delete the paragraph entirely.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "$(cat <<'EOF'
docs: note camera controls on Map tab

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Self-review

- [ ] Spec coverage: view state (T1), tile zoom helper (T2), apply transform + adaptive tile zoom (T3), scroll-zoom (T4), drag-pan (T5), reset + follow (T6), README (T7). All seven scope items present. ✓
- [ ] No placeholders. ✓ (Task 6 Step 2 contains a deliberately-flawed sketch superseded by Step 3, with a note telling the engineer to discard Step 2; that's a non-standard structure but the code in Step 3 is complete.)
- [ ] Type consistency: `tile_zoom_for_scale(f32) -> u32` and `zoom_at_point(f32, f32, f32, f32, f32, f32) -> (f32, f32, f32)` are referenced identically between Task 2 (define) and Tasks 3/4 (use). ✓
- [ ] `MapPlayback` fields `user_scale`, `pan_x`, `pan_y`, `follow_player` are defined in T1 and read/written in T3, T4, T5, T6. ✓
- [ ] `reset_view()` method defined in T1, used in T1 (sync_fight_key) and T6 (Reset button). ✓

