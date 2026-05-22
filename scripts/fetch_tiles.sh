#!/usr/bin/env bash
# scripts/fetch_tiles.sh
# Download every WvW map tile (z0..z7, all 4 maps) into src/assets/tiles/
# so the plugin can render the WvW combat replay without runtime network.
# Idempotent: skips tiles that already exist on disk.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$REPO_ROOT/src/assets/tiles"
mkdir -p "$OUT"

# Continent rectangles in continent-pixel space, mirrored from
# src/map/tiles.rs / axipulse src/shared/wvwTiles.ts. Format: name cx1 cy1 cx2 cy2
MAPS=(
    "ebg    8958  12798 12030 15870"
    "green  5630  11518 8190  15102"
    "blue   12798 10878 15358 14462"
    "red    9214  8958  12286 12030"
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
echo "fetching $total WvW tiles → $OUT" >&2
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
