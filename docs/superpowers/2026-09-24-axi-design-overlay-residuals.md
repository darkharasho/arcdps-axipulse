# axi-design overlay conversion — residuals

**Date:** 2026-09-24
**Spec:** `docs/superpowers/specs/2026-09-24-axi-design-overlay-conversion-design.md`

What this conversion knowingly ships without, plus what the review loop
parked along the way. Each item is a decision with a reason, not an
oversight — recorded so it is found as a residual rather than reported
as a bug.

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

## 5. `team_bar_compact` is now a vacuous option

The option's doc and UI string used to say compact mode hides "the map
header, glow, and legend". The blurred glow strip is deleted in both
modes — the language has no blurs — so the glow half of that claim
describes something that no longer exists. The user-visible string in
`src/ui/options.rs` was already corrected under Ruling 16 in Task 9 to
"Compact hides the map header and legend."

What remains is the flag's *behaviour*, not its wording: one arm of the
compact/non-compact split existed only to toggle a glow that this
conversion removed entirely, so `team_bar_compact` now does less than
its name implies. `draw_bar`'s `compact` parameter was dropped as
unused in Task 9 for the same reason — the option still flips the flag
and the checkbox, but the code path it once distinguished is gone.

**To close:** decide whether `team_bar_compact` still earns its keep as
a two-way option, or whether the map-header/legend hide should be its
own thing with a name that doesn't imply a glow.

## 6. `SCALE` is unvalidated at overlay size

axi-design's 4px border / 6px offset panel step was drawn for a 1280px
page, not a 300px overlay window. `theme::SCALE` exists to retune it in
one place, but the shipped value of `1.0` is a guess until someone has
looked at it in-game at their own GW2 UI scale.

**To close:** look at it, and if it reads heavy, lower `SCALE`. Never
tune the two steps independently.

## Known bugs (real, shipped)

- **`src/ui/pulse.rs:707`** — the composition pill row does not wrap;
  pills past the window's right edge are drawn outside it and clipped.
  **To close:** wrap the row or clip/measure before drawing past the
  edge.
- **Two per-frame `format!` allocations** — one in `pulse.rs`, one in
  the notifier's team-count path — inside a render callback shared with
  the game's render thread. **To close:** cache or avoid the allocation
  on the hot path.
- **`src/ui/notifier.rs`'s heartbeat halo draws at 0.18 alpha over the
  ground**, a genuine rule-2 breach (no colour at partial opacity over
  the ground). Pre-existing: it was 0.18 before the conversion too.
  What the conversion changed is only its shape (a `rounding(halo_r)`
  circle became a square) and its ink (`accent` became `label_ink`).
  Carried forward unchanged rather than silently altered, because
  fixing it means deciding what replaces a halo, which is a design
  question this conversion did not ask. **To close:** decide on a
  halo-free treatment for the heartbeat icon.

## Cosmetic, needs a look at GW2 UI scale

- The boon-name plate in `timeline.rs` reads as eight small dark
  rectangles across a boon row at small UI scale. `PLATE_PAD_X` /
  `PLATE_PAD_Y` are the knob; the plate exists because full-strength
  segments left the name at ~1.6:1 (Ruling 8). **To close:** retune the
  pad constants at in-game scale.
- The area-lane polyline in `timeline.rs` is near-invisible. **To
  close:** raise its stroke weight or ink strength.
- A 1px nick where a profession icon overdraws a control outline in
  `pulse.rs`. **To close:** adjust the icon's draw order or inset.
- The tab-strip `dummy` leaves 8px of trailing space (one
  `ItemSpacing.y`); ruled acceptable in Ruling 13. **To close:** no
  action needed unless revisited.
- imgui paints the title strip itself and we cannot outline it, so the
  shell's top edge is the one un-outlined edge in the app. **To close:**
  not fixable without owning window chrome imgui doesn't expose.
- Sub-pixel fringe where imgui's `ImFloor` on `InnerClipRect` disagrees
  with our unfloored rect. **Ruled: do NOT compensate** — rounding to
  match would desync the moment imgui changes its flooring.
- **Ruling 14:** the scrollbar gutter reads as floating on the game
  world, because the panel fill stops at `InnerRect` and the gutter is
  outside it. **To close:** extend the fill or accept the gutter as
  game-world-adjacent chrome.
- `theme::OK` is visually near-identical to the default `emerald-mint`
  accent, so the notifier's Parsing (accent) and Parsed (`OK`) states
  lose their colour distinction on the default accent. **To close:** pick
  a `OK` that separates from `emerald-mint` specifically, or key the
  states on something other than colour.

## Code hygiene

- `theme::with_alpha` is applied to ink in `notifier.rs` against its own
  doc comment, which said "for FILLS only". Resolved in this task: the
  doc now covers the whole-surface-dissolve case rather than leaving
  the call site looking like a violation.
- `axi.rs` emits `warning: unused import: super::theme` on host runs;
  the import is only used by the `#[cfg(windows)]` helpers and wants a
  matching cfg gate. **To close:** gate the import behind
  `#[cfg(windows)]`.
- `notifier.rs`'s `_icon_alpha` is computed and discarded (the icon's
  alpha is unused because the vendored binding's image-tint path
  appears to crash the host under Wine). Pre-existing — it was already
  underscore-prefixed before this branch. Record it; do not "fix" it by
  tinting the image.
- `ui.window_size()` lags one frame under `ALWAYS_AUTO_RESIZE`, so a HUD
  surface's block is one frame stale on the frame its content resizes.
  **To close:** none known that doesn't cost a frame of layout lag
  elsewhere.
- The `main.rs` panel-rect comment cited imgui.cpp "8003-8006" for the
  `DC.CursorPos = DC.CursorStartPos` step; the correct line is 8007.
  Fixed in this task, since that comment is load-bearing (the third
  attempt at that derivation; two earlier ones were wrong) and a reader
  who checks the line and finds nothing would distrust the whole
  comment.
