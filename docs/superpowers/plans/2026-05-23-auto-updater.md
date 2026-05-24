# Auto-updater Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a GitHub-release-driven auto-updater that surfaces new versions in the AxiPulse window and atomically swaps the DLL on user confirmation.

**Architecture:** A new `src/updater.rs` module owns a `Mutex<UpdateState>` state machine and short-lived background threads (check / download). Plugin init kicks the check; UI reads state read-only; install thread streams the DLL asset to `<dir>/arcdps_axipulse.dll.new`, then renames `dll → dll.old` and `dll.new → dll` (legal even with the DLL loaded). New version loads on next GW2 start.

**Tech Stack:** Rust, `ureq` (already in tree), `serde_json` (already), `semver = "1"` (new).

Spec: `docs/superpowers/specs/2026-05-23-auto-updater-design.md`.

---

## File map

- **Create**: `src/updater.rs` — state machine, pure parsing, HTTP check, install thread.
- **Modify**: `Cargo.toml` — add `semver` dep.
- **Modify**: `src/lib.rs` — register `pub mod updater;`.
- **Modify**: `src/config.rs` — add `auto_update_check: bool` (default `true`).
- **Modify**: `src/plugin.rs` — call `updater::kick_check_on_load(...)` from `init()`; cleanup leftover `.old`.
- **Modify**: `src/ui/main.rs` — header pill driven by `updater::STATE`.
- **Modify**: `src/ui/options.rs` — "Check for updates on startup" checkbox.

`updater.rs` is split so the parsing/version-compare logic is pure and host-testable:

```
parse_latest(json: &str, current: &str) -> ParseOutcome   // pure, tested
http_fetch_latest() -> Result<String, UpdaterError>       // ureq, no test
spawn_check_thread(...)                                   // glue
spawn_download_thread(...)                                // ureq + fs, no test
swap_in_place(dir: &Path) -> io::Result<()>               // fs only
```

---

## Task 1: Add `semver` dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the dep**

Edit `Cargo.toml`, inside the top-level `[dependencies]` (the *host-side* deps list, not the windows-only one — `semver` is pure Rust so it can go in the top section the host-side tests use; if there is no top-level `[dependencies]`, add it under the windows-only section since the updater is `cfg(windows)` for the threads and the pure code can still compile host-side via `[dev-dependencies]` indirectly). The simplest placement: add to the windows-only `[target.'cfg(windows)'.dependencies]` table. Add this line:

```toml
semver = "1"
```

- [ ] **Step 2: Verify the lockfile updates**

Run: `cargo dll-check`
Expected: PASS, `Cargo.lock` now lists `semver 1.x`.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add semver dep for updater"
```

---

## Task 2: Pure `parse_latest` + tests

**Files:**
- Create: `src/updater.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Register the module**

In `src/lib.rs`, add this line in the `pub mod …;` block (alphabetical between `top_skills` and `ui`):

```rust
pub mod updater;
```

- [ ] **Step 2: Write the failing tests first**

Create `src/updater.rs` with ONLY the tests + the type signature, no body:

```rust
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
pub fn parse_latest(_json: &str, _current: &str) -> ParseOutcome {
    todo!()
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
```

- [ ] **Step 3: Run tests, confirm they fail**

Run: `cargo test --lib updater::tests`
Expected: tests compile, all 6 panic at `todo!()`.

- [ ] **Step 4: Implement `parse_latest`**

Replace the `todo!()` body with:

```rust
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
```

- [ ] **Step 5: Run tests, confirm they pass**

Run: `cargo test --lib updater::tests`
Expected: 6 passed.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/updater.rs
git commit -m "feat(updater): pure parse_latest with semver compare"
```

---

## Task 3: `UpdateState` + global `STATE` mutex

**Files:**
- Modify: `src/updater.rs`

- [ ] **Step 1: Add the state machine + accessors**

Append to `src/updater.rs` (above the `#[cfg(test)]` block):

```rust
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo dll-check`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src/updater.rs
git commit -m "feat(updater): UpdateState machine + snapshot/dismiss"
```

---

## Task 4: Add `auto_update_check` to Config

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add the field**

In `src/config.rs`, inside `pub struct Config { … }`, add (after `notifications_pos`):

```rust
    /// Background check for a newer release on plugin init.
    pub auto_update_check: bool,
```

- [ ] **Step 2: Add the default**

In `impl Default for Config { fn default() -> Self { Self { … } } }`, add (after `notifications_pos: None`):

```rust
            auto_update_check: true,
```

- [ ] **Step 3: Verify**

Run: `cargo dll-check`
Expected: PASS. Existing `axipulse.json` files load with `auto_update_check = true` because of `#[serde(default)]` on the struct.

- [ ] **Step 4: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add auto_update_check toggle (default on)"
```

---

## Task 5: HTTP check thread

**Files:**
- Modify: `src/updater.rs`

- [ ] **Step 1: Add the public entry point + thread**

Append to `src/updater.rs` (above `#[cfg(test)]`):

