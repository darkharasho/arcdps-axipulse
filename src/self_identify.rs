//! Identify the local player in a parsed `EiJson`.
//!
//! Priority order:
//!   1. `recorded_account_by` exact match against `EiPlayer.account`
//!   2. first player with `has_commander_tag == true`
//!   3. first player
//!   4. None (empty roster)

use crate::ei_model::EiJson;

pub fn find_self_index(json: &EiJson) -> Option<usize> {
    if json.players.is_empty() {
        return None;
    }
    if let Some(acc) = json.recorded_account_by.as_deref() {
        if let Some(idx) = json.players.iter().position(|p| p.account == acc) {
            return Some(idx);
        }
    }
    if let Some(idx) = json.players.iter().position(|p| p.has_commander_tag) {
        return Some(idx);
    }
    Some(0)
}
