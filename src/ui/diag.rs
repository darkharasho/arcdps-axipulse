#![cfg(windows)]
//! Frame-counted trace helper. After a new fight lands the plugin
//! arms a short burst of frames during which `trace(...)` writes a
//! line to arcdps.log; outside the burst it costs an atomic load.

use std::sync::atomic::{AtomicU32, Ordering};

static FRAMES_LEFT: AtomicU32 = AtomicU32::new(0);

/// Begin a trace burst lasting `frames` imgui frames.
pub fn arm(frames: u32) {
    FRAMES_LEFT.store(frames, Ordering::Relaxed);
    log::warn!("axitrace: arm({frames})");
}

/// Decrement the counter once per imgui frame.
pub fn tick() {
    let n = FRAMES_LEFT.load(Ordering::Relaxed);
    if n > 0 { FRAMES_LEFT.store(n - 1, Ordering::Relaxed); }
}

pub fn active() -> bool {
    FRAMES_LEFT.load(Ordering::Relaxed) > 0
}

pub fn trace(msg: &str) {
    if active() { log::warn!("axitrace: {msg}"); }
}
