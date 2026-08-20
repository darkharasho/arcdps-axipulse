//! Task 3 (native path): the damage, defense, support and contribution
//! scalars `FightData::from_report` projects onto `PlayerData`.
//!
//! Through Task 7 these were proved against a frozen Elite Insights
//! oracle (`tests/fixtures/wvw.ei.json`, deleted by Task 8 along with
//! `ei_model.rs` -- the oracle has served its purpose). What remains
//! below are the native-only invariants that survive it: coverage must
//! never read `not_computed`, and each block must actually be populated
//! rather than a column of structural zeros. Measured on the fixture
//! (see the task-8 report): squad totals are damage=2461723,
//! taken=1376047, deaths=3, strips=219, cleanses=651, heal=838500,
//! barrier=649125, all with `healing_available == true`.

mod common;
use arcdps_axipulse::fight_data::FightData;

/// Damage, defense and support scalars are covered and actually
/// populated -- not a proof of correctness (the EI oracle did that once;
/// see the module doc), but a regression guard against the block going
/// silently empty.
#[test]
fn damage_defense_and_support_scalars_are_populated_and_covered() {
    let n = common::native();
    let f = FightData::from_report(&n);
    for block in ["damage", "defenses", "support"] {
        assert_eq!(
            n.coverage.get(block),
            Some(axilog_api::v1::envelope::CoverageState::Present),
            "block {block} not computed -- PARSE_OPTS has drifted"
        );
    }
    let squad: Vec<_> = f.players.iter().filter(|p| p.in_squad).collect();
    assert!(!squad.is_empty(), "fixture has no squad players");
    let total_damage: u64 = squad.iter().map(|p| p.damage).sum();
    let total_taken: u64 = squad.iter().map(|p| p.damage_taken).sum();
    let total_strips: u64 = squad.iter().map(|p| p.strips as u64).sum();
    let total_cleanses: u64 = squad.iter().map(|p| p.cleanses as u64).sum();
    assert!(total_damage > 0, "squad damage is uniformly zero");
    assert!(total_taken > 0, "squad damage_taken is uniformly zero");
    assert!(total_strips > 0, "squad strips are uniformly zero");
    assert!(total_cleanses > 0, "squad cleanses are uniformly zero");
}

/// `healing_out`/`barrier_out` are populated when the log carries the
/// healing addon. Scoped by `healing_available` rather than asserted
/// unconditionally -- a log without the addon must show absence, not a
/// zero standing in for an absent measurement (see
/// `healing_coverage_and_availability_agree` below).
#[test]
fn healing_and_barrier_scalars_are_populated_when_available() {
    let n = common::native();
    let f = FightData::from_report(&n);
    if !f.healing_available {
        return;
    }
    let squad: Vec<_> = f.players.iter().filter(|p| p.in_squad).collect();
    let total_heal: u64 = squad.iter().map(|p| p.healing_out).sum();
    let total_barrier: u64 = squad.iter().map(|p| p.barrier_out).sum();
    assert!(total_heal > 0, "squad healing_out is uniformly zero");
    assert!(total_barrier > 0, "squad barrier_out is uniformly zero");
}

/// Pins the healing coverage/availability behaviour the `require` fix
/// introduced. This fixture's log DOES carry the arcdps healing addon,
/// so it can only prove the `Present` -> `healing_available == true` leg;
/// the `Unsupported` -> `false` leg has no second, addon-less fixture in
/// this repo to prove against (see the fix report -- not invented here
/// rather than fabricate one).
#[test]
fn healing_coverage_and_availability_agree() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let state = n.coverage.get("healing");
    assert_ne!(
        state,
        Some(axilog_api::v1::envelope::CoverageState::NotComputed),
        "healing must not read not_computed under this fixture's PARSE_OPTS",
    );
    let expect_available = !matches!(
        state,
        Some(axilog_api::v1::envelope::CoverageState::Unsupported)
    );
    assert_eq!(
        f.healing_available, expect_available,
        "healing_available must track blocks.healing's own coverage state (got {state:?})",
    );
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
    assert!(
        any_nonzero,
        "down_contribution is uniformly zero across the whole squad"
    );
}

/// `duration_ms` is a log-controlled `u64` -- native computes it as
/// `last event time - log_start_ms` and never clamps it -- and several
/// consumers size a per-second `Vec` from it. A single corrupt trailing
/// timestamp would therefore ask for an arbitrarily large allocation,
/// and an allocation failure ABORTS the process rather than unwinding,
/// so the parse path's `catch_unwind` could not contain it. The clamp in
/// `FightData::from_report` is the one place that bound is applied.
#[test]
fn an_absurd_duration_is_clamped_to_the_plausible_maximum() {
    use arcdps_axipulse::fight_data::{clamp_duration_ms, MAX_DURATION_MS};

    // The scenario: a 10^12 ms timestamp, which would otherwise size two
    // per-second lanes at ~10^9 entries each.
    assert_eq!(clamp_duration_ms(1_000_000_000_000), MAX_DURATION_MS);
    assert_eq!(clamp_duration_ms(u64::MAX), MAX_DURATION_MS);
    // The bound caps every downstream per-second buffer at this many
    // entries -- the size `timeline_health`/`timeline_distance` reserve.
    assert_eq!((MAX_DURATION_MS / 1000) as usize + 1, 21_601);
}

/// The clamp is a corruption guard, not a policy that touches real logs:
/// anything at or under the bound passes through byte-identical, and the
/// fixture -- a real WvW fight -- is nowhere near it.
#[test]
fn a_plausible_duration_passes_through_unchanged() {
    use arcdps_axipulse::fight_data::{clamp_duration_ms, MAX_DURATION_MS};

    assert_eq!(clamp_duration_ms(0), 0);
    assert_eq!(clamp_duration_ms(137_452), 137_452);
    assert_eq!(clamp_duration_ms(MAX_DURATION_MS), MAX_DURATION_MS);

    let f = FightData::from_report(&common::native());
    assert!(
        f.duration_ms > 0 && f.duration_ms < MAX_DURATION_MS,
        "fixture duration {} is not a plausible un-clamped fight length",
        f.duration_ms,
    );
}
