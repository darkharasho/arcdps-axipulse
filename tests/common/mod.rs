//! Shared fixture loading for the native parse path's tests.

use std::path::PathBuf;

pub const PARSE_OPTS: axilog_api::ParseOpts = axilog_api::ParseOpts {
    replay: true,
    skill_damage: true,
    timeseries: true,
    rotation: true,
    missiles: false,
    modifiers: false,
    everything: false,
};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn fixture_bytes() -> Vec<u8> {
    std::fs::read(fixture_dir().join("wvw.zevtc")).expect("fixture readable")
}

pub fn native() -> axilog_api::v1::ReportV1 {
    axilog_api::parse_report_v1(&fixture_bytes(), &PARSE_OPTS, Some("wvw.zevtc"))
        .expect("fixture parses")
}
