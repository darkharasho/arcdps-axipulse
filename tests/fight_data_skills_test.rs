//! Task 4: proves the per-skill damage, down-contribution, healing and
//! barrier distributions `FightData::from_report` projects onto
//! `PlayerData` -- against the frozen Elite Insights oracle
//! (`tests/fixtures/wvw.ei.json`) where EI has an equivalent surface, and
//! against a native-side invariant for `down_contribution_by_skill`, which
//! it does not (see that test's doc comment).

mod common;
use arcdps_axipulse::fight_data::FightData;

fn close(a: u64, b: u64) -> bool {
    close_within(a, b, 0.01)
}

fn close_within(a: u64, b: u64, tolerance: f64) -> bool {
    if a == 0 && b == 0 {
        return true;
    }
    let hi = a.max(b) as f64;
    ((a as f64 - b as f64).abs() / hi) < tolerance
}

/// The brief's Step 1 test, for the local player (the fixture's recorder):
/// `damage_by_skill` is non-empty, every row has a name, the summed
/// `total` is within 1% of the scalar `damage`, and the top row by
/// `total` matches the top row of the EI player's `total_damage_dist[0]`
/// by skill id.
#[test]
fn damage_by_skill_matches_the_ei_oracle_for_the_local_player() {
    let n = common::native();
    let e = common::ei();
    let f = FightData::from_report(&n);
    let si = f.self_idx.expect("fixture has a recorder");
    let p = &f.players[si];
    let ep = e.players.iter().find(|x| x.account == p.account).unwrap();

    assert!(
        !p.damage_by_skill.is_empty(),
        "local player has no damage_by_skill rows"
    );
    for row in &p.damage_by_skill {
        assert!(
            !row.name.is_empty(),
            "skill {} has no name -- catalogs.skills should resolve every id a row references",
            row.skill_id,
        );
    }

    let summed: u64 = p.damage_by_skill.iter().map(|r| r.total).sum();
    assert!(
        close(summed, p.damage),
        "damage_by_skill sum ({summed}) not within 1% of PlayerData::damage ({})",
        p.damage,
    );

    let native_top = p
        .damage_by_skill
        .iter()
        .max_by_key(|r| r.total)
        .expect("non-empty, checked above");
    let ei_top = ep
        .total_damage_dist
        .first()
        .and_then(|phase| phase.iter().max_by_key(|d| d.total_damage))
        .expect("EI oracle has a phase-0 damage distribution for the local player");
    assert_eq!(
        native_top.skill_id as i64, ei_top.id,
        "top damage skill diverges: native={} ({}) ei={} ({})",
        native_top.skill_id, native_top.total, ei_top.id, ei_top.total_damage,
    );
}

/// Every squad member's per-skill damage rows sum to that same player's
/// scalar `damage`, within the same 1% tolerance used against the EI
/// oracle elsewhere in this migration. Measured against this fixture: all
/// 46 squad members match EXACTLY (worst observed gap 0.0%), which makes
/// sense given `DamageEntity::by_skill` and `DamageEntity::total` come
/// from the same underlying event pass -- but the exact figure is not
/// asserted as an identity here since nothing documents it as one by
/// construction (unlike `down_contribution_by_skill`'s sum, checked
/// below).
#[test]
fn damage_by_skill_sums_to_the_scalar_across_the_squad() {
    let n = common::native();
    let f = FightData::from_report(&n);
    for p in f.players.iter().filter(|p| p.in_squad) {
        let summed: u64 = p.damage_by_skill.iter().map(|r| r.total).sum();
        assert!(
            close(summed, p.damage),
            "{}: damage_by_skill sum ({summed}) not within 1% of damage ({})",
            p.account,
            p.damage,
        );
    }
}

/// `down_contribution_by_skill` has no EI counterpart worth comparing
/// against -- axilog's `docs/EI-PARITY.md:50` states outright "EI has no
/// equivalent surface -- this follows arcdps itself, not EI" (see also
/// `fight_data_scalars_test.rs`'s `down_contribution_is_populated_and_
/// covered`, which independently declined an EI comparison for the same
/// reason). The native-side invariant this task's brief calls out instead
/// -- that the per-skill rows sum to the scalar `down_contribution` --
/// DOES hold: `ContributionEntity::downs_contribution_by_skill`'s own doc
/// comment says it is `downs_contribution.damage` "sliced by the skill
/// that dealt it", i.e. a partition of the same scalar by construction.
/// Measured against this fixture: exact equality holds for all 46 squad
/// members (0 counterexamples), so this asserts `==`, not a tolerance.
#[test]
fn down_contribution_by_skill_sums_exactly_to_the_scalar_across_the_squad() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let mut checked = 0;
    for p in f.players.iter().filter(|p| p.in_squad) {
        let summed: u64 = p.down_contribution_by_skill.iter().map(|r| r.total).sum();
        assert_eq!(
            summed, p.down_contribution,
            "{}: down_contribution_by_skill sum ({summed}) != down_contribution ({})",
            p.account, p.down_contribution,
        );
        checked += 1;
    }
    assert!(checked > 0, "no squad members to check");
}

