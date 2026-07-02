# Release Notes

Version v0.3.0 — July 2, 2026

## Team Bar

A new always-on-screen window that shows who's in the fight as one stacked bar — red, green, and blue player counts side by side, sized by how many of each team were there. It counts your squad and allies alongside enemy players, so a quick glance tells you whether you were outnumbered.

- Team colors come from the log itself when available (newer arcdps logs record which team is which), with a fallback to the known team-id table for older logs. Same logic AxiBridge uses.
- The full view adds the map name, a total player count, and a legend that marks which team is yours — green isn't always your side.
- Prefer something smaller? "Compact team bar" strips it down to just the bar.
- Off by default. Turn it on in the arcdps options pane (or the window list), then drag it wherever you like — it remembers its spot.

NOTE: Logs from before arcdps started recording team data may show a gray "unknown" segment for team ids we can't map.
