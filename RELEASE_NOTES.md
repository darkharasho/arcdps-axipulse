# Release Notes

Version v0.2.2 — May 31, 2026

## Smoother end-of-fight transition

That small stutter that hit right as the "Parsed" toast appeared after a fight is gone. The cached-icon read for each new skill/buff was happening on the render thread; now it runs on the icon worker so the frame that flips to the new fight doesn't have to wait on disk.