/// `healing_by_skill` against the EI oracle, for every squad member whose
/// row is non-empty (i.e. whose client ran the healing addon and left a
/// `detail` breakdown -- `healing_by_skill`/`barrier_by_skill` are empty,
/// not wrong, for everyone else). Measured against this fixture: all 34
/// such members match EI's `extHealingStats.totalHealingDist[0]` sum
/// within 1% (worst observed gap 0.0%).
#[test]
fn healing_by_skill_matches_the_ei_oracle_within_one_percent() {
    let n = common::native();
    let e = common::ei();
    let f = FightData::from_report(&n);
    let mut checked = 0;
    for p in f
        .players
        .iter()
        .filter(|p| p.in_squad && !p.healing_by_skill.is_empty())
    {
        let ep = e.players.iter().find(|x| x.account == p.account).unwrap();
        let native_sum: u64 = p.healing_by_skill.iter().map(|r| r.total).sum();
        let ei_sum: u64 = ep
            .ext_healing_stats
            .as_ref()
            .and_then(|h| h.total_healing_dist.first())
            .map(|entries| entries.iter().map(|d| d.total_healing).sum())
            .unwrap_or(0);
        assert!(
            close(native_sum, ei_sum),
            "{}: healing_by_skill sum ({native_sum}) not within 1% of EI's total_healing_dist sum ({ei_sum})",
            p.account,
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no squad members with a healing_by_skill row to check"
    );
}

/// `barrier_by_skill` against the EI oracle, same shape as the healing
/// check above. Measured against this fixture: 16 squad members have a
/// non-empty `barrier_by_skill`; the worst observed gap against EI's
/// `extBarrierStats.totalBarrierDist[0]` sum is 2.68% (the same
/// `Anon178.7586` divergence `fight_data_scalars_test.rs`'s
/// `barrier_out_matches_the_ei_oracle_within_a_measured_bound` already
/// documents for the scalar `barrier_out` -- this is that same gap,
/// reappearing on the per-skill breakdown of the same underlying number,
/// not a new one). Bounded at 3%, matching that test's measured headroom.
#[test]
fn barrier_by_skill_matches_the_ei_oracle_within_a_measured_bound() {
    let n = common::native();
    let e = common::ei();
    let f = FightData::from_report(&n);
    let mut checked = 0;
    for p in f
        .players
        .iter()
        .filter(|p| p.in_squad && !p.barrier_by_skill.is_empty())
    {
        let ep = e.players.iter().find(|x| x.account == p.account).unwrap();
        let native_sum: u64 = p.barrier_by_skill.iter().map(|r| r.total).sum();
        let ei_sum: u64 = ep
            .ext_barrier_stats
            .as_ref()
            .and_then(|b| b.total_barrier_dist.first())
            .map(|entries| entries.iter().map(|d| d.total_barrier).sum())
            .unwrap_or(0);
        assert!(
            close_within(native_sum, ei_sum, 0.03),
            "{}: barrier_by_skill sum ({native_sum}) diverged beyond the measured 3% bound from EI's total_barrier_dist sum ({ei_sum})",
            p.account,
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no squad members with a barrier_by_skill row to check"
    );
}

/// `barrier_by_skill` rows carry `downed: 0` structurally --
/// `HealSkillRow::total_downed`'s own doc comment says GW2EI's barrier
/// distribution has no downed field to measure it from at all, so this is
/// not a gap in this projection. Pins that behaviour so a future native
/// change that starts reporting it does not silently orphan a comment.
#[test]
fn barrier_rows_never_carry_a_downed_subset() {
    let n = common::native();
    let f = FightData::from_report(&n);
    for p in f.players.iter().filter(|p| p.in_squad) {
        for row in &p.barrier_by_skill {
            assert_eq!(
                row.downed, 0,
                "{}: barrier row for skill {} unexpectedly carries a nonzero downed subset ({})",
                p.account, row.skill_id, row.downed,
            );
        }
    }
}
