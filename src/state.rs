//! Plugin state: the most recent parsed fight, plus a small ring of
//! history. Pulse and Timeline plans both read from here.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use crate::derived::Derived;
use crate::ei_model::EiJson;

const HISTORY_CAP: usize = 32;

#[derive(Debug, Clone)]
pub struct FightRecord {
    pub log_path: PathBuf,
    pub parsed_at: SystemTime,
    pub data: EiJson,
    /// Pre-computed per-fight derives shared across history. Computed
    /// once on the parser worker thread; the UI reads from this each
    /// frame instead of re-traversing the EI JSON.
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
