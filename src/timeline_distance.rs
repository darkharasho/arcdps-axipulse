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

/// One entry per second from 0 to `duration_ms` inclusive.
///
/// `None` means **this second was not measured**: it falls outside the
/// window where both the local player's and the commander's tracks were
/// recording. A commander who tags up 100s into a 300s fight leaves the
/// first 100 entries `None`.
///
/// Absence is carried rather than filled in. An earlier draft clamped
/// out-of-overlap seconds into the overlap window, which reported the
/// distance measured at the window's edge for every second before it --
/// a fabricated measurement, and one the inspector card then folded into
/// the user's "average distance". A zero would be worse still (it reads
/// as "stacked on the tag"), which is why neither is used: the honest
/// answer for an unmeasured second is that there is no answer.
///
/// The returned vec is EMPTY -- a different thing from a vec of `None`s,
/// and rendered as the lane's "no commander tagged" message -- when
/// there is no commander, no replay grid, no track on either side, or
/// the local player IS the commander (distance to self is not a
/// measurement).
pub fn distance_to_commander_per_second(
    fight: &FightData,
    self_idx: usize,
    duration_ms: u64,
) -> Vec<Option<f64>> {
    let poll_ms = fight.poll_ms;
    if poll_ms == 0 {
        return Vec::new();
    }
    let Some(me) = fight.players.get(self_idx) else { return Vec::new() };
    let Some(cmdr_idx) = fight.commander_idx else { return Vec::new() };
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
        let t = (sec as u64) * 1000;
        if t < win_start || t > win_end {
            out.push(None);
            continue;
        }
        let (Some((mx, my)), Some((cx, cy))) =
            (position_at(me, t, poll_ms), position_at(cmdr, t, poll_ms))
        else {
            out.push(None);
            continue;
        };
        let dx = f64::from(mx - cx);
        let dy = f64::from(my - cy);
        out.push(Some((dx * dx + dy * dy).sqrt()));
    }
    out
}

/// What the Timeline's Position inspector card reports.
///
/// Lives here, next to the absence it has to respect, rather than inline
/// in `ui/timeline.rs`: the rule that an unmeasured second must not
/// reach the denominator is the whole point of this type, and
/// `ui/timeline.rs` is `#![cfg(windows)]` and so cannot be tested on the
/// host at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceSummary {
    /// Mean over the MEASURED seconds only. The denominator is
    /// `measured_secs`, never `total_secs`.
    pub avg: f64,
    pub max: f64,
    /// How many of the fight's seconds actually had a distance.
    pub measured_secs: usize,
    /// How many seconds the lane spans in total.
    pub total_secs: usize,
}

impl DistanceSummary {
    /// True when some of the fight had no distance measurement at all --
    /// the case the card must mark visibly, so a user reading an average
    /// over two thirds of a fight can see that is what they are reading.
    pub fn is_partial(&self) -> bool {
        self.measured_secs < self.total_secs
    }
}

/// Summarises a per-second distance lane, ignoring unmeasured seconds.
///
/// `None` when nothing at all was measured -- including for an all-`None`
/// lane, which is distinct from an empty one only in how it got there.
pub fn summarize(samples: &[Option<f64>]) -> Option<DistanceSummary> {
    let mut sum = 0.0;
    let mut max = f64::NEG_INFINITY;
    let mut measured_secs = 0usize;
    for d in samples.iter().flatten() {
        sum += *d;
        max = max.max(*d);
        measured_secs += 1;
    }
    if measured_secs == 0 {
        return None;
    }
    Some(DistanceSummary {
        avg: sum / measured_secs as f64,
        max,
        measured_secs,
        total_secs: samples.len(),
    })
}
