# Release Notes

Version v0.4.2 — August 22, 2026

## Everyone's showing up as their core class

Every player and enemy on the map was drawing the base profession icon
instead of their elite spec — your Firebrand looked like a Guardian, the
enemy Scourges looked like Necromancers. The log actually reports the
core class and the spec as two separate things, and the icon lookups were
reading the wrong one everywhere except the composition panel.

Fixed for the squad roster card, the player dots, the enemy dots, and the
enemy chips in the composition panel. Nothing to do on your end — the
next fight you record will look right.

## Three specs that had no name

Antiquary, Galeshot and Conduit weren't in the parser's spec table yet,
so anyone playing one fell back to their core class even once the above
was fixed. They're named now, and their icons were already bundled.