```rust
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo dll-check`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src/updater.rs
git commit -m "feat(updater): GitHub latest-release check thread"
```

---

## Task 6: Download + atomic swap

**Files:**
- Modify: `src/updater.rs`

- [ ] **Step 1: Add `start_install` + swap helper**

Append to `src/updater.rs`:

```rust
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Called from the UI when the user clicks Install. No-op unless
/// `STATE` is currently `Available`.
pub fn start_install(dll_dir: PathBuf) {
    let (tag, asset_url) = match snapshot() {
        UpdateState::Available { tag, asset_url, .. } => (tag, asset_url),
        _ => return,
    };
    set_state(UpdateState::Downloading { tag: tag.clone(), pct: 0.0 });
    thread::Builder::new()
        .name("axipulse-update-download".into())
        .spawn(move || {
            match download_and_swap(&dll_dir, &asset_url, &tag) {
                Ok(()) => set_state(UpdateState::Installed { tag }),
                Err(msg) => set_state(UpdateState::Failed { msg }),
            }
        })
        .ok();
}

fn download_and_swap(dll_dir: &Path, asset_url: &str, tag: &str) -> Result<(), String> {
    let dll      = dll_dir.join("arcdps_axipulse.dll");
    let dll_new  = dll_dir.join("arcdps_axipulse.dll.new");
    let dll_old  = dll_dir.join("arcdps_axipulse.dll.old");

    // Stream into `.new`. ureq returns an io::Read.
    let ua = format!("arcdps_axipulse/{}", env!("CARGO_PKG_VERSION"));
    let resp = ureq::get(asset_url)
        .set("User-Agent", &ua)
        .timeout(Duration::from_secs(120))
        .call()
        .map_err(|e| format!("download: {e}"))?;
    let total: Option<u64> = resp.header("Content-Length")
        .and_then(|s| s.parse().ok());
    let mut reader = resp.into_reader();
    let mut file = std::fs::File::create(&dll_new)
        .map_err(|e| format!("create .new: {e}"))?;
    let mut buf = [0u8; 64 * 1024];
    let mut read_total: u64 = 0;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("read: {e}"))?;
        if n == 0 { break; }
        file.write_all(&buf[..n]).map_err(|e| format!("write: {e}"))?;
        read_total += n as u64;
        if let Some(t) = total {
            let pct = (read_total as f32 / t as f32) * 100.0;
            set_state(UpdateState::Downloading { tag: tag.to_string(), pct });
        }
    }
    file.sync_all().map_err(|e| format!("fsync: {e}"))?;
    drop(file);

    // Best-effort cleanup of any leftover `.old` from a prior session;
    // ignore failure (Windows may still hold a handle).
    let _ = std::fs::remove_file(&dll_old);

    // Atomic shuffle. Rename of a loaded DLL is permitted on both
    // Windows and Linux/Wine.
    std::fs::rename(&dll, &dll_old)
        .map_err(|e| format!("rename dll → .old: {e}"))?;
    std::fs::rename(&dll_new, &dll)
        .map_err(|e| {
            // Best-effort rollback if the second rename fails.
            let _ = std::fs::rename(&dll_old, &dll);
            format!("rename .new → dll: {e}")
        })?;
    Ok(())
}

