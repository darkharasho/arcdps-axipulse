# AxiPulse auto-updater — design

Date: 2026-05-23
Status: Approved (pending spec review)

## Goal

On each plugin load, check GitHub for a newer stable release of
`arcdps_axipulse`. If one exists, surface it in the AxiPulse window
header. User clicks Install; the new DLL is downloaded and atomically
staged in place. The new version loads on the next GW2 start.

## Non-goals

- Auto-installing on detection (user must click).
- Pre-release / beta channel selection.
- Tile asset updates (releases ship only the DLL).
- Background re-checks on a timer (only at plugin init).

## User-visible flow

1. GW2 starts, plugin loads.
2. If `Config.auto_update_check` is `true`, a background thread hits
   the GitHub "latest release" endpoint.
3. If a newer version exists, a small accented pill appears in the
   AxiPulse window header: `Update available · v0.1.2`. Clicking it
   expands an inline panel with the release body and an **Install**
   button.
4. Install → pill becomes `Downloading… {pct}%`.
5. On success the staged DLL is moved into place and the pill becomes
   amber `Restart GW2 to load v0.1.2`. The currently running DLL keeps
   working until restart.
6. On failure the pill becomes a dismissible red `Update failed:
   <reason>`. We never auto-retry within a session.

The opt-out lives in the AxiPulse settings: a "Check for updates on
startup" checkbox, default on.

## Architecture

### `src/updater.rs` (new)

Owns the state machine and threads.

```rust
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available { tag: String, body: String, asset_url: String },
    Downloading { tag: String, pct: f32 },
    Installed   { tag: String },        // shuffle done; needs GW2 restart
    Failed      { msg: String },
}

pub static STATE: Mutex<UpdateState>;

pub fn kick_check_on_load(install_root: PathBuf, current_version: &str, enabled: bool);
pub fn start_install();   // called from UI
pub fn dismiss_error();   // resets Failed → Idle
```

Internals:

- `kick_check_on_load` is called once during plugin init. If
  `enabled == false` it sets `STATE = Idle` and returns immediately
  (no thread spawned).
- Otherwise spawns a short-lived `std::thread` named
  `axipulse-update-check`.
- `start_install` snapshots the current `Available` state, sets
  `STATE = Downloading { pct: 0 }`, and spawns
  `axipulse-update-download`.
- Threads communicate with the UI only through `STATE` — no channels.

### `src/config.rs`

Add a new field:

```rust
pub struct Config {
    // …
    pub auto_update_check: bool,   // default: true
}
```

Persisted alongside existing settings. Serde default ensures older
config files load cleanly.

### `src/ui/main.rs`

Renders the header pill based on `updater::STATE`. State-by-state UI:

- `Idle` / `UpToDate` — nothing.
- `Checking` — small muted "Checking for updates…" text (optional;
  cheap to add).
- `Available { tag, body, .. }` — green pill, click toggles expanded
  panel with `body` and `Install` button.
- `Downloading { pct }` — pill shows `Downloading {pct:.0}%`.
- `Installed { tag }` — amber pill `Restart GW2 to load {tag}`. Stays
  until the session ends.
- `Failed { msg }` — red pill `Update failed: {msg}`, with a small ×
  to dismiss.

### `src/ui/options.rs`

Inside the existing `render_options_end(ui, config)` add a checkbox
bound to `config.auto_update_check`, placed under a small "Updates"
heading.

## Data flow

```
plugin init
   │
   ▼
config.auto_update_check?
   │ no → STATE = Idle, exit
   │ yes
   ▼
spawn axipulse-update-check
   │
   ▼
GET api.github.com/repos/darkharasho/arcdps-axipulse/releases/latest
   │ network fail / non-2xx → STATE = Failed
   │ ok
   ▼
parse_latest(body) → {tag, asset_url, body}
   │ parse fail → STATE = Failed
   │ ok
   ▼
semver compare tag vs CARGO_PKG_VERSION
   │ ≤ current → STATE = UpToDate
   │ > current → STATE = Available { … }
                       │
                       ▼  user clicks Install
                  spawn axipulse-update-download
                       │
                       ▼
                  STATE = Downloading { pct }
                       │
                       ▼
                  stream asset → <root>/arcdps_axipulse.dll.new
                       │
                       ▼
                  if exists, delete <root>/arcdps_axipulse.dll.old
                  rename arcdps_axipulse.dll → arcdps_axipulse.dll.old
                  rename arcdps_axipulse.dll.new → arcdps_axipulse.dll
                       │
                       ▼
                  STATE = Installed { tag }
```

## Why rename-over-loaded-DLL works

Both Windows and Linux/Wine permit renaming a loaded DLL: the OS
holds a handle to the inode/section, not the path. This is the trick
Firefox's updater uses and is what `scripts/deploy.sh` already relies
on. We deliberately avoid `cp` (truncate in place) since that
corrupts pages of the live DLL under Wine — see `CLAUDE.md`.

A leftover `arcdps_axipulse.dll.old` from a prior session is deleted
at the start of the swap. If the delete fails (Windows holding the
handle for some reason), we proceed anyway and try again next time.

## HTTP details

- Endpoint: `https://api.github.com/repos/darkharasho/arcdps-axipulse/releases/latest`
- Headers: `User-Agent: arcdps_axipulse/<version>` (GitHub requires
  a UA), `Accept: application/vnd.github+json`.
- Latest endpoint already excludes pre-releases.
- Auth: none. Unauthenticated rate limit is 60/hr per IP; one call per
  plugin init is well within.
- Asset selection: the `assets[]` entry whose `name ==
  "arcdps_axipulse.dll"`. Use its `browser_download_url`.
- Download: streaming `ureq::get(url).call()?.into_reader()` → file,
  with periodic `pct` updates based on `Content-Length`. If
  `Content-Length` is missing, leave `pct` at an indeterminate sentinel
  (e.g. `f32::NAN`) and the UI renders a spinner instead of a number.

## Dependencies

- `ureq` — already present, has TLS.
- `serde_json` — already present.
- `semver = "1"` — new, small.

## Testing

Pure logic split out and tested host-side (no `cfg(windows)`):

- `fn parse_latest(json: &str, current: &str) -> ParseOutcome` with
  variants `Newer { tag, body, asset_url }`, `Current`, or
  `ParseError`. Unit tests cover: newer version, equal version, older
  version (downgrade ignored), missing DLL asset, malformed JSON.
- `semver` compare cases including pre-release suffixes if we ever see
  them on `tag_name`.

End-to-end (manual, since the side effects touch the live install):

1. With current = `v0.1.1` installed, push a `v0.1.2` release with a
   dummy DLL.
2. Launch GW2, verify pill appears.
3. Click Install, watch progress, confirm `arcdps_axipulse.dll.old` and
   the new `arcdps_axipulse.dll` are in place.
4. Restart GW2, confirm v0.1.2 loads and the `.old` is cleaned up.

## Error handling

Every failure path: `STATE = Failed { msg }`, log at `log::warn`. Never
panic, never block plugin init. The pill is dismissible. We do not
auto-retry within a session.

## Out of scope (future work)

- Background re-check (e.g. every N hours during a long session).
- Pre-release channel toggle.
- Bundling tile assets in releases and updating them alongside the DLL.
- Self-test that the new DLL loads cleanly before committing the swap
  (would require a launcher process — not worth it for now).
