# arcdps_axipulse

A Rust ArcDPS plugin that runs the bundled Elite Insights CLI against each
.evtc your client writes, parses the JSON output, and (in follow-up plans)
renders Pulse and Timeline overlays in-game.

## Build

```
./scripts/fetch_ei.sh        # one-time: pull GW2EICLI.zip into vendor/
cargo dll                    # release MSVC build via cargo-xwin
./scripts/deploy.sh          # atomic install into addons/arcdps/
```

Foundation milestone: launches and parses logs; no UI yet. Verify by
fighting in WvW and checking arcdps.log for `axipulse: parsed ...` lines.
