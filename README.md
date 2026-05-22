# arcdps_axipulse

A Rust ArcDPS plugin that runs the bundled Elite Insights CLI against each
.evtc your client writes, parses the JSON output, and (in follow-up plans)
renders Pulse and Timeline overlays in-game.

## Features

### WvW Combat Replay (Map tab) — MVP

Renders a static top-down view of the fight on the matching WvW map:
- Tile background sourced from official GW2 tiles (pre-cached on disk).
- Landmark pins (keeps, towers, camps, ruins).
- Each squad member's final position with profession icon.

**One-time setup:** run `./scripts/fetch_tiles.sh` to populate
`src/assets/tiles/` (~25 MB). Re-run `./scripts/deploy.sh` so the
sidecar `axipulse-assets/tiles/` is synced next to the DLL.

Time playback, pan/zoom, and state overlays (down/dead, boons, skill
casts) ship in follow-up plans.

## Build

```
./scripts/fetch_ei.sh        # one-time: pull GW2EICLI.zip into vendor/
cargo dll                    # release MSVC build via cargo-xwin
./scripts/deploy.sh          # atomic install into addons/arcdps/
```

Foundation milestone: launches and parses logs; no UI yet. Verify by
fighting in WvW and checking arcdps.log for `axipulse: parsed ...` lines.
