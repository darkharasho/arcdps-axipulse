# axi-design conversion — arcdps-axipulse overlay

**Date:** 2026-09-24
**Status:** approved

## Intent

The desktop AxiPulse app was redrawn in axi-design in v0.3.0. This plugin still
carries its own ad-hoc dark theme: rounded corners, soft white-alpha borders,
and 106 colour literals spread across five UI files with no shared theme
module. The two halves of the product do not look related.

This spec converts the plugin's reading and HUD surfaces to axi-design so the
in-game overlay and the desktop app read as one product, and routes every
colour through one module so they can stay that way.

**This is a reskin, not a refactor.** Every number the plugin shows today, it
shows tomorrow, in the same place, computed the same way. No behaviour change
is in scope beyond the accent setting this spec adds.

## The design language

axi-design is flat and outlined: near-black outlines and hard offset blocks
instead of blurs, square corners, saturated inks at full strength or not at
all. The normative spec is `docs/RULES.md` in the axi-design repo. The rules
this conversion leans on most:

- **Rule 2** — no colour at partial opacity over the ground.
- **Rule 3** — every raised element is outlined and blocked, at exactly two
  weight steps.
- **Rule 4** — hover lifts, and only on interactive things.
- **Rule 5** — filled means status, outlined means annotation.
- **Rule 6** — one cool ink (`--axi-meta`) reserved for meta.
- **Rule 7** — a quantity is drawn as length, never intensity.
- **Rule 9** — a chart's ink is the accent; domain palettes enter separately.
- **Rule 10** — the diamond is the family motif.

## Scope

**In — the three reading surfaces** (opaque):

| Surface | File | Lines | Colour literals today |
|---|---|---|---|
| Shell: window, tab bar, header | `src/ui/main.rs` | 322 | 21 |
| Pulse | `src/ui/pulse.rs` | 908 | 49 |
| Timeline | `src/ui/timeline.rs` | 595 | 17 |

**In — the two HUD surfaces** (translucent, see *Opacity* below):

| Surface | File | Lines | Colour literals today |
|---|---|---|---|
| Team bar | `src/ui/team_bar.rs` | 321 | 12 |
| Parse notifier toast | `src/ui/notifier.rs` | 175 | 7 |

**In — the accent setting only:** `src/ui/options.rs` gains a combo box to
pick the accent, and `src/config.rs` gains the field to persist it. The pane
itself keeps arcdps's styling — see *Out of scope*.

**In — domain colour relocation:** the four WvW team colours in
`src/wvw_teams.rs:40` and the one in `src/fight_composition.rs:33` move to the
new `src/ui/series.rs`. Their values do not change.

**Out of scope:**

- **The map.** `src/ui/map.rs` (1062 lines) and `src/map/` keep their current
  theme and their own colour constants. They are explicitly **not**
  half-converted: a partly-converted map is worse than an unconverted one,
  because it reads as a bug rather than as a deferral. The guard test
  (below) excludes them by path.
- **The arcdps options pane's styling.** It lives inside arcdps's own window
  and inherits arcdps's style. Restyling it would make it look foreign in its
  host. It gains one control and nothing else.
- **`src/ui/icons.rs`.** A D3D11 texture cache for skill and buff icons. It
  contains no colours and draws no chrome.
- **Fonts and the type scale.** See *Typography* — structurally unavailable.
- **Transitions and animation.** imgui is immediate-mode; there is no
  transition machinery to drive.
- **Reduced-motion detection.** With no transitions, there is nothing for it
  to suppress. `SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION)` is
  reachable from a DLL if this ever changes.

## Opacity

axi-design assumes an opaque ground with opaque panels on it. Here the ground
is Guild Wars 2, so the assumption needs a ruling rather than a port.

The plugin today is uniformly *nominally* translucent — windows at 0.92, cards
at 0.95. That is 5–8% see-through: not enough to track anything through a
panel, but enough to cost legibility when a number is read against a bright
moving effect. It pays transparency's cost and buys none of its benefit.
Occlusion is already solved properly by `toggle_visibility_hotkey` and the
per-window `show_*` flags.

The ruling splits by how a surface is used:

