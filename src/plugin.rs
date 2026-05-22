//! Top-level arcdps callbacks + globals.

#![cfg(windows)]

use std::path::PathBuf;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::config::{default_cbtlogs, Config};
use crate::ei_bundle::{default_install_root, install_from_bytes, BUNDLED_EI_VERSION, BUNDLED_EI_ZIP};
use crate::ei_parser::{parse_log, ParseError};
use crate::ei_settings::EiSettings;
use crate::state::{AppState, FightRecord};

struct Globals {
    state: Mutex<AppState>,
    config: Mutex<Config>,
    install_root: Mutex<Option<PathBuf>>,
    settings: Mutex<EiSettings>,
}

static G: Lazy<Globals> = Lazy::new(|| Globals {
    state: Mutex::new(AppState::new()),
    config: Mutex::new(Config::load()),
    install_root: Mutex::new(None),
    settings: Mutex::new(EiSettings::default()),
});

pub fn init() -> Result<(), Option<String>> {
    let _ = &*G;
    crate::diag::set_enabled(G.config.lock().ok().map(|c| c.debug_logging).unwrap_or(false));

    let Some(install_root) = default_install_root() else {
        log::warn!("axipulse init: no install root (LOCALAPPDATA missing); aborting");
        return Ok(());
    };
    if let Err(e) = install_from_bytes(BUNDLED_EI_ZIP, BUNDLED_EI_VERSION, &install_root) {
        log::warn!("axipulse init: EI extract failed: {e}; subsequent parses will error");
    } else {
        log::warn!("axipulse init: EI installed at {install_root:?}");
    }
    if let Err(e) = crate::ei_bundle::install_dotnet(&install_root) {
        log::warn!("axipulse init: .NET extract failed: {e}; EI will not be able to run");
    } else {
        log::warn!("axipulse init: .NET 8 runtime installed at {:?}", crate::ei_bundle::dotnet_root(&install_root));
    }
    if let Ok(mut slot) = G.install_root.lock() { *slot = Some(install_root); }

    let cbtlogs = match G.config.lock().ok().map(|c| c.cbtlogs_path.clone()).filter(|s| !s.is_empty()) {
        Some(s) => Some(PathBuf::from(s)),
        None => default_cbtlogs(),
    };
    if let Some(dir) = cbtlogs {
        if dir.exists() {
            let _ = crate::watcher::spawn_watcher(dir, on_new_log);
        } else {
            log::warn!("axipulse init: cbtlogs {dir:?} does not exist; watcher not started");
        }
    } else {
        log::warn!("axipulse init: no cbtlogs path resolved; watcher not started");
    }

    Ok(())
}

pub fn release() {
    if let Ok(c) = G.config.lock() { c.save(); }
}

pub fn imgui(ui: &arcdps::imgui::Ui, not_loading: bool) {
    if !not_loading { return; }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::ui::icons::drain_pending();
        let (state, mut config) = match (G.state.lock(), G.config.lock()) {
            (Ok(s), Ok(c)) => (s, c),
            _ => return,
        };
        crate::ui::main::render(ui, &state, &mut config);
    }));
}

pub fn options_windows(ui: &arcdps::imgui::Ui, window_name: Option<&str>) -> bool {
    if window_name.is_some() { return false; }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if let Ok(mut c) = G.config.lock() {
            if crate::ui::options::render_window_checkboxes(ui, &mut c) {
                c.save();
            }
        }
    }));
    false
}

use std::sync::atomic::{AtomicU32, Ordering};

/// How many `on_new_log` invocations are currently parsing. UI reads
/// this to drive the header's "parsing…" indicator.
static PARSING_COUNT: AtomicU32 = AtomicU32::new(0);

pub fn is_parsing() -> bool { PARSING_COUNT.load(Ordering::Relaxed) > 0 }

/// RAII guard that increments PARSING_COUNT for the lifetime of an
/// in-flight parse and decrements it on drop. Survives early returns
/// and panics inside `on_new_log`.
struct ParsingGuard;
impl ParsingGuard {
    fn new() -> Self { PARSING_COUNT.fetch_add(1, Ordering::Relaxed); ParsingGuard }
}
impl Drop for ParsingGuard {
    fn drop(&mut self) { PARSING_COUNT.fetch_sub(1, Ordering::Relaxed); }
}

fn on_new_log(path: PathBuf) {
    let _parsing = ParsingGuard::new();
    let install_root = match G.install_root.lock().ok().and_then(|g| g.clone()) {
        Some(r) => r,
        None => { log::warn!("axipulse: on_new_log fired before install_root set"); return; }
    };
    let settings = G.settings.lock().ok().map(|s| s.clone()).unwrap_or_default();
    log::warn!("axipulse: parsing {path:?}");
    match parse_log(&install_root, &settings, &path) {
        Ok(json) => {
            let record = FightRecord {
                log_path: path,
                parsed_at: std::time::SystemTime::now(),
                data: json,
            };
            log::warn!(
                "axipulse: parsed {:?}, {}ms, {} players",
                record.log_path.file_name(),
                record.data.duration_ms,
                record.data.players.len(),
            );
            if let Ok(mut s) = G.state.lock() { s.push_fight(record); }
            // Arm 120 frames (~2s @ 60fps) of trace output so we can
            // pinpoint where the host crashes when a new fight first
            // renders.
        }
        Err(ParseError::SubprocessExit { code, stderr }) => {
            log::warn!("axipulse: parse failed (code={code:?}): {stderr}");
        }
        Err(e) => log::warn!("axipulse: parse failed: {e}"),
    }
}
