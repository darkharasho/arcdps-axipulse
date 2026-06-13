# Release Notes

Version v0.2.3 — June 13, 2026

## Smoother parsing

No more second-long freeze when a new log lands. The Elite Insights
parser is a .NET app, and under Wine its startup was spinning up a pile
of background threads all at once — enough to stall the game for a beat
every time a fight ended. It now starts lean, so parsing stays out of
your way.

NOTE: If you still feel a hitch on very large fights, let me know — there's
a deeper CPU-isolation fix I can apply next.