- **Reading surfaces — opaque.** Shell, Pulse, Timeline. You open these to
  read them. They fill solid, exactly like the desktop app, and the language
  lands at full strength. `ALPHA_READING = 1.0`.
- **HUD surfaces — translucent.** Team bar and notifier are small, always-on,
  and sit over gameplay continuously rather than being opened and read. The
  team bar is a glanceable strip; the notifier is a transient toast that
  should not punch a solid hole in the screen for three seconds. Here
  translucency does real work. `ALPHA_HUD = 0.82`, matching the team bar's
  current value.

**In both cases the ink is never scaled.** Outlines, offset blocks and text
draw fully opaque, so structure reads regardless of the fill behind it. A
translucent offset block — with the battlefield moving inside it — is the
hard-block effect dissolving into what looks like a rendering fault.

This leaves rule 2 carrying a documented exception on two small surfaces
rather than a blanket waiver across the plugin. That is the honest trade and
it is recorded here deliberately.

## Architecture

### `src/ui/theme.rs` — chrome

The only file in the plugin permitted a chrome colour literal, mirroring the
rule `tokens.css` follows in the web package.

```rust
// The axi surface and text ramp. Values are the axi-design tokens; the
// comment names the token each one is.
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
```

`rgb` is a `const fn` converting 8-bit channels to imgui's `[f32; 4]` at
alpha 1.0. Writing the tokens as hex keeps them diffable against
`accents.json` and `tokens.css` by eye; `[0.133, 0.094, 0.114]` is not.

The eleven accents, by value, so plugin and desktop agree without a shared
file:

```rust
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

/// Resolve the configured accent. An unknown id — a hand-edited or
/// downgraded config — falls back to the default rather than panicking:
/// this runs inside a render callback, and a panic there takes GW2's
/// frame with it.
pub fn accent(id: &str) -> [f32; 4];
```

Accent ink (text drawn on an accent fill) is `INK_LINE`, as
`--axi-accent-ink` resolves to `--axi-ink-line`. Every accent in the list is
bright enough on this ground for near-black to be the right choice.

Geometry, all derived from one knob:

```rust
/// Every border and offset below is this times its token value. If the
/// form reads too heavy at your GW2 UI scale, change this and nothing
/// else. Never tune the two steps independently: the point of two steps
/// is that a panel outweighs a control, and independent tuning destroys
/// that relationship.
pub const SCALE: f32 = 1.0;

pub const BORDER_PANEL: f32   = 4.0 * SCALE; // --axi-border-panel
pub const OFFSET_PANEL: f32   = 6.0 * SCALE; // --axi-offset-panel
pub const BORDER_CONTROL: f32 = 3.0 * SCALE; // --axi-border-control
pub const OFFSET_CONTROL: f32 = 3.0 * SCALE; // --axi-offset-control
pub const OFFSET_PANEL_HOVER: f32   = 10.0 * SCALE;
pub const OFFSET_CONTROL_HOVER: f32 =  6.0 * SCALE;
pub const BORDER_HAIRLINE: f32 = 2.0 * SCALE; // prose-rule weight ONLY

pub const ALPHA_READING: f32 = 1.0;
pub const ALPHA_HUD: f32 = 0.82;

/// Pushed as WindowBg so imgui paints nothing and we own the fill. Not a
/// colour in the language's sense — the absence of one.
pub const TRANSPARENT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
```

`--axi-radius` and `--axi-radius-sm` are both `0` in the language and that is
a contract, not a default. Expressed here as a helper that pushes `0.0` to all
seven rounding style vars:

```rust
/// Push the axi form's style vars: zero rounding everywhere, no imgui
/// borders (we draw our own outlines). Returns the tokens; drop them to
/// pop. Covers all seven rounding vars — a missed one shows up as a
/// single softened widget that is very hard to spot in a screenshot.
pub fn push_form(ui: &Ui) -> Vec<StyleStackToken<'_>>;
```

### `src/ui/series.rs` — domain colours

The data's colours, not the system's. Never recoloured by the accent, mirroring
`src/renderer/themes/series.css` in the desktop app.

Holds: the three WvW team colours (moved from `wvw_teams.rs:40`) and the
no-team colour (from `fight_composition.rs:33`); the boon colours currently
returned by `pulse.rs:655`; and the eight timeline metric colours currently at
`timeline.rs:15-22`.

