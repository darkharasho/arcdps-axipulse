# Release Notes

Version v0.5.5 — October 8, 2026

## Fixes

- Fixed a crash in axipulse that could happen when one of the game's
  threads shut down. arcdps caught it and the game kept running, but
  addon windows could stop responding afterwards. Fights still parse
  without a frame hitch.
