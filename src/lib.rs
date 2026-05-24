//! arcdps_axipulse: post-fight personal performance overlay.

pub mod boon_uptime;
pub mod config;
pub mod derived;
pub mod diag;
pub mod ei_bundle;
pub mod ei_model;
pub mod ei_parser;
pub mod ei_settings;
pub mod fight_composition;
pub mod hotkey;
pub mod map;
pub mod pulse_metrics;
pub mod self_identify;
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