Red, green and blue are the game's team identities. Recolouring them by the
accent would make the team bar lie about which side is which — the same reason
the desktop app leaves profession colours alone.

**`theme.rs` holds no domain colour, and `series.rs` holds no chrome colour.**
That split is what makes "the accent drives chrome only" a checkable property
rather than an aspiration.

Note that professions need no entry: they are drawn as PNG icons from
`src/assets/classes/`, not as colours.

### `src/ui/axi.rs` — draw helpers

Six helpers, each wrapping one block-fill-outline sequence so five surfaces
do not hand-roll it five ways:

```rust
/// Draw a panel: the heavier of the two form steps.
pub fn panel(ui: &Ui, rect: Rect, fill: [f32; 4], hovered: bool);
/// Draw a card or chip: the lighter step.
pub fn card(ui: &Ui, rect: Rect, fill: [f32; 4], hovered: bool);
/// A quantity as length (rule 7): outlined track, filled bar.
pub fn bar(ui: &Ui, rect: Rect, frac: f32, ink: [f32; 4]);
/// The family motif (rule 10), as two triangles.
pub fn diamond(ui: &Ui, center: [f32; 2], size: f32, ink: [f32; 4]);
/// An eyebrow/label: uppercased, TEXT_FAINT.
pub fn label(ui: &Ui, text: &str);
/// An internal rule at hairline weight, inside an outlined panel.
pub fn rule(ui: &Ui, from: [f32; 2], to: [f32; 2]);
```

The block sequence is **block rect, then fill rect, then outline rect, in that
order**, all filled except the outline. Order matters: the block must be
painted before the fill or it covers it.

`Rect` is a plain `{ min: [f32; 2], max: [f32; 2] }` in this module — not an
imgui type — so the geometry functions compile and test on the host.

### Window chrome and the inward block

imgui paints `WindowBg` itself, and **the window draw list is clipped to the
window rect**, so an offset block drawn outside the window would be cut off.

The scheme, which is the one the desktop app already uses for its frameless
window (`.axi-window` draws its block inward):

1. `push_style_color(StyleColor::WindowBg, TRANSPARENT)` and
   `push_style_var(StyleVar::WindowBorderSize(0.0))`.
2. As the first thing in the window body, paint block, fill and outline
   **inset from the window edge** by `OFFSET_PANEL`.
3. Nested cards inside a panel have room around them, so they block outward
   normally.

Consequence, and it is a useful one: the three reading surfaces take their
opacity from *our* fill rect rather than from `WindowBg`. So opaque-vs-
translucent is one constant per surface, not two code paths.

### Stroke centring

**imgui centres `add_rect` stroke on the path.** A 4px outline straddles the
rect edge — 2px inside, 2px outside. An outline flush with its fill must be
inset by `thickness / 2.0`.

Getting this wrong produces a panel 2px larger than its fill with a halo of
ground colour between them: subtle enough to survive a screenshot review and
wrong on every panel in the plugin. It is called out here because it is the
single most likely defect in this conversion, and it is the thing the host
unit tests exist to pin.

## Typography

There is no font work available. `vendor/arcdps/src/` contains no font
references, and imgui's font atlas must be built before rendering — `Ui`
exposes no mutable atlas access at draw time and no `FontId` to push. One
face, one size.

So axi-design's weight and size hierarchy is expressed four other ways:

- **Colour** — `TEXT` → `TEXT_DIM` → `TEXT_FAINT`, which is already the
  existing pattern and survives the conversion unchanged.
- **Case** — uppercase for the eyebrow and label roles, the closest reachable
  analogue to `--axi-t-eyebrow` and `--axi-t-label`.
- **Spacing** — vertical rhythm around headings.
- **Rules** — `axi::rule()` at hairline weight.

The eyebrow's `.1em` tracking is **lost**, not faked. imgui cannot letter-space
and drawing glyph-by-glyph to simulate it would cost per-frame time in a render
callback shared with the game.

## Interaction

Hover lift (rule 4) applies only to genuinely interactive things — tabs,
buttons, the arcdps window-list checkboxes — and never to display cards. A
hover state on a thing you cannot click is a lie about affordance.

