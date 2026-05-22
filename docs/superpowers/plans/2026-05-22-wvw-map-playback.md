# WvW Map Playback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the static WvW map view into a time-scrubbable combat replay: play/pause/speed controls, a slider, interpolated player positions, and short motion trails. Per-fight playback state that resets when the user switches fights.

**Architecture:** All playback state lives in one `static Mutex<MapPlayback>` in `src/ui/map.rs` (same pattern as `TOP_TAB` / `SUBVIEW`). Time advances by `ui.io().delta_time` while `playing` is true. The renderer indexes into each player's `combat_replay_data.positions` via `time_ms / polling_rate` with a fractional component for sub-sample lerp. Controls render at the bottom of the map's child window. Switching fights (detected via log path) resets playback to t=0 and pauses.

**Tech Stack:** Rust (existing), arcdps imgui (slider, button, drag-int), no new deps.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/ui/map.rs` | Modify | Add `MapPlayback` state, frame-tick advance, fight-change reset, slider/button controls, time-indexed render, trails. |
| `tests/wvw_map_lerp_test.rs` | Create | Pure-function tests for the position-lerp helper. |
| `src/lib.rs` | Modify (maybe) | If we extract `lerp_position` to a public helper, ensure visibility. |

The whole feature lives in `src/ui/map.rs`. It will grow from ~140 to ~330 LOC — still well under the size of `src/ui/pulse.rs` or `src/ui/timeline.rs`, so no extraction warranted yet.

---

## Task 1: TDD position interpolation helper

**Files:**
- Modify: `src/ui/map.rs` (add `lerp_position` + make it `pub(crate)` for testability)
- Create: `tests/wvw_map_lerp_test.rs`

The renderer currently grabs `positions.last()`. To support time scrubbing we need: given a time `t_ms`, a list of `Vec<f64>` positions, and a `polling_rate_ms`, return the interpolated `(x, y)` at that time.

- [ ] **Step 1: Write the failing test**

Create `tests/wvw_map_lerp_test.rs`:

```rust
use arcdps_axipulse::ui::map::lerp_position;

fn pos(x: f64, y: f64) -> Vec<f64> { vec![x, y] }

#[test]
fn at_zero_returns_first_sample() {
    let samples = vec![pos(10.0, 20.0), pos(110.0, 220.0)];
    assert_eq!(lerp_position(&samples, 0, 500), Some((10.0, 20.0)));
}

#[test]
fn at_polling_rate_returns_second_sample() {
    let samples = vec![pos(10.0, 20.0), pos(110.0, 220.0)];
    assert_eq!(lerp_position(&samples, 500, 500), Some((110.0, 220.0)));
}

#[test]
fn between_samples_lerps_linearly() {
    let samples = vec![pos(0.0, 0.0), pos(100.0, 200.0)];
    // 250ms is half of polling_rate=500; expect midpoint.
    assert_eq!(lerp_position(&samples, 250, 500), Some((50.0, 100.0)));
}

#[test]
fn past_last_sample_clamps_to_last() {
    let samples = vec![pos(10.0, 20.0), pos(110.0, 220.0)];
    // t=5000ms with polling_rate=500 → idx=10, well past end.
    assert_eq!(lerp_position(&samples, 5000, 500), Some((110.0, 220.0)));
}

#[test]
fn empty_samples_returns_none() {
    let samples: Vec<Vec<f64>> = vec![];
    assert_eq!(lerp_position(&samples, 0, 500), None);
}

#[test]
fn single_sample_returns_it() {
    let samples = vec![pos(7.0, 8.0)];
    assert_eq!(lerp_position(&samples, 1234, 500), Some((7.0, 8.0)));
}

#[test]
fn zero_polling_rate_returns_first_sample() {
    let samples = vec![pos(1.0, 2.0), pos(3.0, 4.0)];
    assert_eq!(lerp_position(&samples, 100, 0), Some((1.0, 2.0)));
}

