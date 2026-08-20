//! `parse::parse_log` — the shipping parse path, end to end from log
//! bytes to `FightData`.

use arcdps_axipulse::parse::parse_log;

fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wvw.zevtc")
}

#[test]
fn parses_the_fixture_to_fight_data() {
    let f = parse_log(&fixture()).expect("parses");
    assert!(!f.players.is_empty());
    assert!(f.duration_ms > 0);
    assert!(f.self_idx.is_some());
}

/// The four suboptions `PARSE_OPTS` turns on must actually produce their
/// data — this is what catches the options constant silently drifting.
#[test]
fn the_parse_options_produce_every_surface_the_ui_reads() {
    let f = parse_log(&fixture()).expect("parses");
    let p = &f.players[f.self_idx.expect("recorder resolves")];
    assert!(f.arena.is_some(), "--replay: no arena");
    assert!(f.poll_ms > 0, "--replay: no polling grid");
    assert!(!p.positions.is_empty(), "--replay: no position track");
    assert!(!p.damage_by_skill.is_empty(), "--skill-damage: no per-skill damage");
    assert!(!p.damage_1s.is_empty(), "--timeseries: no per-second damage");
    assert!(!p.casts.is_empty(), "--rotation: no casts");
    assert!(!f.skill_icons.is_empty(), "no skill art copied out of the catalog");
}

#[test]
fn reports_a_useful_error_on_a_missing_file() {
    let err = parse_log(std::path::Path::new("/nonexistent.zevtc")).unwrap_err();
    assert!(err.to_string().contains("reading the log failed"), "{err}");
}

/// Garbage in must not panic out. `decode_series` and `require` both
/// panic deliberately on a malformed report, and `parse_log` catches
/// that rather than letting it kill the watcher thread inside the game
/// process.
#[test]
fn a_malformed_log_returns_an_error_rather_than_unwinding() {
    let dir = std::env::temp_dir().join("axipulse-parse-test");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("garbage.zevtc");
    std::fs::write(&path, b"this is not an evtc log at all").expect("write");
    let err = parse_log(&path).expect_err("garbage must not parse");
    // Either error is fine; what must NOT happen is a process abort.
    assert!(
        matches!(
            err,
            arcdps_axipulse::parse::ParseError::Parse(_)
                | arcdps_axipulse::parse::ParseError::Panic(_)
        ),
        "unexpected error kind: {err:?}",
    );
    let _ = std::fs::remove_file(&path);
}
