# Release Notes

Version v0.4.1 — August 20, 2026

## The update prompt that wouldn't go away

The v0.4.0 download had the wrong version number baked into it — it
reported itself as 0.3.4. That meant the updater compared itself against
v0.4.0 on GitHub, decided it was out of date, and re-downloaded the exact
same file every time you launched the game. It also showed up as 0.3.4 in
arcdps.log, so there was no way to tell which build you were actually
running.

This release is the same code as v0.4.0, built with the right version in
it. Install it once and the update prompt stops coming back.

NOTE: Nothing else changed. If you're on v0.4.0 you already have every
feature and fix from those notes — only the version stamp was wrong.
