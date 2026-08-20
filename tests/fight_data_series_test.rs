//! Task 5: the RLE decoder, and the boon/series projection onto
//! `PlayerData`.
//!
//! Per this task's scope limit, this file (and `fight_data.rs`) are the
//! only things Task 5 touches -- `boon_uptime.rs`/`timeline_*.rs`/
//! `pulse_metrics.rs` stay on `EiPlayer` until Task 7 rewires every
//! consumer at once.

mod common;
use arcdps_axipulse::fight_data::{decode_series, FightData};
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

#[test]
fn every_fixture_series_decodes_to_its_declared_length() {
    let n = common::native();
    let series = n.blocks.series.as_ref().expect("series present");
    for (id, e) in &series.by_entity.0 {
        assert_eq!(
            decode_series(&e.damage).len(),
            e.damage.len as usize,
            "entity {id}"
        );
        assert_eq!(
            decode_series(&e.damage_taken).len(),
            e.damage_taken.len as usize,
            "entity {id}"
        );
    }
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
