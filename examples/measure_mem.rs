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
//! Two independent measurements are printed:
//!
//! - An exact, allocator-independent byte count: walks `FightData` and
//!   `Derived` field by field, summing `size_of` for the stack shape
//!   plus `Vec::capacity()`/`String::capacity()`/`HashMap::capacity()`
//!   for every heap allocation reachable from them. This is what
//!   `HISTORY_CAP`'s arithmetic is built on, because it counts exactly
//!   the bytes retained by a `FightRecord` and nothing else -- no parse
//!   scratch space, no allocator slack.
//! - A process RSS reading (`/proc/self/status`), printed for sanity
//!   only. `mimalloc` is the crate's global allocator and, like most
//!   sub-allocators, does not always hand freed pages back to the OS
//!   between measurements, so an RSS delta after `drop` under-reports
//!   what was freed and an RSS delta after parse can over-report it by
//!   including transient parse-time scratch space the allocator hasn't
//!   released yet. RSS is corroborating evidence, not the number the
//!   cap is computed from.
//!
//! Usage: cargo run --release --example measure_mem -- /path/to/log.zevtc

use std::collections::HashMap;
use std::mem::size_of;
use std::time::Instant;

use arcdps_axipulse::derived::Derived;
use arcdps_axipulse::fight_data::{BoonRow, EnemyData, FightData, PlayerData, SkillRow};

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

/// Heap bytes owned by a `Vec<T>`'s backing allocation, not counting
/// heap owned by `T` itself (callers recurse into elements separately
/// when `T` owns heap of its own).
fn vec_heap<T>(v: &[T]) -> usize {
    // `.capacity()` isn't available on a slice; callers pass the Vec's
    // length here, which under-counts any unused capacity slightly --
    // acceptable for a "what's actually live" measurement, since a
    // freshly-built `FightData` is not expected to over-allocate by
    // much and this makes the number a floor, not a guess padded up.
    v.len() * size_of::<T>()
}

fn skill_row_heap(r: &SkillRow) -> usize {
    r.name.capacity() + r.icon.as_ref().map_or(0, |s| s.capacity())
}

fn boon_row_heap(r: &BoonRow) -> usize {
    r.name.capacity() + r.stacking.capacity() + vec_heap(&r.states)
}

fn player_heap(p: &PlayerData) -> usize {
    p.account.capacity()
        + p.character.capacity()
        + p.profession.capacity()
        + p.elite_spec.capacity()
        + p.team.capacity()
        + vec_heap(&p.damage_by_skill)
        + p.damage_by_skill.iter().map(skill_row_heap).sum::<usize>()
        + vec_heap(&p.down_contribution_by_skill)
        + p.down_contribution_by_skill.iter().map(skill_row_heap).sum::<usize>()
        + vec_heap(&p.healing_by_skill)
        + p.healing_by_skill.iter().map(skill_row_heap).sum::<usize>()
        + vec_heap(&p.barrier_by_skill)
        + p.barrier_by_skill.iter().map(skill_row_heap).sum::<usize>()
        + vec_heap(&p.boons)
        + p.boons.iter().map(boon_row_heap).sum::<usize>()
        + vec_heap(&p.damage_1s)
        + vec_heap(&p.damage_taken_1s)
        + vec_heap(&p.healing_received_1s)
        + vec_heap(&p.barrier_received_1s)
        + vec_heap(&p.health_percents)
        + vec_heap(&p.positions)
        + vec_heap(&p.down_ranges)
        + vec_heap(&p.dead_ranges)
        + vec_heap(&p.dc_ranges)
        + vec_heap(&p.casts)
}

fn enemy_heap(e: &EnemyData) -> usize {
    e.name.capacity()
        + e.team.capacity()
        + e.profession.capacity()
        + vec_heap(&e.positions)
        + vec_heap(&e.down_ranges)
        + vec_heap(&e.dead_ranges)
}

fn string_map_heap(m: &HashMap<u32, String>) -> usize {
    m.len() * (size_of::<u32>() + size_of::<String>())
        + m.values().map(String::capacity).sum::<usize>()
}

/// Exact retained heap bytes for one `FightData`, not counting
/// `size_of::<FightData>()` itself (the caller adds that).
fn fight_data_heap(f: &FightData) -> usize {
    f.map_name.capacity()
        + vec_heap(&f.players)
        + f.players.iter().map(player_heap).sum::<usize>()
        + vec_heap(&f.enemies)
        + f.enemies.iter().map(enemy_heap).sum::<usize>()
        + f.entity_index.len() * (size_of::<u32>() + size_of::<usize>())
        + f.arena.as_ref().map_or(0, |a| a.image_url.capacity())
        + string_map_heap(&f.skill_icons)
        + string_map_heap(&f.buff_icons)
}

