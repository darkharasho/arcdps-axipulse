# Release Notes

Version v0.2.5 — July 2, 2026

## No More Stutter When a Log Starts Parsing

Parsing a new log no longer lags the game. The parser was technically
running at low priority, but Wine ignores Windows process priorities —
so it still fought GW2 for every CPU core the moment a fight ended.
It's now hard-pinned to 2 cores that the game isn't leaning on, and
told to size all its internal threading for those 2 cores.

Parses take a touch longer, but you shouldn't feel them at all anymore.
