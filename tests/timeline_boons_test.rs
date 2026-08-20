use arcdps_axipulse::fight_data::{BoonRow, PlayerData};
use arcdps_axipulse::timeline_boons::{
    active_segments, defensive_boons, offensive_boons, Segment,
};

fn boon(id: u32, states: Vec<(u64, i32)>) -> BoonRow {
    BoonRow { buff_id: id, states, ..BoonRow::default() }
}

#[test]
fn active_segments_finds_runs_of_positive_values() {
    let states = [(0, 0), (500, 3), (2000, 0), (3000, 1)];
    let segs = active_segments(&states, 5000);
    assert_eq!(segs, vec![
        Segment { start_ms: 500, end_ms: 2000 },
        Segment { start_ms: 3000, end_ms: 5000 },
    ]);
}

#[test]
fn active_segments_no_states_yields_empty() {
    assert!(active_segments(&[], 5000).is_empty());
}

#[test]
fn active_segments_starting_active_at_zero() {
    let segs = active_segments(&[(0, 5), (1500, 0)], 3000);
    assert_eq!(segs, vec![Segment { start_ms: 0, end_ms: 1500 }]);
}

#[test]
fn offensive_boons_returns_might_fury_quickness_alacrity() {
    let p = PlayerData {
        boons: vec![
            boon(740, vec![(0, 5), (1000, 0)]),
            boon(725, vec![(0, 1), (1500, 0)]),
            boon(1187, vec![(500, 1), (1500, 0)]),
            boon(30328, vec![]),
            boon(999, vec![(0, 1)]),
        ],
        ..PlayerData::default()
    };
    let series = offensive_boons(&p, 2000);
    assert_eq!(series.len(), 4);
    assert_eq!(series[0].id, 740);
    assert_eq!(series[0].name, "Might");
    assert_eq!(series[0].segments, vec![Segment { start_ms: 0, end_ms: 1000 }]);
    assert_eq!(series[3].id, 30328);
    assert_eq!(series[3].name, "Alacrity");
    assert!(series[3].segments.is_empty());
}

#[test]
fn defensive_boons_returns_prot_resistance_stability_aegis() {
    let p = PlayerData {
        boons: vec![
            boon(717, vec![(0, 1), (800, 0)]),
            boon(1122, vec![(100, 2), (500, 0)]),
        ],
        ..PlayerData::default()
    };
    let series = defensive_boons(&p, 2000);
    assert_eq!(series.len(), 4);
    assert_eq!(series[0].id, 717);
    assert_eq!(series[0].name, "Protection");
    assert_eq!(series[0].segments, vec![Segment { start_ms: 0, end_ms: 800 }]);
    assert_eq!(series[2].id, 1122);
    assert_eq!(series[2].name, "Stability");
}
