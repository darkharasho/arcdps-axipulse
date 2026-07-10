# Release Notes

Version v0.3.2 — July 9, 2026

## No more stutter when a log finishes parsing

Fixed the split-second game hang when a new log landed. The plugin now
uses its own memory allocator instead of sharing the game's, so
crunching a big fight in the background can't stall the render thread
anymore.

## Team bar cleanup

The team count bar got a proper polish pass:

- Every team now always shows its player count — tiny teams used to
  render as an unreadable sliver with no number.
- Smooth gradient fills and a crisp outline replace the old flat look
  with its hard gloss edge.
- Fixed segments in the middle of the bar drawing rounded corners
  where they should be square.
