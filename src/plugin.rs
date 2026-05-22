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
    tick_frame();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::ui::icons::drain_pending();
        let (state, mut config) = match (G.state.lock(), G.config.lock()) {
            (Ok(s), Ok(c)) => (s, c),
            _ => return,
        };
        crate::ui::main::render(ui, &state, &mut config);
        crate::ui::notifier::render(ui, &mut config);
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

pub fn options_end(ui: &arcdps::imgui::Ui) {
    OPTIONS_OPEN_TICK.store(frame_counter(), Ordering::Relaxed);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if let Ok(mut c) = G.config.lock() {
            crate::ui::options::render_options_end(ui, &mut c);
        }
    }));
}

/// Frame index (incremented in `imgui`) at which `options_end` most
/// recently fired. The notifier checks `options_open_recently()` so it
/// renders a dummy toast while the user is in the settings pane,
/// letting them drag it into position even when no parse is active.
static OPTIONS_OPEN_TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static FRAME_TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn frame_counter() -> u64 { FRAME_TICK.load(Ordering::Relaxed) }

pub fn tick_frame() { FRAME_TICK.fetch_add(1, Ordering::Relaxed); }

/// True if `options_end` ran within the last few frames — arcdps only
/// calls it while the settings pane is visible.
pub fn options_open_recently() -> bool {
    let now = FRAME_TICK.load(Ordering::Relaxed);
    let last = OPTIONS_OPEN_TICK.load(Ordering::Relaxed);
    last != 0 && now.saturating_sub(last) <= 2
}

/// Which hotkey slot the options window is currently rebinding. The
/// next non-modifier keystroke in `wnd_nofilter` captures the chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingTarget {
    ToggleVisibility,
}

static BINDING: Mutex<Option<BindingTarget>> = Mutex::new(None);

pub fn request_bind(target: BindingTarget) {
    if let Ok(mut g) = BINDING.lock() { *g = Some(target); }
}

pub fn binding_in_progress() -> Option<BindingTarget> {
    BINDING.lock().ok().and_then(|g| *g)
}

pub fn cancel_binding() {
    if let Ok(mut g) = BINDING.lock() { *g = None; }
}

fn take_binding() -> Option<BindingTarget> {
    BINDING.lock().ok().and_then(|mut g| g.take())
}

pub fn wnd_nofilter(key: usize, key_down: bool, prev_key_down: bool) -> bool {
    if !key_down || prev_key_down { return true; }
    let needs_processing = binding_in_progress().is_some() || {
        match G.config.lock() {
            Ok(c) => !c.toggle_visibility_hotkey.is_empty(),
            Err(_) => false,
        }
    };
    if !needs_processing { return true; }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wnd_nofilter_inner(key)
    }));
    result.unwrap_or(true)
}

fn wnd_nofilter_inner(key: usize) -> bool {
    let ctrl = key_down_async(0x11);
    let shift = key_down_async(0x10);
    let alt = key_down_async(0x12);

    if let Some(target) = binding_in_progress() {
        let Some(combo) = crate::hotkey::format_keypress(key as u32, ctrl, shift, alt) else {
            return false;
        };
        let _ = take_binding();
        if let Ok(mut c) = G.config.lock() {
            match target {
                BindingTarget::ToggleVisibility => c.toggle_visibility_hotkey = combo,
            }
            c.save();
        }
        return false;
    }

    let toggle_str = match G.config.lock() {
        Ok(c) => c.toggle_visibility_hotkey.clone(),
        Err(_) => return true,
    };
    let pressed = |s: &str| -> bool {
        let Some(hk) = crate::hotkey::Hotkey::parse(s) else { return false };
        crate::hotkey::matches(&hk, key as u32, ctrl, shift, alt)
    };
    if pressed(&toggle_str) {
        if let Ok(mut c) = G.config.lock() {
            c.show_pulse = !c.show_pulse;
            c.save();
        }
        return false;
    }
    true
}

fn key_down_async(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(vk: i32) -> i16;
}

use std::sync::atomic::{AtomicU32, Ordering};

/// How many `on_new_log` invocations are currently parsing. UI reads
/// this to drive the header's "parsing…" indicator.
static PARSING_COUNT: AtomicU32 = AtomicU32::new(0);

pub fn is_parsing() -> bool { PARSING_COUNT.load(Ordering::Relaxed) > 0 }

/// Last successfully-parsed fight: `(label, when)`. Drives the
/// "Parsed: …" toast in the notifier window so users can see logs
/// arrive without keeping the main AxiPulse window open. Wrapped in
/// `Mutex` (instead of an atomic) because the label is a String.
static LAST_PARSED: Mutex<Option<(String, std::time::Instant)>> = Mutex::new(None);

pub fn last_parsed() -> Option<(String, std::time::Instant)> {
    LAST_PARSED.lock().ok().and_then(|g| g.clone())
}

/// File currently being parsed (filename stem, for the toast). Cleared
/// when `ParsingGuard` drops, regardless of success.
static PARSING_LABEL: Mutex<Option<String>> = Mutex::new(None);

pub fn parsing_label() -> Option<String> {
    PARSING_LABEL.lock().ok().and_then(|g| g.clone())
}

/// RAII guard that increments PARSING_COUNT for the lifetime of an
/// in-flight parse and decrements it on drop. Survives early returns
/// and panics inside `on_new_log`.
struct ParsingGuard;
impl ParsingGuard {
    fn new(label: String) -> Self {
        PARSING_COUNT.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut g) = PARSING_LABEL.lock() { *g = Some(label); }
        ParsingGuard
    }
}
impl Drop for ParsingGuard {
    fn drop(&mut self) {
        PARSING_COUNT.fetch_sub(1, Ordering::Relaxed);
        if let Ok(mut g) = PARSING_LABEL.lock() { *g = None; }
    }
}

fn on_new_log(path: PathBuf) {
    let label = path.file_stem().and_then(|s| s.to_str()).unwrap_or("(log)").to_string();
    let _parsing = ParsingGuard::new(label);
    let install_root = match G.install_root.lock().ok().and_then(|g| g.clone()) {
        Some(r) => r,
        None => { log::warn!("axipulse: on_new_log fired before install_root set"); return; }
    };
    let settings = G.settings.lock().ok().map(|s| s.clone()).unwrap_or_default();
    log::warn!("axipulse: parsing {path:?}");
    match parse_log(&install_root, &settings, &path) {
        Ok(json) => {
            // Pre-compute everything heavy the UI used to do per frame.
            let derived = std::sync::Arc::new(crate::derived::Derived::compute(&json));
            let record = FightRecord {
                log_path: path,
                parsed_at: std::time::SystemTime::now(),
                data: json,
                derived,
            };
            log::warn!(
                "axipulse: parsed {:?}, {}ms, {} players",
                record.log_path.file_name(),
                record.data.duration_ms,
                record.data.players.len(),
            );
            let toast_label = format!(
                "{} \u{00b7} {} players",
                if record.data.fight_name.is_empty() { "Fight" } else { record.data.fight_name.as_str() },
                record.data.players.len(),
            );
            if let Ok(mut s) = G.state.lock() { s.push_fight(record); }
            if let Ok(mut g) = LAST_PARSED.lock() {
                *g = Some((toast_label, std::time::Instant::now()));
            }
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
