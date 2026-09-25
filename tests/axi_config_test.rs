//! The accent setting's persistence. The upgrade path that matters is
//! a v0.4.4 `axipulse.json` with no `accent` key at all — the
//! overwhelmingly common one.

use arcdps_axipulse::config::Config;
use arcdps_axipulse::ui::theme;

#[test]
fn the_default_accent_matches_the_desktop_app() {
    assert_eq!(Config::default().accent, "emerald-mint");
    assert_eq!(Config::default().accent, theme::DEFAULT_ACCENT_ID);
}

#[test]
fn a_v0_4_4_config_loads_and_picks_up_the_default_accent() {
    // Verbatim shape of a config written before this field existed.
    // Every other field must survive; only `accent` defaults.
    let json = r#"{
        "cbtlogs_path": "C:\\logs",
        "debug_logging": true,
        "show_pulse": false,
        "pulse_pos": [120.0, 340.0],
        "show_timeline": true,
        "timeline_pos": null,
        "timeline_layers": { "health": false, "damage_dealt": true },
        "toggle_visibility_hotkey": "Ctrl+Shift+P",
        "show_notifications": false,
        "notifications_pos": null,
        "show_team_bar": true,
        "team_bar_pos": [10.0, 20.0],
        "team_bar_compact": true,
        "auto_update_check": false
    }"#;
    let cfg: Config = serde_json::from_str(json).expect("v0.4.4 config must still load");
    assert_eq!(cfg.accent, theme::DEFAULT_ACCENT_ID, "missing accent defaults");
    // Nothing else was reset on the way through.
    assert_eq!(cfg.cbtlogs_path, "C:\\logs");
    assert!(cfg.debug_logging);
    assert!(!cfg.show_pulse);
    assert_eq!(cfg.pulse_pos, Some((120.0, 340.0)));
    assert_eq!(cfg.toggle_visibility_hotkey, "Ctrl+Shift+P");
    assert!(!cfg.show_notifications);
    assert!(cfg.show_team_bar);
    assert_eq!(cfg.team_bar_pos, Some((10.0, 20.0)));
    assert!(cfg.team_bar_compact);
    assert!(!cfg.auto_update_check);
    assert!(!cfg.timeline_layers.health);
    assert!(cfg.timeline_layers.damage_dealt);
}

#[test]
fn an_accent_round_trips_through_json() {
    let mut cfg = Config::default();
    cfg.accent = "violet-purple".to_string();
    let s = serde_json::to_string(&cfg).expect("serialize");
    let back: Config = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(back.accent, "violet-purple");
    assert_eq!(theme::accent(&back.accent), theme::rgb(0x8b, 0x5c, 0xf6));
}

#[test]
fn a_hand_edited_or_downgraded_accent_still_renders() {
    // Someone edits axipulse.json by hand, or runs a build that had a
    // twelfth accent. Neither may panic inside the render callback.
    for id in ["", "not-an-accent", "Violet-Purple", "null"] {
        let mut cfg = Config::default();
        cfg.accent = id.to_string();
        assert_eq!(theme::accent(&cfg.accent), theme::accent(theme::DEFAULT_ACCENT_ID));
        assert!(theme::accent_index(&cfg.accent) < theme::ACCENTS.len());
    }
}
