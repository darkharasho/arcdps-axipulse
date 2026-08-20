//! Host-side memory/timing measurement of the in-process parse path,
//! replaying exactly what `plugin::on_new_log` does: axilog parse →
//! `FightData` projection → `Derived`.
//!
//! The point of the measurement is the `ReportV1` drop: it is released
//! inside `parse_log`, so the retained cost of a fight is the
//! `FightData` + `Derived` pair alone. That is the whole memory
//! rationale for the axilog migration, and this is how to check it did
//! not regress.
//!
//! Usage: cargo run --release --example measure_mem -- /path/to/log.zevtc

use std::time::Instant;

fn rss_mb() -> f64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in s.lines() {
        if let Some(kb) = line.strip_prefix("VmRSS:") {
            let kb: f64 = kb.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0.0);
            return kb / 1024.0;
        }
    }
    0.0
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/wvw.zevtc").into()
    });
    println!("start    rss {:6.1} MB", rss_mb());

    let t = Instant::now();
    let fight = match arcdps_axipulse::parse::parse_log(std::path::Path::new(&path)) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("parse failed: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "parsed   rss {:6.1} MB  ({} players, {} ms fight, {:?})",
        rss_mb(),
        fight.players.len(),
        fight.duration_ms,
        t.elapsed(),
    );

    let t = Instant::now();
    let derived = arcdps_axipulse::derived::Derived::compute(&fight);
    println!("derived  rss {:6.1} MB  ({:?})", rss_mb(), t.elapsed());

    // Approximate retained size: RSS with the pair alive vs after
    // dropping it.
    let before_drop = rss_mb();
    drop(fight);
    drop(derived);
    let after_drop = rss_mb();
    println!(
        "retained fight ≈ {:6.1} MB (rss {:6.1} → {:6.1}; allocator may hold pages)",
        before_drop - after_drop, before_drop, after_drop
    );
}
