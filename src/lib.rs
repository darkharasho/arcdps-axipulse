//! arcdps_axipulse: post-fight personal performance overlay.

pub mod config;
pub mod diag;
pub mod ei_bundle;
pub mod ei_model;
pub mod ei_parser;
pub mod ei_settings;
pub mod pulse_metrics;
pub mod state;

#[cfg(windows)]
pub mod plugin;
#[cfg(windows)]
pub mod watcher;

#[cfg(windows)]
arcdps::export! {
    name: "axipulse",
    sig: 0x4A1B0DBE,
    init: plugin::init,
    release: plugin::release,
}
