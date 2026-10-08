# Release Notes

Version v0.5.3 — October 8, 2026

## Safer self-updates

The updater now checks that a downloaded update is actually a Windows
DLL before swapping it in. An empty or broken download, such as an
error page saved in place of the file, is deleted and the current
plugin is kept, so a bad download can no longer replace a working
axipulse.

A half-written update left behind by an interrupted download is now
cleaned up at startup, alongside the old copy from a previous update.