#[test]
fn malformed_sample_returns_none() {
    let samples = vec![pos(1.0, 2.0), vec![3.0]]; // second sample missing y
    // Lerp asks for idx 1 → that sample is malformed.
    assert_eq!(lerp_position(&samples, 500, 500), None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test wvw_map_lerp_test`
Expected: FAIL — `lerp_position` doesn't exist yet.

- [ ] **Step 3: Add the helper to `src/ui/map.rs`**

In `src/ui/map.rs`, immediately above the existing `struct PlayerDot<'a>` declaration, add:

```rust
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
```

Also confirm `src/ui/map.rs` is reachable from integration tests. The file is already `pub mod map;` in `src/ui/mod.rs`; check that `src/lib.rs` has `pub mod ui;`. If `ui` is not pub, the test won't see `crate::ui::map::lerp_position`. Run:

```bash
grep -n "pub mod ui" src/lib.rs
```

If it prints nothing, change `mod ui;` to `pub mod ui;` in `src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test wvw_map_lerp_test`
Expected: 8 passed.

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs tests/wvw_map_lerp_test.rs src/lib.rs
git commit -m "$(cat <<'EOF'
feat(map): add lerp_position helper for time-indexed playback

Pure function that maps (t_ms, polling_rate) -> (x, y) by indexing
into combat_replay_data.positions with a fractional lerp between
adjacent samples. Clamps to last sample past the end; tolerates
malformed samples and zero polling rate.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Playback state + per-fight reset

**Files:**
- Modify: `src/ui/map.rs`

Introduce a `MapPlayback` struct stored in a single `static Mutex` — same pattern as the existing `TOP_TAB` / `SUBVIEW` / `FIGHT_SEL` statics in the codebase. On every render, detect whether the displayed fight has changed (compare `log_path`); if so, reset to t=0 and pause.

This task wires the state in but doesn't render anything new — controls land in Task 3, interpolated player render in Task 4. Verify by reading the state in a `dbg!` only if desired; no UI change expected.

- [ ] **Step 1: Add state types + getter/mutators to `src/ui/map.rs`**

Add at the top of the file, after the existing `use` imports and `const` declarations:

```rust
use std::sync::Mutex;
use std::path::PathBuf;
use once_cell::sync::Lazy;

/// Playback state for the Map tab. One instance lives for the plugin
/// lifetime; it resets to t=0 / paused whenever the rendered fight
/// changes (detected via `log_path`).
struct MapPlayback {
    /// Current playback time in ms, relative to fight start.
    time_ms: u64,
    /// Whether playback is currently advancing.
    playing: bool,
    /// Playback speed multiplier (1.0 = realtime).
    speed: f32,
    /// Identity of the fight this state is for. None = no fight rendered yet.
    fight_key: Option<PathBuf>,
}

impl MapPlayback {
    fn new() -> Self {
        Self { time_ms: 0, playing: false, speed: 1.0, fight_key: None }
    }
}

static PLAYBACK: Lazy<Mutex<MapPlayback>> = Lazy::new(|| Mutex::new(MapPlayback::new()));
```

- [ ] **Step 2: Add a `sync_fight_key` helper**

Add inside the same file (after the `lerp_position` function):

```rust
/// Reset playback to t=0, paused, when the rendered fight changes.
/// Returns the (possibly updated) (time_ms, playing, speed) tuple.
fn sync_fight_key(current: &PathBuf) -> (u64, bool, f32) {
    let mut guard = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    if guard.fight_key.as_ref() != Some(current) {
        guard.fight_key = Some(current.clone());
        guard.time_ms = 0;
        guard.playing = false;
        // Keep speed across fights — users typically pick once.
    }
    (guard.time_ms, guard.playing, guard.speed)
}
```

- [ ] **Step 3: Call `sync_fight_key` inside `render_content`**

The `render_content` function currently receives `json: &EiJson` but not the fight's log path. We need to either pass the path in or pull it from a different source. The simplest fix: update `render_content`'s signature to also take `&FightRecord` (which has `log_path`).

Check the caller in `src/ui/main.rs:89`:
```rust
TopTab::Map => crate::ui::map::render_content(ui, json, idx, derived),
```
`json` is `&record.data` (from `let json = &record.data;` at line 75). We have access to `record` in scope. Change the call site to pass the record path, and update the signature accordingly. Steps:

1. Change `render_content`'s signature in `src/ui/map.rs` from:
   ```rust
   pub fn render_content(ui: &Ui, json: &EiJson, idx: usize, _derived: &Derived) {
   ```
   to:
   ```rust
   pub fn render_content(ui: &Ui, json: &EiJson, idx: usize, _derived: &Derived, log_path: &std::path::PathBuf) {
   ```

2. Update the call site in `src/ui/main.rs:89`:
   ```rust
   TopTab::Map      => crate::ui::map::render_content(ui, json, idx, derived, &record.log_path),
   ```

3. Inside `render_content`, immediately after `tile_cache::drain_pending();`, add:
   ```rust
   let (_time_ms, _playing, _speed) = sync_fight_key(log_path);
   ```
   (The leading underscores keep the compiler quiet — Task 3 will use them.)

- [ ] **Step 4: Type-check**

Run: `cargo dll-check`
Expected: clean. Common gotcha — if `FightRecord` is not directly visible from `src/ui/map.rs`, you don't need to import it; we just take `&PathBuf` directly.

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs src/ui/main.rs
git commit -m "$(cat <<'EOF'
feat(map): add MapPlayback state with per-fight reset

Single global Mutex<MapPlayback> tracks current playback time, play
state, and speed. sync_fight_key() resets time/play whenever the
rendered fight's log_path changes — switching fights via the picker
won't bleed timestamps across encounters. State is wired but unused
until controls land.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Frame-tick auto-advance + playback controls UI

**Files:**
- Modify: `src/ui/map.rs`

Render a control row at the bottom of the map's child window: play/pause button, speed-cycle button (0.5×/1×/1.5×/2×), `M:SS / M:SS` time label, and a draggable scrubber that spans the rest of the row. When `playing` is true, advance `time_ms` by `delta_seconds * speed * 1000` each frame (capped at `duration_ms`; auto-pause at end).

- [ ] **Step 1: Add a `tick_playback` helper to `src/ui/map.rs`**

After `sync_fight_key`:

```rust
/// Advance `time_ms` by the current frame delta while `playing` is true.
/// Auto-pauses at duration_ms. Returns the current time_ms after the tick.
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
```

- [ ] **Step 2: Add a `render_controls` helper**

Below `tick_playback`:

```rust
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
        // If we're at the end and the user clicks play, rewind to start.
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

    // Scrubber. Fill remaining width.
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
        g.playing = false; // dragging the slider pauses playback
    }
}

fn mmss(ms: u64) -> String {
    let s = ms / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}
```

- [ ] **Step 3: Call `tick_playback` + `render_controls` inside the child window**

In `render_content`, replace the existing `let (_time_ms, _playing, _speed) = sync_fight_key(log_path);` (added in Task 2) with:

```rust
let _ = sync_fight_key(log_path);
let duration_ms = json.duration_ms;
let time_ms = tick_playback(ui, duration_ms);
```

Then inside the `child_window` closure, AFTER all the existing draw calls but BEFORE the closure ends, append:

```rust
// Park imgui's cursor at the bottom of the child window so the
// controls row sits beneath the map graphics.
let inner = ui.content_region_avail();
let row_h = ui.frame_height_with_spacing();
if inner[1] > row_h {
    ui.dummy([0.0, inner[1] - row_h]);
}
render_controls(ui, duration_ms);
```

The full child-window body now looks like:

```rust
ui.child_window("axipulse-map-canvas")
    .size([avail[0], avail[1]])
    .build(|| {
        let inner = ui.content_region_avail();
        let scale = (inner[0] / mw).min(inner[1] / mh).max(0.01);
        // ... existing render code (background, tiles, landmarks, players) ...

        // Push the controls to the bottom of the child window.
        let remaining = ui.content_region_avail();
        let row_h = ui.frame_height_with_spacing();
        if remaining[1] > row_h {
            ui.dummy([0.0, remaining[1] - row_h]);
        }
        render_controls(ui, duration_ms);
    });
```

> **Engineer note:** the existing draw code uses `inner` (snapshot at entry). The new "remaining" snapshot is taken AFTER the draw_list calls, but since draw_list doesn't advance the cursor, `remaining` will equal `inner`. That's fine — the dummy then jumps the cursor to the bottom and `render_controls` lands there. If you find `frame_height_with_spacing()` isn't on this imgui binding, use the literal `28.0_f32`.

- [ ] **Step 4: Build + deploy**

```bash
cargo dll-check 2>&1 | tail -5
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

All clean. Trigger a sim log (see `~/.claude/projects/.../memory/project_trigger_test_log.md`) and verify in GW2:
- Map tab now shows Play / 1.0x / "0:00 / M:SS" / [slider] controls under the map.
- Clicking Play makes the slider advance.
- The slider auto-pauses at the end.
- Dragging the slider pauses playback and moves the time indicator.
- The 1.0x button cycles through 1.5 → 2.0 → 0.5 → 1.0.
- Switching to a different fight via the fight picker resets time to 0:00.

**Important:** player markers are still rendered at the FINAL frame (we didn't touch the player render yet). The scrubber moves but the dots don't yet — that lands in Task 4. The visible verification right now is the controls themselves, not player movement.

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): playback controls (play/pause, speed, scrubber)

Adds a control row pinned to the bottom of the Map tab's child window:
play/pause button, 0.5/1/1.5/2x speed cycle, M:SS / M:SS time label,
and a draggable slider. `playing` advances time_ms by frame-delta *
speed each frame and auto-pauses at duration_ms. Dragging the slider
pauses playback. Player markers still snap to final-frame — the next
commit indexes them by time_ms.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Time-indexed player render

**Files:**
- Modify: `src/ui/map.rs`

Replace `collect_final_positions` (which reads `positions.last()`) with `collect_positions_at_time` that uses `lerp_position` to compute each player's location at the current `time_ms`. The render loop is otherwise unchanged.

- [ ] **Step 1: Replace `collect_final_positions`**

Delete the existing function. Add this in its place (immediately above `render_content`):

```rust
struct PlayerDot<'a> {
    #[allow(dead_code)]
    name: &'a str,
    profession: &'a str,
    x: f32,
    y: f32,
    is_self: bool,
}

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
        out.push(PlayerDot {
            name: p.name.as_str(),
            profession: p.profession.as_str(),
            x: x as f32,
            y: y as f32,
            is_self: i == self_idx,
        });
    }
    out
}
```

- [ ] **Step 2: Update the call site inside the child-window closure**

In `render_content`'s child-window body, change:

```rust
let dots = collect_final_positions(json, idx);
```

to:

```rust
let dots = collect_positions_at_time(json, idx, time_ms);
```

The render loop below it is unchanged.

- [ ] **Step 3: Build + deploy + verify**

```bash
cargo dll-check 2>&1 | tail -3
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

