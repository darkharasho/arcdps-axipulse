use arcdps_axipulse::ui::map::{health_at, status_at, MemberStatus};

#[test]
fn status_alive_with_no_ranges() {
    assert_eq!(status_at(&[], &[], 5000), MemberStatus::Alive);
}

#[test]
fn status_down_when_t_in_down_range() {
    assert_eq!(status_at(&[], &[(2000, 4000)], 3000), MemberStatus::Down);
}

#[test]
fn status_dead_overrides_down() {
    assert_eq!(status_at(&[(2000, 8000)], &[(2000, 4000)], 3000), MemberStatus::Dead);
}

#[test]
fn status_alive_outside_ranges() {
    assert_eq!(status_at(&[(2000, 4000)], &[], 5000), MemberStatus::Alive);
}

#[test]
fn status_inclusive_boundaries() {
    assert_eq!(status_at(&[(1000, 2000)], &[], 1000), MemberStatus::Dead);
    assert_eq!(status_at(&[(1000, 2000)], &[], 2000), MemberStatus::Dead);
}

/// No samples means no measured health for this entity. The party panel
/// draws an unfilled bar and an em dash off this `None`; returning
/// 100.0 would have drawn a dead player a full green bar.
#[test]
fn health_at_empty_is_absent_not_full() {
    assert_eq!(health_at(&[], 0), None);
}

#[test]
fn health_at_picks_last_sample_at_or_before_t() {
    let samples = [(0, 100.0), (1000, 80.0), (2000, 50.0)];
    assert_eq!(health_at(&samples, 500), Some(100.0));
    assert_eq!(health_at(&samples, 1500), Some(80.0));
    assert_eq!(health_at(&samples, 5000), Some(50.0));
}

#[test]
fn health_at_returns_first_when_before_first_sample() {
    assert_eq!(health_at(&[(1000, 80.0)], 0), Some(80.0));
}
