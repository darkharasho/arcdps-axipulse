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
    /// Last-known position of the Pulse window in screen coordinates.
    /// None means "let ImGui pick the default on first frame".
    pub pulse_pos: Option<(f32, f32)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cbtlogs_path: String::new(),
            debug_logging: false,
            show_pulse: true,
            pulse_pos: None,
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
    let userprofile = std::env::var("USERPROFILE").ok()?;
    let mut p = PathBuf::from(userprofile);
    p.push("Documents"); p.push("Guild Wars 2");
    p.push("addons"); p.push("arcdps"); p.push("arcdps.cbtlogs");
    Some(p)
}
