## Project Context

A Rust ArcDPS plugin that runs the bundled Elite Insights CLI on each
new .evtc/.zevtc, deserialises the JSON, and (future) renders Pulse and
Timeline overlays.

## Build

Must be MSVC. Cross-compile from Linux with `cargo-xwin`:
- `cargo dll`        release artifact at `target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll`
- `cargo dll-dev`    unoptimised iteration build
- `cargo dll-check`  type-check only
- `cargo test`       host-side unit tests (non-cfg(windows) modules)

Never `cargo build --target x86_64-pc-windows-gnu` for the DLL — the
GNU binary links but crashes on load inside GW2.

## Deploying

Always use `./scripts/deploy.sh` (tmp + atomic rename). Never `cp` the
DLL straight into `addons/` while GW2 is running — under Wine, `cp`
truncates the existing inode in place and corrupts pages of the loaded
DLL that GW2 has mmap'd as executable.

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
- **One draw list at a time.** `ui.get_window_draw_list()` takes a
  process-global single-instance lock that is released only when the
  list drops, and it **panics** if a second list is acquired while the
  first is live — inside GW2's render callback, so the panic crosses the
  arcdps FFI boundary and takes the game with it. Confine each list to a
  bare block (or `drop` it) before doing anything else, and remember
  that every `axi::` helper takes one of its own: `axi::panel`, `card`,
  `chip`, `panel_inward`, `bar`, `diamond` and `rule` must never be
  called while you hold a list. This is the one invariant that actually
  broke during the conversion, and the type checker cannot see it.
- **Geometry comes from one knob.** `theme::SCALE` multiplies every
  border and offset. If the form reads too heavy at your GW2 UI scale,
  change `SCALE` and nothing else; never tune the two steps
  independently.
- **Opacity splits by surface role.** Shell, Pulse and Timeline are
  reading surfaces and fill opaque (`ALPHA_READING`). The team bar and
  notifier are HUD, sit over gameplay continuously, and fill at
  `ALPHA_HUD`. Ink is never scaled by the HUD alpha constant —
  `ALPHA_HUD` governs a surface's fill, never the text or outlines on
  it. A surface that animates as a whole (the notifier's dissolve) does
  carry its ink with it as it goes; that is the surface leaving, not
  ink drawn dim. `notifier.rs` divides its fade back out of the ink for
  exactly this reason — see `TOAST_PEAK_ALPHA`.
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
