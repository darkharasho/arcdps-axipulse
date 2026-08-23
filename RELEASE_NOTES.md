# Release Notes

Version v0.4.3 — August 23, 2026

## Half your squad showing up as the enemy

On some fights the squad got cut clean in half — part of it stayed on
your side, the rest were drawn as hostiles, and the actual enemy zerg
vanished from the map entirely. It usually announced itself by the enemy
suddenly being labelled the wrong team colour.

The cause was in the parser, not here. It decided who was on your side
from the *last* team the recording player was seen on, and when you zone
out of a map at the end of a fight the game stamps you onto a couple of
other teams on the way out. Whichever one landed last became "your team"
for the whole log. It now uses the first one instead, which is the one
you actually fought on.

This was a coin flip on whether a map transition happened to land inside
the recording — nothing to do with the fight itself — so it hit some
logs and not others. Any fight you record from here on is fixed; logs you
already recorded need to be re-opened to pick up the correction.
