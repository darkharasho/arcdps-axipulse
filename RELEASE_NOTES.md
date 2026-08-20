# Release Notes

Version v0.4.0 — August 19, 2026

## The .NET runtime is gone

AxiPulse used to ship Elite Insights and a whole .NET runtime inside the
plugin, unpack them to disk on first run, and shell out to them to parse
every log. All of that is gone. Parsing now happens inside the plugin
itself, in Rust, with nothing written to disk and no external process
launched. The DLL dropped from 44.6 MB to 8.5 MB, and the first-run unpack
delay is gone with it.

## Longer fight history

Fights are stored in a much smaller form now — about 1.1 MB each — so the
history dropdown keeps the last 32 fights again instead of 8.

## Two new Timeline lanes

Incoming healing and barrier now have their own lanes. If you don't have
the healing addon installed they say so plainly instead of drawing a flat
line at zero.

## Nothing is guessed anymore

Anywhere a number couldn't actually be measured, the overlay now shows a
dash instead of a confident-looking value. A squad member with no health
data gets an empty bar rather than a full green one, and the Position card
tells you how many seconds it actually measured when you and the commander
weren't both being tracked.

NOTE: Two numbers will read differently than they did in v0.3.4. Down
contribution now follows arcdps's own health-anchored methodology and
splits into damage, crowd control, debuffs, and healing denied. Healing
shown is healing you put on other people — your own self-healing is no
longer folded into the total.
