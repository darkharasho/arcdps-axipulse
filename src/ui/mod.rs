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

#[cfg(windows)]
pub mod icons;
#[cfg(windows)]
pub mod main;
#[cfg(windows)]
pub mod notifier;
#[cfg(windows)]
pub mod options;
#[cfg(windows)]
pub mod pulse;
#[cfg(windows)]
pub mod team_bar;
#[cfg(windows)]
pub mod tile_cache;
#[cfg(windows)]
pub mod timeline;
