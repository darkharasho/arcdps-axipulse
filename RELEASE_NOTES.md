# Release Notes

Version v0.2.1 — May 27, 2026

## Smoother Parse-Start

No more ~1s frame stutter when a new fight log lands. The Elite
Insights subprocess now runs at idle priority and the parser thread
runs at the lowest priority, so the .NET cold-start that fires the
moment a log is found yields to GW2 instead of fighting it for CPU.
Parses may take slightly longer on a busy system, but you shouldn't
feel them anymore.

## Fixes

- `scripts/fetch_tiles.sh` now finds the standalone (non-Steam) Guild
  Wars 2 install. Previously the auto-detect only knew about Steam
  layouts, so anyone on the regular ArenaNet client hit "Could not
  figure out where to put the tiles" and the script quit.

NOTE: You don't actually need this script — the plugin auto-downloads
WvW map tiles on launch. The script is just there for developers and
for pre-staging tiles before an offline session.
