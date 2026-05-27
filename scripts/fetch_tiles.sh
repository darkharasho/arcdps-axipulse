#!/usr/bin/env bash
# scripts/fetch_tiles.sh
# Download every WvW map tile (z0..z7, all 4 maps) so the plugin can
# render the WvW combat replay without runtime network. Idempotent:
# skips tiles already on disk.
#
# Destination resolution (first match wins):
#   1. --out <dir>                exact tile dir
#   2. AXIPULSE_TILES_DEST=<dir>   exact tile dir
#   3. AXIPULSE_INSTALL_DIR=<dir>  uses <dir>/axipulse-assets/tiles
#   4. Repo checkout              uses <repo>/src/assets/tiles
#   5. Steam GW2 auto-detect      uses <addons>/axipulse-assets/tiles

set -euo pipefail

OUT=""

usage() {
    cat <<USAGE
fetch_tiles.sh - download WvW map tiles for AxiPulse

Usage:
  fetch_tiles.sh [--out <tile-dir>]

Without args the script tries (in order):
  - AXIPULSE_TILES_DEST env   (full tile dir)
  - AXIPULSE_INSTALL_DIR env  (the GW2 'addons' folder)
  - the repo checkout's src/assets/tiles (developer workflow)
  - Steam's Guild Wars 2 install (auto-detected)

The plugin reads tiles from <addons-dir>/axipulse-assets/tiles/<z>/<x>/<y>.jpg
so end-users typically want either the AXIPULSE_INSTALL_DIR env or to
let the Steam auto-detect handle it.
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out) OUT="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown arg: $1" >&2; usage >&2; exit 2 ;;
    esac
done

if [[ -z "$OUT" && -n "${AXIPULSE_TILES_DEST:-}" ]]; then
    OUT="$AXIPULSE_TILES_DEST"
fi
if [[ -z "$OUT" && -n "${AXIPULSE_INSTALL_DIR:-}" ]]; then
    OUT="$AXIPULSE_INSTALL_DIR/axipulse-assets/tiles"
fi
if [[ -z "$OUT" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    if [[ -f "$SCRIPT_DIR/../Cargo.toml" ]] \
        && grep -q '^name = "arcdps_axipulse"' "$SCRIPT_DIR/../Cargo.toml" 2>/dev/null; then
        OUT="$(cd "$SCRIPT_DIR/.." && pwd)/src/assets/tiles"
    fi
fi
if [[ -z "$OUT" ]]; then
    # Best-effort GW2 install probe: covers standalone, Steam, and
    # common library drives, across git-bash / MSYS / WSL / macOS.
    candidates=(
        # Standalone (non-Steam) installs — the official ArenaNet client.
        "/c/Program Files/Guild Wars 2/addons"
        "/c/Program Files (x86)/Guild Wars 2/addons"
        "/c/Guild Wars 2/addons"
        "/d/Guild Wars 2/addons"
        "/e/Guild Wars 2/addons"
        "/f/Guild Wars 2/addons"
        "/mnt/c/Program Files/Guild Wars 2/addons"
        "/mnt/c/Program Files (x86)/Guild Wars 2/addons"
        "/mnt/c/Guild Wars 2/addons"
        "/mnt/d/Guild Wars 2/addons"
        # Steam installs.
        "/c/Program Files (x86)/Steam/steamapps/common/Guild Wars 2/addons"
        "/c/Program Files/Steam/steamapps/common/Guild Wars 2/addons"
        "/c/SteamLibrary/steamapps/common/Guild Wars 2/addons"
        "/d/SteamLibrary/steamapps/common/Guild Wars 2/addons"
        "/e/SteamLibrary/steamapps/common/Guild Wars 2/addons"
        "/f/SteamLibrary/steamapps/common/Guild Wars 2/addons"
        "/d/Program Files (x86)/Steam/steamapps/common/Guild Wars 2/addons"
        "/mnt/c/Program Files (x86)/Steam/steamapps/common/Guild Wars 2/addons"
        "/mnt/c/Program Files/Steam/steamapps/common/Guild Wars 2/addons"
        "/mnt/c/SteamLibrary/steamapps/common/Guild Wars 2/addons"
        "/mnt/d/SteamLibrary/steamapps/common/Guild Wars 2/addons"
        "$HOME/Library/Application Support/Steam/steamapps/common/Guild Wars 2/addons"
    )
    for c in "${candidates[@]}"; do
        if [[ -d "$c" ]]; then
            OUT="$c/axipulse-assets/tiles"
            echo "auto-detected GW2 install: $c" >&2
            break
        fi
    done
fi
if [[ -z "$OUT" ]]; then
    cat >&2 <<ERR
Could not figure out where to put the tiles.

Pick one:
  fetch_tiles.sh --out <path>
  AXIPULSE_INSTALL_DIR=<gw2>/addons fetch_tiles.sh
  AXIPULSE_TILES_DEST=<path> fetch_tiles.sh

The plugin expects tiles at <addons>/axipulse-assets/tiles/<z>/<x>/<y>.jpg
ERR
    exit 1
fi

mkdir -p "$OUT"

# Continent rectangles in continent-pixel space, mirrored from
# src/map/tiles.rs / axipulse src/shared/wvwTiles.ts. Format: name cx1 cy1 cx2 cy2
MAPS=(
    "ebg    8958  12798 12030 15870"
    "green  5630  11518 8190  15102"
    "blue   12798 10878 15358 14462"
    "red    9214  8958  12286 12030"
    "eotm   5994  8446  9066  11518"
)
CONT=2
FLOOR=3
TILE=256
MAX_Z=7

# Aggregate the union tile set across maps so we don't double-download.
declare -A SEEN
for z in 0 1 2 3 4 5 6 7; do
    span=$(( TILE * (1 << (MAX_Z - z)) ))
    for m in "${MAPS[@]}"; do
        read -r _ cx1 cy1 cx2 cy2 <<< "$m"
        tx_min=$(( cx1 / span ))
        ty_min=$(( cy1 / span ))
        tx_max=$(( (cx2 - 1) / span ))
        ty_max=$(( (cy2 - 1) / span ))
        for (( ty=ty_min; ty<=ty_max; ty++ )); do
            for (( tx=tx_min; tx<=tx_max; tx++ )); do
                SEEN["$z/$tx/$ty"]=1
            done
        done
    done
done

total=${#SEEN[@]}
echo "fetching $total WvW tiles -> $OUT" >&2
i=0
for key in "${!SEEN[@]}"; do
    IFS=/ read -r z tx ty <<< "$key"
    dst="$OUT/$z/$tx/$ty.jpg"
    i=$((i+1))
    if [[ -s "$dst" ]]; then
        continue
    fi
    mkdir -p "$(dirname "$dst")"
    url="https://tiles.guildwars2.com/$CONT/$FLOOR/$z/$tx/$ty.jpg"
    if ! curl -fsSL --retry 3 --retry-delay 1 -o "$dst.tmp" "$url"; then
        echo "  [$i/$total] FAIL $url" >&2
        rm -f "$dst.tmp"
        continue
    fi
    mv "$dst.tmp" "$dst"
    if (( i % 25 == 0 )); then
        echo "  [$i/$total] $z/$tx/$ty" >&2
    fi
done
echo "done." >&2