Trigger a sim log. In GW2:
- Scrub the slider — player icons should follow positions.
- Click Play — icons should walk across the map at ~1x.
- Set 2x — icons walk twice as fast.
- At t=0:00 icons should be at fight-start positions, not the final-frame ones.

- [ ] **Step 4: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): time-indexed player positions via lerp_position

Replace `collect_final_positions` (positions.last()) with
collect_positions_at_time that uses lerp_position to compute each
player's location at the current playback time. Polling rate comes
from combat_replay_meta_data; falls back to 150 ms (EI's default
WvW sample rate) if missing.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Motion trails

**Files:**
- Modify: `src/ui/map.rs`

Mirror upstream's two-tier trail render: a faded "historical" path drawn as a dashed polyline back to the start of the fight, and a brighter "recent" trail of the last `TRAIL_LENGTH` samples. Trails are drawn BEFORE the player marker so the dot sits on top.

- [ ] **Step 1: Constants + helpers**

In `src/ui/map.rs`, near the existing constants:

```rust
const TRAIL_LENGTH_SAMPLES: usize = 15;
const TRAIL_COLOR_HISTORY: [f32; 4] = [0.86, 0.86, 0.92, 0.18];
const TRAIL_COLOR_RECENT_SELF:  [f32; 4] = [0.06, 0.72, 0.51, 0.65];
const TRAIL_COLOR_RECENT_PEER:  [f32; 4] = [0.86, 0.86, 0.92, 0.55];
```

