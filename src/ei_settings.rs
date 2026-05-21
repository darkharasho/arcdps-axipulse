//! Generate the settings.conf passed to GW2EICLI.exe.
//! Mirrors axipulse/src/main/eiParser.ts:generateEiConf so the JSON
//! shape Pulse/Timeline depend on stays identical.

#[derive(Debug, Clone)]
pub struct EiSettings {
    pub detailled_wvw: bool,
    pub compute_damage_modifiers: bool,
    pub parse_phases: bool,
    pub skip_failed_tries: bool,
    pub anonymous: bool,
    pub custom_too_short: u32,
    pub save_out_html: bool,
    pub parse_combat_replay: bool,
    pub raw_timeline_arrays: bool,
    pub single_threaded: bool,
    pub memory_limit: u32,
}

impl Default for EiSettings {
    fn default() -> Self {
        Self {
            detailled_wvw: true,
            compute_damage_modifiers: true,
            parse_phases: true,
            skip_failed_tries: false,
            anonymous: false,
            custom_too_short: 2200,
            save_out_html: false,
            parse_combat_replay: false,
            raw_timeline_arrays: true,
            single_threaded: false,
            memory_limit: 0,
        }
    }
}

fn bool_str(v: bool) -> &'static str {
    if v {
        "True"
    } else {
        "False"
    }
}

pub fn generate_ei_conf(s: &EiSettings, out_location: &str) -> String {
    let mut lines = Vec::with_capacity(24);
    lines.push("SaveOutJSON=True".to_string());
    lines.push(format!("SaveOutHTML={}", bool_str(s.save_out_html)));
    lines.push("SaveOutCSV=False".to_string());
    lines.push("SaveOutTrace=False".to_string());
    lines.push("CompressRaw=True".to_string());
    lines.push("SaveAtOut=False".to_string());
    lines.push(format!("OutLocation={out_location}"));
    lines.push(format!("DetailledWvW={}", bool_str(s.detailled_wvw)));
    lines.push(format!("RawTimelineArrays={}", bool_str(s.raw_timeline_arrays)));
    lines.push(format!(
        "ComputeDamageModifiers={}",
        bool_str(s.compute_damage_modifiers)
    ));
    lines.push(format!(
        "ParseCombatReplay={}",
        bool_str(s.parse_combat_replay)
    ));
    lines.push(format!("ParsePhases={}", bool_str(s.parse_phases)));
    lines.push(format!("SingleThreaded={}", bool_str(s.single_threaded)));
    lines.push(format!("SkipFailedTries={}", bool_str(s.skip_failed_tries)));
    lines.push(format!("Anonymous={}", bool_str(s.anonymous)));
    lines.push("ParseMultipleLogs=False".to_string());
    lines.push("UploadToDPSReports=False".to_string());
    lines.push("UploadToWingman=False".to_string());
    lines.push("IndentJSON=False".to_string());
    lines.push(format!("MemoryLimit={}", s.memory_limit));
    lines.push(format!("CustomTooShort={}", s.custom_too_short));
    lines.push("LightTheme=False".to_string());
    lines.push("HtmlExternalScripts=False".to_string());
    lines.join("\n") + "\n"
}