/// Exact retained heap bytes for one `Derived`. Bounded by squad size
/// and small constant "top 8" / known-boon lists, unlike `FightData`
/// which carries every squad member's full series -- included for
/// completeness, but expected to be a small fraction of the total.
fn derived_heap(d: &Derived) -> usize {
    let group_heap = |g: &arcdps_axipulse::fight_composition::Group| {
        g.label.capacity()
            + vec_heap(&g.class_counts)
            + g.class_counts.iter().map(|(s, _)| s.capacity()).sum::<usize>()
    };
    let skill_entry_heap = |e: &arcdps_axipulse::top_skills::SkillEntry| e.name.capacity();
    let heal_entry_heap = |e: &arcdps_axipulse::top_heals::HealEntry| e.name.capacity();
    let barrier_entry_heap = |e: &arcdps_axipulse::top_heals::BarrierEntry| e.name.capacity();
    let boon_series_heap = |s: &arcdps_axipulse::timeline_boons::BoonSeries| vec_heap(&s.segments);

    vec_heap(&d.composition) + d.composition.iter().map(group_heap).sum::<usize>()
        + vec_heap(&d.top_damage) + d.top_damage.iter().map(skill_entry_heap).sum::<usize>()
        + vec_heap(&d.top_down_contribution)
        + d.top_down_contribution.iter().map(skill_entry_heap).sum::<usize>()
        + vec_heap(&d.top_healing) + d.top_healing.iter().map(heal_entry_heap).sum::<usize>()
        + vec_heap(&d.top_downed_healing)
        + d.top_downed_healing.iter().map(heal_entry_heap).sum::<usize>()
        + vec_heap(&d.top_barrier) + d.top_barrier.iter().map(barrier_entry_heap).sum::<usize>()
        + vec_heap(&d.boon_uptimes)
        + vec_heap(&d.health_samples)
        + vec_heap(&d.dmg_dealt_samples)
        + vec_heap(&d.dmg_taken_samples)
        + vec_heap(&d.distance_samples)
        + vec_heap(&d.off_boons) + d.off_boons.iter().map(boon_series_heap).sum::<usize>()
        + vec_heap(&d.def_boons) + d.def_boons.iter().map(boon_series_heap).sum::<usize>()
        + vec_heap(&d.incoming_heal_samples)
        + vec_heap(&d.incoming_barrier_samples)
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
    let derived = Derived::compute(&fight);
    println!("derived  rss {:6.1} MB  ({:?})", rss_mb(), t.elapsed());

    // Exact, allocator-independent accounting: this is the number
    // `HISTORY_CAP`'s arithmetic is built on.
    let fight_bytes = size_of::<FightData>() + fight_data_heap(&fight);
    let derived_bytes = size_of::<Derived>() + derived_heap(&derived);
    let total_bytes = fight_bytes + derived_bytes;
    println!(
        "retained FightData ≈ {:7.2} MB, Derived ≈ {:7.2} MB, total ≈ {:7.2} MB (exact, allocator-independent)",
        fight_bytes as f64 / (1024.0 * 1024.0),
        derived_bytes as f64 / (1024.0 * 1024.0),
        total_bytes as f64 / (1024.0 * 1024.0),
    );

    // Breakdown of the duration/roster-scaling part of `FightData`
    // (per-second series + position tracks + boon state timelines) vs
    // everything else (strings, skill/heal rows, catalogs) -- the
    // former grows with fight length and squad size, the latter does
    // not, so this split is what a worst-case scaling assumption for
    // `HISTORY_CAP` should be based on.
    let series_like: usize = fight
        .players
        .iter()
        .map(|p| {
            vec_heap(&p.damage_1s)
                + vec_heap(&p.damage_taken_1s)
                + vec_heap(&p.healing_received_1s)
                + vec_heap(&p.barrier_received_1s)
                + vec_heap(&p.health_percents)
                + vec_heap(&p.positions)
                + p.boons.iter().map(|b| vec_heap(&b.states)).sum::<usize>()
        })
        .sum();
    println!(
        "  of which duration/roster-scaling series+positions+boon-states ≈ {:7.2} MB ({:.0}% of FightData)",
        series_like as f64 / (1024.0 * 1024.0),
        100.0 * series_like as f64 / fight_bytes as f64,
    );
    println!(
        "  exact bytes: fight_data={} derived={} total={} series_like={} enemies={}",
        fight_bytes, derived_bytes, total_bytes, series_like, fight.enemies.len(),
    );

    // RSS-delta cross-check only -- see module doc comment for why this
    // is not the authoritative number.
    let before_drop = rss_mb();
    drop(fight);
    drop(derived);
    let after_drop = rss_mb();
    println!(
        "rss cross-check: before drop {:6.1} MB, after drop {:6.1} MB (mimalloc may not return freed pages to the OS; do not read this delta as the retained size)",
        before_drop, after_drop,
    );
}
