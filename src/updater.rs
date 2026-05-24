//! Auto-updater: checks GitHub for a newer release and stages the
//! DLL swap on user confirmation. See
//! docs/superpowers/specs/2026-05-23-auto-updater-design.md.

#[derive(Debug, PartialEq)]
pub enum ParseOutcome {
    Newer { tag: String, body: String, asset_url: String },
    Current,
    ParseError(String),
}

/// Parse a GitHub `/releases/latest` JSON body and decide whether it
/// represents a version newer than `current` (e.g. "0.1.1"). Pure;
/// no IO. The `tag_name` is expected to look like `vX.Y.Z`.
pub fn parse_latest(json: &str, current: &str) -> ParseOutcome {
    use serde_json::Value;
    let v: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => return ParseOutcome::ParseError(format!("json: {e}")),
    };
    let tag = match v.get("tag_name").and_then(|x| x.as_str()) {
        Some(t) => t.to_string(),
        None => return ParseOutcome::ParseError("missing tag_name".into()),
    };
    let body = v.get("body").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let asset_url = v.get("assets").and_then(|a| a.as_array())
        .and_then(|arr| arr.iter().find(|a|
            a.get("name").and_then(|n| n.as_str()) == Some("arcdps_axipulse.dll")
        ))
        .and_then(|a| a.get("browser_download_url").and_then(|u| u.as_str()))
        .map(|s| s.to_string());
    let asset_url = match asset_url {
        Some(u) => u,
        None => return ParseOutcome::ParseError("missing arcdps_axipulse.dll asset".into()),
    };

    let strip = |s: &str| s.strip_prefix('v').unwrap_or(s).to_string();
    let remote = match semver::Version::parse(&strip(&tag)) {
        Ok(v) => v,
        Err(e) => return ParseOutcome::ParseError(format!("tag semver: {e}")),
    };
    let local = match semver::Version::parse(&strip(current)) {
        Ok(v) => v,
        Err(e) => return ParseOutcome::ParseError(format!("current semver: {e}")),
    };
    if remote > local {
        ParseOutcome::Newer { tag, body, asset_url }
    } else {
        ParseOutcome::Current
    }
}

use std::sync::Mutex;

#[derive(Debug, Clone)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available    { tag: String, body: String, asset_url: String },
    Downloading  { tag: String, pct: f32 },
    Installed    { tag: String },
    Failed       { msg: String },
}

static STATE: Mutex<UpdateState> = Mutex::new(UpdateState::Idle);

pub fn snapshot() -> UpdateState {
    STATE.lock().map(|g| g.clone()).unwrap_or(UpdateState::Idle)
}

pub fn dismiss_error() {
    if let Ok(mut g) = STATE.lock() {
        if matches!(*g, UpdateState::Failed { .. }) { *g = UpdateState::Idle; }
    }
}

fn set_state(new: UpdateState) {
    if let Ok(mut g) = STATE.lock() { *g = new; }
}

use std::thread;
use std::time::Duration;

const RELEASES_URL: &str =
    "https://api.github.com/repos/darkharasho/arcdps-axipulse/releases/latest";

/// Called once on plugin init. If `enabled`, spawns a short-lived
/// background thread that hits the GitHub `latest release` endpoint
/// and updates `STATE` accordingly. Cheap to call when disabled.
pub fn kick_check_on_load(enabled: bool) {
    if !enabled {
        set_state(UpdateState::Idle);
        return;
    }
    set_state(UpdateState::Checking);
    let current = env!("CARGO_PKG_VERSION").to_string();
    thread::Builder::new()
        .name("axipulse-update-check".into())
        .spawn(move || {
            match http_fetch_latest() {
                Ok(body) => match parse_latest(&body, &current) {
                    ParseOutcome::Newer { tag, body, asset_url } =>
                        set_state(UpdateState::Available { tag, body, asset_url }),
                    ParseOutcome::Current =>
                        set_state(UpdateState::UpToDate),
                    ParseOutcome::ParseError(msg) =>
                        set_state(UpdateState::Failed { msg: format!("parse: {msg}") }),
                },
                Err(msg) => set_state(UpdateState::Failed { msg }),
            }
        })
        .ok();
}

fn http_fetch_latest() -> Result<String, String> {
    let ua = format!("arcdps_axipulse/{}", env!("CARGO_PKG_VERSION"));
    let resp = ureq::get(RELEASES_URL)
        .set("User-Agent", &ua)
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(15))
        .call()
        .map_err(|e| format!("http: {e}"))?;
    resp.into_string().map_err(|e| format!("read body: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DLL: &str = "arcdps_axipulse.dll";

    fn release_json(tag: &str, body: &str, asset_name: &str) -> String {
        format!(
            r#"{{
                "tag_name": "{tag}",
                "body": "{body}",
                "assets": [
                    {{ "name": "{asset_name}", "browser_download_url": "https://example/{asset_name}" }}
                ]
            }}"#
        )
    }

    #[test]
    fn newer_release_is_detected() {
        let json = release_json("v0.1.2", "changelog", DLL);
        match parse_latest(&json, "0.1.1") {
            ParseOutcome::Newer { tag, body, asset_url } => {
                assert_eq!(tag, "v0.1.2");
                assert_eq!(body, "changelog");
                assert_eq!(asset_url, "https://example/arcdps_axipulse.dll");
            }
            other => panic!("expected Newer, got {other:?}"),
        }
    }

    #[test]
    fn same_version_is_current() {
        let json = release_json("v0.1.1", "x", DLL);
        assert_eq!(parse_latest(&json, "0.1.1"), ParseOutcome::Current);
    }

    #[test]
    fn older_release_is_current() {
        let json = release_json("v0.1.0", "x", DLL);
        assert_eq!(parse_latest(&json, "0.1.1"), ParseOutcome::Current);
    }

    #[test]
    fn missing_dll_asset_is_parse_error() {
        let json = release_json("v0.1.2", "x", "arcdps_other.dll");
        assert!(matches!(parse_latest(&json, "0.1.1"), ParseOutcome::ParseError(_)));
    }

    #[test]
    fn malformed_json_is_parse_error() {
        assert!(matches!(parse_latest("not json", "0.1.1"), ParseOutcome::ParseError(_)));
    }

    #[test]
    fn tag_without_v_prefix_still_parses() {
        let json = release_json("0.1.2", "x", DLL);
        match parse_latest(&json, "0.1.1") {
            ParseOutcome::Newer { tag, .. } => assert_eq!(tag, "0.1.2"),
            other => panic!("expected Newer, got {other:?}"),
        }
    }
}
