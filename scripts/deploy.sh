#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$REPO_ROOT/target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll"
DEST="${AXIPULSE_DEPLOY_DEST:-/var/mnt/data/SteamLibrary/steamapps/common/Guild Wars 2/addons/arcdps_axipulse.dll}"

if [[ ! -f "$SRC" ]]; then
    echo "build artifact missing: $SRC — run 'cargo dll' first" >&2
    exit 1
fi

# Atomic DLL install (tmp + rename so a live GW2 doesn't see a truncated inode).
TMP="${DEST}.new"
cp "$SRC" "$TMP"
mv "$TMP" "$DEST"
ls -lh "$DEST"

# Sidecar tile assets. Placed in axipulse-assets/ next to the DLL; the
# plugin resolves them at runtime via Globals::install_root. Only sync
# if the source tile dir exists (engineers without the WvW map feature
# enabled can skip running scripts/fetch_tiles.sh).
TILES_SRC="$REPO_ROOT/src/assets/tiles"
TILES_DEST="${DEST%/*}/axipulse-assets/tiles"
if [[ -d "$TILES_SRC" ]]; then
    mkdir -p "$TILES_DEST"
    # rsync gives us atomic-ish per-file replacement + delete-on-source-removed.
    rsync -a --delete "$TILES_SRC/" "$TILES_DEST/"
    echo "synced tiles → $TILES_DEST"
else
    echo "no tile assets at $TILES_SRC (skip); run scripts/fetch_tiles.sh to populate" >&2
fi
