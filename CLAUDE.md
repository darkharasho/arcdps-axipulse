## Project Context

A Rust ArcDPS plugin that runs the bundled Elite Insights CLI on each
new .evtc/.zevtc, deserialises the JSON, and (future) renders Pulse and
Timeline overlays.

## Build

Must be MSVC. Cross-compile from Linux with `cargo-xwin`:
- `cargo dll`        release artifact at `target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll`
- `cargo dll-dev`    unoptimised iteration build
- `cargo dll-check`  type-check only
- `cargo test`       host-side unit tests (non-cfg(windows) modules)

Never `cargo build --target x86_64-pc-windows-gnu` for the DLL — the
GNU binary links but crashes on load inside GW2.

## Deploying

Always use `./scripts/deploy.sh` (tmp + atomic rename). Never `cp` the
DLL straight into `addons/` while GW2 is running — under Wine, `cp`
truncates the existing inode in place and corrupts pages of the loaded
DLL that GW2 has mmap'd as executable.
