# Release Notes

Version v0.2.4 — June 13, 2026

## Smoother first frame on a new fight

When a fight wrapped, the overlay decoded all the skill and buff icons
on the render thread, which showed up as a brief stutter the moment the
new fight popped in. That work now happens off to the side, so the
first frame stays smooth.
