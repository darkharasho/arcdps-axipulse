//! Task 3: proves the damage, defense, support and contribution scalars
//! `FightData::from_report` projects onto `PlayerData` against the frozen
//! Elite Insights oracle (`tests/fixtures/wvw.ei.json`).

mod common;
use arcdps_axipulse::fight_data::FightData;

fn close(a: u64, b: u64) -> bool {
    if a == 0 && b == 0 {
        return true;
    }
    let hi = a.max(b) as f64;
    ((a as f64 - b as f64).abs() / hi) < 0.01
}

#[test]
fn event_counts_match_the_ei_oracle_exactly() {
    let n = common::native();
    let e = common::ei();
    let f = FightData::from_report(&n);
    for p in f.players.iter().filter(|p| p.in_squad) {
        let ep = e.players.iter().find(|x| x.account == p.account).unwrap();
        assert_eq!(p.deaths, ep.defenses[0].dead_count, "{} deaths", p.account);
        // `downs` (native `defenses.by_entity[id].downs_taken`) is a
        // pre-filed upstream divergence from EI's `defenses[0].downCount`
        // (see the task-3 brief's context note 5). Measured against this
        // fixture: native is never LOWER than EI, and the per-player gap
        // never exceeds 2 (across all 46 squad members, checked with a
        // throwaway diagnostic harness during implementation -- 11 players
        // differ, by 1 or 2 each). Bounded rather than widened blindly:
        // this still fails if a future regression makes the two counts
        // disagree by more than the measured worst case, or makes native
        // undercount EI (which never happens today).
        let native_downs = p.downs as i64;
        let ei_downs = ep.defenses[0].down_count as i64;
        assert!(
            (0..=2).contains(&(native_downs - ei_downs)),
            "{} downs diverged beyond the measured bound: native={} ei={}",
            p.account,
            p.downs,
            ep.defenses[0].down_count,
        );
        // Native carries these as `u32`, the EI oracle as `u64` -- widen
        // rather than narrow so the comparison can't silently wrap.
        assert_eq!(
            p.strips as u64, ep.support[0].boon_strips,
            "{} strips",
            p.account
        );
        assert_eq!(
            p.cleanses as u64, ep.support[0].condi_cleanse,
            "{} cleanses",
            p.account
        );
    }
}

#[test]
fn summed_quantities_match_the_ei_oracle_within_one_percent() {
    let n = common::native();
    let e = common::ei();
    let f = FightData::from_report(&n);
    for p in f.players.iter().filter(|p| p.in_squad) {
        let ep = e.players.iter().find(|x| x.account == p.account).unwrap();
        assert!(
            close(p.damage, ep.dps_all[0].damage),
            "{} damage",
            p.account
        );
        assert!(
            close(p.damage_taken, ep.defenses[0].damage_taken),
            "{} taken",
            p.account
        );
    }
}

// `down_contribution` is NOT compared against EI below -- axilog
// docs/EI-PARITY.md:50 states that the M11 contribution family
// (`downs_contribution`/`downed_by`) implements the dev-relayed arcdps
// health-anchored-window methodology ("max(last-≥99%-health - 2000ms,
// log start, prev-down + 2100ms reset)", four stats, both directions) and
// says outright: "EI has no equivalent surface -- this follows arcdps
// itself, not EI." The migration spec's Accepted Risks name down
// contribution specifically as a number that moves. An EI-relative
// tolerance here -- 1%, or a wider ratio band -- asserts almost nothing
// while encoding the false claim that the two quantities should be near
// each other. Measured against this fixture: `native.down_contribution >=
// ei_down_contribution` did NOT hold for all 46 squad members (6
// counterexamples, e.g. Anon178.7586: native=450 < ei=588), so no
// directional relationship is asserted either. Do not restore an EI
// comparison here without first re-deriving why the two should agree.
#[test]
fn down_contribution_is_populated_and_covered() {
    let n = common::native();
    let f = FightData::from_report(&n);

    // 1. `blocks.contribution`'s coverage must not read `not_computed`
    // (or any other non-`Present` state) -- that is a hard error in this
    // project, never a silent zero.
    assert_eq!(
        n.coverage.get("contribution"),
        Some(axilog_api::v1::envelope::CoverageState::Present),
        "the contribution block must be present under this fixture's PARSE_OPTS",
    );

    // 2. The field is actually populated, not a column of structural
    // zeros. The strongest true invariant measured against this fixture:
    // every squad player credited with a down or a kill (`downs_dealt >
    // 0 || kills_dealt > 0`) has a nonzero `down_contribution` (28/28, no
    // counterexamples). A broader "every player who dealt ANY damage has
    // down_contribution > 0" does NOT hold -- 2 of 45 damage-dealing
    // squad members (e.g. Anon151.6587, damage=1471) have
    // down_contribution == 0, which makes sense once `down_contribution`
    // is understood as credit for damage landed inside a down's own
    // anchored window, not overall damage dealt: a player can deal
    // damage that never lands within any enemy's down window.
    let mut any_nonzero = false;
    for p in f.players.iter().filter(|p| p.in_squad) {
        any_nonzero |= p.down_contribution > 0;
        if p.downs_dealt > 0 || p.kills_dealt > 0 {
            assert!(
                p.down_contribution > 0,
                "{} landed a down/kill but down_contribution is 0",
                p.account,
            );
        }
    }
    assert!(any_nonzero, "down_contribution is uniformly zero across the whole squad");
}
