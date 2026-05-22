# WvW Map State Overlays Implementation Plan (Phase 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show fight state on the WvW Map tab — replace alive markers with skull (dead) / blue-pin (down) when applicable, and add a sliding side panel that lists the local player's party with profession, name, HP bar, distance-to-commander, boon stacks, and a fading list of recent skill casts.

**Architecture:** All state lookups are pure functions that index into existing EI structs by `time_ms`. EI's combat-replay data needs two new fields (`dead`, `down` ranges) and each player gets a new `rotation` field. Boon and skill icons are loaded via the existing `crate::ui::icons::lookup` infrastructure (URL → D3D11 texture cache from skill_map/buff_map). The side panel is a fixed-width inline column rendered alongside the map; toggled via a button on the controls row.

**Tech Stack:** Rust (existing), arcdps imgui, no new deps.

---

## Data Model Notes

**Real EI emits (per player):**
- `combatReplayData.dead`: `Vec<[start_ms, end_ms]>` time ranges when player is dead.
- `combatReplayData.down`: `Vec<[start_ms, end_ms]>` time ranges when player is downed.
- `rotation`: `Vec<RotationEntry>` where each entry is `{ id: skill_id, skills: [{ castTime: i64, duration: u32, ... }, ...] }`. `castTime` is in ms; can be negative for pre-fight casts (we filter those out).

The fixture at `fixtures/sample-fight.json` is positions-less and replay-less, so we test deserialization with inline JSON snippets in the test and rely on live in-game verification for visuals (same approach as Phase 2).

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/ei_model.rs` | Modify | Extend `ReplayData` with `dead`/`down`; add `RotationEntry` + `SkillCast` types; add `rotation` field to `EiPlayer`. |
| `src/ui/map.rs` | Modify | Add `MemberStatus` enum + lookup helpers (status_at, health_at, boon_stacks_at, recent_skills_at). On-map status markers. Side panel toggle + render. |
| `src/map/boon_panel.rs` | Create | Static `PANEL_BOON_ORDER` array + tiny `boon_name(id)` helper for tooltip text. Mirrors upstream's PANEL_BOON_ORDER. |
| `tests/wvw_map_status_test.rs` | Create | TDD member-status, health-at, boon-stacks-at, recent-skills-at. |
| `tests/wvw_map_replay_parse_test.rs` | Create | Verify ReplayData and rotation deserialize as expected from inline JSON. |
| `README.md` | Modify | Add a bullet noting state overlays + side panel. |

---

## Task 1: Extend EI deserialization (dead, down, rotation)

**Files:**
- Modify: `src/ei_model.rs`
- Create: `tests/wvw_map_replay_parse_test.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/wvw_map_replay_parse_test.rs`:

```rust
use arcdps_axipulse::ei_model::EiJson;

const REPLAY_JSON: &str = r#"{
    "fightName": "Detailed WvW - Blue Alpine Borderlands",
    "durationMs": 60000,
    "success": false,
    "players": [{
        "name": "TestPlayer",
        "account": ":TestAcc",
        "profession": "Firebrand",
        "group": 1,
        "weapons": [],
        "weaponSets": [],
        "combatReplayData": {
            "positions": [[10.0, 20.0], [11.0, 21.0]],
            "dead": [[40000.0, 60000.0]],
            "down": [[30000.0, 40000.0]]
        },
        "rotation": [
            {
                "id": 12345,
                "skills": [
                    { "castTime": 1000, "duration": 500 },
                    { "castTime": 5000, "duration": 800 }
                ]
            }
        ]
    }],
    "targets": [],
    "skillMap": {},
    "buffMap": {}
}"#;

#[test]
fn parses_dead_down_and_rotation() {
    let j: EiJson = serde_json::from_str(REPLAY_JSON).expect("EI JSON parses");
    let p = &j.players[0];
    let rd = p.combat_replay_data.as_ref().expect("replay data present");
    assert_eq!(rd.dead, vec![vec![40000.0, 60000.0]]);
    assert_eq!(rd.down, vec![vec![30000.0, 40000.0]]);
    assert_eq!(p.rotation.len(), 1);
    assert_eq!(p.rotation[0].id, 12345);
    assert_eq!(p.rotation[0].skills.len(), 2);
    assert_eq!(p.rotation[0].skills[0].cast_time, 1000);
    assert_eq!(p.rotation[0].skills[0].duration, 500);
}