/// Called from plugin init. Attempts to delete any leftover `.old`
/// from a previous update. Failure is silent — we'll retry next session.
pub fn cleanup_stale_old(dll_dir: &Path) {
    let _ = std::fs::remove_file(dll_dir.join("arcdps_axipulse.dll.old"));
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo dll-check`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src/updater.rs
git commit -m "feat(updater): download + atomic DLL swap"
```

---

## Task 7: Wire into plugin init

**Files:**
- Modify: `src/plugin.rs`

- [ ] **Step 1: Call into the updater from `init()`**

In `src/plugin.rs`, inside `pub fn init() -> Result<(), Option<String>>`, just before the final `Ok(())` (or wherever init returns success), add:

```rust
    // Auto-updater: best-effort cleanup of any leftover `.old` from
    // the previous session, then kick the check thread if enabled.
    if let Some(dir) = dll_dir() {
        crate::updater::cleanup_stale_old(&dir);
    }
    let auto_update_check = G.config.lock()
        .ok().map(|c| c.auto_update_check).unwrap_or(true);
    crate::updater::kick_check_on_load(auto_update_check);
```

(If `G.config` is not the actual config-handle name in this file, look for the existing `Mutex`/`RwLock` field that wraps the loaded `Config` — there's one — and read `.auto_update_check` off it.)

- [ ] **Step 2: Verify it compiles**

Run: `cargo dll-check`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src/plugin.rs
git commit -m "feat(plugin): kick update check + cleanup .old on init"
```

---

## Task 8: Header pill in the AxiPulse window

**Files:**
- Modify: `src/ui/main.rs`

- [ ] **Step 1: Add the pill renderer + call site**

In `src/ui/main.rs`, find `render_header` (around line 105). At the end of `render_header`, after the existing header content, add a call:

```rust
    render_update_pill(ui);
```

Then add this function below `render_header`:

```rust
fn render_update_pill(ui: &arcdps::imgui::Ui) {
    use crate::updater::{snapshot, start_install, dismiss_error, UpdateState};
    let st = snapshot();
    let (label, color) = match &st {
        UpdateState::Available { tag, .. } =>
            (format!("Update available · {tag}"), [0.40, 0.92, 0.55, 1.0]),
        UpdateState::Downloading { pct, .. } if pct.is_finite() =>
            (format!("Downloading… {:.0}%", pct), [0.50, 0.78, 1.0, 1.0]),
        UpdateState::Downloading { .. } =>
            ("Downloading…".to_string(), [0.50, 0.78, 1.0, 1.0]),
        UpdateState::Installed { tag } =>
            (format!("Restart GW2 to load {tag}"), [0.95, 0.75, 0.40, 1.0]),
        UpdateState::Failed { msg } =>
            (format!("Update failed: {msg}"), [1.00, 0.40, 0.40, 1.0]),
        _ => return,
    };
    ui.same_line();
    ui.text_colored(color, &label);
    if let UpdateState::Available { .. } = &st {
        ui.same_line();
        if ui.small_button("Install") {
            if let Some(dir) = crate::plugin::dll_dir() {
                start_install(dir);
            }
        }
    }
    if let UpdateState::Failed { .. } = &st {
        ui.same_line();
        if ui.small_button("×##dismiss-update") { dismiss_error(); }
    }
    if let UpdateState::Available { body, .. } = &st {
        if !body.is_empty() && ui.collapsing_header("What's new", arcdps::imgui::TreeNodeFlags::empty()) {
            ui.text_wrapped(body);
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo dll-check`
Expected: PASS. If `TreeNodeFlags::empty()` doesn't exist in this imgui version, swap for the appropriate flag literal — search `TreeNodeFlags` in `vendor/arcdps`.

- [ ] **Step 3: Commit**

```bash
git add src/ui/main.rs
git commit -m "feat(ui): header pill for update state"
```

---

## Task 9: Settings checkbox

**Files:**
- Modify: `src/ui/options.rs`

- [ ] **Step 1: Add the checkbox**

In `src/ui/options.rs`, inside `pub fn render_options_end(ui: &Ui, config: &mut Config)`, after the existing options rows, add:

```rust
    ui.separator();
    ui.text_disabled("Updates");
    if ui.checkbox("Check for updates on startup", &mut config.auto_update_check) {
        config.save();
    }
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo dll-check`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src/ui/options.rs
git commit -m "feat(options): toggle for auto update check"
```

---

## Task 10: End-to-end manual verification

**Files:** none — runtime test.

- [ ] **Step 1: Build the release DLL**

Run: `cargo dll`
Expected: `target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll` produced.

- [ ] **Step 2: Deploy and launch GW2**

Run: `./scripts/deploy.sh`
Then launch GW2. Open the AxiPulse window. Since current version is `0.1.1` and latest released is `v0.1.1`, expect the header to show *nothing* (Idle or UpToDate).

- [ ] **Step 3: Simulate a newer release**

Push a temporary `v9.9.9` release on GitHub (with the just-built DLL renamed appropriately is fine, since you'll roll it back):

```bash
gh release create v9.9.9 --title "v9.9.9 test" --notes "updater test" \
  target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll
```

- [ ] **Step 4: Reload the plugin (or restart GW2)**

In GW2, hit ArcDPS's reload-extension hotkey, or restart. The header should show **Update available · v9.9.9** with an Install button.

- [ ] **Step 5: Click Install**

Watch the percentage advance, then see **Restart GW2 to load v9.9.9**. Confirm the install dir contains `arcdps_axipulse.dll` (new) and `arcdps_axipulse.dll.old`.

- [ ] **Step 6: Restart GW2**

Confirm the AxiPulse module logs `axipulse: version 9.9.9` (or similar identifier — at minimum, no `Update available` pill should appear since current ≥ latest). Confirm `.old` is gone after init.

- [ ] **Step 7: Clean up the dummy release**

```bash
gh release delete v9.9.9 --cleanup-tag --yes
```

Roll back to v0.1.1 locally by re-deploying the v0.1.1 DLL (rebuild from the v0.1.1 tag if needed).

- [ ] **Step 8: Toggle off via settings, verify no check fires**

In the ArcDPS options window's AxiPulse pane, uncheck **Check for updates on startup**. Reload the plugin. Confirm no `Checking` state appears, no HTTP request goes out (you can verify via `arcdps.log` — no `axipulse-update-check` thread should log).

---

## Self-review notes

- Spec coverage: every spec section (state machine, threads, HTTP, rename shuffle, opt-out setting, header pill, manual e2e) maps to a numbered task above.
- Types are consistent: `UpdateState` variants used across `snapshot`, `start_install`, and `render_update_pill` match the definition.
- `start_install` takes the DLL dir as a parameter rather than calling `plugin::dll_dir()` itself, so the UI is the only layer that touches the platform-specific resolver.
- All steps have concrete code blocks or commands; no "fill this in later" placeholders.
