# axi-design Overlay Conversion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redraw the arcdps-axipulse in-game overlay's five UI surfaces in the axi-design language, routing every colour through one chrome module and one domain module, so the plugin and the desktop AxiPulse app read as one product.

**Architecture:** Three new host-compilable modules — `src/ui/theme.rs` (chrome tokens, the eleven accents, form geometry), `src/ui/series.rs` (the data's own colours), `src/ui/axi.rs` (a `Rect` type, pure geometry, and six imgui draw helpers) — replace 106 scattered colour literals. The five surfaces are converted one at a time behind a source-text guard test that carries a shrinking `PENDING` allowlist. imgui centres `add_rect` stroke on the path and clips the window draw list to the window rect, so the two structural tricks — inset-by-`thickness/2` outlines and an inward-drawn offset block — live in `axi.rs` and are pinned by host unit tests.

**Tech Stack:** Rust 2021, cdylib for `x86_64-pc-windows-msvc` cross-compiled from Linux with `cargo-xwin`; vendored `arcdps` crate over `arcdps-imgui` 0.13 (fork `cheahjs/imgui-rs`, branch `arcdps-from-upstream`, feature `tables-api`); `serde`/`serde_json` for config; host-side `cargo test` for the pure logic.

**Spec:** `docs/superpowers/specs/2026-09-24-axi-design-overlay-conversion-design.md`

## Global Constraints

- **This is a reskin, not a refactor.** Every number, label and layout position the plugin showed at v0.4.4 it still shows, computed the same way. No behaviour change beyond the accent setting.
- **`src/ui/theme.rs` is the only file in the crate permitted a chrome colour literal.** `src/ui/series.rs` is the only file permitted a domain colour literal. `theme.rs` holds no domain colour; `series.rs` holds no chrome colour.
- **Every corner is square.** `--axi-radius` and `--axi-radius-sm` are both `0` and that is a contract. No `.rounding(n)` with `n != 0.0`, no non-zero `*Rounding` style var.
- **Two form weight steps, never tuned independently.** `BORDER_PANEL 4.0`, `OFFSET_PANEL 6.0`, `BORDER_CONTROL 3.0`, `OFFSET_CONTROL 3.0`, `OFFSET_PANEL_HOVER 10.0`, `OFFSET_CONTROL_HOVER 6.0`, `BORDER_HAIRLINE 2.0` — all times `SCALE`, which ships at `1.0`.
- **Opacity by surface role.** Reading surfaces (`main.rs`, `pulse.rs`, `timeline.rs`) fill at `ALPHA_READING = 1.0`. HUD surfaces (`team_bar.rs`, `notifier.rs`) fill at `ALPHA_HUD = 0.82`. **Ink is never scaled** — outlines, offset blocks and text draw at alpha `1.0` on both.
- **Default accent id is `"emerald-mint"`**, matching the desktop app. Unknown ids fall back to it rather than panicking: this code runs inside a render callback and a panic there takes GW2's frame with it.
- **`src/ui/map.rs` and `src/map/` are out of scope and stay unconverted.** They are excluded from the guard test by path, with a comment saying deferred rather than exempt.
- **`src/ui/icons.rs`, `src/ui/tile_cache.rs` and the arcdps options pane's styling are out of scope.** The options pane gains one control and nothing else.
- **No font or type-scale work.** imgui's atlas is built before rendering and exposes no `FontId` to push. Hierarchy is expressed through colour, case, spacing and rules only.
- **No transitions and no reduced-motion detection.** imgui is immediate-mode; hover lifts snap.
- **Build the DLL only with `cargo dll`** (release, MSVC, via cargo-xwin) or `cargo dll-dev` / `cargo dll-check`. **Never `cargo build --target x86_64-pc-windows-gnu`** — the GNU binary links but crashes on load inside GW2.
- **Never `cp` a DLL into `addons/` while GW2 is running.** Always `./scripts/deploy.sh`, which writes to tmp and atomically renames; under Wine `cp` truncates the live inode in place and corrupts pages GW2 has mmap'd executable.
- **Host tests run with plain `cargo test`** (no target flag). Anything a host test touches must compile without `#[cfg(windows)]`.
- **End every commit message with:** `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`
- **Never `git add -A`.** Stage the exact paths each step names.

### Ruling: where Pulse's eight `ACCENT_*` constants go

The spec enumerates `series.rs`'s contents as the WvW team colours, the
no-team colour, the boon colours and the eight timeline metric colours. It does
not say where `pulse.rs`'s `ACCENT_DAMAGE` / `DOWN` / `SUPPORT` / `CLEANSE` /
`DEFEND` / `SUCCESS` / `DANGER` / `NEUTRAL` go. They are domain metric inks
exactly parallel to the timeline's eight — `ACCENT_DAMAGE` and `COLOR_DMG` are
already the same value — so **they join `series.rs`'s metric palette**, not
`theme.rs`.

Where a Pulse ink and its Timeline counterpart already hold identical values
they become one constant. Where they differ they stay two constants at their
existing values: unifying them would change what the plugin draws, and this is
a reskin. Task 3 gives the exact table.

The accent therefore drives chrome only: selected tabs and toggles, the
`Pulse` half of the wordmark, panel and card structure, the hero banner's
stripe, and the notifier's transient-state label.

## Review Focus

1. **A degenerate rect** — window dragged to zero width, or `content_region_avail()` returning `0.0` on a collapsed window. Inset by `thickness / 2.0` then flips `min` past `max` and imgui draws an inverted rect. Every `axi.rs` draw helper must early-return on a degenerate rect rather than emit one. *Pinned in Task 2.*
2. **A non-finite or out-of-range bar fraction** — a zero-duration fight divides by zero and hands `axi::bar` `NaN` or `inf`. A reasonable person expects an empty bar, not a bar spanning the screen. `bar_fill` clamps to `[0, 1]` and reads non-finite as `0.0`. *Pinned in Task 2.*
3. **A v0.4.4 `axipulse.json` with no `accent` key** — the overwhelmingly common upgrade path. It must load and pick up `emerald-mint`, not reset every other field to default. *Pinned in Task 5.*
4. **A boon or metric name absent from the palette** — a new GW2 boon, or a renamed one. It must resolve to the neutral ink rather than panicking inside the render callback. *Pinned in Task 3.*
5. **The guard test's exclusion list rotting** — if `src/ui/map.rs` is renamed or split, a path-based exclusion silently matches nothing and the guard keeps passing over a file nobody is guarding. The guard asserts every excluded and every pending path exists on disk. *Pinned in Task 4.*

---

## File Structure

**Created:**

| Path | Responsibility |
|---|---|
| `src/ui/theme.rs` | Chrome tokens, the eleven accents, form geometry, alpha policy, `push_form`. Host-compilable except `push_form`. The only home for a chrome colour literal. |
| `src/ui/series.rs` | The data's colours: WvW teams, boons, metric inks. Host-compilable. The only home for a domain colour literal. |
| `src/ui/axi.rs` | `Rect`, pure block/outline/bar geometry, and the six draw helpers. Geometry is host-compilable; the helpers are `#[cfg(windows)]`. |
| `tests/axi_geometry_test.rs` | Host tests for `theme.rs` and `axi.rs`: accent lookup, geometry ordering, stroke inset, degenerate rects, bar fractions. |
| `tests/axi_series_test.rs` | Host tests for `series.rs`: team colours unchanged from v0.4.4, boon/metric fallbacks. |
| `tests/axi_guard_test.rs` | Source-text guard: no colour literal outside `theme.rs`/`series.rs`, no non-zero rounding. Carries the `PENDING` and `DEFERRED` lists. |
| `docs/superpowers/2026-09-24-axi-design-overlay-residuals.md` | The known residuals this conversion ships with, the deferred map first. |

**Modified:**

| Path | Change |
|---|---|
| `src/ui/mod.rs` | Declare `axi`, `series`, `theme` ungated; document the host/Windows split. |
| `src/config.rs` | `pub accent: String`, default `"emerald-mint"`. |
| `src/ui/options.rs` | One accent combo in `render_options_end`. |
| `src/wvw_teams.rs` | `TeamColor::rgba()` delegates to `series`; its literals removed. |
| `src/fight_composition.rs` | `NO_TEAM_GREEN` removed; reads `series::NO_TEAM`. |
| `src/ui/main.rs` | Shell: transparent `WindowBg`, inward block, square tabs, accent wordmark. |
| `src/ui/pulse.rs` | Five subviews: cards, hero banner, value/skill/boon bars, class chips. |
| `src/ui/timeline.rs` | Eight lanes, layer toggles, tooltip, inspector cards. |
| `src/ui/team_bar.rs` | HUD: square segmented bar, no gloss, no glow, `ALPHA_HUD`. |
| `src/ui/notifier.rs` | HUD: square toast, `ALPHA_HUD`, accent/OK label inks. |
| `CLAUDE.md` | A `## Design` section recording the contract, mirroring the desktop app's. |

---

### Task 1: `theme.rs` — chrome tokens, accents, form geometry

**Files:**
- Create: `src/ui/theme.rs`
- Modify: `src/ui/mod.rs`
- Test: `tests/axi_geometry_test.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `arcdps_axipulse::ui::theme` with `const fn rgb(u8,u8,u8) -> [f32;4]`, `const fn with_alpha([f32;4], f32) -> [f32;4]`, the colour consts `GROUND SURFACE SURFACE_RAISED INK_LINE RULE TEXT TEXT_DIM TEXT_FAINT META OK WARN DANGER TRANSPARENT: [f32;4]`, `ACCENTS: [(&str,&str,[f32;4]); 11]`, `DEFAULT_ACCENT_ID: &str`, `fn accent(&str) -> [f32;4]`, `fn accent_index(&str) -> usize`, `fn accent_labels() -> Vec<&'static str>`, the f32 consts `SCALE BORDER_PANEL OFFSET_PANEL BORDER_CONTROL OFFSET_CONTROL OFFSET_PANEL_HOVER OFFSET_CONTROL_HOVER BORDER_HAIRLINE ALPHA_READING ALPHA_HUD`, the tuples `TOKEN_BORDERS TOKEN_OFFSETS TOKEN_OFFSETS_HOVER: (f32, f32)`, and `#[cfg(windows)] fn push_form(ui: &Ui) -> Vec<StyleStackToken<'_>>`.

- [ ] **Step 1: Write the failing test**

Create `tests/axi_geometry_test.rs`:

```rust
//! Host tests for the axi chrome tokens and the pure draw geometry.
//! These run under plain `cargo test` on Linux: nothing here touches
//! `arcdps::imgui`, which is Windows-only.

use arcdps_axipulse::ui::theme;

#[test]
fn rgb_converts_eight_bit_channels_at_full_alpha() {
    assert_eq!(theme::rgb(0x00, 0x00, 0x00), [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(theme::rgb(0xff, 0xff, 0xff), [1.0, 1.0, 1.0, 1.0]);
    let mid = theme::rgb(0x80, 0x40, 0x20);
    assert!((mid[0] - 128.0 / 255.0).abs() < 1e-6);
    assert!((mid[1] - 64.0 / 255.0).abs() < 1e-6);
    assert!((mid[2] - 32.0 / 255.0).abs() < 1e-6);
    assert_eq!(mid[3], 1.0);
}

#[test]
fn tokens_match_the_axi_design_values() {
    // Verbatim from @axiapps/axi-design dist/axi.css. If a token
    // changes upstream, this test is where it surfaces.
    assert_eq!(theme::GROUND, theme::rgb(0x15, 0x18, 0x1d));
    assert_eq!(theme::SURFACE, theme::rgb(0x22, 0x27, 0x31));
    assert_eq!(theme::SURFACE_RAISED, theme::rgb(0x2b, 0x31, 0x3d));
    assert_eq!(theme::INK_LINE, theme::rgb(0x0c, 0x0e, 0x12));
    assert_eq!(theme::RULE, theme::rgb(0x3a, 0x42, 0x50));
    assert_eq!(theme::TEXT, theme::rgb(0xf4, 0xf6, 0xf9));
    assert_eq!(theme::TEXT_DIM, theme::rgb(0xa7, 0xb0, 0xbe));
    assert_eq!(theme::TEXT_FAINT, theme::rgb(0x7c, 0x86, 0x95));
    assert_eq!(theme::META, theme::rgb(0x4e, 0xc3, 0xff));
    assert_eq!(theme::OK, theme::rgb(0x2f, 0xd3, 0x8a));
    assert_eq!(theme::WARN, theme::rgb(0xff, 0x7a, 0x2f));
    assert_eq!(theme::DANGER, theme::rgb(0xff, 0x52, 0x52));
}

#[test]
fn all_eleven_accents_resolve_to_their_own_value() {
    let expected: [(&str, [f32; 4]); 11] = [
        ("axi-gold", theme::rgb(0xff, 0xc5, 0x3d)),
        ("electric-blue", theme::rgb(0x3b, 0x82, 0xf6)),
        ("refined-cyan", theme::rgb(0x5e, 0xad, 0xd5)),
        ("amber-warm", theme::rgb(0xf5, 0x9e, 0x0b)),
        ("emerald-mint", theme::rgb(0x34, 0xd3, 0x99)),
        ("rose-pink", theme::rgb(0xf4, 0x3f, 0x5e)),
        ("violet-purple", theme::rgb(0x8b, 0x5c, 0xf6)),
        ("crimson-red", theme::rgb(0xef, 0x44, 0x44)),
        ("slate-silver", theme::rgb(0x94, 0xa3, 0xb8)),
        ("teal-ocean", theme::rgb(0x14, 0xb8, 0xa6)),
        ("gold-bronze", theme::rgb(0xd4, 0xa0, 0x17)),
    ];
    assert_eq!(expected.len(), theme::ACCENTS.len());
    for (id, value) in expected {
        assert_eq!(theme::accent(id), value, "accent {id}");
    }
}

#[test]
fn unknown_accent_ids_fall_back_to_the_default_without_panicking() {
    let default = theme::accent(theme::DEFAULT_ACCENT_ID);
    assert_eq!(default, theme::rgb(0x34, 0xd3, 0x99));
    // A hand-edited config, a downgrade, a locale-cased id, and a
    // partial match must all land on the default rather than panic:
    // this runs inside GW2's render callback.
    for id in ["", "Emerald-Mint", "EMERALD-MINT", "emerald", "nope", "  emerald-mint  "] {
        assert_eq!(theme::accent(id), default, "id {id:?}");
    }
}

#[test]
fn accent_index_is_the_combo_position_and_defaults_in_range() {
    assert_eq!(theme::accent_index("axi-gold"), 0);
    assert_eq!(theme::accent_index("emerald-mint"), 4);
    assert_eq!(theme::accent_index("gold-bronze"), 10);
    let fallback = theme::accent_index("nope");
    assert_eq!(fallback, theme::accent_index(theme::DEFAULT_ACCENT_ID));
    assert!(fallback < theme::ACCENTS.len());
}

#[test]
fn accent_labels_line_up_with_accents_positionally() {
    let labels = theme::accent_labels();
    assert_eq!(labels.len(), theme::ACCENTS.len());
    for (i, label) in labels.iter().enumerate() {
        assert_eq!(*label, theme::ACCENTS[i].1);
    }
    assert_eq!(labels[4], "Emerald Mint");
}

#[test]
fn the_two_form_steps_stay_ordered_at_every_positive_scale() {
    // The point of two weight steps is that a panel outweighs a
    // control. Tuning SCALE must never break that, and tuning the two
    // steps independently is what would.
    for scale in [0.5_f32, 0.75, 1.0, 1.5, 2.0, 3.0] {
        let (bp, bc) = (theme::TOKEN_BORDERS.0 * scale, theme::TOKEN_BORDERS.1 * scale);
        let (op, oc) = (theme::TOKEN_OFFSETS.0 * scale, theme::TOKEN_OFFSETS.1 * scale);
        let (ph, ch) = (theme::TOKEN_OFFSETS_HOVER.0 * scale, theme::TOKEN_OFFSETS_HOVER.1 * scale);
        assert!(bp > bc, "panel border must outweigh control at scale {scale}");
        assert!(op > oc, "panel offset must outweigh control at scale {scale}");
        assert!(ph > op, "panel hover must lift above rest at scale {scale}");
        assert!(ch > oc, "control hover must lift above rest at scale {scale}");
        assert!(ph > ch, "panel hover must outweigh control hover at scale {scale}");
        assert!(bc > 0.0 && oc > 0.0, "no weight vanishes at scale {scale}");
    }
}

#[test]
fn shipped_geometry_consts_are_the_tokens_times_scale() {
    assert_eq!(theme::SCALE, 1.0);
    assert_eq!(theme::BORDER_PANEL, theme::TOKEN_BORDERS.0 * theme::SCALE);
    assert_eq!(theme::BORDER_CONTROL, theme::TOKEN_BORDERS.1 * theme::SCALE);
    assert_eq!(theme::OFFSET_PANEL, theme::TOKEN_OFFSETS.0 * theme::SCALE);
    assert_eq!(theme::OFFSET_CONTROL, theme::TOKEN_OFFSETS.1 * theme::SCALE);
    assert_eq!(theme::OFFSET_PANEL_HOVER, theme::TOKEN_OFFSETS_HOVER.0 * theme::SCALE);
    assert_eq!(theme::OFFSET_CONTROL_HOVER, theme::TOKEN_OFFSETS_HOVER.1 * theme::SCALE);
    assert_eq!(theme::BORDER_PANEL, 4.0);
    assert_eq!(theme::OFFSET_PANEL, 6.0);
    assert_eq!(theme::BORDER_CONTROL, 3.0);
    assert_eq!(theme::OFFSET_CONTROL, 3.0);
    assert_eq!(theme::OFFSET_PANEL_HOVER, 10.0);
    assert_eq!(theme::OFFSET_CONTROL_HOVER, 6.0);
    assert_eq!(theme::BORDER_HAIRLINE, 2.0);
}

#[test]
fn alpha_policy_splits_by_surface_role_and_never_touches_ink() {
    assert_eq!(theme::ALPHA_READING, 1.0);
    assert_eq!(theme::ALPHA_HUD, 0.82);
    // `with_alpha` is for fills. Ink consts stay fully opaque, so a
    // translucent offset block can never happen by accident.
    assert_eq!(theme::with_alpha(theme::SURFACE, theme::ALPHA_HUD)[3], 0.82);
    assert_eq!(theme::with_alpha(theme::SURFACE, theme::ALPHA_READING), theme::SURFACE);
    assert_eq!(theme::INK_LINE[3], 1.0);
    assert_eq!(theme::TEXT[3], 1.0);
    assert_eq!(theme::TRANSPARENT, [0.0, 0.0, 0.0, 0.0]);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test --test axi_geometry_test`
Expected: FAIL to compile — `could not find 'theme' in 'ui'`.

- [ ] **Step 3: Write `src/ui/theme.rs`**

```rust
//! The axi-design chrome layer: surface and text ramp, the eleven
//! accents, and the form's two weight steps.
//!
//! This is the ONLY file in the crate permitted a chrome colour
//! literal — the same rule `tokens.css` follows in the web package.
//! Domain colours (team, boon, metric) live in `super::series` and
//! never appear here; that split is what makes "the accent drives
//! chrome only" a checkable property rather than an aspiration.
//!
//! Everything but `push_form` compiles on the host, so the token
//! values and the accent lookup are unit-tested on Linux without GW2.

/// Convert 8-bit channels to imgui's normalized RGBA at full alpha.
///
/// The tokens below are written as hex so they stay diffable by eye
/// against the design package's `accents.json` and `tokens.css`;
/// `[0.133, 0.094, 0.114]` is not reviewable.
pub const fn rgb(r: u8, g: u8, b: u8) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

/// Restate a colour at a different alpha. For FILLS only — see
/// `ALPHA_HUD`. Ink is never scaled.
pub const fn with_alpha(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

// --- the surface and text ramp -------------------------------------------

pub const GROUND: [f32; 4]         = rgb(0x15, 0x18, 0x1d); // --axi-ground
pub const SURFACE: [f32; 4]        = rgb(0x22, 0x27, 0x31); // --axi-surface
pub const SURFACE_RAISED: [f32; 4] = rgb(0x2b, 0x31, 0x3d); // --axi-surface-raised
pub const INK_LINE: [f32; 4]       = rgb(0x0c, 0x0e, 0x12); // --axi-ink-line
pub const RULE: [f32; 4]           = rgb(0x3a, 0x42, 0x50); // --axi-rule
pub const TEXT: [f32; 4]           = rgb(0xf4, 0xf6, 0xf9); // --axi-text
pub const TEXT_DIM: [f32; 4]       = rgb(0xa7, 0xb0, 0xbe); // --axi-text-dim
pub const TEXT_FAINT: [f32; 4]     = rgb(0x7c, 0x86, 0x95); // --axi-text-faint
pub const META: [f32; 4]           = rgb(0x4e, 0xc3, 0xff); // --axi-meta, meta only
pub const OK: [f32; 4]             = rgb(0x2f, 0xd3, 0x8a); // --axi-ok
pub const WARN: [f32; 4]           = rgb(0xff, 0x7a, 0x2f); // --axi-warn
pub const DANGER: [f32; 4]         = rgb(0xff, 0x52, 0x52); // --axi-danger

/// Pushed as `WindowBg` so imgui paints nothing and we own the fill.
/// Not a colour in the language's sense — the absence of one.
pub const TRANSPARENT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// Text drawn on an accent fill. `--axi-accent-ink` resolves to
/// `--axi-ink-line`, and every accent below is bright enough on this
/// ground for near-black to be right.
pub const ACCENT_INK: [f32; 4] = INK_LINE;

// --- the eleven accents --------------------------------------------------

/// `(id, label, value)` for each accent in the design package's
/// `accents.json`, in its order. Carried by value rather than read from
/// a shared file so the plugin and the desktop app agree without the
/// plugin depending on a path that differs between Windows and Wine.
pub const ACCENTS: [(&str, &str, [f32; 4]); 11] = [
    ("axi-gold",      "Axi Gold",      rgb(0xff, 0xc5, 0x3d)),
    ("electric-blue", "Electric Blue", rgb(0x3b, 0x82, 0xf6)),
    ("refined-cyan",  "Refined Cyan",  rgb(0x5e, 0xad, 0xd5)),
    ("amber-warm",    "Amber Warm",    rgb(0xf5, 0x9e, 0x0b)),
    ("emerald-mint",  "Emerald Mint",  rgb(0x34, 0xd3, 0x99)),
    ("rose-pink",     "Rose Pink",     rgb(0xf4, 0x3f, 0x5e)),
    ("violet-purple", "Violet Purple", rgb(0x8b, 0x5c, 0xf6)),
    ("crimson-red",   "Crimson Red",   rgb(0xef, 0x44, 0x44)),
    ("slate-silver",  "Slate Silver",  rgb(0x94, 0xa3, 0xb8)),
    ("teal-ocean",    "Teal Ocean",    rgb(0x14, 0xb8, 0xa6)),
    ("gold-bronze",   "Gold Bronze",   rgb(0xd4, 0xa0, 0x17)),
];

pub const DEFAULT_ACCENT_ID: &str = "emerald-mint";

/// Position of `id` in `ACCENTS`, for the options combo. An unknown id
/// resolves to the default's position.
pub fn accent_index(id: &str) -> usize {
    ACCENTS
        .iter()
        .position(|(k, _, _)| *k == id)
        .unwrap_or_else(|| {
            ACCENTS
                .iter()
                .position(|(k, _, _)| *k == DEFAULT_ACCENT_ID)
                .unwrap_or(0)
        })
}

/// Resolve the configured accent. Matching is exact: an id that differs
/// in case or whitespace is not a near miss to be repaired, it is a
/// config we did not write. An unknown id — hand-edited, or left by a
/// downgrade — falls back to the default rather than panicking: this
/// runs inside a render callback, and a panic there takes GW2's frame
/// with it.
pub fn accent(id: &str) -> [f32; 4] {
    ACCENTS[accent_index(id)].2
}

/// Display labels in `ACCENTS` order, for `combo_simple_string`.
pub fn accent_labels() -> Vec<&'static str> {
    ACCENTS.iter().map(|(_, label, _)| *label).collect()
}

// --- form geometry ------------------------------------------------------

/// Every border and offset below is this times its token value. If the
/// form reads too heavy at your GW2 UI scale, change this and nothing
/// else. Never tune the two steps independently: the point of two steps
/// is that a panel outweighs a control, and independent tuning destroys
/// that relationship.
pub const SCALE: f32 = 1.0;

/// The unscaled token values as `(panel, control)` pairs. The shipped
/// consts below are these times `SCALE`. They are public because the
/// host test walks candidate scales over them to prove the two steps
/// stay ordered for any positive scale — something the flat consts
/// alone cannot express.
pub const TOKEN_BORDERS: (f32, f32)       = (4.0, 3.0);  // --axi-border-panel / -control
pub const TOKEN_OFFSETS: (f32, f32)       = (6.0, 3.0);  // --axi-offset-panel / -control
pub const TOKEN_OFFSETS_HOVER: (f32, f32) = (10.0, 6.0); // --axi-offset-*-hover

pub const BORDER_PANEL: f32   = TOKEN_BORDERS.0 * SCALE;
pub const BORDER_CONTROL: f32 = TOKEN_BORDERS.1 * SCALE;
pub const OFFSET_PANEL: f32   = TOKEN_OFFSETS.0 * SCALE;
pub const OFFSET_CONTROL: f32 = TOKEN_OFFSETS.1 * SCALE;
pub const OFFSET_PANEL_HOVER: f32   = TOKEN_OFFSETS_HOVER.0 * SCALE;
pub const OFFSET_CONTROL_HOVER: f32 = TOKEN_OFFSETS_HOVER.1 * SCALE;

/// Prose-rule weight ONLY — `--axi-border-hairline`. Never a panel or
/// control outline; those are the two steps above.
pub const BORDER_HAIRLINE: f32 = 2.0 * SCALE;

// --- opacity ------------------------------------------------------------

/// Reading surfaces (shell, Pulse, Timeline) fill solid. You open these
/// to read them, so the language lands at full strength.
pub const ALPHA_READING: f32 = 1.0;

/// HUD surfaces (team bar, notifier) are small, always-on, and sit over
/// gameplay rather than being opened and read. Here translucency does
/// real work. Matches the team bar's pre-conversion value.
pub const ALPHA_HUD: f32 = 0.82;

// --- style vars ---------------------------------------------------------

/// Push the axi form's style vars: zero rounding on every corner imgui
/// can round, and no imgui-drawn borders — we draw our own outlines.
/// Returns the tokens; drop them to pop.
///
/// All eight rounding vars are covered deliberately. A missed one shows
/// up as a single softened widget, which is very hard to spot in a
/// screenshot and trivially easy to leave in.
#[cfg(windows)]
pub fn push_form<'ui>(
    ui: &'ui arcdps::imgui::Ui,
) -> Vec<arcdps::imgui::StyleStackToken<'ui>> {
    use arcdps::imgui::StyleVar;
    vec![
        ui.push_style_var(StyleVar::WindowRounding(0.0)),
        ui.push_style_var(StyleVar::ChildRounding(0.0)),
        ui.push_style_var(StyleVar::PopupRounding(0.0)),
        ui.push_style_var(StyleVar::FrameRounding(0.0)),
        ui.push_style_var(StyleVar::ScrollbarRounding(0.0)),
        ui.push_style_var(StyleVar::GrabRounding(0.0)),
        ui.push_style_var(StyleVar::ImageRounding(0.0)),
        ui.push_style_var(StyleVar::TabRounding(0.0)),
        ui.push_style_var(StyleVar::WindowBorderSize(0.0)),
        ui.push_style_var(StyleVar::ChildBorderSize(0.0)),
        ui.push_style_var(StyleVar::PopupBorderSize(0.0)),
        ui.push_style_var(StyleVar::FrameBorderSize(0.0)),
    ]
}
```

- [ ] **Step 4: Declare the module**

In `src/ui/mod.rs`, extend the module doc and add the ungated declaration. Replace the doc comment and the `pub mod map;` line with:

```rust
//! ImGui overlay rendering for axipulse. Windows-only submodules are
//! gated on `cfg(windows)` because `arcdps::imgui` ships Windows-only.
//! `map` compiles on the host too: its pure playback math (lerp,
//! status/health sampling) is host-tested, so only its render items
//! carry per-item `#[cfg(windows)]` gates.
//!
//! `theme`, `series` and `axi` follow the same pattern for the same
//! reason: the axi-design token values, the accent lookup and the
//! block/outline geometry are host-tested, so only the functions that
//! touch `Ui` carry gates.

pub mod axi;
pub mod map;
pub mod series;
pub mod theme;
```

`axi` and `series` do not exist yet, so for this task add only `pub mod theme;` and leave `pub mod map;` as-is; Tasks 2 and 3 add their own lines. The final state of the block is the four `pub mod` lines above.

- [ ] **Step 5: Run the test to verify it passes**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test --test axi_geometry_test`
Expected: PASS, 8 tests.

- [ ] **Step 6: Verify the Windows-only part compiles**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo dll-check`
Expected: finishes with no errors. This is the only thing that proves the twelve `StyleVar` variant names in `push_form` exist in the pinned imgui fork.

- [ ] **Step 7: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add src/ui/theme.rs src/ui/mod.rs tests/axi_geometry_test.rs
git commit -m "$(cat <<'EOF'
feat(design): add the axi chrome token module

theme.rs carries the axi-design surface and text ramp, the eleven
accents by value, and the form's two weight steps derived from one
SCALE knob. Tokens are written as hex so they stay diffable against
the design package by eye.

Accent lookup is exact-match with a fallback to emerald-mint rather
than a panic: this runs inside GW2's render callback.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: `axi.rs` — `Rect`, block/outline geometry, and the six draw helpers

**Files:**
- Create: `src/ui/axi.rs`
- Modify: `src/ui/mod.rs`
- Test: `tests/axi_geometry_test.rs` (append)

**Interfaces:**
- Consumes: `theme::{INK_LINE, RULE, TEXT_FAINT, BORDER_PANEL, BORDER_CONTROL, BORDER_HAIRLINE, OFFSET_PANEL, OFFSET_CONTROL, OFFSET_PANEL_HOVER, OFFSET_CONTROL_HOVER}` from Task 1.
- Produces: `arcdps_axipulse::ui::axi` with `struct Rect { pub min: [f32;2], pub max: [f32;2] }` and `Rect::{new, at, w, h, is_degenerate, inset, shifted}`; free functions `outline_path(Rect, f32) -> Rect`, `block_path(Rect, f32) -> Rect`, `inward_body(Rect, f32) -> Rect`, `bar_fill(Rect, f32) -> Rect`, `clamp_frac(f32) -> f32`; and `#[cfg(windows)]` draw helpers `panel(&Ui, Rect, [f32;4], bool)`, `card(&Ui, Rect, [f32;4], bool)`, `panel_inward(&Ui, Rect, [f32;4])`, `bar(&Ui, Rect, f32, [f32;4])`, `diamond(&Ui, [f32;2], f32, [f32;4])`, `label(&Ui, &str)`, `rule(&Ui, [f32;2], [f32;2])`.

- [ ] **Step 1: Write the failing test**

Append to `tests/axi_geometry_test.rs`:

```rust
use arcdps_axipulse::ui::axi::{self, Rect};

fn r(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::new([x0, y0], [x1, y1])
}

#[test]
fn rect_reports_its_own_size() {
    let a = r(10.0, 20.0, 110.0, 70.0);
    assert_eq!(a.w(), 100.0);
    assert_eq!(a.h(), 50.0);
    let b = Rect::at([10.0, 20.0], [100.0, 50.0]);
    assert_eq!(b, a);
}

#[test]
fn outline_path_insets_by_half_the_stroke_on_all_four_sides() {
    // imgui centres add_rect stroke on the path: a 4px outline
    // straddles the edge 2px in and 2px out. An outline flush with its
    // fill must therefore sit thickness/2 inside. Get this wrong and
    // every panel wears a 2px halo of ground colour.
    let fill = r(100.0, 100.0, 300.0, 200.0);
    let path = axi::outline_path(fill, 4.0);
    assert_eq!(path, r(102.0, 102.0, 298.0, 198.0));
    // The stroke's outer edge lands exactly on the fill's edge.
    assert_eq!(path.min[0] - 2.0, fill.min[0]);
    assert_eq!(path.min[1] - 2.0, fill.min[1]);
    assert_eq!(path.max[0] + 2.0, fill.max[0]);
    assert_eq!(path.max[1] + 2.0, fill.max[1]);
    // Odd thicknesses land on half-pixels rather than rounding.
    assert_eq!(axi::outline_path(fill, 3.0), r(101.5, 101.5, 298.5, 198.5));
}

#[test]
fn block_path_shifts_down_right_without_resizing() {
    let fill = r(100.0, 100.0, 300.0, 200.0);
    let block = axi::block_path(fill, 6.0);
    assert_eq!(block, r(106.0, 106.0, 306.0, 206.0));
    assert_eq!(block.w(), fill.w());
    assert_eq!(block.h(), fill.h());
}

#[test]
fn inward_body_leaves_exactly_the_offset_for_the_block() {
    // The window draw list is clipped to the window rect, so a block
    // drawn outside the window is cut off. The body shrinks instead,
    // and its block lands flush with the window's own edge — the same
    // trick `.axi-window` uses in the desktop app.
    let window = r(0.0, 0.0, 300.0, 400.0);
    let body = axi::inward_body(window, 6.0);
    assert_eq!(body, r(0.0, 0.0, 294.0, 394.0));
    let block = axi::block_path(body, 6.0);
    assert_eq!(block.max, window.max);
    assert!(block.max[0] <= window.max[0] && block.max[1] <= window.max[1]);
}

#[test]
fn inward_body_never_inverts_on_a_window_smaller_than_the_offset() {
    let tiny = r(0.0, 0.0, 4.0, 3.0);
    let body = axi::inward_body(tiny, 6.0);
    assert!(body.is_degenerate(), "a window narrower than its own block has no body");
    assert!(body.max[0] >= body.min[0] && body.max[1] >= body.min[1]);
}

#[test]
fn degenerate_rects_are_recognised_rather_than_drawn() {
    assert!(!r(0.0, 0.0, 10.0, 10.0).is_degenerate());
    assert!(r(0.0, 0.0, 0.0, 10.0).is_degenerate(), "zero width");
    assert!(r(0.0, 0.0, 10.0, 0.0).is_degenerate(), "zero height");
    assert!(r(10.0, 0.0, 0.0, 10.0).is_degenerate(), "inverted");
    assert!(r(f32::NAN, 0.0, 10.0, 10.0).is_degenerate(), "NaN corner");
    assert!(r(0.0, 0.0, f32::INFINITY, 10.0).is_degenerate(), "infinite corner");
}

#[test]
fn insetting_past_the_middle_collapses_instead_of_inverting() {
    // A window dragged to 3px wide, outlined at 4px, would otherwise
    // hand imgui a rect whose min is past its max.
    let thin = r(0.0, 0.0, 3.0, 40.0);
    let path = axi::outline_path(thin, 4.0);
    assert!(path.is_degenerate());
    assert!(path.max[0] >= path.min[0], "collapsed, not inverted");
    assert!(path.max[1] >= path.min[1], "collapsed, not inverted");
}

#[test]
fn bar_fill_draws_a_quantity_as_length_clamped_to_the_track() {
    let track = r(0.0, 0.0, 200.0, 20.0);
    assert_eq!(axi::bar_fill(track, 0.5), r(0.0, 0.0, 100.0, 20.0));
    assert_eq!(axi::bar_fill(track, 1.0), track);
    assert!(axi::bar_fill(track, 0.0).is_degenerate());
    // Over- and under-range fractions clamp rather than overrun the
    // track or reach back past its left edge.
    assert_eq!(axi::bar_fill(track, 4.0), track);
    assert!(axi::bar_fill(track, -3.0).is_degenerate());
}

#[test]
fn a_non_finite_fraction_reads_as_empty() {
    // A zero-duration fight divides by zero. A bar spanning the screen
    // is worse than an empty one, and a NaN rect is worse than both.
    let track = r(0.0, 0.0, 200.0, 20.0);
    for frac in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(axi::clamp_frac(frac), 0.0, "frac {frac}");
        let fill = axi::bar_fill(track, frac);
        assert!(fill.is_degenerate());
        assert!(fill.max[0].is_finite() && fill.max[1].is_finite());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test --test axi_geometry_test`
Expected: FAIL to compile — `could not find 'axi' in 'ui'`.

- [ ] **Step 3: Write `src/ui/axi.rs`**

```rust
//! The axi-design draw layer: one place where a raised element's
//! block, fill and outline are laid down, so five surfaces do not
//! hand-roll the sequence five ways.
//!
//! The geometry is pure and host-compilable — `Rect` is a plain pair of
//! corners, not an imgui type — because the two structural traps in
//! this conversion are arithmetic:
//!
//!   * imgui centres `add_rect` stroke ON the path, so an outline flush
//!     with its fill must sit `thickness / 2` inside it;
//!   * the window draw list is clipped to the window rect, so the
//!     shell's offset block is drawn inward.
//!
//! Both are wrong-everywhere-and-subtle if mis-derived, which is why
//! they are unit-tested on the host rather than eyeballed in-game.

use super::theme;

/// An axis-aligned rectangle by its two corners. Deliberately not an
/// imgui type: this compiles and tests on Linux.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Rect {
    pub fn new(min: [f32; 2], max: [f32; 2]) -> Self {
        Self { min, max }
    }

    /// From a top-left corner and a size.
    pub fn at(pos: [f32; 2], size: [f32; 2]) -> Self {
        Self { min: pos, max: [pos[0] + size[0], pos[1] + size[1]] }
    }

    pub fn w(&self) -> f32 { self.max[0] - self.min[0] }
    pub fn h(&self) -> f32 { self.max[1] - self.min[1] }

    /// Nothing worth drawing: no area, or a coordinate imgui cannot
    /// use. Every draw helper below early-returns on this rather than
    /// emitting an inverted or NaN rect.
    pub fn is_degenerate(&self) -> bool {
        let finite = self.min.iter().chain(self.max.iter()).all(|v| v.is_finite());
        !finite || self.w() <= 0.0 || self.h() <= 0.0
    }

    /// Shrink by `d` on all four sides. Collapses to a zero-area rect
    /// at its own centre rather than inverting, so an over-inset rect
    /// is degenerate instead of drawn inside out.
    pub fn inset(&self, d: f32) -> Rect {
        let min = [self.min[0] + d, self.min[1] + d];
        let max = [self.max[0] - d, self.max[1] - d];
        Rect {
            min,
            max: [max[0].max(min[0]), max[1].max(min[1])],
        }
    }

    /// Translate without resizing.
    pub fn shifted(&self, dx: f32, dy: f32) -> Rect {
        Rect {
            min: [self.min[0] + dx, self.min[1] + dy],
            max: [self.max[0] + dx, self.max[1] + dy],
        }
    }
}

/// The rect to hand `add_rect(..).thickness(t)` so the stroke's OUTER
/// edge lands exactly on `r`'s edge.
pub fn outline_path(r: Rect, thickness: f32) -> Rect {
    r.inset(thickness / 2.0)
}

/// The hard offset block for `r`: same size, shifted down and right.
pub fn block_path(r: Rect, offset: f32) -> Rect {
    r.shifted(offset, offset)
}

/// The body rect for a window whose block must be drawn INWARD: the
/// window shrunk on its right and bottom by `offset`, so
/// `block_path(body, offset)` lands flush with the window's own edge
/// and inside the draw list's clip rect.
pub fn inward_body(window: Rect, offset: f32) -> Rect {
    let max = [
        (window.max[0] - offset).max(window.min[0]),
        (window.max[1] - offset).max(window.min[1]),
    ];
    Rect { min: window.min, max }
}

/// A fraction fit to draw with: clamped to `[0, 1]`, with anything
/// non-finite reading as empty. A zero-duration fight divides by zero
/// somewhere upstream, and a bar spanning the screen is worse than no
/// bar at all.
pub fn clamp_frac(frac: f32) -> f32 {
    if frac.is_finite() { frac.clamp(0.0, 1.0) } else { 0.0 }
}

/// The filled length of a bar track — rule 7: a quantity is drawn as
/// length, never intensity.
pub fn bar_fill(track: Rect, frac: f32) -> Rect {
    Rect {
        min: track.min,
        max: [track.min[0] + track.w() * clamp_frac(frac), track.max[1]],
    }
}

// --- draw helpers -------------------------------------------------------

#[cfg(windows)]
use arcdps::imgui::Ui;

/// Lay down one raised element: block, then fill, then outline, in
/// that order. Order matters — the block must be painted before the
/// fill or it covers it.
#[cfg(windows)]
fn blocked(ui: &Ui, r: Rect, fill: [f32; 4], border: f32, offset: f32) {
    if r.is_degenerate() { return; }
    let draw = ui.get_window_draw_list();

    let block = block_path(r, offset);
    if !block.is_degenerate() {
        draw.add_rect(block.min, block.max, theme::INK_LINE)
            .filled(true).build();
    }
    draw.add_rect(r.min, r.max, fill).filled(true).build();

    let path = outline_path(r, border);
    if !path.is_degenerate() {
        draw.add_rect(path.min, path.max, theme::INK_LINE)
            .thickness(border).build();
    }
}

/// A panel: the heavier of the two form steps. `hovered` deepens the
/// offset — pass `false` for anything the user cannot click, because a
/// hover state on a non-interactive thing is a lie about affordance.
#[cfg(windows)]
pub fn panel(ui: &Ui, r: Rect, fill: [f32; 4], hovered: bool) {
    let offset = if hovered { theme::OFFSET_PANEL_HOVER } else { theme::OFFSET_PANEL };
    blocked(ui, r, fill, theme::BORDER_PANEL, offset);
}

/// A card or chip: the lighter of the two form steps.
#[cfg(windows)]
pub fn card(ui: &Ui, r: Rect, fill: [f32; 4], hovered: bool) {
    let offset = if hovered { theme::OFFSET_CONTROL_HOVER } else { theme::OFFSET_CONTROL };
    blocked(ui, r, fill, theme::BORDER_CONTROL, offset);
}

/// A panel for a window whose block cannot go outside it. Takes the
/// WINDOW rect and derives the body; returns the body so the caller
/// knows how much room the content has.
#[cfg(windows)]
pub fn panel_inward(ui: &Ui, window: Rect, fill: [f32; 4]) -> Rect {
    let body = inward_body(window, theme::OFFSET_PANEL);
    blocked(ui, body, fill, theme::BORDER_PANEL, theme::OFFSET_PANEL);
    body
}

/// A quantity as length: an outlined track with a filled bar in it.
#[cfg(windows)]
pub fn bar(ui: &Ui, track: Rect, frac: f32, ink: [f32; 4]) {
    if track.is_degenerate() { return; }
    let draw = ui.get_window_draw_list();
    draw.add_rect(track.min, track.max, theme::GROUND).filled(true).build();
    let fill = bar_fill(track, frac);
    if !fill.is_degenerate() {
        draw.add_rect(fill.min, fill.max, ink).filled(true).build();
    }
    let path = outline_path(track, theme::BORDER_CONTROL);
    if !path.is_degenerate() {
        draw.add_rect(path.min, path.max, theme::INK_LINE)
            .thickness(theme::BORDER_CONTROL).build();
    }
}

/// The family motif, as two filled triangles meeting on the
/// horizontal. `size` is the full diagonal.
#[cfg(windows)]
pub fn diamond(ui: &Ui, center: [f32; 2], size: f32, ink: [f32; 4]) {
    if !size.is_finite() || size <= 0.0 { return; }
    if !center.iter().all(|v| v.is_finite()) { return; }
    let h = size * 0.5;
    let [cx, cy] = center;
    let draw = ui.get_window_draw_list();
    draw.add_triangle([cx, cy - h], [cx + h, cy], [cx - h, cy], ink)
        .filled(true).build();
    draw.add_triangle([cx - h, cy], [cx + h, cy], [cx, cy + h], ink)
        .filled(true).build();
}

/// An eyebrow or label. Uppercase and faint: with one font face at one
/// size, case and colour are the only weight hierarchy reachable.
#[cfg(windows)]
pub fn label(ui: &Ui, text: &str) {
    ui.text_colored(theme::TEXT_FAINT, text.to_uppercase());
}

/// An internal rule at hairline weight, for dividing content inside an
/// already-outlined panel. Rule 5: outlined means annotation.
#[cfg(windows)]
pub fn rule(ui: &Ui, from: [f32; 2], to: [f32; 2]) {
    if !from.iter().chain(to.iter()).all(|v| v.is_finite()) { return; }
    ui.get_window_draw_list()
        .add_line(from, to, theme::RULE)
        .thickness(theme::BORDER_HAIRLINE)
        .build();
}
```

- [ ] **Step 4: Declare the module**

In `src/ui/mod.rs`, add `pub mod axi;` as the first `pub mod` line (alphabetical, ungated).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test --test axi_geometry_test`
Expected: PASS, 17 tests.

- [ ] **Step 6: Verify the draw helpers compile for Windows**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo dll-check`
Expected: no errors. This proves `add_rect(..).thickness(..).build()`, `add_triangle(..).filled(true)` and `add_line(..).thickness(..)` all exist with these signatures in the pinned fork.

- [ ] **Step 7: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add src/ui/axi.rs src/ui/mod.rs tests/axi_geometry_test.rs
git commit -m "$(cat <<'EOF'
feat(design): add the axi draw layer and its geometry

axi.rs owns the block-fill-outline sequence so the five surfaces do
not each hand-roll it. Rect and the geometry are pure and host-tested
because the two traps here are arithmetic, not visual: imgui centres
add_rect stroke on the path, so outlines inset by thickness/2, and the
window draw list is clipped to the window, so the shell's block goes
inward.

Degenerate rects and non-finite bar fractions collapse rather than
reaching imgui — a zero-width window and a zero-duration fight both
produce them.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: `series.rs` — the data's colours, relocated

**Files:**
- Create: `src/ui/series.rs`
- Modify: `src/ui/mod.rs`, `src/wvw_teams.rs:41-49`, `src/fight_composition.rs:30-46`
- Test: `tests/axi_series_test.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks (`series.rs` deliberately imports no `theme` item — it holds no chrome colour).
- Produces: `arcdps_axipulse::ui::series` with `TEAM_RED TEAM_GREEN TEAM_BLUE TEAM_UNKNOWN NO_TEAM BOON_NEUTRAL METRIC_NEUTRAL: [f32;4]`; the metric inks `METRIC_DAMAGE METRIC_DAMAGE_TAKEN METRIC_DOWN METRIC_SUPPORT METRIC_CLEANSE METRIC_DEFEND METRIC_SUCCESS METRIC_HEALTH METRIC_DISTANCE METRIC_OFF_BOONS METRIC_DEF_BOONS METRIC_HEAL_IN METRIC_BARRIER_IN METRIC_BARRIER: [f32;4]`; and `fn boon(name: &str) -> [f32;4]`.
- `src/wvw_teams.rs`'s `TeamColor::rgba(self) -> [f32; 4]` keeps its signature and every call site; only its body moves.

- [ ] **Step 1: Write the failing test**

Create `tests/axi_series_test.rs`:

```rust
//! The domain palettes. These are the DATA's colours, not the
//! system's: the accent never touches them, and this conversion must
//! not change a single value — recolouring a WvW team would make the
//! team bar lie about which side is which.

use arcdps_axipulse::ui::series;
use arcdps_axipulse::wvw_teams::TeamColor;

#[test]
fn wvw_team_colours_are_byte_identical_to_v0_4_4() {
    // axibridge's hex palette: #f87171 / #4ade80 / #60a5fa / #9ca3af,
    // as they were spelled before the move to series.rs.
    assert_eq!(series::TEAM_RED, [0.973, 0.443, 0.443, 1.0]);
    assert_eq!(series::TEAM_GREEN, [0.290, 0.871, 0.502, 1.0]);
    assert_eq!(series::TEAM_BLUE, [0.376, 0.647, 0.980, 1.0]);
    assert_eq!(series::TEAM_UNKNOWN, [0.612, 0.639, 0.686, 1.0]);
    assert_eq!(series::NO_TEAM, [0.29, 0.86, 0.50, 1.0]);
}

#[test]
fn team_color_rgba_still_answers_from_series() {
    assert_eq!(TeamColor::Red.rgba(), series::TEAM_RED);
    assert_eq!(TeamColor::Green.rgba(), series::TEAM_GREEN);
    assert_eq!(TeamColor::Blue.rgba(), series::TEAM_BLUE);
    assert_eq!(TeamColor::Unknown.rgba(), series::TEAM_UNKNOWN);
}

#[test]
fn known_boons_keep_their_pre_conversion_inks() {
    let expected: [(&str, [f32; 4]); 13] = [
        ("Might",        [0.91, 0.36, 0.23, 1.0]),
        ("Fury",         [0.91, 0.60, 0.23, 1.0]),
        ("Quickness",    [0.75, 0.42, 0.94, 1.0]),
        ("Alacrity",     [0.94, 0.42, 0.74, 1.0]),
        ("Protection",   [0.36, 0.61, 0.83, 1.0]),
        ("Regeneration", [0.29, 0.86, 0.50, 1.0]),
        ("Vigor",        [0.64, 0.90, 0.21, 1.0]),
        ("Swiftness",    [0.98, 0.80, 0.08, 1.0]),
        ("Resistance",   [0.77, 0.64, 0.35, 1.0]),
        ("Stability",    [0.96, 0.62, 0.04, 1.0]),
        ("Aegis",        [0.49, 0.83, 0.99, 1.0]),
        ("Resolution",   [0.65, 0.51, 0.91, 1.0]),
        ("Retaliation",  [0.98, 0.57, 0.20, 1.0]),
    ];
    for (name, ink) in expected {
        assert_eq!(series::boon(name), ink, "boon {name}");
    }
}

#[test]
fn an_unknown_boon_name_resolves_to_neutral_without_panicking() {
    // A new GW2 boon, a renamed one, or a log from a build we have
    // never seen. This runs inside GW2's render callback.
    assert_eq!(series::BOON_NEUTRAL, [0.55, 0.55, 0.62, 1.0]);
    for name in ["", "Alacrity ", "alacrity", "Sharpened Edges", "\u{1f600}"] {
        assert_eq!(series::boon(name), series::BOON_NEUTRAL, "name {name:?}");
    }
}

#[test]
fn metric_inks_are_byte_identical_to_v0_4_4() {
    // Pulse and Timeline shared five of these values already; those
    // five are now one constant each. The four that differed keep
    // their own values — unifying them would change what the plugin
    // draws, and this is a reskin.
    assert_eq!(series::METRIC_DAMAGE, [0.95, 0.38, 0.38, 1.0]);       // was ACCENT_DAMAGE / COLOR_DMG
    assert_eq!(series::METRIC_DOWN, [0.97, 0.55, 0.42, 1.0]);         // was ACCENT_DOWN / COLOR_TAKEN
    assert_eq!(series::METRIC_DAMAGE_TAKEN, series::METRIC_DOWN);
    assert_eq!(series::METRIC_SUPPORT, [0.40, 0.85, 0.65, 1.0]);      // was ACCENT_SUPPORT
    assert_eq!(series::METRIC_CLEANSE, [0.32, 0.78, 0.92, 1.0]);      // was ACCENT_CLEANSE / COLOR_DEF
    assert_eq!(series::METRIC_DEF_BOONS, series::METRIC_CLEANSE);
    assert_eq!(series::METRIC_DEFEND, [0.95, 0.62, 0.30, 1.0]);       // was ACCENT_DEFEND
    assert_eq!(series::METRIC_SUCCESS, [0.40, 0.85, 0.55, 1.0]);      // was ACCENT_SUCCESS
    assert_eq!(series::METRIC_NEUTRAL, [0.55, 0.62, 0.78, 1.0]);      // was ACCENT_NEUTRAL
    assert_eq!(series::METRIC_HEALTH, [0.29, 0.86, 0.50, 1.0]);       // was COLOR_HEALTH
    assert_eq!(series::METRIC_DISTANCE, [0.95, 0.75, 0.40, 1.0]);     // was COLOR_DIST
    assert_eq!(series::METRIC_OFF_BOONS, [0.42, 0.65, 0.94, 1.0]);    // was COLOR_OFF
    assert_eq!(series::METRIC_HEAL_IN, [0.35, 0.88, 0.62, 1.0]);      // was COLOR_HEAL_IN
    assert_eq!(series::METRIC_BARRIER_IN, [0.85, 0.72, 0.32, 1.0]);   // was COLOR_BARRIER_IN
    assert_eq!(series::METRIC_BARRIER, [0.65, 0.51, 0.91, 1.0]);      // was pulse.rs:218 inline
}

#[test]
fn every_domain_ink_is_fully_opaque() {
    // Domain inks are drawn at full strength or not at all (rule 2).
    // Surfaces apply their own fill alpha; they never dim the data.
    let all = [
        series::TEAM_RED, series::TEAM_GREEN, series::TEAM_BLUE,
        series::TEAM_UNKNOWN, series::NO_TEAM, series::BOON_NEUTRAL,
        series::METRIC_DAMAGE, series::METRIC_DOWN, series::METRIC_DAMAGE_TAKEN,
        series::METRIC_SUPPORT, series::METRIC_CLEANSE, series::METRIC_DEFEND,
        series::METRIC_SUCCESS, series::METRIC_NEUTRAL, series::METRIC_HEALTH,
        series::METRIC_DISTANCE, series::METRIC_OFF_BOONS, series::METRIC_DEF_BOONS,
        series::METRIC_HEAL_IN, series::METRIC_BARRIER_IN, series::METRIC_BARRIER,
    ];
    for ink in all {
        assert_eq!(ink[3], 1.0);
    }
    assert_eq!(series::boon("Might")[3], 1.0);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test --test axi_series_test`
Expected: FAIL to compile — `could not find 'series' in 'ui'`.

- [ ] **Step 3: Write `src/ui/series.rs`**

```rust
//! The domain palettes: the data's own colours, not the system's.
//!
//! This is the only file besides `super::theme` permitted a colour
//! literal, and it is the only one permitted a DOMAIN colour. Nothing
//! here is ever recoloured by the accent: red, green and blue are the
//! game's team identities, and recolouring them would make the team bar
//! lie about which side is which. The desktop app leaves its profession
//! colours alone for the same reason
//! (`src/renderer/themes/series.css`).
//!
//! `theme.rs` holds no domain colour and this file holds no chrome
//! colour. That split is what makes "the accent drives chrome only" a
//! checkable property.
//!
//! Professions need no entry here: they are drawn as PNG icons from
//! `src/assets/classes/`, not as colours.

// --- WvW teams ----------------------------------------------------------

/// axibridge's hex palette (#f87171 / #4ade80 / #60a5fa / #9ca3af) as
/// normalized RGBA. `wvw_teams::TeamColor::rgba` reads these.
pub const TEAM_RED: [f32; 4]     = [0.973, 0.443, 0.443, 1.0];
pub const TEAM_GREEN: [f32; 4]   = [0.290, 0.871, 0.502, 1.0];
pub const TEAM_BLUE: [f32; 4]    = [0.376, 0.647, 0.980, 1.0];
pub const TEAM_UNKNOWN: [f32; 4] = [0.612, 0.639, 0.686, 1.0];

/// Fallback for Squad/Allies when the log never resolved our own team
/// colour (PvE, or a WvW log with no team data) — the green the card
/// used to hard-code for everyone.
pub const NO_TEAM: [f32; 4] = [0.29, 0.86, 0.50, 1.0];

// --- metric inks --------------------------------------------------------

/// One palette for both tabs, so Pulse's "damage" and Timeline's
/// "damage dealt" cannot disagree about what damage looks like.
///
/// Where a Pulse ink and its Timeline counterpart already held the same
/// value they are one constant with an alias. Where they differed they
/// stay two constants at their pre-conversion values: unifying them
/// would change what the plugin draws, and this is a reskin.
pub const METRIC_DAMAGE: [f32; 4]      = [0.95, 0.38, 0.38, 1.0];
pub const METRIC_DOWN: [f32; 4]        = [0.97, 0.55, 0.42, 1.0];
/// Damage taken shares the down-contribution ink; they were already
/// the same value in Pulse and Timeline respectively.
pub const METRIC_DAMAGE_TAKEN: [f32; 4] = METRIC_DOWN;
pub const METRIC_SUPPORT: [f32; 4]     = [0.40, 0.85, 0.65, 1.0];
pub const METRIC_CLEANSE: [f32; 4]     = [0.32, 0.78, 0.92, 1.0];
/// Defensive boons share the cleanse ink; already the same value.
pub const METRIC_DEF_BOONS: [f32; 4]   = METRIC_CLEANSE;
pub const METRIC_DEFEND: [f32; 4]      = [0.95, 0.62, 0.30, 1.0];
pub const METRIC_SUCCESS: [f32; 4]     = [0.40, 0.85, 0.55, 1.0];
pub const METRIC_HEALTH: [f32; 4]      = [0.29, 0.86, 0.50, 1.0];
pub const METRIC_DISTANCE: [f32; 4]    = [0.95, 0.75, 0.40, 1.0];
pub const METRIC_OFF_BOONS: [f32; 4]   = [0.42, 0.65, 0.94, 1.0];
pub const METRIC_HEAL_IN: [f32; 4]     = [0.35, 0.88, 0.62, 1.0];
pub const METRIC_BARRIER_IN: [f32; 4]  = [0.85, 0.72, 0.32, 1.0];
pub const METRIC_BARRIER: [f32; 4]     = [0.65, 0.51, 0.91, 1.0];

/// For a metric with no ink of its own.
pub const METRIC_NEUTRAL: [f32; 4] = [0.55, 0.62, 0.78, 1.0];

// --- boons --------------------------------------------------------------

/// For a boon name we do not know: a new one, a renamed one, or a log
/// from a build we have never seen.
pub const BOON_NEUTRAL: [f32; 4] = [0.55, 0.55, 0.62, 1.0];

/// The boon's own ink. Matching is exact on axilog's boon name; an
/// unrecognised name resolves to `BOON_NEUTRAL` rather than panicking,
/// because this runs inside GW2's render callback.
pub fn boon(name: &str) -> [f32; 4] {
    match name {
        "Might"        => [0.91, 0.36, 0.23, 1.0],
        "Fury"         => [0.91, 0.60, 0.23, 1.0],
        "Quickness"    => [0.75, 0.42, 0.94, 1.0],
        "Alacrity"     => [0.94, 0.42, 0.74, 1.0],
        "Protection"   => [0.36, 0.61, 0.83, 1.0],
        "Regeneration" => [0.29, 0.86, 0.50, 1.0],
        "Vigor"        => [0.64, 0.90, 0.21, 1.0],
        "Swiftness"    => [0.98, 0.80, 0.08, 1.0],
        "Resistance"   => [0.77, 0.64, 0.35, 1.0],
        "Stability"    => [0.96, 0.62, 0.04, 1.0],
        "Aegis"        => [0.49, 0.83, 0.99, 1.0],
        "Resolution"   => [0.65, 0.51, 0.91, 1.0],
        "Retaliation"  => [0.98, 0.57, 0.20, 1.0],
        _              => BOON_NEUTRAL,
    }
}
```

- [ ] **Step 4: Point `wvw_teams.rs` at `series`**

In `src/wvw_teams.rs`, replace the `rgba` method (currently lines 41-49, including its doc comment) with:

```rust
    /// The team's ink. The values live in `ui::series` with the rest of
    /// the domain palettes; this method keeps its signature and all its
    /// call sites.
    pub fn rgba(self) -> [f32; 4] {
        use crate::ui::series;
        match self {
            TeamColor::Red => series::TEAM_RED,
            TeamColor::Green => series::TEAM_GREEN,
            TeamColor::Blue => series::TEAM_BLUE,
            TeamColor::Unknown => series::TEAM_UNKNOWN,
        }
    }
```

- [ ] **Step 5: Point `fight_composition.rs` at `series`**

In `src/fight_composition.rs`, delete the `NO_TEAM_GREEN` const and its doc comment (currently lines 30-33) and rewrite `home_colors` to read from `series`:

```rust
fn home_colors(self_team: &str) -> ([f32; 4], [f32; 4]) {
    let squad = match crate::wvw_teams::team_color(self_team) {
        // No resolved team colour (PvE, or a WvW log with no team
        // data) — the green the card used to hard-code for everyone.
        crate::wvw_teams::TeamColor::Unknown => crate::ui::series::NO_TEAM,
        c => c.rgba(),
    };
    let allies = [squad[0] * 0.62, squad[1] * 0.62, squad[2] * 0.62, squad[3]];
    (squad, allies)
}
```

- [ ] **Step 6: Declare the module**

In `src/ui/mod.rs`, add `pub mod series;` after `pub mod map;` (ungated).

- [ ] **Step 7: Run the whole host suite to verify nothing regressed**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test`
Expected: PASS. `tests/axi_series_test.rs` is 6 new tests; every pre-existing test — `wvw_teams_test`, `fight_data_*`, `pulse_metrics_test` and the rest — still passes, because only the colours' address changed.

- [ ] **Step 8: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add src/ui/series.rs src/ui/mod.rs src/wvw_teams.rs src/fight_composition.rs tests/axi_series_test.rs
git commit -m "$(cat <<'EOF'
feat(design): collect the domain palettes into series.rs

The WvW team colours, the no-team green, the boon inks and the metric
inks are the data's colours, not the system's — the accent never
touches them. Gathering them into one host-tested module is what makes
"the accent drives chrome only" checkable rather than aspirational.

Pulse's eight stat inks join the Timeline's eight as one metric
palette, so the two tabs cannot disagree about what damage looks like.
Five pairs already held identical values and become one constant each;
the four that differed keep their own values. No colour changes.

TeamColor::rgba keeps its signature and every call site.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: The guard test, with a shrinking `PENDING` list

**Files:**
- Create: `tests/axi_guard_test.rs`

**Interfaces:**
- Consumes: the existence of `src/ui/theme.rs` and `src/ui/series.rs` from Tasks 1 and 3.
- Produces: nothing the crate imports. Tasks 6-9 each delete one entry from the `PENDING` array; Task 10 asserts it is empty.

This is the parity of `tests/renderer/tokens.test.ts` in the desktop app, and it is the only thing that will stop the 106 literals creeping back one convenience constant at a time. It reads source **as text**, so it runs on Linux even though the UI modules are `#[cfg(windows)]`.

- [ ] **Step 1: Write the failing test**

Create `tests/axi_guard_test.rs`:

```rust
//! The axi-design contract, enforced against the UI sources as TEXT.
//!
//! Reading source rather than calling it is what lets this run on Linux
//! while the modules it guards are `#[cfg(windows)]`. It is the parity
//! of `tests/renderer/tokens.test.ts` in the desktop app, and it is the
//! only thing that will stop the 106 colour literals this conversion
//! removed from creeping back one convenience constant at a time.

use std::path::{Path, PathBuf};

/// Files allowed to hold a colour literal — and ONLY a colour literal.
/// `theme.rs` owns chrome, `series.rs` owns the domain palettes, and
/// nothing else owns either. Both are still held to the square-corner
/// rule, which is why they are exempted inside the colour test rather
/// than dropped from the walk: `theme::push_form` pushes eight rounding
/// vars and every one of them must be zero.
const ALLOWED: [&str; 2] = ["src/ui/theme.rs", "src/ui/series.rs"];

/// DEFERRED, NOT EXEMPT. The map keeps its own theme and its own colour
/// constants for now: a half-converted map reads as a bug rather than
/// as a deferral, so it is converted whole later or not at all. When it
/// is converted, delete these entries — do not add to them.
///
/// See the residuals note in `docs/superpowers/`.
const DEFERRED: [&str; 5] = [
    "src/ui/map.rs",
    "src/map/mod.rs",
    "src/map/boon_panel.rs",
    "src/map/tiles.rs",
    "src/map/wvw.rs",
];

/// Surfaces not yet converted. Each conversion task deletes its own
/// entry; the list reaching empty is the conversion being done. Unlike
/// DEFERRED this is temporary scaffolding — if you are reading this
/// after the branch merged and it is non-empty, something was skipped.
const PENDING: [&str; 6] = [
    "src/ui/options.rs",
    "src/ui/main.rs",
    "src/ui/pulse.rs",
    "src/ui/timeline.rs",
    "src/ui/team_bar.rs",
    "src/ui/notifier.rs",
];

/// Roots walked for guardable `.rs` files. Walking rather than listing
/// means a NEW ui file is guarded by default, which is the only way an
/// allowlist stays honest.
const ROOTS: [&str; 1] = ["src/ui"];

/// Individually guarded files outside the walked roots: the two modules
/// whose domain colours moved into `series.rs`.
const EXTRA_FILES: [&str; 2] = ["src/wvw_teams.rs", "src/fight_composition.rs"];

/// Opt-out marker. A line carrying this is skipped, and the text after
/// it is the reason. Never add one without a reason a reviewer can
/// weigh.
const ALLOW_MARKER: &str = "axi-guard: allow";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rel(p: &Path) -> String {
    p.strip_prefix(repo_root())
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every file this test is responsible for right now.
fn guarded_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut found = Vec::new();
    for r in ROOTS {
        collect_rs(&root.join(r), &mut found);
    }
    for f in EXTRA_FILES {
        found.push(root.join(f));
    }
    found.sort();
    found.retain(|p| {
        let r = rel(p);
        !DEFERRED.contains(&r.as_str()) && !PENDING.contains(&r.as_str())
    });
    found
}

/// Is this a four-component float array — i.e. a colour?
///
/// Deliberately narrow: exactly four comma-separated components that
/// all parse as `f32`. `[0.0, 4.0]` (a two-component imgui vec) and
/// `[x, y, w, h]` (identifiers) are not colours and are not flagged.
fn is_colour_literal(body: &str) -> bool {
    let parts: Vec<&str> = body.split(',').map(str::trim).collect();
    parts.len() == 4 && parts.iter().all(|p| p.parse::<f32>().is_ok())
}

/// Every top-level `[...]` body on a line, without nesting.
fn bracket_bodies(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut buf = String::new();
    for ch in line.chars() {
        match ch {
            '[' => {
                depth += 1;
                if depth == 1 { buf.clear(); }
            }
            ']' => {
                if depth == 1 { out.push(buf.clone()); }
                depth = depth.saturating_sub(1);
            }
            _ if depth >= 1 => buf.push(ch),
            _ => {}
        }
    }
    out
}

/// The argument text of every `rounding(` / `Rounding(` call on a line.
fn rounding_args(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let rest = &line[i..];
        let hit = rest.starts_with("rounding(") || rest.starts_with("Rounding(");
        if !hit {
            i += 1;
            continue;
        }
        let open = i + rest.find('(').expect("just matched");
        let mut depth = 0usize;
        let mut end = None;
        for (off, ch) in line[open..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 { end = Some(open + off); break; }
                }
                _ => {}
            }
        }
        if let Some(close) = end {
            out.push(line[open + 1..close].trim().to_string());
            i = close + 1;
        } else {
            i = open + 1;
        }
    }
    out
}

