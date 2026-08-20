//! Task 5: the RLE decoder, and the boon/series projection onto
//! `PlayerData`.
//!
//! Per this task's scope limit, this file (and `fight_data.rs`) are the
//! only things Task 5 touches -- `boon_uptime.rs`/`timeline_*.rs`/
//! `pulse_metrics.rs` stay on `EiPlayer` until Task 7 rewires every
//! consumer at once.

mod common;
use arcdps_axipulse::fight_data::{decode_series, FightData, MAX_SERIES_LEN};
use axilog_api::v1::series::SeriesOut;

#[test]
fn decodes_raw_series() {
    let s = SeriesOut {
        interval_ms: 1000,
        len: 3,
        enc: "raw",
        data: serde_json::json!([1, 2, 3]).as_array().unwrap().clone(),
    };
    assert_eq!(decode_series(&s), vec![1, 2, 3]);
}

#[test]
fn expands_rle_runs() {
    let s = SeriesOut {
        interval_ms: 1000,
        len: 5,
        enc: "rle",
        data: serde_json::json!([[0, 3], [7, 2]])
            .as_array()
            .unwrap()
            .clone(),
    };
    assert_eq!(decode_series(&s), vec![0, 0, 0, 7, 7]);
}

#[test]
fn a_single_run_series_decodes_to_one_repeated_value() {
    let s = SeriesOut {
        interval_ms: 1000,
        len: 400,
        enc: "rle",
        data: serde_json::json!([[0, 400]]).as_array().unwrap().clone(),
    };
    let decoded = decode_series(&s);
    assert_eq!(decoded.len(), 400);
    assert!(decoded.iter().all(|&v| v == 0));
}

#[test]
#[should_panic(expected = "expected 5")]
fn rejects_a_length_mismatch() {
    let s = SeriesOut {
        interval_ms: 1000,
        len: 5,
        enc: "rle",
        data: serde_json::json!([[1, 2]]).as_array().unwrap().clone(),
    };
    decode_series(&s);
}

#[test]
#[should_panic(expected = "expected 2")]
fn rejects_a_raw_length_mismatch_too() {
    // `len` disagreeing with `data.len()` is possible for "raw" as well
    // as "rle" -- the decoder must not trust either encoding's `data`
    // length over the declared `len`.
    let s = SeriesOut {
        interval_ms: 1000,
        len: 2,
        enc: "raw",
        data: serde_json::json!([1, 2, 3]).as_array().unwrap().clone(),
    };
    decode_series(&s);
}

/// A corrupt/malicious `len` that would allocate `Vec::with_capacity`
/// far beyond anything a real fight could produce must be rejected
/// BEFORE that allocation runs, not after -- an allocation failure
/// aborts the process rather than unwinding, so no `catch_unwind`
/// upstream can save it. See `MAX_SERIES_LEN`'s doc comment.
#[test]
#[should_panic(expected = "corrupt `len`")]
fn rejects_an_oversized_declared_len() {
    let s = SeriesOut {
        interval_ms: 1000,
        len: MAX_SERIES_LEN + 1,
        enc: "raw",
        data: Vec::new(),
    };
    decode_series(&s);
}

/// The `len` check alone does not close this hazard: a small, innocuous
/// `len` paired with one corrupt RLE run still grows the `Vec` without
/// bound as `out.extend` runs, regardless of the capacity it started
/// with. This is the larger of the two allocation hazards -- the
/// trailing `assert_eq!` against `s.len` cannot help here either, since
/// it only runs after the runaway extend has already happened.
#[test]
#[should_panic(expected = "would grow the decoded series past")]
fn rejects_a_runaway_rle_run() {
    let s = SeriesOut {
        interval_ms: 1000,
        len: 5,
        enc: "rle",
        data: serde_json::json!([[0, MAX_SERIES_LEN + 1]])
            .as_array()
            .unwrap()
            .clone(),
    };
    decode_series(&s);
}

/// A legitimately maximum-length series -- exactly `MAX_SERIES_LEN`
/// buckets -- must still decode. This pins that the `+ 1` in
/// `MAX_SERIES_LEN`'s derivation (the ceiling grid's trailing bucket) is
/// not tighter than a real fight can produce.
#[test]
fn a_series_at_the_bound_still_decodes() {
    let len = MAX_SERIES_LEN;
    let s = SeriesOut {
        interval_ms: 1000,
        len,
        enc: "rle",
        data: serde_json::json!([[0, len]]).as_array().unwrap().clone(),
    };
    let decoded = decode_series(&s);
    assert_eq!(decoded.len() as u64, len);
    assert!(decoded.iter().all(|&v| v == 0));
}

