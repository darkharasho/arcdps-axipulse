//! ImGui overlay rendering for axipulse. Windows-only submodules are
//! gated on `cfg(windows)` because `arcdps::imgui` ships Windows-only.
//! Pure-logic modules (e.g. `map`) are always compiled for host tests.

pub mod map;

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
pub mod tile_cache;
#[cfg(windows)]
pub mod timeline;
