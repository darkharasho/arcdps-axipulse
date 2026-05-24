//! Plugin config persisted to JSON next to the DLL (axipulse.json).

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Empty = autodetect under %USERPROFILE%\Documents\Guild Wars 2\addons\arcdps\arcdps.cbtlogs
    pub cbtlogs_path: String,
    pub debug_logging: bool,
    /// Whether the Pulse window is currently rendered.
    pub show_pulse: bool,
    pub pulse_pos: Option<(f32, f32)>,
    /// Whether the Timeline window is currently rendered.
    pub show_timeline: bool,
    pub timeline_pos: Option<(f32, f32)>,
    /// Per-lane visibility toggles for the Timeline.
    pub timeline_layers: TimelineLayers,
    /// Chord string (e.g. "Ctrl+Shift+P") that toggles the AxiPulse
    /// window. Empty = no hotkey bound.
    pub toggle_visibility_hotkey: String,
    /// Show a small transparent toast when a new log is being parsed
    /// and briefly after a parse completes — independent of the main
    /// AxiPulse window.
    pub show_notifications: bool,
    pub notifications_pos: Option<(f32, f32)>,
    /// Background check for a newer release on plugin init.
    pub auto_update_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TimelineLayers {
    pub health: bool,
    pub damage_dealt: bool,
    pub damage_taken: bool,
    pub distance_to_tag: bool,
    pub offensive_boons: bool,
    pub defensive_boons: bool,
}

impl Default for TimelineLayers {
    fn default() -> Self {
        Self {
            health: true,
            damage_dealt: true,
            damage_taken: true,
            distance_to_tag: true,
            offensive_boons: true,
            defensive_boons: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cbtlogs_path: String::new(),
            debug_logging: false,
            show_pulse: true,
            pulse_pos: None,
            show_timeline: true,
            timeline_pos: None,
            timeline_layers: TimelineLayers::default(),
            toggle_visibility_hotkey: String::new(),
            show_notifications: true,
            notifications_pos: None,
            auto_update_check: true,
        }
    }
}

pub fn config_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.set_file_name("axipulse.json");
    p
}

impl Config {
    pub fn load() -> Self {
        let p = config_path();
        std::fs::read_to_string(&p).ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let p = config_path();
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, s);
        }
    }
}

pub fn default_cbtlogs() -> Option<PathBuf> {
    // Primary: Win32 FOLDERID_Documents, which honours OneDrive/Documents
    // redirection. Fallback: %USERPROFILE%\Documents — used on non-Windows
    // hosts (tests, host-side compile) and as a safety net if the shell
    // API call ever fails.
    let mut p = known_documents_dir()
        .or_else(|| {
            let userprofile = std::env::var("USERPROFILE").ok()?;
            let mut p = PathBuf::from(userprofile);
            p.push("Documents");
            Some(p)
        })?;
    p.push("Guild Wars 2");
    p.push("addons"); p.push("arcdps"); p.push("arcdps.cbtlogs");
    Some(p)
}

#[cfg(windows)]
fn known_documents_dir() -> Option<PathBuf> {
    use windows::core::PWSTR;
    use windows::Win32::UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
    unsafe {
        let pwstr: PWSTR = SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None).ok()?;
        if pwstr.is_null() { return None; }
        // Read the null-terminated UTF-16 string.
        let mut len = 0usize;
        while *pwstr.0.add(len) != 0 { len += 1; }
        let slice = std::slice::from_raw_parts(pwstr.0, len);
        let s = String::from_utf16(slice).ok();
        windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.0 as *const _));
        s.map(PathBuf::from)
    }
}

#[cfg(not(windows))]
fn known_documents_dir() -> Option<PathBuf> { None }