#[test]
fn replay_data_dead_down_default_empty_when_absent() {
    const NO_RANGES: &str = r#"{
        "fightName":"X","durationMs":1000,"success":false,
        "players":[{
            "name":"P","account":":A","profession":"X","group":1,
            "weapons":[],"weaponSets":[],
            "combatReplayData":{"positions":[[0,0]]}
        }],
        "targets":[],"skillMap":{},"buffMap":{}
    }"#;
    let j: EiJson = serde_json::from_str(NO_RANGES).unwrap();
    let rd = j.players[0].combat_replay_data.as_ref().unwrap();
    assert!(rd.dead.is_empty());
    assert!(rd.down.is_empty());
    assert!(j.players[0].rotation.is_empty());
}
```

- [ ] **Step 2: Run test, confirm FAIL**

Run: `cargo test --test wvw_map_replay_parse_test 2>&1 | tail -10`
Expected: build error — `ReplayData` has no field `dead`/`down`, `EiPlayer` has no field `rotation`.

- [ ] **Step 3: Extend `ReplayData` in `src/ei_model.rs`**

Find `pub struct ReplayData` (around line 311). Replace with:

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayData {
    #[serde(default)]
    pub positions: Vec<Vec<f64>>,
    #[serde(default)]
    pub start: Option<i64>,
    /// Time ranges (`[[start_ms, end_ms], ...]`) when the player is dead.
    #[serde(default)]
    pub dead: Vec<Vec<f64>>,
    /// Time ranges (`[[start_ms, end_ms], ...]`) when the player is downed.
    #[serde(default)]
    pub down: Vec<Vec<f64>>,
}
```

- [ ] **Step 4: Add rotation types**

Below `ReplayData` in `src/ei_model.rs`, add:

```rust
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCast {
    /// Cast start time in ms relative to fight start. Can be negative
    /// for pre-fight casts; filter those out for "recent skills" views.
    #[serde(default)]
    pub cast_time: i64,
    /// Cast duration in ms (informational; not used for "recent" logic).
    #[serde(default)]
    pub duration: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationEntry {
    /// Skill id — key into the top-level `skill_map` for icon + name.
    pub id: i64,
    #[serde(default)]
    pub skills: Vec<SkillCast>,
}
```

- [ ] **Step 5: Add `rotation` to `EiPlayer`**

Find `pub struct EiPlayer` (around line 61). Locate a sensible spot for a new field — after `combat_replay_data` works. Add:

```rust
    /// Skill cast history, grouped by skill id. `rotation[i].skills[j]`
    /// has `cast_time` in ms relative to fight start.
    #[serde(default)]
    pub rotation: Vec<RotationEntry>,
```

- [ ] **Step 6: Run tests, confirm PASS**

Run: `cargo test --test wvw_map_replay_parse_test 2>&1 | tail -10`
Expected: 2 passed.

Also run: `cargo test 2>&1 | grep "test result"` — all existing tests must still pass (we only added optional fields).

- [ ] **Step 7: Commit**

```bash
git add src/ei_model.rs tests/wvw_map_replay_parse_test.rs
git commit -m "$(cat <<'EOF'
feat(ei_model): deserialize dead/down ranges + per-player skill rotation

Adds `dead`/`down` Vec<Vec<f64>> ranges on ReplayData and a `rotation`
Vec<RotationEntry> on EiPlayer where each entry groups SkillCast
records by skill id. All fields default-empty so existing fixtures
keep parsing unchanged.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: TDD `status_at` + `health_at`

**Files:**
- Modify: `src/ui/map.rs`
- Create: `tests/wvw_map_status_test.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/wvw_map_status_test.rs`:

```rust
use arcdps_axipulse::ui::map::{status_at, health_at, MemberStatus};

fn range(start: f64, end: f64) -> Vec<f64> { vec![start, end] }

#[test]
fn status_alive_with_no_ranges() {
    let dead: Vec<Vec<f64>> = vec![];
    let down: Vec<Vec<f64>> = vec![];
    assert_eq!(status_at(&dead, &down, 5000), MemberStatus::Alive);
}

