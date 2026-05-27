//! Background tile downloader. On plugin init we compute the full set
//! of WvW map tiles (z 0..=7 across the four maps), check the
//! sidecar `axipulse-assets/tiles/` dir for what's already on disk,
//! and stream the missing ones from `tiles.guildwars2.com`.
//!
//! State is exposed via [`snapshot`] so the Map tab can render a
//! progress strip while the cache warms.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum FetchState {
    Idle,
    Running { done: u32, total: u32, failed: u32 },
    Complete,
    Failed(String),
}

static STATE: Mutex<FetchState> = Mutex::new(FetchState::Idle);

pub fn snapshot() -> FetchState {
    STATE.lock().map(|g| g.clone()).unwrap_or(FetchState::Idle)
}

fn set_state(new: FetchState) {
    if let Ok(mut g) = STATE.lock() { *g = new; }
}

/// Called from plugin init. Cheap when tiles are already complete:
/// just stats the file set and returns. When tiles are missing,
/// spawns a small worker pool that streams them in the background.
#[cfg(windows)]
pub fn kick_on_init() {
    let Some(dll_dir) = crate::plugin::dll_dir() else {
        set_state(FetchState::Failed("dll_dir unavailable".into()));
        return;
    };
    let root = dll_dir.join("axipulse-assets").join("tiles");

    let all = expected_tile_keys();
    let total = all.len() as u32;
    let missing = missing_tiles(&root, &all);

    if missing.is_empty() {
        set_state(FetchState::Complete);
        return;
    }

    let done_initial = total - missing.len() as u32;
    set_state(FetchState::Running { done: done_initial, total, failed: 0 });

    if let Err(e) = std::fs::create_dir_all(&root) {
        set_state(FetchState::Failed(format!("create tiles dir: {e}")));
        return;
    }

    spawn_workers(root, missing, done_initial, total);
}

fn expected_tile_keys() -> Vec<(u32, u32, u32)> {
    use crate::map::wvw::WvwMap;
    let maps = [
        WvwMap::EternalBattlegrounds,
        WvwMap::GreenBorderlands,
        WvwMap::BlueBorderlands,
        WvwMap::RedBorderlands,
        WvwMap::EdgeOfTheMists,
    ];
    let mut set: HashSet<(u32, u32, u32)> = HashSet::new();
    for zoom in 0u32..=7 {
        for m in maps {
            for t in crate::map::tiles::get_map_tiles(m, zoom) {
                set.insert((t.zoom, t.tx, t.ty));
            }
        }
    }
    set.into_iter().collect()
}

fn missing_tiles(root: &Path, all: &[(u32, u32, u32)]) -> Vec<(u32, u32, u32)> {
    all.iter()
        .copied()
        .filter(|(z, x, y)| {
            let p = root.join(format!("{z}/{x}/{y}.jpg"));
            match std::fs::metadata(&p) {
                Ok(m) => m.len() == 0,
                Err(_) => true,
            }
        })
        .collect()
}

const WORKERS: usize = 4;
const TILE_TIMEOUT_SECS: u64 = 30;

fn spawn_workers(root: PathBuf, missing: Vec<(u32, u32, u32)>, done_initial: u32, total: u32) {
    let queue = Arc::new(Mutex::new(missing));
    let done = Arc::new(AtomicU32::new(done_initial));
    let failed = Arc::new(AtomicU32::new(0));
    let remaining_workers = Arc::new(AtomicU32::new(WORKERS as u32));

    let ua = format!("arcdps_axipulse/{}", env!("CARGO_PKG_VERSION"));

    for _ in 0..WORKERS {
        let queue = queue.clone();
        let root = root.clone();
        let done = done.clone();
        let failed = failed.clone();
        let remaining_workers = remaining_workers.clone();
        let ua = ua.clone();
        std::thread::Builder::new()
            .name("axipulse-tile-fetch".into())
            .spawn(move || {
                loop {
                    let job = queue.lock().ok().and_then(|mut g| g.pop());
                    let Some((z, x, y)) = job else { break };
                    match fetch_one(&root, z, x, y, &ua) {
                        Ok(()) => { done.fetch_add(1, Ordering::Relaxed); }
                        Err(e) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                            log::debug!("axipulse: tile {z}/{x}/{y} failed: {e}");
                        }
                    }
                    let d = done.load(Ordering::Relaxed);
                    let f = failed.load(Ordering::Relaxed);
                    set_state(FetchState::Running { done: d, total, failed: f });
                }
                if remaining_workers.fetch_sub(1, Ordering::AcqRel) == 1 {
                    let d = done.load(Ordering::Relaxed);
                    let f = failed.load(Ordering::Relaxed);
                    if f == 0 {
                        set_state(FetchState::Complete);
                    } else {
                        set_state(FetchState::Running { done: d, total, failed: f });
                        log::warn!("axipulse: tile fetch finished with {f} failures (downloaded {d}/{total})");
                    }
                }
            })
            .ok();
    }
}

fn fetch_one(root: &Path, z: u32, x: u32, y: u32, ua: &str) -> Result<(), String> {
    let dst = root.join(format!("{z}/{x}/{y}.jpg"));
    let dst_dir = dst.parent().ok_or_else(|| "no parent".to_string())?;
    std::fs::create_dir_all(dst_dir).map_err(|e| format!("mkdir: {e}"))?;
    let tmp = root.join(format!("{z}/{x}/{y}.jpg.tmp"));

    let url = format!("https://tiles.guildwars2.com/2/3/{z}/{x}/{y}.jpg");
    let resp = ureq::get(&url)
        .set("User-Agent", ua)
        .timeout(Duration::from_secs(TILE_TIMEOUT_SECS))
        .call()
        .map_err(|e| format!("http: {e}"))?;
    let mut reader = resp.into_reader();
    let mut file = std::fs::File::create(&tmp).map_err(|e| format!("create tmp: {e}"))?;
    std::io::copy(&mut reader, &mut file).map_err(|e| format!("copy: {e}"))?;
    file.sync_all().map_err(|e| format!("fsync: {e}"))?;
    drop(file);
    std::fs::rename(&tmp, &dst).map_err(|e| format!("rename: {e}"))?;
    Ok(())
}