The lift deepens the offset from `OFFSET_*` to `OFFSET_*_HOVER` and **snaps**:
imgui is immediate-mode with no transition machinery. This is the same end
state the desktop app produces under `prefers-reduced-motion`, so it is a
legitimate rendering of the rule rather than a compromise.

## Testing

### Host-side `cargo test`

Runs on Linux without GW2, covering the pure logic:

- **Stroke inset** — the outline rect for a given fill rect and thickness is
  inset by exactly `thickness / 2.0` on all four sides.
- **Accent lookup** — each of the eleven ids resolves to its expected value;
  an unknown id, the empty string, and a differently-cased id all resolve to
  `emerald-mint` rather than panicking.
- **Alpha policy** — a reading surface resolves to `1.0`, a HUD surface to
  `0.82`, and ink is `1.0` on both.
- **Geometry derivation** — the two form steps stay ordered under a changed
  `SCALE`: panel border > control border, panel offset > control offset.

This requires the geometry and lookup functions to be host-compilable. They
carry no `#[cfg(windows)]`; only the functions that touch `Ui` do. That is the
same split `src/ui/map.rs` already uses and documents in `src/ui/mod.rs`.

### The guard test

A test that reads the in-scope UI source files **as text** and fails on:

- a colour literal outside `theme.rs` and `series.rs`
- a non-zero rounding value
- an `add_rect` call with a non-zero `rounding()`

This runs on Linux even though the UI modules are `#[cfg(windows)]`, because
it reads source rather than calling it. It is the parity of
`tests/renderer/tokens.test.ts` in the desktop app, and it is the only thing
that will stop the 106 literals creeping back one convenience constant at a
time.

It must exclude `src/ui/map.rs` and `src/map/` by path, and that exclusion
must carry a comment saying the map is deferred rather than exempt — an
undocumented exclusion becomes a permanent one.

### Visual verification

Manual, and there is no way around it: `cargo dll` → `scripts/deploy.sh` →
look at it in-game. Per `CLAUDE.md`, never `cp` the DLL into `addons/` while
GW2 is running.

This is a materially worse feedback loop than the desktop conversion had, and
it has a consequence for how the work should be planned: correctness of the
*mechanism* (inset maths, ordering, token values) must be pinned by host tests,
because the in-game pass can only catch things that look wrong, and only on
the screens you happen to open.

## Config

`src/config.rs` gains one field:

```rust
/// axi-design accent id from the design package's accents.json.
/// Unknown values fall back to emerald-mint at read time.
pub accent: String,
```

Default is `"emerald-mint"`, matching the desktop app. The struct already
carries `#[serde(default)]`, so an existing `axipulse.json` written by v0.4.4
loads without the field and picks up the default — no migration needed.

`src/ui/options.rs` gains a combo listing the eleven labels, writing the id
and marking the config dirty, in the same shape as the existing checkbox rows.

## Success criteria

1. The shell, Pulse and Timeline screens are recognisably the same design
   language as the desktop app at v0.3.0: square corners, near-black outlines,
   hard offset blocks, no soft borders.
2. Every number, label and layout position the plugin showed at v0.4.4, it
   still shows.
3. The team bar and notifier remain legible over gameplay and remain
   translucent.
4. Changing the accent in the arcdps options pane changes chrome, and changes
   no team colour, boon colour or timeline metric colour.
5. `cargo test` passes on the host, including the guard test.
6. `cargo dll` builds and the plugin loads in GW2 without a crash.

## Risks

- **The form may read too heavy at overlay scale.** axi-design's 4px/6px panel
  step was drawn for a 1280px page, not a 300px overlay window. Mitigated by
  the single `SCALE` knob; not eliminated, because we cannot know until it is
  in-game.
- **Stroke centring.** Highest-likelihood defect, pinned by host tests.
- **Opacity may still feel wrong.** Reading surfaces going fully opaque is the
  biggest perceptual change and the one most likely to want revisiting. It is
  one constant per surface.
- **The deferred map will look foreign.** After this lands, the map tab will
  visibly not match the two tabs beside it. That is the accepted cost of
  deferring it, and it should be recorded as a known residual rather than
  discovered as a bug.
