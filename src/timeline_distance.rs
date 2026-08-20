//! Euclidean distance between the local player and the commander at
//! each 1-second tick, in in-game inches.
//!
//! # Why this no longer indexes two tracks in lockstep
//!
//! The Elite Insights version walked both players' `positions` arrays
//! with the SAME index, on the assumption that
//! `combatReplayMetaData.pollingRate` implied one shared sample grid
//! starting at t=0 for everyone. Position tracks do share a grid
//! INTERVAL (`FightData::poll_ms`) but not a grid ORIGIN: each track
//! starts at that player's own first-aware time rounded up to the grid
//! and ends at their own last-aware time. Measured on this crate's
//! fixture, the 93 tracks start anywhere from 0ms to 100800ms and run
//! from 105 to 462 samples, so `me.positions[i]` and
//! `cmdr.positions[i]` are generally two different instants, and
//! subtracting them produced a distance between two moments up to a
//! minute and a half apart.
//!
//! Every lookup here therefore goes through [`position_at`], which
//! converts a TIME to that track's own index via its own
//! `track_start_ms`. Nothing in this module compares two tracks by
//! index.

use crate::fight_data::{FightData, PlayerData};

/// Position of `p` at `t_ms`, holding at the track's own first/last
/// sample for times outside its window.
///
/// Holding (rather than reporting absence) matches axilog's own
/// `interp_at` clamp at the ends of a track, and matters here because
/// the caller only ever asks about times inside the overlap window it
/// computed first -- the clamp is a guard against rounding at the two
/// edges, not a licence to invent a position minutes outside a track.
pub fn position_at(p: &PlayerData, t_ms: u64, poll_ms: u64) -> Option<(f32, f32)> {
    if p.positions.is_empty() {
        return None;
    }
    if poll_ms == 0 {
        return p.positions.first().copied();
    }
    let idx = t_ms.saturating_sub(p.track_start_ms) / poll_ms;
    let idx = (idx as usize).min(p.positions.len() - 1);
    p.positions.get(idx).copied()
}

/// Inclusive `[first, last]` timestamp span a track covers, or `None`
/// when it has no samples.
fn track_span(p: &PlayerData, poll_ms: u64) -> Option<(u64, u64)> {
    if p.positions.is_empty() {
        return None;
    }
    let last = p.track_start_ms + (p.positions.len() as u64 - 1) * poll_ms;
    Some((p.track_start_ms, last))
}

/// One sample per second from 0 to `duration_ms` inclusive.
///
/// Seconds outside the window where BOTH tracks were recording are
/// clamped INTO that window rather than filled with a zero: the two
/// players were some real distance apart at the nearest instant we
/// measured, and 0 would read as "on top of the tag", which is the
/// single most misleading value this lane could show. Returns an empty
/// vec -- which the Timeline renders as "no commander tagged" rather
/// than as a flat line -- when there is no commander, no local track, or
/// no overlap at all.
pub fn distance_to_commander_per_second(
    fight: &FightData,
    self_idx: usize,
    duration_ms: u64,
) -> Vec<f64> {
    let poll_ms = fight.poll_ms;
    if poll_ms == 0 {
        return Vec::new();
    }
    let Some(me) = fight.players.get(self_idx) else { return Vec::new() };
    let Some(cmdr_idx) = fight.commander_idx else { return Vec::new() };
    // The local player IS the commander: distance to self is not a
    // meaningful lane, and rendering a flat 0 would claim a measurement.
    if cmdr_idx == self_idx {
        return Vec::new();
    }
    let Some(cmdr) = fight.players.get(cmdr_idx) else { return Vec::new() };

    let (Some((my_start, my_end)), Some((c_start, c_end))) =
        (track_span(me, poll_ms), track_span(cmdr, poll_ms))
    else {
        return Vec::new();
    };
    let win_start = my_start.max(c_start);
    let win_end = my_end.min(c_end);
    if win_start > win_end {
        return Vec::new();
    }

    let seconds = (duration_ms / 1000) as usize + 1;
    let mut out = Vec::with_capacity(seconds);
    for sec in 0..seconds {
        let t = ((sec as u64) * 1000).clamp(win_start, win_end);
        let (Some((mx, my)), Some((cx, cy))) =
            (position_at(me, t, poll_ms), position_at(cmdr, t, poll_ms))
        else {
            return Vec::new();
        };
        let dx = f64::from(mx - cx);
        let dy = f64::from(my - cy);
        out.push((dx * dx + dy * dy).sqrt());
    }
    out
}
