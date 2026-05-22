#![cfg(windows)]
//! Filesystem watcher on the cbtlogs directory. Hands new `.zevtc`
//! paths to a worker thread which calls back into the plugin. The
//! split keeps EI subprocess spawns off the notify-receive path and
//! ensures parses run serially even when many Create events fire in
//! a burst.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use notify::{Config, Event, EventKind, PollWatcher, RecursiveMode, Watcher};

pub fn spawn_watcher<F>(cbtlogs_dir: PathBuf, on_log: F) -> std::io::Result<()>
where
    F: Fn(PathBuf) + Send + 'static,
{
    let (tx_work, rx_work) = mpsc::channel::<PathBuf>();

    // Worker: drains paths, awaits size stability, calls on_log. Serial.
    thread::Builder::new()
        .name("axipulse-parser".into())
        .spawn(move || {
            for path in rx_work {
                if !await_stable(&path) {
                    log::warn!("axipulse: log {path:?} never stabilised, skipping");
                    continue;
                }
                on_log(path);
            }
        })?;

    // Watcher: routes notify events. Only Create on `.zevtc`, deduped.
    thread::Builder::new()
        .name("axipulse-watcher".into())
        .spawn(move || run(cbtlogs_dir, tx_work))?;
    Ok(())
}

fn run(dir: PathBuf, tx_work: mpsc::Sender<PathBuf>) {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    // notify's recommended (inotify on Linux, ReadDirectoryChangesW on
    // Windows) misses arcdps's rename-from-temp under Wine, so logs
    // never reach us. PollWatcher walks the directory on a fixed
    // interval — slower to react but never misses a file.
    let cfg = Config::default().with_poll_interval(Duration::from_secs(2));
    let mut watcher = match PollWatcher::new(tx, cfg) {
        Ok(w) => w,
        Err(e) => { log::warn!("axipulse watcher init failed: {e}"); return; }
    };
    if let Err(e) = watcher.watch(&dir, RecursiveMode::Recursive) {
        log::warn!("axipulse watcher cannot watch {dir:?}: {e}");
        return;
    }
    log::warn!("axipulse watcher started on {dir:?} (poll mode, 2s)");

    // Seed `seen` with every `.zevtc` already present so we don't
    // re-parse the entire backlog on startup. PollWatcher's first scan
    // emits Create events for the baseline set.
    let mut seen: HashSet<PathBuf> = HashSet::new();
    seed_existing(&dir, &mut seen);

    for res in rx {
        let Ok(event) = res else { continue };
        // PollWatcher surfaces new files as Create; rename-into and
        // direct writes can come through as Modify(_) too. Accept both
        // (the seen-set dedups; `await_stable` handles partial files).
        if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
            continue;
        }
        for path in event.paths {
            if !is_zevtc(&path) { continue; }
            if !seen.insert(path.clone()) { continue; }
            log::warn!("axipulse watcher: new log {path:?}");
            if tx_work.send(path).is_err() {
                log::warn!("axipulse: parser worker gone, watcher exiting");
                return;
            }
        }
    }
}

fn seed_existing(dir: &Path, seen: &mut HashSet<PathBuf>) {
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_zevtc(&p) {
                seen.insert(p);
            }
        }
    }
    log::warn!("axipulse watcher: seeded {} existing logs", seen.len());
}

fn is_zevtc(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("zevtc"))
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
