//! `pulse_metrics` over the real fixture, with equality oracles against
//! Elite Insights for the scalars that have an EI counterpart.

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

/// Ratio-equal within 1%, the same tolerance
/// `fight_data_scalars_test::close` already established for the two
/// summed damage quantities. Not widened here: measured on this
/// fixture's local player, `damage` is exact and `damage_taken` is
/// 54987 native vs 54983 EI -- 4 absolute, 0.007%.
fn close(a: u64, b: u64) -> bool {
    if a == 0 && b == 0 {
        return true;
    }
    ((a as f64 - b as f64).abs() / a.max(b) as f64) < 0.01
}

/// **Equality oracle** for the six scalars the Pulse Overview shows.
#[test]
fn the_overview_scalars_match_the_ei_oracle() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let e = common::ei();
    let p = &f.players[f.self_idx.expect("fixture resolves a local player")];
    let ei = e
        .players
        .iter()
        .find(|x| x.account == p.account)
        .expect("the local player appears in the EI baseline");

    let ei_damage = ei.dps_all.first().map(|d| d.damage).unwrap_or(0);
    assert!(close(damage(p), ei_damage), "damage: {} vs {ei_damage}", damage(p));
    let ei_taken = ei.defenses.first().map(|d| d.damage_taken).unwrap_or(0);
    assert!(
        close(damage_taken(p), ei_taken),
        "damage taken: {} vs {ei_taken}",
        damage_taken(p),
    );
    assert_eq!(deaths(p), ei.defenses.first().map(|d| d.dead_count).unwrap_or(0), "deaths");
    // `downs` is a pre-filed upstream divergence, already bounded by
    // `fight_data_scalars_test`: native is never lower than EI and the
    // gap never exceeds 2. Measured here for the local player: native 1,
    // EI 0. Same bound, not a wider one.
    let ei_downs = ei.defenses.first().map(|d| d.down_count).unwrap_or(0) as i64;
    assert!(
        (0..=2).contains(&(downs(p) as i64 - ei_downs)),
        "downs diverged beyond the measured bound: native={} ei={ei_downs}",
        downs(p),
    );
    assert_eq!(
        strips(p),
        ei.support.first().map(|s| s.boon_strips).unwrap_or(0),
        "strips",
    );
    assert_eq!(
        cleanses(p),
        ei.support
            .first()
            .map(|s| s.condi_cleanse + s.condi_cleanse_self)
            .unwrap_or(0),
        "cleanses",
    );
}

/// **Equality oracle** for the six mitigation counters the Defense
/// subview shows -- the fields this migration added to `PlayerData`.
#[test]
fn the_mitigation_counters_match_the_ei_oracle() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let e = common::ei();
    let p = &f.players[f.self_idx.expect("fixture resolves a local player")];
    let ei = e
        .players
        .iter()
        .find(|x| x.account == p.account)
        .expect("the local player appears in the EI baseline");
    let d = ei.defenses.first().expect("EI defenses row");

    assert_eq!(blocked(p), d.blocked_count, "blocked");
    assert_eq!(evaded(p), d.evaded_count, "evaded");
    assert_eq!(dodges(p), d.dodge_count, "dodges");
    assert_eq!(missed(p), d.missed_count, "missed");
    assert_eq!(interrupted(p), d.interrupted_count, "interrupted");
    assert_eq!(invulned(p), d.invulned_count, "invulned");
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