#[test]
fn status_down_when_t_in_down_range() {
    let dead: Vec<Vec<f64>> = vec![];
    let down = vec![range(2000.0, 4000.0)];
    assert_eq!(status_at(&dead, &down, 3000), MemberStatus::Down);
}

#[test]
fn status_dead_overrides_down() {
    let dead = vec![range(2000.0, 8000.0)];
    let down = vec![range(2000.0, 4000.0)];
    assert_eq!(status_at(&dead, &down, 3000), MemberStatus::Dead);
}

#[test]
fn status_alive_outside_ranges() {
    let dead = vec![range(2000.0, 4000.0)];
    let down: Vec<Vec<f64>> = vec![];
    assert_eq!(status_at(&dead, &down, 5000), MemberStatus::Alive);
}

#[test]
fn status_inclusive_boundaries() {
    let dead = vec![range(1000.0, 2000.0)];
    let down: Vec<Vec<f64>> = vec![];
    assert_eq!(status_at(&dead, &down, 1000), MemberStatus::Dead);
    assert_eq!(status_at(&dead, &down, 2000), MemberStatus::Dead);
}

#[test]
fn health_at_empty_returns_100() {
    let samples: Vec<Vec<f64>> = vec![];
    assert_eq!(health_at(&samples, 0), 100.0);
}

#[test]
fn health_at_picks_last_sample_at_or_before_t() {
    let samples = vec![vec![0.0, 100.0], vec![1000.0, 80.0], vec![2000.0, 50.0]];
    assert_eq!(health_at(&samples, 500), 100.0);
    assert_eq!(health_at(&samples, 1500), 80.0);
    assert_eq!(health_at(&samples, 5000), 50.0);
}

#[test]
fn health_at_returns_first_when_before_first_sample() {
    let samples = vec![vec![1000.0, 80.0]];
    assert_eq!(health_at(&samples, 0), 80.0);
}
```

- [ ] **Step 2: Run, confirm FAIL**

Run: `cargo test --test wvw_map_status_test 2>&1 | tail -10`
Expected: build error — `status_at` / `health_at` / `MemberStatus` not found.

- [ ] **Step 3: Implement in `src/ui/map.rs`**

Add near the top of `src/ui/map.rs` (after `lerp_position` is fine):

```rust
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
```

- [ ] **Step 4: Run, confirm PASS**

Run: `cargo test --test wvw_map_status_test 2>&1 | tail -10`
Expected: 8 passed.

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs tests/wvw_map_status_test.rs
git commit -m "$(cat <<'EOF'
feat(map): add status_at + health_at helpers

Pure functions that index dead/down ranges and health_percents
samples to answer "what state was this player in at time t". Dead
overrides Down; health_at picks the most recent sample at or before t.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: TDD `boon_stacks_at` + `recent_skill_casts`

**Files:**
- Modify: `src/ui/map.rs`
- Modify: `tests/wvw_map_status_test.rs` (append tests)

- [ ] **Step 1: Append failing tests**

Append to `tests/wvw_map_status_test.rs`:

```rust
use arcdps_axipulse::ui::map::{boon_stacks_at, recent_skill_casts};
use arcdps_axipulse::ei_model::{RotationEntry, SkillCast};

#[test]
fn boon_stacks_picks_last_state_at_or_before_t() {
    // states: [[time_ms, stacks], ...]
    let states = vec![vec![0.0, 0.0], vec![1000.0, 3.0], vec![2000.0, 5.0]];
    assert_eq!(boon_stacks_at(&states, 500), 0);
    assert_eq!(boon_stacks_at(&states, 1500), 3);
    assert_eq!(boon_stacks_at(&states, 5000), 5);
}

#[test]
fn boon_stacks_empty_returns_zero() {
    let states: Vec<Vec<f64>> = vec![];
    assert_eq!(boon_stacks_at(&states, 1000), 0);
}

fn cast(t: i64, dur: u32) -> SkillCast {
    SkillCast { cast_time: t, duration: dur }
}

fn rot(id: i64, skills: Vec<SkillCast>) -> RotationEntry {
    RotationEntry { id, skills }
}

