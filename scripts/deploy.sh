#!/usr/bin/env bash
# Atomic install of arcdps_axipulse.dll into the GW2 addons dir under Wine.
# Never `cp` straight into the live folder — see CLAUDE.md.
set -euo pipefail

SRC="target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll"
DEST="${AXIPULSE_DEPLOY_DEST:-/var/mnt/data/SteamLibrary/steamapps/common/Guild Wars 2/addons/arcdps_axipulse.dll}"

if [[ ! -f "$SRC" ]]; then
    echo "build artifact missing: $SRC — run 'cargo dll' first" >&2
    exit 1
fi

TMP="${DEST}.new"
cp "$SRC" "$TMP"
mv "$TMP" "$DEST"
ls -lh "$DEST"
