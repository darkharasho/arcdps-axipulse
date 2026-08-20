//! Top-level arcdps callbacks + globals.

#![cfg(windows)]

use std::path::PathBuf;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::config::{default_cbtlogs, Config};
use crate::state::{AppState, FightRecord};

struct Globals {
    state: Mutex<AppState>,
    config: Mutex<Config>,
}

static G: Lazy<Globals> = Lazy::new(|| Globals {
    state: Mutex::new(AppState::new()),
    config: Mutex::new(Config::load()),
});

pub fn init() -> Result<(), Option<String>> {
    let _ = &*G;
    crate::diag::set_enabled(G.config.lock().ok().map(|c| c.debug_logging).unwrap_or(false));

    let cbtlogs = match G.config.lock().ok().map(|c| c.cbtlogs_path.clone()).filter(|s| !s.is_empty()) {
        Some(s) => Some(PathBuf::from(s)),
        None => default_cbtlogs(),
    };
    if let Some(dir) = cbtlogs {
        if dir.exists() {
            log::info!("axipulse init: watching cbtlogs at {dir:?}");
            let _ = crate::watcher::spawn_watcher(dir, on_new_log);
        } else {
            log::warn!("axipulse init: cbtlogs {dir:?} does not exist; \
                watcher not started — set the path in the AxiPulse options pane");
        }
    } else {
        log::warn!("axipulse init: could not resolve cbtlogs path; \
            watcher not started — set the path in the AxiPulse options pane");
    }

    // Auto-updater: best-effort cleanup of any leftover `.old` from
    // the previous session, then kick the check thread if enabled.
    if let Some(dir) = dll_dir() {
        crate::updater::cleanup_stale_old(&dir);
    }
    let auto_update_check = G.config.lock()
        .ok().map(|c| c.auto_update_check).unwrap_or(true);
    crate::updater::kick_check_on_load(auto_update_check);

    // Tile fetcher: warm the WvW map tile sidecar in the background
    // so the Map tab works for users who never ran fetch_tiles.sh.
    crate::tile_fetcher::kick_on_init();

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
        crate::ui::team_bar::render(ui, &state, &mut config);
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

/// Directory containing the loaded `arcdps_axipulse.dll` (e.g.
/// `<gw2>/addons/`). Resolved lazily on first call via the standard
/// `GetModuleHandleExW(FROM_ADDRESS) + GetModuleFileNameW` Windows
/// idiom. Used by the tile cache to find sidecar assets at
/// `<dll_dir>/axipulse-assets/tiles/` (placed there by deploy.sh).
///
/// Returns `None` only if the Windows API call fails — should never
/// happen in practice once the DLL is loaded.
pub fn dll_dir() -> Option<std::path::PathBuf> {
    use once_cell::sync::Lazy;
    static DLL_DIR: Lazy<Option<std::path::PathBuf>> = Lazy::new(resolve_dll_dir);
    DLL_DIR.clone()
}

fn resolve_dll_dir() -> Option<std::path::PathBuf> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    };
    let mut hmod = HMODULE::default();
    // Use a function inside our crate as the "address" anchor.
    let anchor = resolve_dll_dir as *const () as *const u16;
    unsafe {
        GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, PCWSTR(anchor), &mut hmod).ok()?;
    }
    let mut buf = [0u16; 32768];
    let len = unsafe { GetModuleFileNameW(Some(hmod), &mut buf) } as usize;
    if len == 0 { return None; }
    let path_str = String::from_utf16(&buf[..len]).ok()?;
    let path = std::path::PathBuf::from(path_str);
    path.parent().map(|p| p.to_path_buf())
}

/// Snapshot the notifier renders for the "Parsed" toast. `map` is the
/// stripped fight name (e.g. "Green Alpine Borderlands"); `counts` is
/// the same per-team breakdown the team bar shows, so the toast's
/// numbers and colours always match it.
#[derive(Clone)]
pub struct ParsedToast {
    pub map: String,
    pub counts: crate::wvw_teams::TeamCounts,
}

/// Last successfully-parsed fight + when it landed. Drives the "Parsed: …"
/// toast in the notifier window so users can see logs arrive without
/// keeping the main AxiPulse window open.
static LAST_PARSED: Mutex<Option<(ParsedToast, std::time::Instant)>> = Mutex::new(None);

pub fn last_parsed() -> Option<(ParsedToast, std::time::Instant)> {
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

/// Runs on the `axipulse-parser` thread, once per detected log.
///
/// The whole body is inside a `catch_unwind` for the same reason
/// `parse::parse_log` has one, extended one layer out: `parse_log`'s own
/// guard ends at `FightData::from_report`, but `Derived::compute` (which
/// fans out to eleven leaf modules), `wvw_teams::count_teams` and
/// `push_fight` all run afterwards on this same thread. A panic in any
/// of them used to unwind the parser thread, which drops the work
/// receiver; the watcher's next `tx_work.send` then fails and the
/// watcher thread returns, so NO further log is parsed for the rest of
/// the GW2 session. Catching here costs one log instead.
fn on_new_log(path: PathBuf) {
    let label = path.file_stem().and_then(|s| s.to_str()).unwrap_or("(log)").to_string();
    let _parsing = ParsingGuard::new(label);
    // Unwind-safe: everything shared is behind a Mutex (poisoning is
    // already handled at every lock site here), and the only values
    // being built are local and dropped on the unwind path.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        on_new_log_inner(path)
    }));
    if let Err(payload) = outcome {
        log::warn!(
            "axipulse: post-parse processing panicked: {}",
            crate::parse::panic_message(payload.as_ref()),
        );
    }
}

fn on_new_log_inner(path: PathBuf) {
    log::warn!("axipulse: parsing {path:?}");
    // In-process, no install root and no Elite Insights settings: the
    // log bytes go straight into axilog and come back as a `FightData`.
    // The `ReportV1` behind it is dropped inside `parse_log`, so there
    // is nothing left to slim afterwards either.
    match crate::parse::parse_log(&path) {
        Ok(fight) => {
            // Pre-compute everything heavy the UI used to do per frame.
            let derived = std::sync::Arc::new(crate::derived::Derived::compute(&fight));
            // `encounter.map` is the map's own name; there is no
            // "Detailed WvW - " prefix to strip any more.
            let map = fight.map_name.clone();
            let counts = crate::wvw_teams::count_teams(&fight);
            let record = FightRecord {
                log_path: path,
                parsed_at: std::time::SystemTime::now(),
                data: fight,
                derived,
            };
            log::warn!(
                "axipulse: parsed {:?}, {}ms, {} players",
                record.log_path.file_name(),
                record.data.duration_ms,
                record.data.players.len(),
            );
            let toast = ParsedToast { map, counts };
            let evicted = match G.state.lock() {
                Ok(mut s) => s.push_fight(record),
                Err(_) => Vec::new(),
            };
            // Free the evicted fight here, outside the lock.
            drop(evicted);
            if let Ok(mut g) = LAST_PARSED.lock() {
                *g = Some((toast, std::time::Instant::now()));
            }
        }
        Err(e) => log::warn!("axipulse: parse failed: {e}"),
    }
}
