//! `slim::slim_after_derive` must keep every `pulse_metrics` accessor
//! bit-identical while collapsing the heavy ext-stat arrays.

use arcdps_axipulse::ei_model::*;
use arcdps_axipulse::pulse_metrics as pm;
use arcdps_axipulse::slim::slim_after_derive;

fn heavy_player() -> EiPlayer {
    EiPlayer {
        name: "Test".into(),
        account: "Test.1234".into(),
        profession: "Guardian".into(),
        ext_healing_stats: Some(ExtHealingStats {
            outgoing_healing_allies: vec![
                vec![OutgoingHealEntry { healing: 10, hps: 2, downed_healing: 1 }],
                vec![OutgoingHealEntry { healing: 20, hps: 3, downed_healing: 4 },
                     OutgoingHealEntry { healing: 999, hps: 999, downed_healing: 999 }],
                vec![], // ally with no phase entries
                vec![OutgoingHealEntry { healing: 5, hps: 1, downed_healing: 0 }],
            ],
            total_healing_dist: vec![vec![HealDistEntry {
                id: 42, total_healing: 1000, total_downed_healing: 50, hits: 10,
            }]],
            healing_received_1s: vec![vec![3, 7, 12], vec![100, 200]],
        }),
        ext_barrier_stats: Some(ExtBarrierStats {
            outgoing_barrier_allies: vec![
                vec![OutgoingBarrierEntry { barrier: 7, bps: 1 }],
                vec![OutgoingBarrierEntry { barrier: 9, bps: 2 }],
            ],
            total_barrier_dist: vec![vec![BarrierDistEntry { id: 7, total_barrier: 500, hits: 3 }]],
            barrier_received_1s: vec![vec![11, 22, 33]],
        }),
        ..Default::default()
    }
}

fn json_with(players: Vec<EiPlayer>) -> EiJson {
    // EiJson has no Default (fight_name/duration are required); build via serde.
    let mut j: EiJson = serde_json::from_str(
        r#"{"fightName":"t","durationMS":1000,"players":[]}"#,
    ).unwrap();
    j.players = players;
    j
}

#[test]
fn pulse_metrics_identical_after_slim() {
    let mut json = json_with(vec![heavy_player()]);
    let p = &json.players[0];
    let before = (
        pm::has_healing_data(p),
        pm::healing(p), pm::hps(p), pm::healing_downed(p),
        pm::incoming_healing(p),
        pm::barrier(p), pm::incoming_barrier(p),
    );
    assert_eq!(before.1, 35, "sanity: healing sums first entry per ally");
    assert_eq!(before.4, 12, "sanity: incoming healing = last of phase 0");

    slim_after_derive(&mut json);

    let p = &json.players[0];
    let after = (
        pm::has_healing_data(p),
        pm::healing(p), pm::hps(p), pm::healing_downed(p),
        pm::incoming_healing(p),
        pm::barrier(p), pm::incoming_barrier(p),
    );
    assert_eq!(before, after);

    // The heavy arrays must actually be gone.
    let h = p.ext_healing_stats.as_ref().unwrap();
    assert!(h.total_healing_dist.is_empty());
    assert_eq!(h.outgoing_healing_allies.len(), 1);
    assert_eq!(h.healing_received_1s, vec![vec![12]]);
    let b = p.ext_barrier_stats.as_ref().unwrap();
    assert!(b.total_barrier_dist.is_empty());
    assert_eq!(b.outgoing_barrier_allies.len(), 1);
}

#[test]
fn absent_ext_stats_stay_absent() {
    let mut json = json_with(vec![EiPlayer::default()]);
    slim_after_derive(&mut json);
    let p = &json.players[0];
    assert!(p.ext_healing_stats.is_none());
    assert!(p.ext_barrier_stats.is_none());
    assert!(!pm::has_healing_data(p));
    assert_eq!(pm::healing(p), 0);
    assert_eq!(pm::incoming_barrier(p), 0);
}