- [ ] **Step 2: Extend `PlayerDot` with the index needed for trail slicing**

Update the struct and `collect_positions_at_time`:

```rust
struct PlayerDot<'a> {
    #[allow(dead_code)]
    name: &'a str,
    profession: &'a str,
    x: f32,
    y: f32,
    is_self: bool,
    /// Index of the most recent sample at or before time_ms. Used to
    /// slice the positions vec into history vs recent trail.
    sample_idx: usize,
    /// The full positions vec, borrowed for the duration of this frame.
    positions: &'a [Vec<f64>],
}
```

Inside `collect_positions_at_time`, compute `sample_idx` and pass `positions`:

```rust
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
```

- [ ] **Step 3: Draw trails BEFORE the player markers**

In `render_content`'s child-window body, find the player-render loop:

```rust
let dots = collect_positions_at_time(json, idx, time_ms);
for dot in &dots {
    // existing marker render
}
```

Insert a trail-render pass BEFORE the marker loop:

```rust
let dots = collect_positions_at_time(json, idx, time_ms);

// Trails (drawn first so markers sit on top).
for dot in &dots {
    let recent_start = dot.sample_idx.saturating_sub(TRAIL_LENGTH_SAMPLES);
    // Historical: faded dots every other sample to keep draw count down.
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
    // Recent: solid bright line of last TRAIL_LENGTH_SAMPLES segments.
    let recent_end = (dot.sample_idx + 1).min(dot.positions.len());
    if recent_end > recent_start + 1 {
        let recent_slice = &dot.positions[recent_start..recent_end];
        let color = if dot.is_self { TRAIL_COLOR_RECENT_SELF } else { TRAIL_COLOR_RECENT_PEER };
        let mut prev: Option<[f32; 2]> = None;
        for sample in recent_slice {
            if sample.len() < 2 { continue; }
            let p = [ox + (sample[0] as f32) * scale, oy + (sample[1] as f32) * scale];
            if let Some(q) = prev {
                draw.add_line(q, p, color).thickness(if dot.is_self { 2.0 } else { 1.5 }).build();
            }
            prev = Some(p);
        }
    }
}

// Player markers.
for dot in &dots {
    // existing marker render (unchanged)
}
```

