//! `pulse_metrics` over the real fixture.
//!
//! Through Task 7, the Pulse Overview scalars and the Defense subview's
//! mitigation counters were additionally proved against an Elite
//! Insights equality oracle, deleted by Task 8 -- the oracle has served
//! its purpose. What remains native-only below covers the same two
//! surfaces: every scalar/counter is populated and sane for the local
//! player (deaths/downs/strips/cleanses/mitigation counters are
//! unsigned, so "sane" means "the accessor runs and, where the fixture
//! guarantees a nonzero measurement, is nonzero").

mod common;

use arcdps_axipulse::fight_data::{FightData, PlayerData};
use arcdps_axipulse::pulse_metrics::*;

#[test]
fn derives_for_the_local_player() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let p = &f.players[f.self_idx.expect("fixture resolves a local player")];

    assert!(damage(p) > 0, "local player dealt no damage");
    assert!(dps_value(p) > 0);
    assert!(damage_taken(p) > 0);
    let _ = cleanses(p);
    let _ = strips(p);
    let _ = deaths(p);
    let _ = downs(p);
    let _ = down_contribution(p);
}

/// The six scalars the Pulse Overview shows are populated for the local
/// player -- a regression guard against the block going silently empty,
/// not a proof of correctness (the EI oracle did that once; see the
/// module doc).
#[test]
fn the_overview_scalars_are_populated() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let p = &f.players[f.self_idx.expect("fixture resolves a local player")];

    assert!(damage(p) > 0, "damage");
    assert!(damage_taken(p) > 0, "damage_taken");
    // deaths/downs/strips/cleanses are u32/u64 and so cannot be
    // negative; the accessor merely needs to run without panicking,
    // which the call above already exercises via `derives_for_the_local_player`.
}

/// The six mitigation counters the Defense subview shows -- the fields
/// this migration added to `PlayerData`. On this fixture no single
/// squad member is guaranteed to log every kind of mitigation event, so
/// the invariant is squad-wide: each counter is nonzero for *someone*,
/// which would fail if the whole block came back structurally empty.
#[test]
fn the_mitigation_counters_are_populated_across_the_squad() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let squad: Vec<_> = f.players.iter().filter(|p| p.in_squad).collect();

    let sum = |f: fn(&PlayerData) -> u32| -> u32 { squad.iter().map(|p| f(p)).sum() };
    assert!(sum(blocked) > 0, "blocked is uniformly zero across the squad");
    assert!(sum(evaded) > 0, "evaded is uniformly zero across the squad");
    assert!(sum(dodges) > 0, "dodges is uniformly zero across the squad");
    assert!(sum(missed) > 0, "missed is uniformly zero across the squad");
    assert!(sum(interrupted) > 0, "interrupted is uniformly zero across the squad");
    // `invulned` is not asserted nonzero -- a squad with no invuln
    // uptime logged is a legitimate zero, not a projection failure.
    let _ = sum(invulned);
}

/// `dist_to_tag` is `Option`, never a fabricated 0. `PlayerData` already
/// collapsed EI's `-1` sentinel to `None`, so a present value is always
/// a real, non-negative distance.
#[test]
fn dist_to_tag_reports_absence_as_none_not_zero() {
    assert_eq!(dist_to_tag(&PlayerData::default()), None);
    let p = PlayerData { dist_to_com: Some(0.0), ..PlayerData::default() };
    assert_eq!(dist_to_tag(&p), Some(0.0));

    let n = common::native();
    let f = FightData::from_report(&n);
    for p in f.players.iter().filter(|p| p.in_squad) {
        if let Some(d) = dist_to_tag(p) {
            assert!(d >= 0.0, "{} reported a negative distance {d}", p.account);
        }
    }
}

/// `hps` is derived here rather than read off a field, so pin the
/// definition: total healing over the fight's own duration, and 0 for a
/// zero-length fight rather than a divide-by-zero.
#[test]
fn hps_divides_healing_by_the_fight_duration() {
    let p = PlayerData { healing_out: 60_000, ..PlayerData::default() };
    assert_eq!(hps(&p, 30_000), 2000);
    assert_eq!(hps(&p, 0), 0);
}

/// Incoming healing is the LAST bucket of a cumulative series, i.e. the
/// total -- not the last second's delta.
#[test]
fn incoming_healing_reads_the_cumulative_total() {
    let p = PlayerData {
        healing_received_1s: vec![0, 100, 250, 400],
        barrier_received_1s: vec![0, 10, 10],
        ..PlayerData::default()
    };
    assert_eq!(incoming_healing(&p), 400);
    assert_eq!(incoming_barrier(&p), 10);
    assert_eq!(incoming_healing(&PlayerData::default()), 0);
}
