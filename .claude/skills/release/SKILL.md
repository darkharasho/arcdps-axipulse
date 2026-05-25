---
name: release
description: Release a new version of arcdps-axipulse (major, minor, patch, or none for rebuild only)
---

# Release Skill

Bump type: $ARGUMENTS (must be one of: major, minor, patch, none)

This repo has no CI build pipeline — the DLL is built locally with
`cargo dll` and uploaded as the release asset. You have exactly 3 jobs.

## Job 1: Generate Release Notes

1. Read `docs/release-notes-style.md` for the style guide.
2. Gather commit data:
   ```bash
   git tag --sort=-v:refname
   # Pick the first tag that is NOT v<current_version>
   git log <LAST_TAG>..HEAD --no-merges --pretty=format:"%s"
   git diff <LAST_TAG>..HEAD --stat
   git diff <LAST_TAG>..HEAD --unified=2 --no-color
   ```
3. Compute target version:
   - `none`: use current version from `Cargo.toml`
   - Otherwise: bump semver of current version (e.g. 0.1.7 + minor → 0.2.0)
4. Analyze commits/diffs and write release notes following the style guide.
5. Write to `RELEASE_NOTES.md`:
   ```
   # Release Notes

   Version v<VERSION> — <Month Day, Year>

   <notes>
   ```
6. Show notes to user. **Wait for approval before proceeding.**

## Job 2: Version Bump, Build, Tag & Push

After release notes are approved:

1. If bump type is NOT `none`, edit `Cargo.toml` (`version = "<NEW>"`) and run
   `cargo dll-check` so `Cargo.lock` updates.
2. Build the DLL — **must succeed before tagging**:
   ```bash
   cargo dll
   ```
   Artifact path: `target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll`
3. Commit release notes + version bump together:
   ```bash
   git add RELEASE_NOTES.md Cargo.toml Cargo.lock
   git commit -m "chore: release v<VERSION>"
   git push origin main
   ```
4. Tag and push (the tag triggers the Discord-post workflow once the
   release is created in Job 3):
   ```bash
   git tag v<VERSION>
   git push origin v<VERSION>
   ```

## Job 3: Create GitHub Release with DLL

```bash
gh release create v<VERSION> \
  target/x86_64-pc-windows-msvc/release/arcdps_axipulse.dll \
  --title "v<VERSION> — <SHORT_HEADLINE>" \
  --notes-file RELEASE_NOTES.md
```

The asset MUST be named `arcdps_axipulse.dll` exactly — the in-plugin
auto-updater (`src/updater.rs`) only downloads an asset with that
literal name.

`<SHORT_HEADLINE>` should be a one-line summary of the headline change
(under ~50 chars). For `none` rebuilds use the previous release's
headline or "Rebuild".

## Job 4: Verify Discord Post

GitHub Actions `.github/workflows/release.yml` listens on
`release: published` and posts an embed to the Discord webhook stored
in the `DISCORD_WEBHOOK_URL` repo variable. Confirm the run completed:

```bash
gh run list --workflow=release.yml --limit 3
```

Report to the user:
- The new version number
- Link to the release: `https://github.com/darkharasho/arcdps-axipulse/releases/tag/v<VERSION>`
- Whether the Discord post workflow ran (and link the run if not green)
- Reminder that users will see the update on their next GW2 launch
  (the auto-updater only checks on plugin init)

## Notes

- Do NOT `./scripts/deploy.sh` from this skill. The user tests the
  auto-updater end-to-end, which requires the DLL to be downloaded
  from the GitHub release rather than copied locally.
- Never `cargo build --target x86_64-pc-windows-gnu` — the GNU binary
  links but crashes inside GW2.