- [ ] **Step 4: Build + deploy + verify**

```bash
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

Trigger a sim log. In GW2:
- Scrub forward — you should see a bright trail behind each moving player and a faint dotted trail for the older path.
- Your own trail is greenish; teammates' is pale white.
- At t=0:00 there's no trail (nothing happened yet).

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): motion trails behind each player marker

Two-tier trail render mirroring axipulse upstream's MovementView:
a faded historical path (every other sample, single-pixel line) and
a brighter recent trail of the last 15 samples. Self gets a green
recent trail; peers a pale white one. Drawn before markers so dots
sit on top.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: README note

**Files:**
- Modify: `README.md`

Append two lines to the existing "WvW Combat Replay (Map tab)" section noting that playback is now live.

- [ ] **Step 1: Read current README**

Run: `grep -n "WvW Combat Replay" README.md`

- [ ] **Step 2: Edit the existing section**

Find the bullet list under the WvW Combat Replay heading. After "Each squad member's final position with profession icon.", append a new bullet:

```markdown
- Time playback: scrubber, play/pause, speed (0.5×–2×), motion trails.
```

And update the trailing paragraph that says "Time playback ... ship in follow-up plans." — drop "Time playback, " so it reads "pan/zoom, and state overlays (down/dead, boons, skill casts) ship in follow-up plans."

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "$(cat <<'EOF'
docs: note playback controls + trails on Map tab

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Self-review notes

- [ ] All file paths absolute or repo-root-relative ✓
- [ ] Every verify step has a concrete command + expectation ✓
- [ ] No TODO / "fill in" placeholders ✓
- [ ] Type consistency: `MapPlayback` field names (`time_ms`, `playing`, `speed`, `fight_key`) used identically in Tasks 2, 3, 4 ✓
- [ ] `lerp_position` signature `(samples: &[Vec<f64>], t_ms: u64, polling_rate_ms: u64) -> Option<(f64, f64)>` is consistent between Task 1 and Task 4's `collect_positions_at_time` call ✓
- [ ] `render_content` signature change (adds `log_path: &PathBuf`) propagates to call site in `src/ui/main.rs:89` (Task 2 Step 3) ✓
- [ ] `PlayerDot` field additions in Task 5 are additive (sample_idx, positions) — doesn't break Task 4's render loop because the marker loop only reads the existing fields ✓
- [ ] Trail rendering uses `dot.positions` and `dot.sample_idx` which exist in `PlayerDot` after Task 5 ✓
- [ ] The Task 3 controls task uses `frame_height_with_spacing()`; engineer note offers the literal `28.0` fallback ✓
