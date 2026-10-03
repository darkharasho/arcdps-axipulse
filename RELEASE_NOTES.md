# Release Notes

Version v0.5.1 — October 2, 2026

## The notifier's icon sits where it should

The heartbeat in the parsing toast was riding low — twelve pixels of
padding above it and six below, so the whole content block sat half an
offset off centre in the face.

The cause was in the shared helper that compensates a window for the
depth its own block draws inward. The reservation it makes is deliberately
weightless, and a weightless reservation disappears entirely onto a
layout line that is already a line and a half tall — which is exactly the
line the notifier ends on. The helper now closes the open line first, so
every inward-blocked surface gets the symmetric padding it was always
meant to have. The team bar's empty state was quietly off by the same
amount and is fixed with it.
