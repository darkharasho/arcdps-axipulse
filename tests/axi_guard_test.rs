//! The axi-design contract, enforced against the UI sources as TEXT.
//!
//! Reading source rather than calling it is what lets this run on Linux
//! while the modules it guards are `#[cfg(windows)]`. It is the parity
//! of `tests/renderer/tokens.test.ts` in the desktop app, and it is the
//! only thing that will stop the 106 colour literals this conversion
//! removed from creeping back one convenience constant at a time.

use std::path::{Path, PathBuf};

/// Files allowed to hold a colour literal — and ONLY a colour literal.
/// `theme.rs` owns chrome, `series.rs` owns the domain palettes, and
/// nothing else owns either. Both are still held to the square-corner
/// rule, which is why they are exempted inside the colour test rather
/// than dropped from the walk: `theme::push_form` pushes eight rounding
/// vars and every one of them must be zero.
const ALLOWED: [&str; 2] = ["src/ui/theme.rs", "src/ui/series.rs"];

/// DEFERRED, NOT EXEMPT. The map keeps its own theme and its own colour
/// constants for now: a half-converted map reads as a bug rather than
/// as a deferral, so it is converted whole later or not at all. When it
/// is converted, delete these entries — do not add to them.
///
/// See the residuals note in `docs/superpowers/`.
const DEFERRED: [&str; 5] = [
    "src/ui/map.rs",
    "src/map/mod.rs",
    "src/map/boon_panel.rs",
    "src/map/tiles.rs",
    "src/map/wvw.rs",
];

/// Surfaces not yet converted. Each conversion task deletes its own
/// entry; the list reaching empty is the conversion being done. Unlike
/// DEFERRED this is temporary scaffolding — if you are reading this
/// after the branch merged and it is non-empty, something was skipped.
const PENDING: [&str; 6] = [
    "src/ui/options.rs",
    "src/ui/main.rs",
    "src/ui/pulse.rs",
    "src/ui/timeline.rs",
    "src/ui/team_bar.rs",
    "src/ui/notifier.rs",
];

/// Roots walked for guardable `.rs` files. Walking rather than listing
/// means a NEW ui file is guarded by default, which is the only way an
/// allowlist stays honest.
const ROOTS: [&str; 1] = ["src/ui"];

/// Individually guarded files outside the walked roots: the two modules
/// whose domain colours moved into `series.rs`.
const EXTRA_FILES: [&str; 2] = ["src/wvw_teams.rs", "src/fight_composition.rs"];

/// Opt-out marker. A line carrying this is skipped, and the text after
/// it is the reason. Never add one without a reason a reviewer can
/// weigh.
const ALLOW_MARKER: &str = "axi-guard: allow";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rel(p: &Path) -> String {
    p.strip_prefix(repo_root())
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every file this test is responsible for right now.
fn guarded_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut found = Vec::new();
    for r in ROOTS {
        collect_rs(&root.join(r), &mut found);
    }
    for f in EXTRA_FILES {
        found.push(root.join(f));
    }
    found.sort();
    found.retain(|p| {
        let r = rel(p);
        !DEFERRED.contains(&r.as_str()) && !PENDING.contains(&r.as_str())
    });
    found
}

/// Is this a four-component float array — i.e. a colour?
///
/// Deliberately narrow: exactly four comma-separated components that
/// all parse as `f32`. `[0.0, 4.0]` (a two-component imgui vec) and
/// `[x, y, w, h]` (identifiers) are not colours and are not flagged.
fn is_colour_literal(body: &str) -> bool {
    let parts: Vec<&str> = body.split(',').map(str::trim).collect();
    parts.len() == 4 && parts.iter().all(|p| p.parse::<f32>().is_ok())
}

/// Every top-level `[...]` body on a line, without nesting.
fn bracket_bodies(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut buf = String::new();
    for ch in line.chars() {
        match ch {
            '[' => {
                depth += 1;
                if depth == 1 { buf.clear(); }
            }
            ']' => {
                if depth == 1 { out.push(buf.clone()); }
                depth = depth.saturating_sub(1);
            }
            _ if depth >= 1 => buf.push(ch),
            _ => {}
        }
    }
    out
}

/// The argument text of every `rounding(` / `Rounding(` call on a line.
fn rounding_args(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let rest = &line[i..];
        let hit = rest.starts_with("rounding(") || rest.starts_with("Rounding(");
        if !hit {
            i += 1;
            continue;
        }
        let open = i + rest.find('(').expect("just matched");
        let mut depth = 0usize;
        let mut end = None;
        for (off, ch) in line[open..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 { end = Some(open + off); break; }
                }
                _ => {}
            }
        }
        if let Some(close) = end {
            out.push(line[open + 1..close].trim().to_string());
            i = close + 1;
        } else {
            i = open + 1;
        }
    }
    out
}

/// Comment and string content confuse every rule here, so lines that
/// are wholly a comment are skipped. A colour literal hiding inside a
/// doc comment is not a colour the plugin draws.
fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("*") || t.starts_with("/*")
}

#[test]
fn no_colour_literal_lives_outside_theme_and_series() {
    let mut violations: Vec<String> = Vec::new();
    for path in guarded_files() {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        if ALLOWED.contains(&rel(&path).as_str()) { continue; }
        for (n, line) in text.lines().enumerate() {
            if is_comment(line) || line.contains(ALLOW_MARKER) { continue; }
            for body in bracket_bodies(line) {
                if is_colour_literal(&body) {
                    violations.push(format!("{}:{}: [{}]", rel(&path), n + 1, body.trim()));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "colour literals belong in src/ui/theme.rs (chrome) or \
         src/ui/series.rs (domain palettes), nowhere else:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn every_corner_is_square() {
    // --axi-radius and --axi-radius-sm are both 0 in the language, and
    // that is a contract rather than a default. A single missed
    // rounding var shows up as one softened widget, which is very hard
    // to spot in a screenshot.
    let mut violations: Vec<String> = Vec::new();
    for path in guarded_files() {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        for (n, line) in text.lines().enumerate() {
            if is_comment(line) || line.contains(ALLOW_MARKER) { continue; }
            for arg in rounding_args(line) {
                let square = arg == "0.0" || arg == "0" || arg == "0.0_f32";
                if !square {
                    violations.push(format!("{}:{}: rounding({arg})", rel(&path), n + 1));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "axi-design has square corners; rounding must be 0.0:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn the_exclusion_lists_still_point_at_real_files() {
    // A path-based exclusion for a file that has been renamed or split
    // silently matches nothing, and the guard keeps passing over a file
    // nobody is guarding. This is the check that makes the lists rot
    // loudly instead of quietly.
    let root = repo_root();
    for group in [&DEFERRED[..], &PENDING[..], &ALLOWED[..]] {
        for p in group {
            assert!(
                root.join(p).is_file(),
                "{p} is listed in the guard's exclusions but does not exist; \
                 the file moved and the list did not"
            );
        }
    }
}

#[test]
fn the_guard_is_actually_guarding_something() {
    // If the walk, the roots, or the allowlists ever conspire to leave
    // nothing guarded, both rules above pass vacuously. This is the
    // canary for that.
    let files = guarded_files();
    assert!(
        files.len() >= 2,
        "only {} file(s) guarded — check ROOTS and the exclusion lists: {:?}",
        files.len(),
        files.iter().map(|p| rel(p)).collect::<Vec<_>>()
    );
}
