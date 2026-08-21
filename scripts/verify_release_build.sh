#!/usr/bin/env bash
# Guard against shipping a DLL built before the version bump landed.
#
# v0.4.0 shipped with CARGO_PKG_VERSION baked in as 0.3.4 because `cargo dll`
# ran against a Cargo.toml that hadn't been bumped yet. The binary was correct
# in every other way, but src/updater.rs compares env!("CARGO_PKG_VERSION")
# against the latest release tag, so every launch saw itself as out of date and
# re-downloaded the identical asset.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DLL="$REPO_ROOT/target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll"
MANIFEST="$REPO_ROOT/Cargo.toml"

VERSION="${1:-$(sed -n '/^\[package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$MANIFEST" | head -1)}"
[[ -n "$VERSION" ]] || { echo "could not determine version" >&2; exit 1; }

[[ -f "$DLL" ]] || { echo "build artifact missing: $DLL — run 'cargo dll'" >&2; exit 1; }

# 1. The artifact must be newer than the manifest. If Cargo.toml was touched
#    after the build, the version baked into the DLL is stale by definition.
if [[ "$MANIFEST" -nt "$DLL" ]]; then
    echo "STALE BUILD: Cargo.toml is newer than the DLL." >&2
    echo "  Cargo.toml: $(date -r "$MANIFEST" '+%F %T')" >&2
    echo "  DLL:        $(date -r "$DLL" '+%F %T')" >&2
    echo "Re-run 'cargo dll' before tagging." >&2
    exit 1
fi

# 2. The version string must actually be present in the binary.
if ! grep -aqF "$VERSION" "$DLL"; then
    echo "STALE BUILD: '$VERSION' does not appear anywhere in the DLL." >&2
    exit 1
fi

echo "verified: $DLL carries version $VERSION"