#[test]
fn recent_casts_returns_empty_when_no_rotation() {
    let rotation: Vec<RotationEntry> = vec![];
    let out = recent_skill_casts(&rotation, 5000, 4);
    assert!(out.is_empty());
}

#[test]
fn recent_casts_returns_casts_before_t_in_descending_order() {
    let rotation = vec![rot(101, vec![cast(1000, 500), cast(3000, 500), cast(8000, 500)])];
    // At t=4000, casts at 1000 and 3000 are visible. Newest first.
    let out = recent_skill_casts(&rotation, 4000, 4);
    assert_eq!(out, vec![(101, 3000), (101, 1000)]);
}

#[test]
fn recent_casts_ignores_negative_cast_time() {
    let rotation = vec![rot(101, vec![cast(-500, 200), cast(1000, 500)])];
    let out = recent_skill_casts(&rotation, 4000, 4);
    assert_eq!(out, vec![(101, 1000)]);
}

#[test]
fn recent_casts_caps_at_max_results() {
    let mut casts = Vec::new();
    for t in (1000..10000).step_by(1000) { casts.push(cast(t, 100)); }
    let rotation = vec![rot(7, casts)];
    let out = recent_skill_casts(&rotation, 20000, 3);
    assert_eq!(out.len(), 3);
    // Newest three: 9000, 8000, 7000
    assert_eq!(out, vec![(7, 9000), (7, 8000), (7, 7000)]);
}

#[test]
fn recent_casts_merges_multiple_skill_ids_in_time_order() {
    let rotation = vec![
        rot(101, vec![cast(2000, 500)]),
        rot(202, vec![cast(3000, 500)]),
        rot(303, vec![cast(1000, 500)]),
    ];
    let out = recent_skill_casts(&rotation, 5000, 4);
    assert_eq!(out, vec![(202, 3000), (101, 2000), (303, 1000)]);
}
```

- [ ] **Step 2: Run, confirm FAIL**

Run: `cargo test --test wvw_map_status_test 2>&1 | tail -10`
Expected: build error — `boon_stacks_at` / `recent_skill_casts` not found.

- [ ] **Step 3: Implement in `src/ui/map.rs`**

Add (after `health_at`):

```rust
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
    all.sort_by(|a, b| b.1.cmp(&a.1)); // newest first
    all.truncate(max_results);
    all
}
```

- [ ] **Step 4: Run, confirm PASS**

Run: `cargo test --test wvw_map_status_test 2>&1 | tail -15`
Expected: 14 passed (8 from Task 2 + 6 new).

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs tests/wvw_map_status_test.rs
git commit -m "$(cat <<'EOF'
feat(map): add boon_stacks_at + recent_skill_casts helpers

Both pure functions. boon_stacks_at indexes a buff state timeline
(`[[t_ms, stacks], ...]`) at a given time. recent_skill_casts walks
the rotation tree, drops pre-fight (negative) casts, sorts by time
descending, and returns the top N.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: On-map status markers (skull / down-pin)

**Files:**
- Modify: `src/ui/map.rs`

- [ ] **Step 1: Extend `PlayerDot` with status + health**

Find the `PlayerDot` struct in `src/ui/map.rs`. Replace with:

```rust
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
    sample_idx: usize,
    positions: &'a [Vec<f64>],
    player_index: usize,
}
```

- [ ] **Step 2: Populate the new fields in `collect_positions_at_time`**

Replace the function body with:

```rust
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
        group: p.group,
        status: status_at(&rd.dead, &rd.down, time_ms),
        health_pct: health_at(&p.health_percents, time_ms),
        sample_idx,
        positions: &rd.positions,
        player_index: i,
    });
}
out
```

> **Engineer note:** Confirm via `grep -n "pub has_commander_tag\|pub account\|pub group\b" src/ei_model.rs` that those exact field names exist on `EiPlayer`. If `has_commander_tag` is missing, look for `has_commander` or `hasCommanderTag` and adapt. If `group` is missing or non-i32, adapt the type accordingly.

- [ ] **Step 3: Replace the on-map marker draw**

Find the player-marker loop (after the trail loop) in `render_content`. The current code branches on `lookup_bundled(dot.profession)`. Replace the entire inner of the loop with:

```rust
let cx = ox + dot.x * scale;
let cy = oy + dot.y * scale;
let sz_alive = if dot.is_self { 18.0 } else { 14.0 };

