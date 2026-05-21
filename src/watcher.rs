#![cfg(windows)]
//! Filesystem watcher on the cbtlogs directory. Posts new .evtc/.zevtc
//! paths to a callback after the file size has stabilised (i.e. arcdps
//! is done writing).

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

pub fn spawn_watcher<F>(cbtlogs_dir: PathBuf, on_log: F) -> std::io::Result<()>
where
    F: Fn(PathBuf) + Send + 'static,
{
    thread::Builder::new()
        .name("axipulse-watcher".into())
        .spawn(move || run(cbtlogs_dir, on_log))?;
    Ok(())
}

fn run<F: Fn(PathBuf) + Send + 'static>(dir: PathBuf, on_log: F) {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = match notify::recommended_watcher(tx) {
        Ok(w) => w,
        Err(e) => { log::warn!("axipulse watcher init failed: {e}"); return; }
    };
    if let Err(e) = watcher.watch(&dir, RecursiveMode::Recursive) {
        log::warn!("axipulse watcher cannot watch {dir:?}: {e}");
        return;
    }
    log::warn!("axipulse watcher started on {dir:?}");

    for res in rx {
        let Ok(event) = res else { continue };
        if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) { continue; }
        for path in event.paths {
            if !is_log_extension(&path) { continue; }
            if !await_stable(&path) {
                log::warn!("axipulse: log {path:?} never stabilised, skipping");
                continue;
            }
            on_log(path);
        }
    }
}

fn is_log_extension(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("evtc") || e.eq_ignore_ascii_case("zevtc"))
        .unwrap_or(false)
}

fn await_stable(path: &Path) -> bool {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last: Option<u64> = None;
    while Instant::now() < deadline {
        let size = std::fs::metadata(path).ok().map(|m| m.len()).unwrap_or(0);
        if size > 0 && Some(size) == last {
            return true;
        }
        last = Some(size);
        thread::sleep(Duration::from_millis(250));
    }
    false
}
