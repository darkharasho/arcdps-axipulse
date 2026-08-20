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

#[test]
fn health_at_empty_returns_100() {
    assert_eq!(health_at(&[], 0), 100.0);
}

#[test]
fn health_at_picks_last_sample_at_or_before_t() {
    let samples = [(0, 100.0), (1000, 80.0), (2000, 50.0)];
    assert_eq!(health_at(&samples, 500), 100.0);
    assert_eq!(health_at(&samples, 1500), 80.0);
    assert_eq!(health_at(&samples, 5000), 50.0);
}

#[test]
fn health_at_returns_first_when_before_first_sample() {
    assert_eq!(health_at(&[(1000, 80.0)], 0), 80.0);
}