match dot.status {
    MemberStatus::Dead => {
        let r = 7.0;
        draw.add_circle([cx, cy], r, [0.93, 0.27, 0.27, 0.95]).filled(true).build();
        draw.add_circle([cx, cy], r, [0.55, 0.10, 0.10, 1.0]).thickness(1.5).build();
        // X mark inside the dot.
        let h = r * 0.55;
        draw.add_line([cx - h, cy - h], [cx + h, cy + h], [1.0, 1.0, 1.0, 0.95]).thickness(1.8).build();
        draw.add_line([cx + h, cy - h], [cx - h, cy + h], [1.0, 1.0, 1.0, 0.95]).thickness(1.8).build();
    }
    MemberStatus::Down => {
        let r = 6.5;
        draw.add_circle([cx, cy], r, [0.23, 0.51, 0.96, 0.85]).filled(true).build();
        draw.add_circle([cx, cy], r, [0.10, 0.30, 0.70, 1.0]).thickness(1.5).build();
        // Downward triangle inside.
        let h = r * 0.55;
        draw.add_triangle([cx - h, cy - h * 0.6], [cx + h, cy - h * 0.6], [cx, cy + h * 0.7], [1.0, 1.0, 1.0, 0.95])
            .filled(true).build();
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
```

> **Engineer note:** If `add_triangle` isn't on this binding, use three `add_line` calls forming a triangle, or substitute `add_circle` with the same radius.

- [ ] **Step 4: Build + deploy**

```bash
cargo dll-check 2>&1 | tail -5
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

Trigger a sim log. In GW2 → Map tab → scrub the playback slider into a window where someone was downed/dead in the source fight. Their marker should swap to a blue triangle (down) or red X-circle (dead).

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): swap player marker for skull/down-pin when downed or dead

Reads dead/down ranges via status_at(time_ms) for each player. Dead
shows a red circle with a white X; Down shows a blue circle with a
white downward triangle. Alive keeps the existing profession icon
with a green ring for self and an orange ring for the commander.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Side panel scaffold (toggle + party listing without boons/skills)

**Files:**
- Modify: `src/ui/map.rs`

The panel is a fixed-width column on the LEFT of the map area. When toggled on, the map's render rect shrinks by the panel width.

- [ ] **Step 1: Add panel state to `MapPlayback`**

Find `struct MapPlayback` (added in P2.T2). Extend:

```rust
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
```

- [ ] **Step 2: Add a "Party" toggle button to the controls row**

In `render_controls`, replace the current snapshot block with:

```rust
let (cur_time, playing, speed, panel_open) = {
    let g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    (g.time_ms, g.playing, g.speed, g.show_party_panel)
};
```

After the speed cycle button and BEFORE the time label, insert a Party toggle:

```rust
let party_label = if panel_open { "Party*" } else { "Party " };
if ui.button(party_label) {
    let mut g = PLAYBACK.lock().expect("PLAYBACK mutex poisoned");
    g.show_party_panel = !g.show_party_panel;
}
ui.same_line();
```

(The asterisk gives a visible "active" hint without needing colored buttons.)

- [ ] **Step 3: Add panel render in the child window**

Inside the child-window closure in `render_content`, the current code computes `let inner = ui.content_region_avail();` then `let scale = ...`. We need to reserve panel width BEFORE computing scale.

Add a `panel_open` lookup near the top of the closure:

```rust
let panel_open = PLAYBACK.lock().ok().map(|g| g.show_party_panel).unwrap_or(false);
let panel_w: f32 = if panel_open { 260.0 } else { 0.0 };
```

Change the `let inner = ui.content_region_avail();` line and the scale calc to:

```rust
let inner = ui.content_region_avail();
let map_avail_w = (inner[0] - panel_w).max(10.0);
let scale = (map_avail_w / mw).min(inner[1] / mh).max(0.01);
```

Change the ox computation so the map is centered in the REMAINING space (after the panel):

```rust
let origin = ui.cursor_screen_pos();
let ox = origin[0] + panel_w + (map_avail_w - render_w) * 0.5;
let oy = origin[1] + (inner[1] - render_h) * 0.5;
```

Then BEFORE all the draw_list calls, render the panel:

```rust
if panel_open {
    render_party_panel(ui, json, idx, time_ms, [origin[0], origin[1]], [panel_w, inner[1]]);
}
```

- [ ] **Step 4: Add `render_party_panel`**

Below `render_controls`, add:

```rust
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

    // Panel background.
    let bg = [0.08, 0.10, 0.13, 0.92];
    draw.add_rect(
        panel_origin,
        [panel_origin[0] + panel_size[0], panel_origin[1] + panel_size[1]],
        bg,
    ).filled(true).rounding(6.0).build();

    // Local player's group.
    let local_group = json.players.get(self_idx).map(|p| p.group).unwrap_or(-1);
    let commander_pos: Option<(f64, f64)> = find_commander_position(json, time_ms);
    let inch_to_pixel = json
        .combat_replay_meta_data
        .as_ref()
        .and_then(|m| m.inch_to_pixel)
        .unwrap_or(1.0);

    // Header.
    let pad = 10.0_f32;
    let mut y = panel_origin[1] + pad;
    draw.add_text(
        [panel_origin[0] + pad, y],
        [0.55, 0.58, 0.65, 1.0],
        "PARTY",
    );
    y += 18.0;

    let row_h = 56.0_f32;
    let polling_rate = json
        .combat_replay_meta_data
        .as_ref()
        .and_then(|m| m.polling_rate)
        .unwrap_or(150);
    for (i, p) in json.players.iter().enumerate() {
        if p.group != local_group { continue; }
        if p.not_in_squad { continue; }

        // Resolve current position for distance calc.
        let rd_pos = p.combat_replay_data.as_ref()
            .and_then(|rd| lerp_position(&rd.positions, time_ms, polling_rate));
        let status = p.combat_replay_data.as_ref()
            .map(|rd| status_at(&rd.dead, &rd.down, time_ms))
            .unwrap_or(MemberStatus::Alive);
        let hp = health_at(&p.health_percents, time_ms);

        // Row background.
        let row_y0 = y;
        let row_y1 = y + row_h;
        draw.add_rect(
            [panel_origin[0] + 4.0, row_y0],
            [panel_origin[0] + panel_size[0] - 4.0, row_y1],
            [1.0, 1.0, 1.0, 0.04],
        ).filled(true).rounding(4.0).build();

        // Profession icon.
        let icon_size = 20.0;
        let icon_x = panel_origin[0] + pad;
        let icon_y = row_y0 + 6.0;
        if let Some(icon) = crate::ui::icons::lookup_bundled(p.profession.as_str()) {
            draw.add_image(
                icon.tex,
                [icon_x, icon_y],
                [icon_x + icon_size, icon_y + icon_size],
            ).build();
        }

        // Name.
        let name_x = icon_x + icon_size + 8.0;
        let name_color = if i == self_idx { [0.06, 0.72, 0.51, 1.0] }
            else if p.has_commander_tag { [0.96, 0.62, 0.04, 1.0] }
            else { [0.97, 0.97, 1.00, 1.0] };
        draw.add_text([name_x, icon_y + 2.0], name_color, p.name.as_str());

        // Distance to commander (if not commander themselves).
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

        // HP bar.
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
```

> **Engineer note:** `p.not_in_squad` — confirm this field name with `grep -n "not_in_squad\|notInSquad" src/ei_model.rs`. If absent, drop that filter (no-op). Same for `has_commander_tag` (confirm in Task 4).

- [ ] **Step 5: Build + deploy**

```bash
cargo dll-check 2>&1 | tail -5
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

Trigger a sim log. In GW2 → Map tab → click "Party". Panel should slide in on the left with party members (your group only), each row showing icon + name + HP bar + status / distance.

- [ ] **Step 6: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): party side panel (HP bars + distance + status badges)

Toggle via "Party" button on the controls row. Lists local player's
group only, with profession icon, name (coloured by role), HP bar
that colour-shifts on status (green/yellow/red alive, blue downed,
dark red dead), and distance-to-commander in inches (green/orange/red
zones at 300/600). Boon icons and recent skills land next.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Side panel — boon stack icons

**Files:**
- Modify: `src/ui/map.rs`
- Create: `src/map/boon_panel.rs`
- Modify: `src/map/mod.rs`

- [ ] **Step 1: Create the boon order constants**

Create `src/map/boon_panel.rs`:

```rust
//! Boon ordering for the WvW Map party side panel. Ported from
//! axipulse upstream MovementView.tsx (PANEL_BOON_ORDER).
//!
//! Order is intentional: defensive boons first (Aegis, Prot), then
//! damage/quickness/alacrity, then utility. Limited list keeps the
//! row visually scannable.

pub const PANEL_BOON_ORDER: &[i64] = &[
    740,   // Might
    725,   // Fury
    717,   // Protection
    718,   // Regeneration
    726,   // Vigor
    1122,  // Stability
    719,   // Swiftness
    743,   // Aegis
    873,   // Resolution
    1187,  // Quickness
    30328, // Alacrity
    26980, // Resistance
];

/// Short display name for the tooltip text on each stack tile.
pub fn boon_name(id: i64) -> &'static str {
    match id {
        740 => "Might",
        725 => "Fury",
        717 => "Protection",
        718 => "Regeneration",
        726 => "Vigor",
        1122 => "Stability",
        719 => "Swiftness",
        743 => "Aegis",
        873 => "Resolution",
        1187 => "Quickness",
        30328 => "Alacrity",
        26980 => "Resistance",
        _ => "Boon",
    }
}
```

- [ ] **Step 2: Wire module into `src/map/mod.rs`**

Add `pub mod boon_panel;` alongside the existing module decls.

- [ ] **Step 3: Render boon stacks in `render_party_panel`**

In `src/ui/map.rs`'s `render_party_panel`, increase `row_h` from `56.0` to `82.0` (more vertical room for boons + future skills). Then AFTER the HP label `draw.add_text(...)` (but BEFORE `y += row_h + 4.0;`), add:

```rust
        // Boon stack tiles.
        let icon_px = 18.0_f32;
        let gap = 2.0_f32;
        let mut bx = name_x;
        let by = bar_y0 + bar_h + 18.0;
        for boon_id in crate::map::boon_panel::PANEL_BOON_ORDER {
            // Locate the BuffEntry for this id.
            let stacks = p.buff_uptimes.iter()
                .find(|b| b.id == *boon_id)
                .map(|b| boon_stacks_at(&b.states, time_ms))
                .unwrap_or(0);
            if stacks == 0 { bx += 0.0; continue; }
            // Try to fetch the icon.
            let icon = crate::ui::icons::lookup(
                json,
                crate::ui::icons::IconKey { kind: crate::ui::icons::IconKind::Buff, id: *boon_id },
            );
            if let Some(handle) = icon {
                draw.add_image(handle.tex, [bx, by], [bx + icon_px, by + icon_px]).build();
            } else {
                draw.add_rect([bx, by], [bx + icon_px, by + icon_px], [1.0, 1.0, 1.0, 0.15])
                    .filled(true).rounding(3.0).build();
            }
            if stacks > 1 {
                draw.add_text(
                    [bx + icon_px - 8.0, by + icon_px - 10.0],
                    [0.97, 0.97, 1.0, 1.0],
                    format!("{stacks}"),
                );
            }
            bx += icon_px + gap;
            if bx + icon_px > panel_origin[0] + panel_size[0] - pad { break; }
        }
