//! Task 2 (native path): identity fields `FightData::from_report`
//! projects for the roster and the encounter.
//!
//! Through Task 7 the roster and identity fields here were proved
//! against a frozen Elite Insights oracle (deleted by Task 8 -- the
//! oracle has served its purpose). What remains are native-only
//! invariants: the roster is non-empty with unique, non-empty accounts;
//! every squad player resolves a profession; self/commander resolve.

mod common;
use arcdps_axipulse::fight_data::FightData;

#[test]
fn roster_is_populated_with_unique_accounts() {
    let f = FightData::from_report(&common::native());
    let mut accounts: Vec<&str> = f
        .players
        .iter()
        .filter(|p| p.in_squad)
        .map(|p| p.account.as_str())
        .collect();
    assert!(!accounts.is_empty(), "fixture has no squad players");
    assert!(accounts.iter().all(|a| !a.is_empty()), "a squad account is empty");
    let before = accounts.len();
    accounts.sort_unstable();
    accounts.dedup();
    assert_eq!(accounts.len(), before, "squad roster has a duplicate account");
}

#[test]
fn identity_fields_are_populated_for_every_squad_player() {
    let f = FightData::from_report(&common::native());
    for p in f.players.iter().filter(|p| p.in_squad) {
        // At least one of profession/elite_spec must be resolved -- an
        // entirely blank identity would mean the catalog lookup silently
        // failed rather than the field being genuinely absent.
        assert!(
            !p.profession.is_empty() || !p.elite_spec.is_empty(),
            "{} has neither profession nor elite_spec resolved",
            p.account,
        );
    }
}

#[test]
fn self_and_commander_resolve() {
    let f = FightData::from_report(&common::native());
    let si = f.self_idx.expect("fixture has a recorder");
    assert!(si < f.players.len());
    if let Some(ci) = f.commander_idx {
        assert!(f.players[ci].is_commander);
    }
}

#[test]
fn encounter_scalars_are_populated() {
    let f = FightData::from_report(&common::native());
    assert!(f.duration_ms > 0);
    assert!(!f.map_name.is_empty());
    // started_at_unix is ABSENT, not zero, on a log with no CBTS_LOGSTART.
    if let Some(t) = f.started_at_unix {
        assert!(t > 1_600_000_000);
    }
}

/// The map's roster card and player dots render an icon keyed on a class
/// name, and the native report splits that name in two: `profession` is
/// the CORE class, the spec lives in `elite_spec`. Feeding `profession`
/// straight in made every specced player render as their core class --
/// `spec_label` is the single accessor that picks correctly.
#[test]
fn spec_label_prefers_the_elite_spec_over_the_core_profession() {
    let f = FightData::from_report(&common::native());

    let specced: Vec<_> = f
        .players
        .iter()
        .filter(|p| p.in_squad && !p.elite_spec.is_empty())
        .collect();
    assert!(
        !specced.is_empty(),
        "fixture has no specced squad player -- this test would pass vacuously",
    );
    for p in &specced {
        assert_eq!(p.spec_label(), p.elite_spec, "{} fell back to core", p.account);
        assert_ne!(
            p.spec_label(),
            p.profession,
            "{} rendered as their core class",
            p.account,
        );
    }

    // The fallback is the core class, never an empty label.
    for p in f.players.iter().filter(|p| p.elite_spec.is_empty()) {
        assert_eq!(p.spec_label(), p.profession);
    }
}

/// Same bug, enemy side: the map's enemy dots and the composition
/// panel's enemy chips both read `profession` (core) while the fixture's
/// 46 enemy players all carry an `elite_spec`.
#[test]
fn enemy_spec_label_prefers_the_elite_spec_over_the_core_profession() {
    let f = FightData::from_report(&common::native());
    let specced: Vec<_> = f.enemies.iter().filter(|e| !e.elite_spec.is_empty()).collect();
    assert!(
        !specced.is_empty(),
        "fixture has no specced enemy -- this test would pass vacuously",
    );
    for e in &specced {
        assert_eq!(e.spec_label(), e.elite_spec, "{} fell back off its spec", e.name);
    }
    // Core class before the name prefix; name prefix only as a last resort.
    for e in f.enemies.iter().filter(|e| e.elite_spec.is_empty()) {
        let expected = if e.profession.is_empty() {
            e.name.split(" pl-").next().unwrap_or("")
        } else {
            e.profession.as_str()
        };
        assert_eq!(e.spec_label(), expected);
    }
}
