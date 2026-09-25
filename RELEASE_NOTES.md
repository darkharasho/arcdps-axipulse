# Release Notes

Version v0.5.0 — September 25, 2026

## The overlay is redrawn in axi-design

AxiPulse's desktop app moved to axi-design — the Axi family's design
language — in its own 0.3.0. This release brings the in-game overlay
across to match, so the two stop looking like different products.

The language is flat and outlined: near-black outlines and hard offset
blocks instead of blurred shadows, square corners everywhere, and
saturated colour used at full strength or not at all. Every surface in
the overlay has been redrawn against it — Pulse, Timeline, the shell
around them, the team bar and the notifier toast.

## Your accent, eleven ways

The chrome now takes an accent colour you pick yourself, from the same
eleven the desktop app offers. It lives in the AxiPulse options pane
inside arcdps and persists across sessions. The default is Emerald Mint.

The accent drives chrome only. Profession colours, team colours and
chart series are the data's own colours, not the system's, and they are
never recoloured by your pick — a red team stays red whatever the
overlay is wearing.

## Reading surfaces went opaque

Pulse, Timeline and the shell are surfaces you open in order to read,
and reading through a moving battlefield is harder than it sounds. They
now paint solid. The team bar and the notifier stay translucent: they
are glance surfaces that sit over gameplay rather than covering it, and
there the transparency earns its keep.

## Smaller things

- Bar labels that cross their fill are inked in two tones, so a label
  never disappears into the colour behind it.
- The fight picker is a proper blocked control with a blocked popup,
  instead of the flat rectangle imgui draws by default.
- The team bar is tighter vertically and centres its empty state.
- Webfonts are gone; the overlay draws in arcdps's own font.

## Notes

The map view is unchanged in this release. It keeps its current styling
and will be brought across separately.