/// Comment and string content confuse every rule here, so lines that
/// are wholly a comment are skipped. A colour literal hiding inside a
/// doc comment is not a colour the plugin draws.
fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("*") || t.starts_with("/*")
}

#[test]
fn no_colour_literal_lives_outside_theme_and_series() {
    let mut violations: Vec<String> = Vec::new();
    for path in guarded_files() {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        if ALLOWED.contains(&rel(&path).as_str()) { continue; }
        for (n, line) in text.lines().enumerate() {
            if is_comment(line) || line.contains(ALLOW_MARKER) { continue; }
            for body in bracket_bodies(line) {
                if is_colour_literal(&body) {
                    violations.push(format!("{}:{}: [{}]", rel(&path), n + 1, body.trim()));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "colour literals belong in src/ui/theme.rs (chrome) or \
         src/ui/series.rs (domain palettes), nowhere else:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn every_corner_is_square() {
    // --axi-radius and --axi-radius-sm are both 0 in the language, and
    // that is a contract rather than a default. A single missed
    // rounding var shows up as one softened widget, which is very hard
    // to spot in a screenshot.
    let mut violations: Vec<String> = Vec::new();
    for path in guarded_files() {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        for (n, line) in text.lines().enumerate() {
            if is_comment(line) || line.contains(ALLOW_MARKER) { continue; }
            for arg in rounding_args(line) {
                let square = arg == "0.0" || arg == "0" || arg == "0.0_f32";
                if !square {
                    violations.push(format!("{}:{}: rounding({arg})", rel(&path), n + 1));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "axi-design has square corners; rounding must be 0.0:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn the_exclusion_lists_still_point_at_real_files() {
    // A path-based exclusion for a file that has been renamed or split
    // silently matches nothing, and the guard keeps passing over a file
    // nobody is guarding. This is the check that makes the lists rot
    // loudly instead of quietly.
    let root = repo_root();
    for group in [&DEFERRED[..], &PENDING[..], &ALLOWED[..]] {
        for p in group {
            assert!(
                root.join(p).is_file(),
                "{p} is listed in the guard's exclusions but does not exist; \
                 the file moved and the list did not"
            );
        }
    }
}

#[test]
fn the_guard_is_actually_guarding_something() {
    // If the walk, the roots, or the allowlists ever conspire to leave
    // nothing guarded, both rules above pass vacuously. This is the
    // canary for that.
    let files = guarded_files();
    assert!(
        files.len() >= 2,
        "only {} file(s) guarded — check ROOTS and the exclusion lists: {:?}",
        files.len(),
        files.iter().map(|p| rel(p)).collect::<Vec<_>>()
    );
}
```

- [ ] **Step 2: Run the test**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test --test axi_guard_test`
Expected: PASS, 4 tests. With the six unconverted surfaces in `PENDING`, the files actually walked are `src/ui/axi.rs`, `src/ui/icons.rs`, `src/ui/mod.rs`, `src/ui/theme.rs`, `src/ui/series.rs`, `src/ui/tile_cache.rs`, `src/wvw_teams.rs` and `src/fight_composition.rs`. Tasks 1-3 left all of them clean, and `theme.rs`/`series.rs` are checked for square corners even though the colour rule skips them.

Note that only `src/ui/map.rs` in `DEFERRED` is load-bearing: `ROOTS` is `src/ui`, so the four `src/map/` entries are never walked in the first place. They stay listed anyway — when someone widens `ROOTS`, the exclusion is already there and already explained, and `the_exclusion_lists_still_point_at_real_files` keeps them honest meanwhile.

- [ ] **Step 3: Prove the guard fails on a violation**

A guard that has never failed is a guard nobody has tested. Temporarily append a violation to a guarded file and confirm both rules catch it:

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
printf '\nconst GUARD_CANARY: [f32; 4] = [0.1, 0.2, 0.3, 1.0];\n' >> src/ui/tile_cache.rs
cargo test --test axi_guard_test 2>&1 | grep -q 'colour literals belong' && echo "colour rule fires"
git checkout -- src/ui/tile_cache.rs
```
Expected: prints `colour rule fires`, and the `git checkout` restores the file. Confirm `git status` is clean afterward.

- [ ] **Step 4: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add tests/axi_guard_test.rs
git commit -m "$(cat <<'EOF'
test(design): guard the axi contract against the UI sources

Reads the UI sources as text and fails on a colour literal outside
theme.rs/series.rs or a non-zero rounding value. Reading source rather
than calling it is what lets this run on Linux while the modules it
guards are cfg(windows).

PENDING carries the five surfaces not yet converted; each conversion
commit deletes its own entry. DEFERRED carries the map, which is
deferred rather than exempt — an undocumented exclusion becomes a
permanent one. A third test asserts every listed path still exists, so
a rename makes the lists rot loudly instead of quietly.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: The accent setting — config field and options combo

**Files:**
- Modify: `src/config.rs:7-36` (struct), `src/config.rs:65-83` (`Default`), `src/ui/options.rs:27-38`, `src/ui/options.rs:76`
- Modify: `tests/axi_guard_test.rs` (delete the `src/ui/options.rs` entry from `PENDING`)
- Test: `tests/axi_config_test.rs` (create)

**Interfaces:**
- Consumes: `theme::{DEFAULT_ACCENT_ID, accent, accent_index, accent_labels, ACCENTS}` from Task 1.
- Produces: `Config::accent: String`. Tasks 6-9 read it as `theme::accent(&config.accent)`.

- [ ] **Step 1: Write the failing test**

Create `tests/axi_config_test.rs`:

```rust
//! The accent setting's persistence. The upgrade path that matters is
//! a v0.4.4 `axipulse.json` with no `accent` key at all — the
//! overwhelmingly common one.

use arcdps_axipulse::config::Config;
use arcdps_axipulse::ui::theme;

#[test]
fn the_default_accent_matches_the_desktop_app() {
    assert_eq!(Config::default().accent, "emerald-mint");
    assert_eq!(Config::default().accent, theme::DEFAULT_ACCENT_ID);
}

#[test]
fn a_v0_4_4_config_loads_and_picks_up_the_default_accent() {
    // Verbatim shape of a config written before this field existed.
    // Every other field must survive; only `accent` defaults.
    let json = r#"{
        "cbtlogs_path": "C:\\logs",
        "debug_logging": true,
        "show_pulse": false,
        "pulse_pos": [120.0, 340.0],
        "show_timeline": true,
        "timeline_pos": null,
        "timeline_layers": { "health": false, "damage_dealt": true },
        "toggle_visibility_hotkey": "Ctrl+Shift+P",
        "show_notifications": false,
        "notifications_pos": null,
        "show_team_bar": true,
        "team_bar_pos": [10.0, 20.0],
        "team_bar_compact": true,
        "auto_update_check": false
    }"#;
    let cfg: Config = serde_json::from_str(json).expect("v0.4.4 config must still load");
    assert_eq!(cfg.accent, theme::DEFAULT_ACCENT_ID, "missing accent defaults");
    // Nothing else was reset on the way through.
    assert_eq!(cfg.cbtlogs_path, "C:\\logs");
    assert!(cfg.debug_logging);
    assert!(!cfg.show_pulse);
    assert_eq!(cfg.pulse_pos, Some((120.0, 340.0)));
    assert_eq!(cfg.toggle_visibility_hotkey, "Ctrl+Shift+P");
    assert!(!cfg.show_notifications);
    assert!(cfg.show_team_bar);
    assert_eq!(cfg.team_bar_pos, Some((10.0, 20.0)));
    assert!(cfg.team_bar_compact);
    assert!(!cfg.auto_update_check);
    assert!(!cfg.timeline_layers.health);
    assert!(cfg.timeline_layers.damage_dealt);
}

#[test]
fn an_accent_round_trips_through_json() {
    let mut cfg = Config::default();
    cfg.accent = "violet-purple".to_string();
    let s = serde_json::to_string(&cfg).expect("serialize");
    let back: Config = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(back.accent, "violet-purple");
    assert_eq!(theme::accent(&back.accent), theme::rgb(0x8b, 0x5c, 0xf6));
}

#[test]
fn a_hand_edited_or_downgraded_accent_still_renders() {
    // Someone edits axipulse.json by hand, or runs a build that had a
    // twelfth accent. Neither may panic inside the render callback.
    for id in ["", "not-an-accent", "Violet-Purple", "null"] {
        let mut cfg = Config::default();
        cfg.accent = id.to_string();
        assert_eq!(theme::accent(&cfg.accent), theme::accent(theme::DEFAULT_ACCENT_ID));
        assert!(theme::accent_index(&cfg.accent) < theme::ACCENTS.len());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test --test axi_config_test`
Expected: FAIL to compile — `no field 'accent' on type 'Config'`.

- [ ] **Step 3: Add the field**

In `src/config.rs`, add to the `Config` struct, immediately after `auto_update_check`:

```rust
    /// axi-design accent id, from the design package's `accents.json`.
    /// Drives chrome only — never a team, boon or metric colour.
    /// Unknown values fall back to emerald-mint at read time rather
    /// than at load time, so a config we did not write is never
    /// silently rewritten.
    pub accent: String,
```

and to `impl Default for Config`, after `auto_update_check: true,`:

```rust
            accent: crate::ui::theme::DEFAULT_ACCENT_ID.to_string(),
```

The struct already carries `#[serde(default)]`, so a v0.4.4 `axipulse.json` with no `accent` key loads and picks up the default. No migration is needed.

- [ ] **Step 4: Run the config test to verify it passes**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test --test axi_config_test`
Expected: PASS, 4 tests.

- [ ] **Step 5: Add the combo to the options pane**

In `src/ui/options.rs`, inside `render_options_end`, insert between the `ui.separator();` that opens the function's body and the `render_hotkey_row` call — i.e. immediately after `let mut dirty = false;`:

```rust
    // Accent. The pane itself keeps arcdps's styling: it lives inside
    // arcdps's own window, and restyling it would make it look foreign
    // in its host. It gains this one control and nothing else.
    let labels = crate::ui::theme::accent_labels();
    let mut idx = crate::ui::theme::accent_index(&config.accent);
    ui.text("Accent:");
    ui.same_line();
    let _w = ui.push_item_width(180.0);
    if ui.combo_simple_string("##axi-accent", &mut idx, &labels) {
        if let Some((id, _, _)) = crate::ui::theme::ACCENTS.get(idx) {
            config.accent = (*id).to_string();
            dirty = true;
        }
    }
    drop(_w);
    ui.same_line();
    ui.text_colored(
        crate::ui::theme::accent(&config.accent),
        "\u{25c6}",
    );
    ui.text_disabled(
        "Colours the overlay's chrome — tabs, panels, highlights. Team, \
         boon and metric colours never change.",
    );
    ui.separator();
```

The `\u{25c6}` diamond is the family motif (rule 10) and doubles as a live swatch of the chosen accent.

- [ ] **Step 6: Move the path-status colours onto status tokens**

`src/ui/options.rs:76` picks the combat-log path's status colour from two
literals:

```rust
    let status_color = if exists { [0.40, 0.92, 0.55, 1.0] } else { [1.00, 0.40, 0.40, 1.0] };
```

Replace it with:

```rust
    let status_color = if exists { crate::ui::theme::OK } else { crate::ui::theme::DANGER };
```

This is not restyling the pane — "found" and "not found" are a status
(rule 5), and these are the only two colour literals in the file. The
pane's layout, controls and arcdps-inherited styling are untouched.

Then delete the `"src/ui/options.rs"` entry from `PENDING` in
`tests/axi_guard_test.rs` and drop the array length to 5.

- [ ] **Step 7: Verify it compiles for Windows**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo dll-check`
Expected: no errors. `combo_simple_string(&str, &mut usize, &[&str]) -> bool` is the same call `render_fight_picker_combo` in `src/ui/main.rs:287` already uses, so the signature is proven in this codebase.

- [ ] **Step 8: Run the full host suite**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test`
Expected: PASS, including the guard now walking `src/ui/options.rs`.

- [ ] **Step 9: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add src/config.rs src/ui/options.rs tests/axi_config_test.rs tests/axi_guard_test.rs
git commit -m "$(cat <<'EOF'
feat(design): let the accent be chosen and persisted

Config gains an `accent` id and the arcdps options pane gains a combo
for it, with a diamond swatch of the current choice. The struct already
carried #[serde(default)], so a v0.4.4 axipulse.json loads without the
field and picks up emerald-mint — no migration, and every other field
survives, which the test pins against a verbatim v0.4.4 config.

The pane keeps arcdps's own styling: it lives inside arcdps's window,
and restyling it would make it look foreign in its host. Its only other
change is the combat-log path's found/not-found colour moving onto the
OK and DANGER tokens — a status, and the file's only two literals.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: `main.rs` — the shell

**Files:**
- Modify: `src/ui/main.rs` (whole file; palette at 16-19, style push at 38-51, header at 116-160, update pill at 162-196, parsing pulse at 198-240, tabs at 295-317)
- Modify: `tests/axi_guard_test.rs` (delete the `src/ui/main.rs` entry from `PENDING`, and drop the array length to 4)

**Interfaces:**
- Consumes: `theme::{TRANSPARENT, SURFACE, SURFACE_RAISED, GROUND, INK_LINE, RULE, TEXT, TEXT_DIM, TEXT_FAINT, OK, WARN, DANGER, META, ACCENT_INK, ALPHA_READING, BORDER_PANEL, BORDER_CONTROL, OFFSET_CONTROL, OFFSET_CONTROL_HOVER, with_alpha, push_form, accent}`; `axi::{Rect, panel_inward, diamond}`.
- Produces: `pulse::render_content(ui, fight, idx, derived, accent)` and `timeline::render_content(ui, fight, idx, derived, accent, layers)` are called with a new trailing-or-inserted `accent: [f32; 4]` argument. Tasks 7 and 8 add those parameters; until then this task passes them and the build breaks on Windows, so **this task adds the parameter to the call site only after Tasks 7 and 8 have added it to the signature** — see Step 4.

**Ordering note for the executor:** Tasks 6, 7 and 8 share the `render_content` signatures. Do them in the order 7, 8, 6 if you prefer each task to end green on `cargo dll-check`; the plan lists them 6, 7, 8 for narrative order. Either order ends in the same place, and `cargo test` (host) is unaffected because these modules are `#[cfg(windows)]`. Whichever order you choose, say so in your report.

- [ ] **Step 1: Replace the palette and style push**

Delete `src/ui/main.rs:14-19` (the `// --- palette` comment block and its four consts). Replace the `style_tokens` / `color_tokens` blocks at lines 37-51 with:

```rust
    // imgui paints WindowBg itself and clips the window draw list to
    // the window rect, so we take the fill away from imgui and paint
    // block, fill and outline ourselves, INWARD from the window edge.
    // Consequence: this surface's opacity is one constant, not two
    // code paths.
    let form_tokens = theme::push_form(ui);
    let accent = theme::accent(&config.accent);
    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([
            14.0 + theme::BORDER_PANEL,
            12.0 + theme::BORDER_PANEL,
        ])),
        ui.push_style_var(StyleVar::ItemSpacing([8.0, 8.0])),
    ];
    let color_tokens = [
        ui.push_style_color(StyleColor::WindowBg, theme::TRANSPARENT),
        ui.push_style_color(StyleColor::TitleBg, theme::SURFACE),
        ui.push_style_color(StyleColor::TitleBgActive, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::Separator, theme::RULE),
        ui.push_style_color(StyleColor::Text, theme::TEXT),
        ui.push_style_color(StyleColor::TextDisabled, theme::TEXT_FAINT),
        ui.push_style_color(StyleColor::Button, theme::SURFACE),
        ui.push_style_color(StyleColor::ButtonHovered, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::ButtonActive, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::FrameBg, theme::SURFACE),
        ui.push_style_color(StyleColor::FrameBgHovered, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::FrameBgActive, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::PopupBg, theme::SURFACE),
        ui.push_style_color(StyleColor::Header, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::HeaderHovered, theme::SURFACE_RAISED),
        ui.push_style_color(StyleColor::HeaderActive, theme::SURFACE_RAISED),
    ];
```

Add to the `use` block at the top:

```rust
use crate::ui::axi::{self, Rect};
use crate::ui::theme;
```

Extend the existing `arcdps::imgui` import to `use arcdps::imgui::{Condition, StyleColor, StyleVar, Ui};` (unchanged — it already lists all four).

Note the padding: the body content must clear our own 4px outline, so `WindowPadding` grows by `BORDER_PANEL`. Without this the first line of text sits on the outline.

- [ ] **Step 2: Paint the inward panel as the first thing in the window body**

At the top of the `window.opened(&mut open).build(|| { ... })` closure, before `render_header(ui, state);`, insert:

```rust
        // Our own fill, inside the window and inside the clip rect.
        // ALPHA_READING: this is a surface you open in order to read.
        let win = Rect::at(ui.window_pos(), ui.window_size());
        axi::panel_inward(ui, win, theme::with_alpha(theme::SURFACE, theme::ALPHA_READING));
```

- [ ] **Step 3: Pop the form tokens**

After the existing `for tok in style_tokens { tok.pop(); }` at the end of `render`, add:

```rust
    for tok in form_tokens { tok.pop(); }
```

- [ ] **Step 4: Thread the accent to the tab content**

Change the `match tab` arms to pass `accent`:

```rust
        match tab {
            TopTab::Pulse    => crate::ui::pulse::render_content(ui, fight, idx, derived, accent),
            TopTab::Timeline => crate::ui::timeline::render_content(ui, fight, idx, derived, accent, &mut config.timeline_layers),
            TopTab::Map      => crate::ui::map::render_content(ui, fight, idx, derived, &record.log_path),
        }
```

`accent` is a `[f32; 4]` copied out of `config` before the closure, so this does not conflict with the `&mut config.timeline_layers` borrow. The `Map` arm is unchanged — the map is deferred.

- [ ] **Step 5: Redraw the header wordmark and parsing indicator**

In `render_header`, replace the brand text draw (lines 143-147) so `Pulse` wears the accent rather than a hard-coded green. `render_header` gains an `accent: [f32; 4]` parameter and `render` passes it:

```rust
fn render_header(ui: &Ui, state: &AppState, accent: [f32; 4]) {
```

```rust
    {
        let draw = ui.get_window_draw_list();
        draw.add_text([x, brand_text_y], theme::TEXT, "Axi");
        draw.add_text([x + axi_w, brand_text_y], accent, "Pulse");
    }
```

`render_parsing_pulse` gains the same parameter. Replace its halo and fallback-dot blocks (lines 206-232) with a square halo and a diamond fallback — the language has no round shapes:

```rust
    let draw = ui.get_window_draw_list();
    if let Some(handle) = icon {
        let half = icon_size * 0.5;
        let x0 = cx - half;
        let y0 = cy - half;
        // Square halo: the language has no soft round glow. Scaled
        // with the beat so the pulse still reads on a busy backdrop.
        let halo_r = icon_size * 0.65 + 2.0 * intensity;
        let halo = theme::with_alpha(accent, 0.10 + 0.25 * intensity * alpha);
        draw.add_rect([cx - halo_r, cy - halo_r], [cx + halo_r, cy + halo_r], halo)
            .filled(true)
            .build();
        // No tint on the texture itself — the vendored imgui binding's
        // image-tint path appears to crash the host under Wine when
        // exercised. The icon was rasterised already coloured so
        // untinted is fine.
        draw.add_image(handle.tex, [x0, y0], [x0 + icon_size, y0 + icon_size]).build();
    } else {
        // Bundled icon not loaded yet (D3D11 device unavailable on the
        // first frame). Fall back to the family motif so we still show
        // *some* parsing indicator.
        drop(draw);
        axi::diamond(ui, [cx, cy], 10.0 + 4.0 * intensity, theme::with_alpha(accent, alpha));
    }
```

and its label:

```rust
    let label = "parsing...";
    let text_color = theme::with_alpha(theme::TEXT_FAINT, 0.60 + 0.35 * intensity);
    ui.get_window_draw_list()
        .add_text([cx + base_size * 0.6 + 8.0, label_y], text_color, label);
```

`drop(draw)` before calling `axi::diamond` matters: `get_window_draw_list` hands out a mutable borrow, and `diamond` takes its own.

- [ ] **Step 6: Recolour the update pill by status role**

Replace the `(label, color)` match in `render_update_pill` (lines 165-177):

```rust
    let (label, color) = match &st {
        UpdateState::Available { tag, .. } =>
            (format!("Update available \u{00b7} {tag}"), theme::OK),
        UpdateState::Downloading { pct, .. } if pct.is_finite() =>
            (format!("Downloading... {:.0}%", pct), theme::META),
        UpdateState::Downloading { .. } =>
            ("Downloading...".to_string(), theme::META),
        UpdateState::Installed { tag } =>
            (format!("Restart GW2 to load {tag}"), theme::WARN),
        UpdateState::Failed { msg } =>
            (format!("Update failed: {msg}"), theme::DANGER),
        _ => return,
    };
```

Rule 6: `META` is the cool ink reserved for meta, and an update's download progress is exactly that — information about the software rather than about the fight.

- [ ] **Step 7: Redraw the top tabs as blocked controls**

Replace `render_top_tabs` entirely. The selected tab wears the accent with near-black ink; the rest are surface with dim text. Both get the control-step block, and only the hovered one lifts — they are buttons, so the lift is honest.

```rust
fn render_top_tabs(ui: &Ui, accent: [f32; 4]) {
    let mut current = TOP_TAB.lock().ok().map(|g| *g).unwrap_or(TopTab::Pulse);
    let tabs = [("Pulse", TopTab::Pulse), ("Timeline", TopTab::Timeline), ("Map", TopTab::Map)];
    let n = tabs.len();
    let pad = [14.0_f32, 6.0_f32];
    let h = ui.text_line_height() + pad[1] * 2.0;

    for (i, (label, tab)) in tabs.iter().enumerate() {
        let selected = current == *tab;
        let w = ui.calc_text_size(label)[0] + pad[0] * 2.0;
        let origin = ui.cursor_screen_pos();
        let r = Rect::at(origin, [w, h]);

        // Hit-test before drawing so the lift lands on the same frame
        // the cursor arrives, rather than one frame late.
        ui.set_cursor_screen_pos(origin);
        let clicked = ui.invisible_button(format!("##top-tab-{label}"), [w, h]);
        let hovered = ui.is_item_hovered();

        let fill = if selected { accent } else { theme::SURFACE };
        axi::card(ui, r, fill, hovered);
        let ink = if selected { theme::ACCENT_INK } else { theme::TEXT_DIM };
        let text_pos = [origin[0] + pad[0], origin[1] + pad[1]];
        ui.get_window_draw_list().add_text(text_pos, ink, *label);

        if clicked { current = *tab; }
        if i + 1 < n {
            // Leave room for the block so tabs do not overlap it.
            ui.set_cursor_screen_pos([origin[0] + w + theme::OFFSET_CONTROL + 6.0, origin[1]]);
        } else {
            ui.set_cursor_screen_pos([origin[0], origin[1] + h + theme::OFFSET_CONTROL]);
        }
    }
    if let Ok(mut g) = TOP_TAB.lock() { *g = current; }
}
```

Update the call in `render` to `render_top_tabs(ui, accent);` and the header call to `render_header(ui, state, accent);`.

- [ ] **Step 8: Remove `main.rs` from the guard's `PENDING`**

In `tests/axi_guard_test.rs`, change the `PENDING` declaration to:

```rust
const PENDING: [&str; 4] = [
    "src/ui/pulse.rs",
    "src/ui/timeline.rs",
    "src/ui/team_bar.rs",
    "src/ui/notifier.rs",
];
```

- [ ] **Step 9: Run the guard and the host suite**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test`
Expected: PASS. The guard now walks `src/ui/main.rs` and must find no colour literal and no non-zero rounding in it. If it reports a literal, that is a real miss — fix it rather than adding an `axi-guard: allow`.

- [ ] **Step 10: Verify it compiles for Windows**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo dll-check`
Expected: no errors — but only once Tasks 7 and 8 have added the `accent` parameter to `pulse::render_content` and `timeline::render_content`. If you are doing this task before those, expect exactly two argument-count errors here and no others, and record that in your report.

- [ ] **Step 11: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add src/ui/main.rs tests/axi_guard_test.rs
git commit -m "$(cat <<'EOF'
feat(design): redraw the shell in axi-design

WindowBg goes transparent and the window paints its own block, fill and
outline INWARD from its edge: the window draw list is clipped to the
window rect, so an outward block would be cut off. The desktop app's
.axi-window uses the same trick.

Tabs become blocked controls that lift on hover — they are buttons, so
the lift is honest. The wordmark's "Pulse" and the parsing indicator
wear the configured accent; the update pill moves to status inks, with
META for download progress since that is meta about the software rather
than about the fight.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: `pulse.rs` — the five subviews

**Files:**
- Modify: `src/ui/pulse.rs` (palette 20-33; `render_content` 42; `render_tab_strip` 56-86; inline inks at 218, 259, 264, 269, 284-286; `draw_value_bar` 322-387; `hero_banner` 481-535; `draw_2col_card_grid` 537-588; `draw_skill_bar` 590-653; `boon_color` 655-672; `draw_boon_bar` 674-735; `render_fight_composition` 737-845; `draw_class_chips` 847-908)
- Modify: `tests/axi_guard_test.rs` (delete the `src/ui/pulse.rs` entry from `PENDING`)

**Interfaces:**
- Consumes: `theme::{SURFACE, SURFACE_RAISED, GROUND, INK_LINE, TEXT, TEXT_DIM, TEXT_FAINT, ACCENT_INK, ALPHA_READING, BORDER_CONTROL, OFFSET_CONTROL, OFFSET_CONTROL_HOVER}`; `axi::{self, Rect}`; `series::{boon, METRIC_*}`.
- Produces: `pub fn render_content(ui: &Ui, fight: &FightData, idx: usize, derived: &Derived, accent: [f32; 4])` — `main.rs` calls this (Task 6).

- [ ] **Step 1: Replace the palette with imports**

Delete `src/ui/pulse.rs:18-33` (the `// --- palette` comment and all thirteen consts). Add to the `use` block:

```rust
use crate::ui::axi::{self, Rect};
use crate::ui::series;
use crate::ui::theme;
```

Then replace every removed name at its call sites, mechanically:

| Removed | Replacement | Why |
|---|---|---|
| `BG_CARD` | `theme::SURFACE_RAISED` | A card sits on the shell's surface, so it is the raised step. |
| `BG_CARD_BORDER` | *(deleted)* | Soft white-alpha borders are gone; `axi::card` draws a near-black outline instead. |
| `TEXT_PRIMARY` | `theme::TEXT` | |
| `TEXT_SECONDARY` | `theme::TEXT_DIM` | |
| `TEXT_MUTED` | `theme::TEXT_FAINT` | |
| `ACCENT_DAMAGE` | `series::METRIC_DAMAGE` | Domain metric ink (Task 3). |
| `ACCENT_DOWN` | `series::METRIC_DOWN` | |
| `ACCENT_SUPPORT` | `series::METRIC_SUPPORT` | |
| `ACCENT_CLEANSE` | `series::METRIC_CLEANSE` | |
| `ACCENT_DEFEND` | `series::METRIC_DEFEND` | |
| `ACCENT_SUCCESS` | `series::METRIC_SUCCESS` | |
| `ACCENT_DANGER` | `series::METRIC_DAMAGE` | `ACCENT_DANGER` and `ACCENT_DAMAGE` held the same value and both mean "this hurt". |
| `ACCENT_NEUTRAL` | `series::METRIC_NEUTRAL` | |

Also replace the three inline literals:
- line 218, `[0.65, 0.51, 0.91, 1.0]` → `series::METRIC_BARRIER`
- lines 259/264/269, the three `0.55`-alpha bar tints `[0.40,0.85,0.65,0.55]` / `[0.97,0.55,0.42,0.55]` / `[0.65,0.51,0.91,0.55]` → `series::METRIC_SUPPORT` / `series::METRIC_DOWN` / `series::METRIC_BARRIER`, **at full alpha**. Rule 2: no colour at partial opacity over the ground. These are bar fills, and a bar's length already carries the quantity (rule 7), so the dimming was doing no work.
- Every text drop-shadow `[0.0, 0.0, 0.0, 0.55]` / `[0.0, 0.0, 0.0, 0.5]` (lines 367, 375, 517, 568, 633, 641, 717, 723) → **delete the shadow draw entirely**. A drop shadow is a blur, and the language replaced blurs with hard offset blocks. The fills these sit on are opaque now, so the shadow's legibility job is done by the fill.

- [ ] **Step 2: Add the accent parameter and redraw the tab strip**

```rust
pub fn render_content(ui: &Ui, fight: &FightData, idx: usize, derived: &Derived, accent: [f32; 4]) {
    render_tab_strip(ui, accent);
```

Replace `render_tab_strip`'s body with the same blocked-control shape the shell's top tabs use, so the two strips are one design rather than two:

```rust
fn render_tab_strip(ui: &Ui, accent: [f32; 4]) {
    let mut current = SUBVIEW.lock().ok().map(|g| *g).unwrap_or(Subview::Overview);
    let tabs = [
        ("Overview", Subview::Overview),
        ("Damage", Subview::Damage),
        ("Support", Subview::Support),
        ("Defense", Subview::Defense),
        ("Boons", Subview::Boons),
    ];
    let n = tabs.len();
    let pad = [12.0_f32, 5.0_f32];
    let h = ui.text_line_height() + pad[1] * 2.0;

    for (i, (label, tab)) in tabs.iter().enumerate() {
        let selected = current == *tab;
        let w = ui.calc_text_size(label)[0] + pad[0] * 2.0;
        let origin = ui.cursor_screen_pos();
        let r = Rect::at(origin, [w, h]);

        ui.set_cursor_screen_pos(origin);
        let clicked = ui.invisible_button(format!("##pulse-tab-{label}"), [w, h]);
        let hovered = ui.is_item_hovered();

        let fill = if selected { accent } else { theme::SURFACE };
        axi::card(ui, r, fill, hovered);
        let ink = if selected { theme::ACCENT_INK } else { theme::TEXT_DIM };
        ui.get_window_draw_list()
            .add_text([origin[0] + pad[0], origin[1] + pad[1]], ink, *label);

        if clicked { current = *tab; }
        if i + 1 < n {
            ui.set_cursor_screen_pos([origin[0] + w + theme::OFFSET_CONTROL + 5.0, origin[1]]);
        } else {
            ui.set_cursor_screen_pos([origin[0], origin[1] + h + theme::OFFSET_CONTROL]);
        }
    }
    if let Ok(mut g) = SUBVIEW.lock() { *g = current; }
}
```

Apply the identical treatment to `render_support_mode_toggle` (lines 274-300), which pushes the same three `Button*` colours: it becomes two blocked controls, selected one wearing `accent` with `ACCENT_INK`. Repeat the code rather than factoring a helper out of `pulse.rs` into `axi.rs` — the two strips differ in their state cell and their id prefix, and a four-argument generic "tab strip" that takes a lock guard is harder to read than the twenty lines above.

- [ ] **Step 3: Redraw the bar rows through `axi::bar`**

In `draw_value_bar` (lines 322-387), replace the three `add_rect` calls (card fill, bar, soft border) with one `axi::bar` and keep the rest of the row's layout untouched:

```rust
    {
        let track = Rect::at(cursor, [avail, row_h]);
        axi::bar(ui, track, frac, bar_color);

        let draw = ui.get_window_draw_list();
        let pad_left = 6.0 + theme::BORDER_CONTROL;
        let mut text_x = cursor[0] + pad_left;
        if let Some(handle) = icon {
            let icon_h = row_h - 4.0 - theme::BORDER_CONTROL * 2.0;
            let icon_w = (icon_h * handle.aspect).max(1.0);
            let icon_y = cursor[1] + 2.0 + theme::BORDER_CONTROL;
            draw.add_image(handle.tex, [text_x, icon_y], [text_x + icon_w, icon_y + icon_h]).build();
            text_x += icon_w + 6.0;
        }
        let text_y = cursor[1] + (row_h - ui.text_line_height()) * 0.5;
        draw.add_text([text_x, text_y], theme::TEXT, name);

        let pad_right = 10.0 + theme::BORDER_CONTROL;
        let pct_label = if pct >= 0.1 { format!("{:.1}%", pct) } else { String::new() };
        let val_w = ui.calc_text_size(value)[0];
        let pct_w = ui.calc_text_size(&pct_label)[0];
        draw.add_text([cursor[0] + avail - pad_right - val_w, text_y], theme::TEXT, value);
        if !pct_label.is_empty() {
            draw.add_text(
                [cursor[0] + avail - pad_right - val_w - 14.0 - pct_w, text_y],
                theme::TEXT_DIM, &pct_label,
            );
        }
    }
```

`axi::bar` draws a `GROUND` track, the `bar_color` fill at `frac`, and a near-black control-weight outline inset by `thickness / 2`. Icon and text insets grow by `BORDER_CONTROL` so nothing sits on the outline.

Apply the same substitution to `draw_skill_bar` (lines 590-653) and `draw_boon_bar` (lines 674-735). Both have the identical three-rect prelude; both become one `axi::bar` call plus their own icon/label layout. `draw_boon_bar`'s `color` argument now comes from `series::boon(name)`.

- [ ] **Step 4: Delete `boon_color` and call `series::boon`**

Delete `pulse.rs:655-672` (`fn boon_color`) entirely. At its two call sites in `render_boons`, replace `boon_color(name)` with `series::boon(name)`.

- [ ] **Step 5: Redraw the hero banner**

Replace `hero_banner`'s drawing block (lines 495-534) with a blocked panel, a filled accent-stripe left edge, and no tint wash — rule 2 forbids the `0.08`-alpha accent tint and rule 3 replaces the `0.22`-alpha border with a near-black outline:

```rust
    let r = Rect::at([x, y], [avail, HERO_H]);
    // A display card: no hover lift. A hover state on something you
    // cannot click is a lie about affordance.
    axi::card(ui, r, theme::SURFACE_RAISED, false);

    let draw = ui.get_window_draw_list();
    // Accent stripe on the left edge, inside our own outline.
    let stripe_x = x + theme::BORDER_CONTROL;
    draw.add_rect(
        [stripe_x, y + theme::BORDER_CONTROL],
        [stripe_x + 4.0, y + HERO_H - theme::BORDER_CONTROL],
        accent,
    ).filled(true).build();

    let pad_x = 16.0 + theme::BORDER_CONTROL;
    let pad_y = 10.0 + theme::BORDER_CONTROL;
    let line_h = ui.text_line_height();

    // Uppercase eyebrow: with one font face at one size, case and
    // colour are the whole weight hierarchy available.
    draw.add_text([x + pad_x, y + pad_y], accent, label.to_uppercase());

    let primary_y = y + pad_y + line_h + 8.0;
    draw.add_text([x + pad_x, primary_y], theme::TEXT, primary);

    if !secondary.is_empty() {
        let sec_w = ui.calc_text_size(secondary)[0];
        draw.add_text([x + avail - pad_x - sec_w, primary_y + 2.0], theme::TEXT_DIM, secondary);
    }
    if let Some(rk) = rank {
        let rw = ui.calc_text_size(rk)[0];
        draw.add_text(
            [x + avail - pad_x - rw, y + HERO_H - pad_y - line_h],
            theme::TEXT_FAINT, rk,
        );
    }

    ui.dummy([avail, HERO_H + theme::OFFSET_CONTROL]);
```

`hero_banner`'s existing `accent: [f32; 4]` parameter is reused: its callers pass a metric ink today. Leave those call sites alone — the stripe is the metric's colour, which is correct under rule 9's "domain palettes enter separately". Rename the parameter to `ink` for clarity and update the five call sites in `render_overview`, `render_damage`, `render_support` and `render_defense` accordingly. The `dummy` grows by `OFFSET_CONTROL` so the next element clears the block.

- [ ] **Step 6: Redraw the 2-column card grid**

In `draw_2col_card_grid` (lines 537-588), replace the two `add_rect` calls (fill + soft border) with one `axi::card(ui, Rect::at([x, y], [col_w, CARD_H]), theme::SURFACE_RAISED, false)`, keep the accent stripe (moved inside the outline by `BORDER_CONTROL`, as above), drop the value's drop shadow, and pad the text insets by `BORDER_CONTROL`. Grow the trailing `dummy` height by `OFFSET_CONTROL` so the block clears. Rename the tuple's `accent` binding to `ink`.

Also widen the grid's gap so blocks do not collide: change `const GAP: f32 = 8.0;` to

```rust
/// Gap between grid cells. Must exceed OFFSET_CONTROL or a cell's
/// offset block lands under its neighbour.
const GAP: f32 = 8.0 + theme::OFFSET_CONTROL;
```

- [ ] **Step 7: Redraw the composition chips**

In `render_fight_composition` (lines 785-786), the group header currently picks `bg` from two literals and `border` from either the group's colour or a white-alpha wash. Replace with:

```rust
            let bg = if active { theme::SURFACE_RAISED } else { theme::SURFACE };
```

and delete the `border` binding — `axi::card` draws the outline, and the group's own colour goes where it belongs: as a filled diamond before the label.

```rust
            axi::card(ui, Rect::at(origin, [w, h]), bg, hovered);
            axi::diamond(
                ui,
                [origin[0] + theme::BORDER_CONTROL + 7.0, origin[1] + h * 0.5],
                8.0,
                g.color,
            );
```

In `draw_class_chips` (line 886), replace the chip's `add_rect(...)` fill with `axi::card(ui, Rect::at([x, y], [chip.chip_w, chip_h]), theme::SURFACE, false)` and rename the function's `accent` parameter to `ink`.

- [ ] **Step 8: Use `axi::label` for the section eyebrows**

Replace `section_label`'s body (lines 474-477):

```rust
fn section_label(ui: &Ui, label: &str) {
    axi::label(ui, label);
    ui.dummy([0.0, 1.0]);
}
```

`axi::label` uppercases and draws in `TEXT_FAINT`, which is the closest reachable analogue of `--axi-t-eyebrow` with one font face. The eyebrow's `.1em` tracking is lost, not faked: imgui cannot letter-space, and drawing glyph-by-glyph to simulate it would cost per-frame time in a callback shared with the game.

- [ ] **Step 9: Remove `pulse.rs` from the guard's `PENDING`**

In `tests/axi_guard_test.rs`, drop the `"src/ui/pulse.rs"` entry and change the array length to 3.

- [ ] **Step 10: Run the guard and the host suite**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test`
Expected: PASS. The guard now walks `src/ui/pulse.rs`. Its failure message lists file:line for anything missed; fix each rather than adding an `axi-guard: allow`. This file held 49 of the 106 literals, so expect to iterate here.

- [ ] **Step 11: Verify it compiles for Windows**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo dll-check`
Expected: no errors in `pulse.rs`.

- [ ] **Step 12: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add src/ui/pulse.rs tests/axi_guard_test.rs
git commit -m "$(cat <<'EOF'
feat(design): redraw Pulse in axi-design

The 49 colour literals in this file are gone: chrome comes from theme,
metric and boon inks from series. Cards, the hero banner, the grid and
the three bar families all go through axi::card / axi::bar, so the
block-fill-outline sequence exists once.

Three things the language removes rather than ports: the hero banner's
0.08-alpha accent wash and 0.22-alpha border (rule 2 — no colour at
partial opacity over the ground), the 0.55-alpha bar tints (a bar's
length already carries the quantity), and every text drop shadow (a
blur, which the language replaced with hard offset blocks; the fills
are opaque now so it was doing no work).

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: `timeline.rs` — lanes, toggles, tooltip, inspector

**Files:**
- Modify: `src/ui/timeline.rs` (palette 9-22; `render_content` 31-37; `render_layer_toggles` 137-153; `render_time_axis` 155-174; `draw_hover_crosshair` 176-266; `draw_tooltip` 268-315; `draw_area_lane` 329-395; `draw_empty_lane` 397-413; `draw_boon_lane` 415-465; `draw_inspector_card` 549-578; `section_label` 586-588)
- Modify: `tests/axi_guard_test.rs` (delete the `src/ui/timeline.rs` entry from `PENDING`)

**Interfaces:**
- Consumes: `theme::*`, `axi::{self, Rect}`, `series::{METRIC_*}`.
- Produces: `pub fn render_content(ui: &Ui, fight: &FightData, idx: usize, derived: &crate::derived::Derived, accent: [f32; 4], layers: &mut crate::config::TimelineLayers)` — `main.rs` calls this (Task 6). Note `accent` goes **before** `layers`, so the `&mut` borrow stays last.

- [ ] **Step 1: Replace the palette with imports**

Delete `src/ui/timeline.rs:9-22` (all thirteen consts). Add:

```rust
use crate::ui::axi::{self, Rect};
use crate::ui::series;
use crate::ui::theme;
```

Substitutions:

| Removed | Replacement |
|---|---|
| `BG_CARD` | `theme::SURFACE_RAISED` |
| `BG_CARD_BORDER` | *(deleted — `axi::card` outlines)* |
| `TEXT_PRIMARY` | `theme::TEXT` |
| `TEXT_SECONDARY` | `theme::TEXT_DIM` |
| `TEXT_MUTED` | `theme::TEXT_FAINT` |
| `COLOR_HEALTH` | `series::METRIC_HEALTH` |
| `COLOR_DMG` | `series::METRIC_DAMAGE` |
| `COLOR_TAKEN` | `series::METRIC_DAMAGE_TAKEN` |
| `COLOR_DIST` | `series::METRIC_DISTANCE` |
| `COLOR_OFF` | `series::METRIC_OFF_BOONS` |
| `COLOR_DEF` | `series::METRIC_DEF_BOONS` |
| `COLOR_HEAL_IN` | `series::METRIC_HEAL_IN` |
| `COLOR_BARRIER_IN` | `series::METRIC_BARRIER_IN` |

And the four inline literals:
- line 205, the crosshair `[1.0, 1.0, 1.0, 0.35]` → `theme::RULE`. A crosshair is an annotation (rule 5), drawn at `theme::BORDER_HAIRLINE` thickness, at full strength (rule 2).
- line 298, the tooltip fill `[0.05, 0.06, 0.08, 0.95]` → `theme::SURFACE_RAISED` at `ALPHA_READING`, via `axi::card`.
- line 300, the tooltip border `[1.0, 1.0, 1.0, 0.10]` → deleted; `axi::card` outlines it.
- line 455, the boon-name drop shadow `[0.0, 0.0, 0.0, 0.55]` → the shadow draw is deleted, as in Task 7.

- [ ] **Step 2: Add the accent parameter and redraw the layer toggles**

```rust
pub fn render_content(
    ui: &Ui,
    fight: &FightData,
    idx: usize,
    derived: &crate::derived::Derived,
    accent: [f32; 4],
    layers: &mut crate::config::TimelineLayers,
) {
    render_layer_toggles(ui, layers, accent);
```

`render_layer_toggles` currently draws eight imgui checkboxes. Convert each to a blocked toggle chip so it matches the two tab strips — on wears `accent` + `ACCENT_INK`, off wears `SURFACE` + `TEXT_DIM`, and both lift on hover because they are clickable:

```rust
fn render_layer_toggles(ui: &Ui, layers: &mut crate::config::TimelineLayers, accent: [f32; 4]) {
    let pad = [10.0_f32, 4.0_f32];
    let h = ui.text_line_height() + pad[1] * 2.0;
    let row_x0 = ui.cursor_screen_pos()[0];
    let right = row_x0 + ui.content_region_avail()[0].max(120.0);
    let mut origin = ui.cursor_screen_pos();

    let mut entries: [(&str, &mut bool); 8] = [
        ("Health", &mut layers.health),
        ("Damage", &mut layers.damage_dealt),
        ("Taken", &mut layers.damage_taken),
        ("Distance", &mut layers.distance_to_tag),
        ("Off boons", &mut layers.offensive_boons),
        ("Def boons", &mut layers.defensive_boons),
        ("Healing in", &mut layers.incoming_healing),
        ("Barrier in", &mut layers.incoming_barrier),
    ];

    for (label, on) in entries.iter_mut() {
        let w = ui.calc_text_size(*label)[0] + pad[0] * 2.0;
        // Wrap before overrunning the content region, leaving room for
        // the offset block.
        if origin[0] > row_x0 && origin[0] + w + theme::OFFSET_CONTROL > right {
            origin = [row_x0, origin[1] + h + theme::OFFSET_CONTROL + 3.0];
        }
        ui.set_cursor_screen_pos(origin);
        let clicked = ui.invisible_button(format!("##tl-layer-{label}"), [w, h]);
        let hovered = ui.is_item_hovered();
        if clicked { **on = !**on; }

        let fill = if **on { accent } else { theme::SURFACE };
        axi::card(ui, Rect::at(origin, [w, h]), fill, hovered);
        let ink = if **on { theme::ACCENT_INK } else { theme::TEXT_DIM };
        ui.get_window_draw_list()
            .add_text([origin[0] + pad[0], origin[1] + pad[1]], ink, *label);

        origin = [origin[0] + w + theme::OFFSET_CONTROL + 5.0, origin[1]];
    }
    ui.set_cursor_screen_pos([row_x0, origin[1] + h + theme::OFFSET_CONTROL]);
    ui.dummy([0.0, 6.0]);
}
```

**Behaviour note, and it must hold:** the eight toggles keep their labels, their order, and the `TimelineLayers` fields they write. A checkbox becoming a chip changes how the control looks, not what it controls. If the current labels differ from the eight above, use the current ones — read them out of the file rather than trusting this table.

- [ ] **Step 3: Redraw the lanes**

In `draw_area_lane` (lines 329-395), the lane's backing rect becomes `axi::card(ui, Rect::at(origin, [lane_w, AREA_LANE_H]), theme::SURFACE_RAISED, false)` — a display surface, so no hover lift — and the area fill keeps its existing polyline/rect geometry at the metric ink's **full** alpha. Any place the existing code multiplies the accent's alpha down for the area fill, drop the multiplication: rule 2, and rule 7 means the shape already carries the quantity.

`draw_empty_lane` (397-413) gets the same card, with its reason string in `theme::TEXT_FAINT`.

`draw_boon_lane` (415-465) gets the same card; each boon row's filled segment keeps its ink and loses its drop shadow.

Lane labels go through `axi::label` so they read as eyebrows.

- [ ] **Step 4: Redraw the tooltip and inspector cards**

In `draw_tooltip` (268-315), replace the two `add_rect` calls with:

```rust
    axi::card(ui, Rect::at([tx, ty], [w, h]), theme::SURFACE_RAISED, false);
```

and inset its text by `theme::BORDER_CONTROL`. A tooltip is not clickable, so `hovered` is `false`.

In `draw_inspector_card` (549-578), the same: one `axi::card`, text inset by `BORDER_CONTROL`, drop shadows deleted, and the trailing `dummy` grown by `OFFSET_CONTROL`.

- [ ] **Step 5: Use `axi::label` and `axi::rule`**

Replace `section_label` (586-588) with `axi::label(ui, label);`. Replace the `ui.separator()` call in `render_content` (line 38) with an explicit `axi::rule` at hairline weight across the content region, so the divider is ours rather than imgui's:

```rust
    let sep_y = ui.cursor_screen_pos()[1];
    let sep_x0 = ui.cursor_screen_pos()[0];
    let sep_w = ui.content_region_avail()[0].max(0.0);
    axi::rule(ui, [sep_x0, sep_y], [sep_x0 + sep_w, sep_y]);
    ui.dummy([0.0, theme::BORDER_HAIRLINE + 4.0]);
```

- [ ] **Step 6: Remove `timeline.rs` from the guard's `PENDING`**

Drop the `"src/ui/timeline.rs"` entry; array length becomes 2.

- [ ] **Step 7: Run the guard and the host suite**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test`
Expected: PASS.

- [ ] **Step 8: Verify it compiles for Windows**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo dll-check`
Expected: no errors.

- [ ] **Step 9: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add src/ui/timeline.rs tests/axi_guard_test.rs
git commit -m "$(cat <<'EOF'
feat(design): redraw Timeline in axi-design

The eight metric inks now come from series, the chrome from theme, and
every lane, tooltip and inspector card from axi::card. The eight layer
checkboxes become blocked toggle chips matching the two tab strips —
same labels, same order, same TimelineLayers fields.

The hover crosshair moves to theme::RULE at hairline weight: it is an
annotation (rule 5), drawn at full strength (rule 2) rather than as
35%-alpha white.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: `team_bar.rs` and `notifier.rs` — the two HUD surfaces

**Files:**
- Modify: `src/ui/team_bar.rs` (consts 19-37; style push 51-58; `draw_header` 119-146; `lighten`/`darken` 148-156; `draw_bar` 157-277; `draw_legend` 279-321)
- Modify: `src/ui/notifier.rs` (style push 38-42; the `(bg_alpha, accent, label, body)` match 47-85; `WindowBg` push 86; the halo at 129-136)
- Modify: `tests/axi_guard_test.rs` (empty `PENDING`)

**Interfaces:**
- Consumes: `theme::{TRANSPARENT, SURFACE, GROUND, INK_LINE, TEXT, TEXT_DIM, TEXT_FAINT, ACCENT_INK, OK, ALPHA_HUD, BORDER_CONTROL, BORDER_PANEL, OFFSET_PANEL, with_alpha, accent, push_form}`; `axi::{self, Rect}`; `series::{TEAM_*}` via the unchanged `TeamColor::rgba()`.
- Produces: nothing new. `team_bar::render(ui, state, config)` and `notifier::render(ui, config)` keep their signatures — both already receive `config`, so each reads its own accent.

These are the two surfaces where **translucency does real work**: small, always-on, sitting over gameplay continuously rather than being opened and read. They fill at `ALPHA_HUD`. **Their ink is not scaled** — outlines, blocks and text draw fully opaque, because a translucent offset block with the battlefield moving inside it reads as a rendering fault rather than as a hard block.

- [ ] **Step 1: `team_bar.rs` — square the form and drop the gloss**

Replace the const block (19-37) with:

```rust
const BAR_WIDTH: f32 = 264.0;
const BAR_HEIGHT: f32 = 20.0;
/// Gap between segments; the dark track shows through.
const SEG_GAP: f32 = 2.0;
/// Floor on segment width so a 1-player team stays a legible pip
/// instead of a 2px sliver.
const MIN_SEG_WIDTH: f32 = 14.0;
```

`BAR_ROUNDING`, `GLOW_HEIGHT`, `GLOW_GAP`, `TRACK_BG`, `BAR_OUTLINE`, `SEG_TEXT` and the three `HEADER_*` consts all go. Delete `lighten` and `darken` (148-156) and every call to them: they existed to make a glossy pill, and the language has neither gloss nor pills. The track is `theme::GROUND`, the outline is near-black via `axi::bar`, segment text is `theme::ACCENT_INK`, and the header ramp is `theme::TEXT_FAINT` / `TEXT_DIM` / `TEXT`.

**`config.team_bar_compact` keeps its exact meaning**: compact still hides the map header, the glow strip and the legend. The glow strip no longer exists in either mode, so in compact mode that clause is now vacuous — leave the flag, the checkbox and the option text alone. Task 10 records this in the residuals note.

- [ ] **Step 2: `team_bar.rs` — own the fill**

Replace the style/colour push (51-58) with:

```rust
    let form_tokens = theme::push_form(ui);
    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([
            12.0 + theme::BORDER_PANEL,
            9.0 + theme::BORDER_PANEL,
        ])),
    ];
    let color_tokens = [
        ui.push_style_color(StyleColor::WindowBg, theme::TRANSPARENT),
        ui.push_style_color(StyleColor::Text, theme::TEXT),
        ui.push_style_color(StyleColor::TextDisabled, theme::TEXT_FAINT),
    ];
```

and as the first statement inside `win.build(|| { ... })`:

```rust
        // HUD surface: translucent fill, fully opaque ink. A see-through
        // offset block with the battlefield moving inside it reads as a
        // rendering fault, not as a hard block.
        let win_rect = Rect::at(ui.window_pos(), ui.window_size());
        axi::panel_inward(ui, win_rect, theme::with_alpha(theme::SURFACE, theme::ALPHA_HUD));
```

Pop `form_tokens` alongside the existing token pops at the end of `render`.

- [ ] **Step 3: `team_bar.rs` — square the bar**

In `draw_bar` (157-277), the track becomes one `axi::bar`-style track drawn directly (the segments are not one fraction, so `axi::bar` does not fit): a `theme::GROUND` filled rect, the three segments as square filled rects at their team inks, and one near-black outline via `axi::outline_path` at `theme::BORDER_CONTROL`:

```rust
    let track = Rect::at(origin, [BAR_WIDTH, BAR_HEIGHT]);
    let draw = ui.get_window_draw_list();
    draw.add_rect(track.min, track.max, theme::GROUND).filled(true).build();

    // ... existing segment-width maths, unchanged ...
    for (color, seg) in segments {
        let seg_rect = Rect::new([sx, track.min[1]], [sx + seg_w, track.max[1]]);
        if !seg_rect.is_degenerate() {
            draw.add_rect(seg_rect.min, seg_rect.max, color.rgba()).filled(true).build();
        }
        sx += seg_w + SEG_GAP;
    }

    let path = axi::outline_path(track, theme::BORDER_CONTROL);
    if !path.is_degenerate() {
        draw.add_rect(path.min, path.max, theme::INK_LINE)
            .thickness(theme::BORDER_CONTROL)
            .build();
    }
```

Delete the glow strip block (around 233-244) and its `GLOW_*` geometry: it was a blurred highlight line, and the language has no blurs. The segment count labels stay where they are, in `theme::ACCENT_INK` — the team inks are all bright enough on this ground for near-black to be the right ink on them, the same reason `--axi-accent-ink` is near-black.

**The segment-width maths, the `MIN_SEG_WIDTH` floor, the `SEG_GAP`, and which counts appear are all unchanged.** This step changes corners and colours only.

- [ ] **Step 4: `team_bar.rs` — the legend's "you" marker**

In `draw_legend` (279-321), replace the two white-alpha text colours at line 315 with `theme::TEXT` (yours) and `theme::TEXT_DIM` (everyone else's), and draw each team's swatch as an `axi::diamond` at `color.rgba()` rather than a rounded rect — rule 10, and it is the one place a swatch is small enough for the motif to read.

- [ ] **Step 5: `notifier.rs` — own the fill and square the toast**

Replace the style push (38-42) with:

```rust
    let form_tokens = theme::push_form(ui);
    let style_tokens = [
        ui.push_style_var(StyleVar::WindowPadding([
            10.0 + theme::BORDER_PANEL,
            8.0 + theme::BORDER_PANEL,
        ])),
    ];
```

Replace the `WindowBg` push at line 86 with `let bg = ui.push_style_color(StyleColor::WindowBg, theme::TRANSPARENT);` and paint our own fill as the first statement inside `win.build`:

```rust
        // HUD surface. The toast's own fade multiplies ALPHA_HUD, so a
        // finishing toast dissolves without ever dimming its ink.
        let win_rect = Rect::at(ui.window_pos(), ui.window_size());
        axi::panel_inward(
            ui,
            win_rect,
            theme::with_alpha(theme::SURFACE, theme::ALPHA_HUD * fade_scale),
        );
```

where `fade_scale` replaces the current `bg_alpha` plumbing. Rework the match at 47-85 so it yields a `fade_scale: f32` in `[0, 1]` instead of an absolute `bg_alpha`, preserving every existing timing:

```rust
    let (fade_scale, label_ink, label, body): (f32, [f32; 4], &str, Vec<(String, [f32; 4])>) = match msg {
        Msg::Parsing(name) => {
            // Same 3 rad/s pulse as before, expressed as a fraction of
            // the HUD alpha rather than as an absolute alpha.
            let t = ui.time() as f32;
            let pulse = 0.5 + 0.5 * ((t * 3.0).sin());
            let scale = (0.55 + 0.15 * pulse) / 0.82;
            (scale.min(1.0), accent, "Parsing...", vec![(name.to_string(), theme::TEXT)])
        }
        Msg::Parsed(toast, age) => {
            // Linear fade across the final 1.5s of the linger window,
            // unchanged.
            let remain = (PARSED_LINGER_SECS - age).max(0.0);
            let scale = (remain / 1.5).min(1.0) * (0.75 / 0.82);
            let mut segs: Vec<(String, [f32; 4])> = Vec::new();
            let teams = toast.counts.segments();
            if !toast.map.is_empty() {
                let sep = if teams.is_empty() { "" } else { " \u{00b7} " };
                segs.push((format!("{}{}", toast.map, sep), theme::TEXT));
            }
            for (i, (color, count)) in teams.iter().enumerate() {
                if i > 0 { segs.push((" v ".to_string(), theme::TEXT)); }
                segs.push((count.to_string(), color.rgba()));
            }
            (scale.min(1.0), theme::OK, "Parsed", segs)
        }
        Msg::Placeholder => (
            0.70 / 0.82,
            accent,
            "AxiPulse Notifier",
            vec![(
                "Drag to reposition. Hidden until a parse fires.".to_string(),
                theme::TEXT,
            )],
        ),
    };
```

with `let accent = theme::accent(&config.accent);` before the match. `Msg::Parsed` uses `theme::OK` because a completed parse is a status (rule 5); `Parsing` and `Placeholder` use the accent because they are transient chrome states.

- [ ] **Step 6: `notifier.rs` — the halo and the body fade**

Replace the rounded halo (129-136) with a square one at the accent, and keep the icon untinted for the documented Wine reason:

```rust
            let halo_r = icon_size * 0.65;
            let halo = theme::with_alpha(label_ink, 0.18);
            draw.add_rect([cx - halo_r, cy - halo_r], [cx + halo_r, cy + halo_r], halo)
                .filled(true)
                .build();
```

The body-segment fade at the end of the closure keeps its behaviour but reads from `fade_scale` directly:

```rust
        // Ink is not scaled by the surface's HUD alpha, only by the
        // toast's own fade — otherwise a fully-visible toast would
        // draw dim text.
        for (text, color) in &body {
            let c = theme::with_alpha(*color, color[3] * fade_scale);
            draw.add_text([bx, body_y], c, text);
            bx += ui.calc_text_size(text)[0];
        }
```

Pop `form_tokens` alongside the existing pops.

- [ ] **Step 7: Empty the guard's `PENDING`**

```rust
/// Surfaces not yet converted. Each conversion task deletes its own
/// entry; the list reaching empty is the conversion being done. Unlike
/// DEFERRED this is temporary scaffolding — if you are reading this
/// after the branch merged and it is non-empty, something was skipped.
const PENDING: [&str; 0] = [];
```

- [ ] **Step 8: Run the guard and the host suite**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test`
Expected: PASS. All five surfaces are now guarded. `the_exclusion_lists_still_point_at_real_files` iterates an empty `PENDING`, which is fine.

- [ ] **Step 9: Verify it compiles for Windows**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo dll-check`
Expected: no errors anywhere. This is the first point at which the whole conversion type-checks together.

- [ ] **Step 10: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add src/ui/team_bar.rs src/ui/notifier.rs tests/axi_guard_test.rs
git commit -m "$(cat <<'EOF'
feat(design): redraw the team bar and notifier as HUD surfaces

These two are small, always-on, and sit over gameplay rather than being
opened and read, so translucency does real work here: they fill at
ALPHA_HUD while the three reading surfaces went opaque. Their ink is
NOT scaled — a see-through offset block with the battlefield moving
inside it reads as a rendering fault, not as a hard block.

The glossy pill becomes a square segmented bar in a GROUND track with
one near-black outline; lighten/darken and the blurred glow strip are
deleted, since the language has neither gloss nor blurs. Segment
widths, the MIN_SEG_WIDTH floor, SEG_GAP and every fade timing are
unchanged. team_bar_compact keeps its meaning.

The guard's PENDING list is now empty.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: Close the gate — docs, residuals, and the release build

**Files:**
- Modify: `CLAUDE.md`
- Modify: `tests/axi_guard_test.rs` (add the closed-gate assertion)
- Create: `docs/superpowers/2026-09-24-axi-design-overlay-residuals.md`

**Interfaces:**
- Consumes: everything. Produces nothing the crate imports.

- [ ] **Step 1: Assert the gate is closed**

Append to `tests/axi_guard_test.rs`:

```rust
#[test]
fn the_conversion_is_complete() {
    // PENDING is scaffolding for the conversion branch, not a
    // permanent exemption list. If this fails, a surface was added to
    // it and never converted — the guard is passing over a file
    // nobody is guarding, which is the exact failure DEFERRED's
    // comment warns about.
    assert!(
        PENDING.is_empty(),
        "unconverted surfaces still in PENDING: {PENDING:?}"
    );
    // The five converted surfaces are actually being walked.
    let guarded: Vec<String> = guarded_files().iter().map(|p| rel(p)).collect();
    for surface in [
        "src/ui/options.rs",
        "src/ui/main.rs",
        "src/ui/pulse.rs",
        "src/ui/timeline.rs",
        "src/ui/team_bar.rs",
        "src/ui/notifier.rs",
    ] {
        assert!(
            guarded.contains(&surface.to_string()),
            "{surface} is not being guarded; guarded set is {guarded:?}"
        );
    }
}
```

- [ ] **Step 2: Record the contract in `CLAUDE.md`**

Append this section to `CLAUDE.md`:

```markdown
## Design

- **axi-design**, the Axi app family's design language: flat and
  outlined, square corners, hard offset blocks instead of blurred
  shadows, saturated inks at full strength only. The normative spec is
  `docs/RULES.md` in the axi-design repo.
- **`src/ui/theme.rs` is the only file permitted a chrome colour
  literal**; `src/ui/series.rs` is the only file permitted a domain one
  (WvW teams, boons, metric inks). `theme.rs` holds no domain colour and
  `series.rs` holds no chrome colour — that split is what makes "the
  accent drives chrome only" checkable.
- **Draw raised elements through `src/ui/axi.rs`**, never by hand.
  `panel` / `card` are the two form weight steps; `panel_inward` is for
  a window, whose draw list is clipped to itself so its block must go
  inward. imgui centres `add_rect` stroke on the path, so outlines inset
  by `thickness / 2` — `axi::outline_path` does this and is host-tested.
- **Geometry comes from one knob.** `theme::SCALE` multiplies every
  border and offset. If the form reads too heavy at your GW2 UI scale,
  change `SCALE` and nothing else; never tune the two steps
  independently.
- **Opacity splits by surface role.** Shell, Pulse and Timeline are
  reading surfaces and fill opaque (`ALPHA_READING`). The team bar and
  notifier are HUD, sit over gameplay continuously, and fill at
  `ALPHA_HUD`. **Ink is never scaled on either** — a translucent offset
  block reads as a rendering fault.
- **Accents** come from `theme::ACCENTS`, carried by value from the
  design package's `accents.json`. The picker is in the arcdps options
  pane, the choice persists in `axipulse.json`, and the default is
  `emerald-mint`, matching the desktop app. An unknown id falls back
  rather than panicking: this runs inside GW2's render callback.
- **No font work is possible.** imgui's atlas is built before rendering
  and exposes no `FontId` to push. Hierarchy is colour, case, spacing
  and rules only; the eyebrow's `.1em` tracking is lost, not faked.
- **`tests/axi_guard_test.rs` enforces the contract** by reading the UI
  sources as text, so it runs on Linux despite the modules being
  `#[cfg(windows)]`. Run `cargo test`. `src/ui/map.rs` and `src/map/`
  are excluded: deferred, not exempt.
```

- [ ] **Step 3: Write the residuals note**

Create `docs/superpowers/2026-09-24-axi-design-overlay-residuals.md`:

```markdown
# axi-design overlay conversion — residuals

**Date:** 2026-09-24
**Spec:** `docs/superpowers/specs/2026-09-24-axi-design-overlay-conversion-design.md`

What this conversion knowingly ships without. Each item is a decision
with a reason, not an oversight — recorded so it is found as a residual
rather than reported as a bug.

## 1. The map tab does not match its neighbours

`src/ui/map.rs` (1062 lines) and `src/map/` keep their pre-conversion
theme: rounded corners, soft white-alpha borders, their own colour
constants. Opening the Map tab after Pulse or Timeline is a visible
discontinuity, and it will be visible every time.

Deferred rather than half-done on purpose: a partly-converted map reads
as a bug, where an unconverted one reads as a deferral. The guard test
excludes it by path, in a `DEFERRED` array whose comment says so.

**To close:** convert `src/ui/map.rs` and the four files under
`src/map/` in one pass, then delete the `DEFERRED` entries.

## 2. The eyebrow's letter-spacing is lost, not faked

axi-design's `--axi-t-eyebrow` carries `.1em` tracking. imgui cannot
letter-space, and drawing glyph-by-glyph to simulate it would cost
per-frame time inside a render callback shared with the game's render
thread. `axi::label` gives uppercase and `TEXT_FAINT` and stops there.

**To close:** nothing to do short of owning the font atlas, which the
vendored arcdps crate does not expose.

## 3. The whole type scale is unavailable

One font face at one size. `vendor/arcdps/src/` contains no font
references, and imgui's atlas must be built before rendering — `Ui`
exposes no mutable atlas access at draw time and no `FontId` to push.
Weight and size hierarchy is expressed through colour, case, vertical
spacing and rules instead.

## 4. Hover lifts snap

imgui is immediate-mode; there is no transition machinery to drive. This
is the same end state the desktop app produces under
`prefers-reduced-motion`, so it is a legitimate rendering of rule 4
rather than a compromise — but it is a harder edge than the web UI's.

Reduced-motion detection is not implemented for the same reason: with no
transitions, `SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION)` would
have nothing to suppress. It is reachable from a DLL if that changes.

## 5. `team_bar_compact`'s glow clause is now vacuous

The option's text says compact mode hides "the map header, glow, and
legend". The blurred glow strip is deleted in both modes — the language
has no blurs — so that clause describes something that no longer exists.
The flag, the checkbox and the text are deliberately unchanged: the
option still does what a user expects it to, and rewording it was out of
this conversion's scope.

**To close:** reword the two option strings in `src/ui/options.rs` and
the module doc in `src/ui/team_bar.rs`.

## 6. `SCALE` is unvalidated at overlay size

axi-design's 4px border / 6px offset panel step was drawn for a 1280px
page, not a 300px overlay window. `theme::SCALE` exists to retune it in
one place, but the shipped value of `1.0` is a guess until someone has
looked at it in-game at their own GW2 UI scale.

**To close:** look at it, and if it reads heavy, lower `SCALE`. Never
tune the two steps independently.
```

- [ ] **Step 4: Run the full host suite**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo test`
Expected: PASS, including `the_conversion_is_complete`.

- [ ] **Step 5: Build the release DLL**

Run: `cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse && cargo dll`
Expected: builds `target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll`. **Do not** use the `x86_64-pc-windows-gnu` target — that binary links but crashes on load inside GW2.

- [ ] **Step 6: Commit**

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
git add CLAUDE.md tests/axi_guard_test.rs docs/superpowers/2026-09-24-axi-design-overlay-residuals.md
git commit -m "$(cat <<'EOF'
docs(design): close the axi conversion gate

The guard now asserts PENDING is empty and that all five converted
surfaces are actually being walked — an allowlist nobody empties is an
allowlist that becomes permanent.

CLAUDE.md records the contract for the next person: where colour
literals may live, why outlines inset by thickness/2, why the shell's
block goes inward, and why opacity splits by surface role.

The residuals note records the six things this ships without, the
deferred map first, so they are found as decisions rather than
reported as bugs.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 7: Visual verification in-game — hand back to your human partner**

This cannot be automated and must not be skipped. Report to your human partner that the branch is ready to look at, and give them the commands:

```bash
cd /var/home/mstephens/Documents/GitHub/arcdps-axipulse
cargo dll
./scripts/deploy.sh
```

**Never `cp` the DLL into `addons/` while GW2 is running** — under Wine `cp` truncates the live inode in place and corrupts pages GW2 has mmap'd as executable. `deploy.sh` writes to tmp and atomically renames.

What to look at, in the order that finds problems fastest:

1. **Any panel with a thin halo of ground colour between its fill and its outline** — that is the stroke-centring bug, and it would be wrong on every panel at once. The host tests pin `outline_path`, so this would mean a call site bypassing it.
2. **Whether the 4px/6px form reads too heavy at your GW2 UI scale.** If it does, lower `theme::SCALE` and rebuild. This is the spec's top risk and the reason `SCALE` exists.
3. **Any offset block clipped off at a window edge** — a surface painting its block outward instead of through `axi::panel_inward`.
4. **A single softened corner anywhere** — a rounding style var the guard's text scan cannot see because it is set outside the guarded files.
5. **The three reading surfaces going opaque.** This is the biggest perceptual change in the conversion and the most likely thing to want revisiting. It is one constant per surface (`ALPHA_READING`).
6. **The team bar and notifier over actual gameplay** — legible, still translucent, and no ink dimmed.
7. **Switch the accent through several of the eleven** and confirm chrome changes while no team, boon or timeline metric colour moves.
8. **Open the Map tab** and confirm it looks like the residual it is, not like a broken screen.
```
