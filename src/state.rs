//! Plugin state: the most recent parsed fight, plus a small ring of
//! history. Pulse and Timeline plans both read from here.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use crate::derived::Derived;
use crate::fight_data::FightData;

// Each retained fight costs real memory inside the game process. A
// `FightData` is a purpose-built projection rather than a whole parsed
// document -- the `ReportV1` it was read from is dropped inside
// `parse::parse_log` and never stored -- and it turns out to be far
// smaller than the retained Elite Insights JSON it replaced (~15-25 MB
// slimmed, ~78 MB before), which is why this cap is measured up again
// rather than left at the value the Elite Insights era's footprint
// forced.
//
// Measured 2026-08-19 with `cargo run --release --example measure_mem`
// against `tests/fixtures/wvw.zevtc` (a real mid-size WvW fight: 47
// squad players, 46 enemies, 138.3s): one retained `FightRecord`
// (`FightData` + `Derived`) is exactly 1,126,034 bytes (~1.07 MB),
// computed field-by-field (`size_of` + every `Vec`/`String`/`HashMap`
// heap allocation reachable from it) rather than read off process RSS,
// since `mimalloc` (this crate's global allocator) does not reliably
// hand freed pages back to the OS between an RSS-before and an
// RSS-after reading -- an RSS delta over- or under-reports the retained
// size depending on which side of a `drop` it is taken on. Of that
// total, ~507 KB (~45%) is the part that scales with fight length and
// roster size (per-second damage/health/boon series and position
// tracks); the rest (strings, per-skill/heal rows, icon catalogs)
// scales with roster only.
//
// Worst case, assuming a squad+enemy roster up to ~1.5x this fixture's
// 93 tracked entities and a fight running up to ~900s (~6.5x this
// fixture's 138s, well past a typical WvW engagement into siege
// territory): non-scaling part 605,898 B x1.5 + scaling part 507,320 B
// x6.5x1.5 ≈ 5.86 MB, rounded up to 6 MB/fight for approximation slack
// (Vec capacity vs len, HashMap bucket overhead, catalog growth this
// estimate does not model). Against the same ~200 MB worst-case budget
// the previous cap targeted (8 slots x ~25 MB), 200 MB / 6 MB ≈ 33.3,
// which floors to 33 -- picked 32 instead for a small extra margin
// (32 x 6 MB = 192 MB) and because it is a clean, familiar number --
// the cap Elite Insights forced this constant down from in the first
// place. Against the fight actually measured, 32 slots is nowhere near
// tight: 32 x 1.07 MB ≈ 34 MB.
const HISTORY_CAP: usize = 32;

#[derive(Debug, Clone)]
pub struct FightRecord {
    pub log_path: PathBuf,
    pub parsed_at: SystemTime,
    pub data: FightData,
    /// Pre-computed per-fight derives shared across history. Computed
    /// once on the parser worker thread; the UI reads from this each
    /// frame instead of re-traversing `FightData`.
    pub derived: Arc<Derived>,
}

#[derive(Debug, Default)]
pub struct AppState {
    current: Option<FightRecord>,
    history: VecDeque<FightRecord>,
}

impl AppState {
    pub fn new() -> Self { Self::default() }

    /// Returns any records evicted from history. Dropping a full
    /// FightRecord frees a large allocation tree — callers should let
    /// the return value drop *after* releasing the state lock so the
    /// render thread never waits on the free.
    #[must_use]
    pub fn push_fight(&mut self, record: FightRecord) -> Vec<FightRecord> {
        let mut evicted = Vec::new();
        if let Some(prev) = self.current.take() {
            self.history.push_back(prev);
            while self.history.len() > HISTORY_CAP {
                evicted.extend(self.history.pop_front());
            }
        }
        self.current = Some(record);
        evicted
    }

    pub fn current(&self) -> Option<&FightRecord> { self.current.as_ref() }
    pub fn history_len(&self) -> usize { self.history.len() }
    pub fn history(&self, idx: usize) -> Option<&FightRecord> { self.history.get(idx) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal `FightRecord` distinguishable only by `log_path` --
    /// `FightData`/`Derived` are both `Default`-constructible for
    /// exactly this kind of cheap test build, so nothing here needs to
    /// go through `parse::parse_log`.
    fn fight(id: u32) -> FightRecord {
        FightRecord {
            log_path: PathBuf::from(format!("fight-{id}")),
            parsed_at: SystemTime::now(),
            data: FightData::default(),
            derived: Arc::new(Derived::default()),
        }
    }

    fn label(r: &FightRecord) -> String {
        r.log_path.to_string_lossy().into_owned()
    }

    /// Pushing more than `HISTORY_CAP` fights must stop `history_len`
    /// at the cap, and it must be the OLDEST fights that fall off --
    /// the retained set is always the most recent `HISTORY_CAP` fights
    /// that aren't `current`, oldest at index 0.
    #[test]
    fn history_caps_at_history_cap_and_evicts_oldest_first() {
        let mut state = AppState::new();
        let total = HISTORY_CAP + 5;
        for id in 0..total as u32 {
            let _evicted = state.push_fight(fight(id));
        }

        assert_eq!(state.history_len(), HISTORY_CAP);
        // The most recently pushed fight is `current`, never in history.
        assert_eq!(label(state.current().unwrap()), format!("fight-{}", total - 1));
        // Oldest retained history entry is the fight pushed right after
        // the ones old enough to have been evicted.
        assert_eq!(label(state.history(0).unwrap()), format!("fight-{}", total - 1 - HISTORY_CAP));
        // Newest history entry is the fight pushed just before current.
        assert_eq!(
            label(state.history(HISTORY_CAP - 1).unwrap()),
            format!("fight-{}", total - 2),
        );
    }
}