```

> **Engineer note:** Confirm via `grep -n "pub buff_uptimes" src/ei_model.rs` that the field is `Vec<BuffEntry>` and `BuffEntry { id, states }`. Also confirm via `grep -n "pub fn lookup\|pub enum IconKind\|pub struct IconKey" src/ui/icons.rs` that the lookup signature matches. If `IconKey` is private or shaped differently, look at how `src/ui/pulse.rs` calls into icons.rs for skill icons — match that idiom.

- [ ] **Step 4: Build + deploy + verify**

```bash
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

Trigger sim log. Party panel rows should now show small boon icons under the HP bar, with stack counts on stacking boons like Might.

- [ ] **Step 5: Commit**

```bash
git add src/ui/map.rs src/map/boon_panel.rs src/map/mod.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): boon stack icons in party panel

For each party member, draw a row of up to ~12 boon icons (defensive
first, then offensive, then utility — matching axipulse upstream's
PANEL_BOON_ORDER) at the current playback time. Stack counts overlaid
on stacking boons. Falls back to a translucent square when the icon
texture hasn't loaded yet.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Side panel — recent skill casts (with fade)

**Files:**
- Modify: `src/ui/map.rs`

- [ ] **Step 1: Render recent skills below boons**

Increase `row_h` from `82.0` to `108.0`. After the boon-stack loop (and the boon-loop's break), add:

```rust
        // Recent skill casts (last 4, newest first, latest shown larger).
        let skills = recent_skill_casts(&p.rotation, time_ms, 4);
        if !skills.is_empty() {
            let skill_px = 18.0_f32;
            let mut sx = name_x;
            let sy = by + 22.0; // below the boon row
            let latest_hold_ms: i64 = 1200;
            let latest_fade_ms: i64 = 2500;
            let fade_ms: i64 = 1500;
            let t = time_ms as i64;
            for (i, (id, cast_t)) in skills.iter().enumerate() {
                let age = t - cast_t;
                let opacity = if i == 0 {
                    // Latest: full alpha for hold window, then fade.
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
```

> **Engineer note:** `draw.add_image(...).col(...)` may not be available on this binding — if it errors, drop the `.col(...)` builder and just draw at full opacity. Mention the substitution in the report.

- [ ] **Step 2: Build + deploy + verify**

```bash
cargo dll 2>&1 | tail -3
./scripts/deploy.sh 2>&1 | tail -3
```

Trigger sim log. Scrub through the fight. Below each party row's boons you should see up to 4 small skill icons that fade in and out as the player casts.

- [ ] **Step 3: Commit**

```bash
git add src/ui/map.rs
git commit -m "$(cat <<'EOF'
feat(ui/map): recent skill casts in party panel

Up to 4 icons per row, newest first. Latest cast holds full alpha
for 1.2s then fades over 2.5s; older casts fade linearly over 1.5s.
Icons resolved through the existing skill_map URL cache (same path
the Pulse tab uses).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: README update

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the WvW Map section bullet list**

Add a bullet under the "WvW Combat Replay (Map tab)" section:

```markdown
- Per-player state overlays: skull (dead) / downed-pin markers on the map; sliding party panel with HP bars, distance-to-commander, boon stacks, and recent skill casts.
```

And remove "state overlays (down/dead, boons, skill casts) " from the trailing "ship in follow-up plans" paragraph, leaving just "Pan/zoom ships in a follow-up plan."

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "$(cat <<'EOF'
docs: note state overlays + party side panel on Map tab

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Self-review

- [ ] All paths repo-root-relative ✓
- [ ] Every verify step has a concrete command + expectation ✓
- [ ] No placeholders ✓
- [ ] Type consistency: `MemberStatus { Alive, Down, Dead }` used identically in Tasks 2, 4, 5 ✓
- [ ] Function signatures: `status_at(&[Vec<f64>], &[Vec<f64>], u64) -> MemberStatus`, `health_at(&[Vec<f64>], u64) -> f64`, `boon_stacks_at(&[Vec<f64>], u64) -> i32`, `recent_skill_casts(&[RotationEntry], u64, usize) -> Vec<(i64, i64)>` — consistent between definition (Tasks 2 & 3) and call sites (Tasks 4, 5, 6, 7) ✓
- [ ] `PlayerDot` field additions (account, group, is_commander, status, health_pct, player_index) propagate to one place only (the marker render in Task 4) — no other PlayerDot consumers exist ✓
- [ ] Boon icons rely on `crate::ui::icons::lookup` + `IconKey` + `IconKind::Buff` (Task 6) — engineer note prompts a `grep` confirmation, with fallback to drawing a translucent square if the texture isn't loaded yet ✓
- [ ] Skill icons use the same path with `IconKind::Skill` (Task 7) ✓
- [ ] `not_in_squad` and `has_commander_tag` field names are tagged with engineer-note grep instructions so a renamed field won't silently break the panel ✓
