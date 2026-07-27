//! Host-side memory/timing measurement of the post-EI parse steps,
//! replaying exactly what `parse_log` does after the subprocess exits
//! (ISIZE-sized decompress → serde → Derived → slim).
//! Usage: cargo run --release --example measure_mem -- /tmp/ei-sample.json.gz

use std::io::Read;
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
    let path = std::env::args().nth(1).unwrap_or_else(|| "/tmp/ei-sample.json.gz".into());
    println!("start rss {:6.1} MB", rss_mb());

    let t = Instant::now();
    let bytes = std::fs::read(&path).expect("read gz");
    println!("gz read  rss {:6.1} MB  ({} bytes, {:?})", rss_mb(), bytes.len(), t.elapsed());

    let t = Instant::now();
    let cap = arcdps_axipulse::ei_parser::gzip_isize(&bytes)
        .filter(|&n| n <= 1_500_000_000)
        .unwrap_or(bytes.len().saturating_mul(4));
    let mut decompressed = Vec::with_capacity(cap);
    {
        let mut gz = flate2::read::GzDecoder::new(&bytes[..]);
        gz.read_to_end(&mut decompressed).expect("gunzip");
    }
    drop(bytes);
    println!(
        "gunzip   rss {:6.1} MB  (len {} cap {} — {:?})",
        rss_mb(), decompressed.len(), decompressed.capacity(), t.elapsed()
    );

    let t = Instant::now();
    let mut json: arcdps_axipulse::ei_model::EiJson =
        serde_json::from_slice(&decompressed).expect("deserialise");
    println!("serde    rss {:6.1} MB  ({:?})", rss_mb(), t.elapsed());

    drop(decompressed);
    println!("dropped buffers rss {:6.1} MB", rss_mb());

    let t = Instant::now();
    let derived = arcdps_axipulse::derived::Derived::compute(&json);
    println!("derived  rss {:6.1} MB  ({:?})", rss_mb(), t.elapsed());

    let t = Instant::now();
    arcdps_axipulse::slim::slim_after_derive(&mut json);
    println!("slimmed  rss {:6.1} MB  ({:?})", rss_mb(), t.elapsed());

    // Approximate retained size: RSS with tree alive vs after dropping it.
    let before_drop = rss_mb();
    drop(json);
    drop(derived);
    let after_drop = rss_mb();
    println!(
        "retained tree ≈ {:6.1} MB (rss {:6.1} → {:6.1}; allocator may hold pages)",
        before_drop - after_drop, before_drop, after_drop
    );
}
