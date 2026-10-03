# Release Notes

Version v0.5.2 — October 3, 2026

## The HUD surfaces are centred in their face

v0.5.1 fixed the notifier's heartbeat riding low and overshot: both the
toast and the team bar came back with a wide dead gap under their
content instead, which is no more centred than being short was.

The helper that compensates a window for the depth its own block draws
inward was reserving a whole blank line of text on top of the block. It
had been written to force the layout engine down a particular branch by
emitting a weightless item first — but that item closes the line by
itself, so the branch it was trying to reach was never the one taken,
and the line it inserted was a real one.

The helper no longer touches layout lines at all. It gives back the gap
the engine had already charged after the last item and reserves exactly
the block's depth, nothing more. Content in both HUD surfaces now sits
centred in the gray face, with the border and the offset block outside
it where they belong.
