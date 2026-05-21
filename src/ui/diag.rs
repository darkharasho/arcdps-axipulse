#![cfg(windows)]
//! Frame-counted trace helper. After a new fight lands the plugin
//! arms a short burst of frames during which `trace(...)` writes a
//! line to both arcdps.log and a dedicated trace file (with
//! sync_all-per-line so no entry is ever lost to a buffer that the
//! host crashes before flushing).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static FRAMES_LEFT: AtomicU32 = AtomicU32::new(0);

pub fn arm(frames: u32) {
    FRAMES_LEFT.store(frames, Ordering::Relaxed);
    log::warn!("axitrace: arm({frames})");
    // Truncate previous run's trace file so each parse-trace starts fresh.
    if let Some(path) = trace_path() {
        let _ = std::fs::write(&path, format!("=== arm({frames}) ===\n"));
    }
    write_to_file(&format!("arm({frames})"));
}

pub fn tick() {
    let n = FRAMES_LEFT.load(Ordering::Relaxed);
    if n > 0 { FRAMES_LEFT.store(n - 1, Ordering::Relaxed); }
}

pub fn active() -> bool {
    FRAMES_LEFT.load(Ordering::Relaxed) > 0
}

pub fn trace(msg: &str) {
    if !active() { return; }
    log::warn!("axitrace: {msg}");
    write_to_file(msg);
}

fn trace_path() -> Option<PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA")?;
    let mut p = PathBuf::from(root);
    p.push("Axipulse");
    std::fs::create_dir_all(&p).ok()?;
    p.push("trace.log");
    Some(p)
}

fn write_to_file(msg: &str) {
    let Some(path) = trace_path() else { return };
    let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) else { return };
    let _ = writeln!(f, "{msg}");
    let _ = f.sync_all();
}
