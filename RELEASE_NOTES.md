# Release Notes

Version v0.3.3 — July 27, 2026

## No more post-fight lag

The severe lag right after a fight while the log parses is gone. It turned
out the parse was hungry enough on memory to make the whole system choke —
the game itself was getting pushed out of RAM for several seconds. Parsing
is now strictly memory-bounded end to end, and in testing you can't tell a
parse is happening at all.

## Long sessions no longer get worse

Every parsed fight used to stay in memory at full size, so a long WvW night
made the game fatter (and the lag nastier) with every fight. Parsed fights
are now stripped down to just what the overlays actually read — same stats,
a fraction of the memory — and the footprint stays flat no matter how long
you play.

NOTE: The fight history dropdown now keeps the last 8 fights instead of 32.

## Giant fights still parse

Truly massive logs (multi-blob three-way zerg fights) get one bounded parse
attempt first, and automatically retry with the limits off if they're too
big for it. Slightly slower for those monsters, but they won't be dropped.
