//! arcdps_axipulse: post-fight personal performance overlay.

// Keep the plugin's allocations off the shared process heap (see
// Cargo.toml note on mimalloc); its lock is contended by the game.
#[cfg(windows)]
#[global_allocator]
static GLOBAL_ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod boon_uptime;
pub mod config;
pub mod derived;
pub mod diag;
pub mod ei_bundle;
pub mod ei_model;
pub mod ei_parser;
pub mod ei_settings;
pub mod fight_composition;
pub mod fight_data;
pub mod hotkey;
pub mod map;
pub mod parse;
pub mod pulse_metrics;
pub mod slim;
pub mod squad_rank;
pub mod state;
pub mod timeline_boons;
pub mod timeline_buckets;
pub mod timeline_distance;
pub mod timeline_health;
pub mod tile_fetcher;
pub mod top_heals;
pub mod top_skills;
pub mod updater;
pub mod wvw_teams;

#[cfg(windows)]
pub mod plugin;
pub mod ui;
#[cfg(windows)]
pub mod watcher;

#[cfg(windows)]
arcdps::export! {
    name: "axipulse",
    sig: 0x4A1B0DBE,
    init: plugin::init,
    release: plugin::release,
    imgui: plugin::imgui,
    options_windows: plugin::options_windows,
    options_end: plugin::options_end,
    wnd_nofilter: plugin::wnd_nofilter,
}
