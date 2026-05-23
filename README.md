# arcdps_axipulse

A Rust ArcDPS plugin that runs the bundled Elite Insights CLI against each
.evtc your client writes, parses the JSON output, and renders WvW combat
overlays in-game.

## Features

### WvW Combat Replay (Map tab)

Renders a top-down view of the fight on the matching WvW map:
- Tile background sourced from official GW2 tiles (pre-cached on disk)
- Landmark pins (keeps, towers, camps, ruins)
- Each squad member's position with profession icon
- Time playback: scrubber, play/pause, speed (0.5×–2×), motion trails
- Per-player state overlays: skull (dead) / down-pin markers on the map; sliding party panel with HP bars, distance-to-commander, boon stacks, and recent skill casts
- Camera: mouse-wheel zoom (cursor-anchored), left-click drag to pan, Reset button, Follow toggle to keep your dot centred, auto-zoom + centre on the squad when a fight opens

### Notifier toast

A small "Parsed" toast appears on each new log with the WvW map name and
coloured ally/enemy counts, so you can confirm logs are landing without
keeping the main AxiPulse window open.

### Parser

- Bundles the Elite Insights CLI + .NET 8 runtime — no separate install
- Parser thread and EI subprocess run at below-normal priority so the
  cold-start doesn't cost you a frame at parse-start

## Install (manual)

1. Download `arcdps_axipulse.dll` from the [latest release](https://github.com/darkharasho/arcdps-axipulse/releases/latest)
2. Drop it into your GW2 ArcDPS addons folder, alongside `arcdps.dll`:
   - Steam: `…/Guild Wars 2/addons/arcdps/arcdps_axipulse.dll`
   - Standalone: `<GW2 install>/addons/arcdps/arcdps_axipulse.dll`
3. Launch GW2. Confirm by opening the ArcDPS options window — an
   **AxiPulse** entry should appear.

The WvW Map tab needs the GW2 map tiles. They are not shipped with the
DLL — run `./scripts/fetch_tiles.sh` from a source checkout (or build
from source, below) to populate them. Without the tiles the rest of the
plugin still works; the Map tab will just render without a background.

To uninstall, delete `arcdps_axipulse.dll` (and the `axipulse-assets/`
folder, if you grabbed it) and restart GW2.

## Build from source

Target is `x86_64-pc-windows-msvc`. From Linux, cross-compile via
`cargo-xwin`:

```
./scripts/fetch_ei.sh        # one-time: pull GW2EICLI.zip into vendor/
./scripts/fetch_tiles.sh     # one-time: populate src/assets/tiles/ (~25 MB)
cargo dll                    # release MSVC build
./scripts/deploy.sh          # atomic install into addons/arcdps/
```

Set `AXIPULSE_DEPLOY_DEST` to point `deploy.sh` at a non-default install.

Verify the build by fighting in WvW and checking `arcdps.log` for
`axipulse: parsed …` lines.
