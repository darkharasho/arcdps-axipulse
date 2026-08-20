//! Task 4: proves the per-skill damage, down-contribution, healing and
//! barrier distributions `FightData::from_report` projects onto
//! `PlayerData`.
//!
//! Through Task 7, the surfaces with an Elite Insights equivalent
//! (damage, healing, barrier) were additionally proved against a frozen
//! EI oracle (`tests/fixtures/wvw.ei.json`, deleted by Task 8 along with
//! `ei_model.rs` -- the oracle has served its purpose). What remains
//! below are native-only invariants: rows resolve names, per-skill sums
//! reconcile against their own scalar where the fixture shows they
//! should, and `down_contribution_by_skill` -- which never had an EI
//! counterpart -- keeps its own by-construction sum check.

mod common;
use arcdps_axipulse::fight_data::FightData;

/// The brief's Step 1 test, for the local player (the fixture's
/// recorder): `damage_by_skill` is non-empty, every row has a name, and
/// the summed `total` equals the scalar `damage` exactly. Through
/// Task 7 the top row by `total` was additionally proved to match the
/// top row of the EI player's `total_damage_dist[0]` by skill id --
/// dropped with the oracle it depended on.
#[test]
fn damage_by_skill_is_named_and_sums_to_the_scalar_for_the_local_player() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let si = f.self_idx.expect("fixture has a recorder");
    let p = &f.players[si];

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
    // Exact, not within a band: both sides come from the same native
    // event pass -- see `damage_by_skill_sums_to_the_scalar_across_the_
    // squad` below.
    assert_eq!(
        summed, p.damage,
        "damage_by_skill sum ({summed}) != PlayerData::damage ({})",
        p.damage,
    );
}

/// Every squad member's per-skill damage rows sum to that same player's
/// scalar `damage`, EXACTLY. `DamageEntity::by_skill` and
/// `DamageEntity::total` come out of the same event pass, so this is an
/// identity, not an approximation -- all 46 squad members in this
/// fixture match to the unit. The 1% band this used to carry was an
/// Elite-Insights-era tolerance for comparing two different parsers;
/// there is only one parser now, and a same-source identity asserted
/// with a tolerance would hide a real drift of up to 1%. Matches its
/// sibling `barrier_by_skill_sums_to_the_scalar_across_the_squad`.
#[test]
fn damage_by_skill_sums_to_the_scalar_across_the_squad() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let mut checked = 0;
    for p in f.players.iter().filter(|p| p.in_squad) {
        let summed: u64 = p.damage_by_skill.iter().map(|r| r.total).sum();
        assert_eq!(
            summed, p.damage,
            "{}: damage_by_skill sum ({summed}) != damage ({})",
            p.account, p.damage,
        );
        checked += 1;
    }
    assert!(checked > 0, "no squad members to check");
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

/// `healing_by_skill` is populated for every squad member whose row is
/// non-empty (i.e. whose client ran the healing addon and left a
/// `detail` breakdown -- `healing_by_skill`/`barrier_by_skill` are empty,
/// not wrong, for everyone else). Through Task 7 the sum was additionally
/// proved against Elite Insights' `extHealingStats.totalHealingDist[0]`
/// sum -- dropped with the oracle, and deliberately NOT replaced with a
/// same-scope native reconciliation: `healing_by_skill`'s per-row totals
/// include self-healing while the scalar `healing_out` is ally-only
/// (see `fight_data_scalars_test`'s module doc), so the two are not
/// expected to agree and measured on this fixture, do not (up to 75%
/// apart for players who mostly self-healed).
#[test]
fn healing_by_skill_rows_are_populated_when_present() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let mut checked = 0;
    for p in f
        .players
        .iter()
        .filter(|p| p.in_squad && !p.healing_by_skill.is_empty())
    {
        let native_sum: u64 = p.healing_by_skill.iter().map(|r| r.total).sum();
        assert!(native_sum > 0, "{}: healing_by_skill sums to zero despite non-empty rows", p.account);
        checked += 1;
    }
    assert!(
        checked > 0,
        "no squad members with a healing_by_skill row to check"
    );
}

/// `barrier_by_skill` has no self/allies split (unlike healing), so its
/// sum reconciles exactly with the scalar `barrier_out` -- both are read
/// off the same underlying block. Through Task 7 the sum was additionally
/// proved against Elite Insights' oracle within a measured 3% bound;
/// dropped with the oracle, replaced with the stronger same-source
/// identity, which holds exactly (0% gap) for all 16 squad members with a
/// non-empty row on this fixture.
#[test]
fn barrier_by_skill_sums_to_the_scalar_across_the_squad() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let mut checked = 0;
    for p in f
        .players
        .iter()
        .filter(|p| p.in_squad && !p.barrier_by_skill.is_empty())
    {
        let native_sum: u64 = p.barrier_by_skill.iter().map(|r| r.total).sum();
        assert_eq!(
            native_sum, p.barrier_out,
            "{}: barrier_by_skill sum ({native_sum}) != barrier_out ({})",
            p.account, p.barrier_out,
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
