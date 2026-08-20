mod common;

use arcdps_axipulse::boon_uptime::{boon_name, collect_uptimes, BoonStacking, BoonUptime};
use arcdps_axipulse::fight_data::{BoonRow, FightData, PlayerData};

fn boon(id: u32, uptime_pct: f64, avg_stacks: Option<f64>) -> BoonRow {
    BoonRow { buff_id: id, uptime_pct, avg_stacks, ..BoonRow::default() }
}

#[test]
fn boon_name_returns_known_names() {
    assert_eq!(boon_name(740), Some("Might"));
    assert_eq!(boon_name(725), Some("Fury"));
    assert_eq!(boon_name(1187), Some("Quickness"));
    assert_eq!(boon_name(30328), Some("Alacrity"));
    assert_eq!(boon_name(717), Some("Protection"));
    assert_eq!(boon_name(1122), Some("Stability"));
    assert_eq!(boon_name(743), Some("Aegis"));
    assert_eq!(boon_name(999_999), None);
}

#[test]
fn boon_name_classifies_stacking() {
    use arcdps_axipulse::boon_uptime::boon_stacking;
    assert_eq!(boon_stacking(740), BoonStacking::Intensity);
    assert_eq!(boon_stacking(1122), BoonStacking::Intensity);
    assert_eq!(boon_stacking(725), BoonStacking::Duration);
    assert_eq!(boon_stacking(717), BoonStacking::Duration);
    assert_eq!(boon_stacking(1187), BoonStacking::Duration);
    assert_eq!(boon_stacking(30328), BoonStacking::Duration);
    assert_eq!(boon_stacking(743), BoonStacking::Duration);
}

/// An intensity boon reports AVERAGE STACKS and a duration boon reports
/// PERCENT UPTIME, from two different native fields. Mixing them up
/// would misdraw the bar (÷25 vs ÷100) rather than fail, so this pins
/// which field each reads: Might's row below carries an uptime_pct of
/// 99.0 that must NOT surface, and an avg_stacks of 18.3 that must.
#[test]
fn collect_uptimes_returns_known_boons_in_canonical_order() {
    let p = PlayerData {
        boons: vec![
            boon(725, 85.5, None),
            boon(740, 99.0, Some(18.3)),
            boon(999_999, 50.0, None),
            boon(1187, 42.1, None),
        ],
        ..PlayerData::default()
    };
    let ups = collect_uptimes(&p);
    assert_eq!(ups.len(), 3);
    assert_eq!(ups[0], BoonUptime { id: 740, name: "Might", uptime: 18.3, stacking: BoonStacking::Intensity });
    assert_eq!(ups[1], BoonUptime { id: 725, name: "Fury", uptime: 85.5, stacking: BoonStacking::Duration });
    assert_eq!(ups[2], BoonUptime { id: 1187, name: "Quickness", uptime: 42.1, stacking: BoonStacking::Duration });
}

/// An intensity boon whose native `avg_stacks` is absent reports 0.0
/// stacks. That IS the measurement (the player never held it long enough
/// to average), not a stand-in for an unknown -- the row's presence is
/// what says the buff was measured at all.
#[test]
fn an_intensity_boon_with_no_average_reports_zero_stacks() {
    let p = PlayerData { boons: vec![boon(740, 0.0, None)], ..PlayerData::default() };
    assert_eq!(collect_uptimes(&p)[0].uptime, 0.0);
}

/// A boon the player never held has no row at all and is OMITTED, not
/// reported as 0%.
#[test]
fn a_boon_with_no_row_is_omitted_rather_than_zeroed() {
    let p = PlayerData { boons: vec![boon(725, 10.0, None)], ..PlayerData::default() };
    let ups = collect_uptimes(&p);
    assert_eq!(ups.len(), 1);
    assert_eq!(ups[0].id, 725);
}

/// **Equality oracle.** Every duration boon's uptime on the real fixture
/// must match Elite Insights' `buffUptimes[].buffData[0].uptime` for the
/// same player and buff.
#[test]
fn duration_boon_uptimes_match_the_ei_oracle() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let e = common::ei();
    let p = &f.players[f.self_idx.expect("fixture resolves a local player")];
    let ei = e
        .players
        .iter()
        .find(|x| x.account == p.account)
        .expect("the local player appears in the EI baseline");

    let mut checked = 0;
    for up in collect_uptimes(p) {
        if up.stacking != BoonStacking::Duration {
            continue;
        }
        let Some(ei_row) = ei.buff_uptimes.iter().find(|b| b.id == i64::from(up.id)) else {
            continue;
        };
        let ei_uptime = ei_row.buff_data.first().map(|d| d.uptime).unwrap_or(0.0);
        assert!(
            (up.uptime - ei_uptime).abs() < 0.05,
            "{} uptime: native {} vs EI {}",
            up.name,
            up.uptime,
            ei_uptime,
        );
        checked += 1;
    }
    assert!(checked > 0, "no duration boon compared -- the check above is vacuous");
}
