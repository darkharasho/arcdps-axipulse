//! Panic-safe write-to-log helper. Mirrors team-breakdown's diag.rs in
//! spirit but minimal: arcdps's `log` macros already route to arcdps.log
//! when the `log` feature is enabled in Cargo.toml.

use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(v: bool) { ENABLED.store(v, Ordering::Relaxed); }
pub fn enabled() -> bool { ENABLED.load(Ordering::Relaxed) }

#[macro_export]
macro_rules! diag {
    ($($arg:tt)*) => {
        if $crate::diag::enabled() {
            ::log::warn!("axipulse: {}", format!($($arg)*));
        }
    }
}