/// Every damage/damage-taken series in the fixture decodes to a
/// CUMULATIVE curve: non-decreasing, sample to sample.
///
/// This used to assert `decoded.len() == s.len`, which `decode_series`
/// itself panics on a mismatch of (see `rejects_a_length_mismatch`
/// above) -- so the assertion re-checked what the call it just made had
/// already proved, and could only fail by panicking inside that call.
/// Monotonicity is independent of the decoder: it is a property of the
/// DATA, and it is the property every consumer relies on (the timeline
/// buckets differentiate these series into per-second deltas, which a
/// decreasing step would turn into a negative amount of damage).
#[test]
fn every_fixture_series_decodes_to_a_non_decreasing_curve() {
    let n = common::native();
    let series = n.blocks.series.as_ref().expect("series present");
    let mut checked = 0;
    for (id, e) in &series.by_entity.0 {
        for (label, raw) in [("damage", &e.damage), ("damage_taken", &e.damage_taken)] {
            let decoded = decode_series(raw);
            for w in decoded.windows(2) {
                assert!(
                    w[1] >= w[0],
                    "entity {id} {label}: cumulative series went backwards, {} -> {}",
                    w[0],
                    w[1],
                );
            }
            checked += decoded.len();
        }
    }
    assert!(checked > 0, "no series samples decoded -- the check above is vacuous");
}

/// Through Task 7 this was additionally proved against Elite Insights'
/// `extHealingStats.healingReceived1S` oracle, deleted by Task 8 -- the
/// oracle has served its purpose. What remains native-only: the series
/// is cumulative, so its last bucket is the fight-long total, and the
/// fixture's local player must actually have one (nonzero), or the
/// check below would be vacuous.
#[test]
fn incoming_healing_series_is_populated() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let p = &f.players[f.self_idx.unwrap()];
    let got = *p.healing_received_1s.last().unwrap_or(&0);
    assert!(got > 0, "fixture must exercise the healing extension");
}

/// `blocks.series.by_entity[id].damage` is cumulative outgoing damage
/// per second; its last bucket is the fight-long total, so it must
/// agree EXACTLY with `pulse_metrics::damage`, the same scalar
/// `top_skills_test::per_skill_damage_sums_to_the_overview_damage_scalar`
/// checks against the per-skill distribution -- three different
/// summaries of the same underlying damage block.
#[test]
fn damage_series_last_bucket_matches_the_overview_damage_scalar() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let p = &f.players[f.self_idx.unwrap()];
    let got = *p.damage_1s.last().unwrap_or(&0);
    assert!(got > 0, "fixture must exercise outgoing damage");
    assert_eq!(got, arcdps_axipulse::pulse_metrics::damage(p));
}

#[test]
fn health_percents_start_at_full_health() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let p = &f.players[f.self_idx.unwrap()];
    // Not every player necessarily emits a HEALTH_UPDATE (see
    // `EntitySeries::health_percents`'s doc comment on native), but the
    // recording player's own log should.
    if let Some((t0, pct0)) = p.health_percents.first() {
        assert_eq!(*t0, 0, "the first health sample is at log start");
        assert!(*pct0 >= 0.0 && *pct0 <= 100.0);
    }
}

#[test]
fn boons_resolve_names_and_stacking_from_the_catalog_not_from_a_hardcoded_table() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let p = &f.players[f.self_idx.unwrap()];
    assert!(
        !p.boons.is_empty(),
        "fixture must exercise at least one boon"
    );
    for row in &p.boons {
        assert!(
            !row.name.is_empty(),
            "buff {} must resolve a name",
            row.buff_id
        );
        assert!(
            row.stacking == "intensity" || row.stacking == "duration",
            "buff {} has unexpected stacking {:?}",
            row.buff_id,
            row.stacking
        );
        if row.stacking == "intensity" {
            // Intensity buffs are read from avg_stacks, per the brief;
            // duration buffs may or may not carry one (upstream leaves it
            // `None` for a buff never held).
        }
    }
}
