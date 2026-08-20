//! Reading a log, start to finish, inside the game process.
//!
//! There is no subprocess, no separate managed runtime and no intermediate JSON: the
//! bytes go straight into axilog and come back out as a [`FightData`].
//! The `ReportV1` is dropped at the end of [`parse_log`] and never
//! retained -- that is the whole reason the plugin no longer needs a
//! post-parse slimming pass.

use crate::fight_data::FightData;
use std::path::Path;

/// Exactly the four suboptions this plugin's UI reads, and nothing else.
///
/// Never `everything: true`: each extra pass costs parse time and peak
/// memory inside the game process, and this crate's own coverage checks
/// (`fight_data::require`) are written against this exact set -- turning
/// options ON would not break them, but it would make a future
/// regression in one of them invisible here.
///
/// Kept byte-identical to `tests/common/mod.rs::PARSE_OPTS`, which is
/// what every `FightData` test parses with. If the two ever diverge the
/// tests stop testing the shipping parse.
const PARSE_OPTS: axilog_api::ParseOpts = axilog_api::ParseOpts {
    replay: true,
    skill_damage: true,
    timeseries: true,
    rotation: true,
    missiles: false,
    modifiers: false,
    everything: false,
};

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    Parse(String),
    /// A panic escaped the parse or the projection. Caught rather than
    /// allowed to unwind because this runs on the watcher's own thread
    /// inside the game process: an unwind past the arcdps boundary is
    /// undefined behaviour, and a thread that dies silently takes every
    /// subsequent log with it. `fight_data::decode_series` and
    /// `fight_data::require` both panic deliberately on a malformed
    /// report, and this is where that lands.
    Panic(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Io(e) => write!(f, "reading the log failed: {e}"),
            ParseError::Parse(e) => write!(f, "parsing the log failed: {e}"),
            ParseError::Panic(e) => write!(f, "the parser panicked: {e}"),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_log(path: &Path) -> Result<FightData, ParseError> {
    let bytes = std::fs::read(path).map_err(ParseError::Io)?;
    let name = path.file_name().and_then(|s| s.to_str());
    // `catch_unwind` needs its closure to be unwind-safe; `bytes` and
    // `name` are only read, and nothing observable is left half-mutated
    // if the parse panics -- the only value being built is dropped.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let report = axilog_api::parse_report_v1(&bytes, &PARSE_OPTS, name)
            .map_err(|e| ParseError::Parse(e.to_string()))?;
        Ok(FightData::from_report(&report))
        // `report` drops here. Nothing downstream ever sees a ReportV1.
    }));
    match result {
        Ok(inner) => inner,
        Err(payload) => Err(ParseError::Panic(panic_message(payload.as_ref()))),
    }
}

/// Best-effort text of a caught panic payload. `panic!` with a literal
/// gives a `&'static str`, with a format gives a `String`, and anything
/// else is opaque.
pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "(panic payload was not a string)".to_string()
    }
}
