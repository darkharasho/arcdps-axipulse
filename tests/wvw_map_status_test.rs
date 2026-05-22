use arcdps_axipulse::ui::map::{status_at, health_at, MemberStatus};

fn range(start: f64, end: f64) -> Vec<f64> { vec![start, end] }

#[test]
fn status_alive_with_no_ranges() {
    let dead: Vec<Vec<f64>> = vec![];
    let down: Vec<Vec<f64>> = vec![];
    assert_eq!(status_at(&dead, &down, 5000), MemberStatus::Alive);
}

#[test]
fn status_down_when_t_in_down_range() {
    let dead: Vec<Vec<f64>> = vec![];
    let down = vec![range(2000.0, 4000.0)];
    assert_eq!(status_at(&dead, &down, 3000), MemberStatus::Down);
}

#[test]
fn status_dead_overrides_down() {
    let dead = vec![range(2000.0, 8000.0)];
    let down = vec![range(2000.0, 4000.0)];
    assert_eq!(status_at(&dead, &down, 3000), MemberStatus::Dead);
}

#[test]
fn status_alive_outside_ranges() {
    let dead = vec![range(2000.0, 4000.0)];
    let down: Vec<Vec<f64>> = vec![];
    assert_eq!(status_at(&dead, &down, 5000), MemberStatus::Alive);
}

#[test]
fn status_inclusive_boundaries() {
    let dead = vec![range(1000.0, 2000.0)];
    let down: Vec<Vec<f64>> = vec![];
    assert_eq!(status_at(&dead, &down, 1000), MemberStatus::Dead);
    assert_eq!(status_at(&dead, &down, 2000), MemberStatus::Dead);
}

#[test]
fn health_at_empty_returns_100() {
    let samples: Vec<Vec<f64>> = vec![];
    assert_eq!(health_at(&samples, 0), 100.0);
}

#[test]
fn health_at_picks_last_sample_at_or_before_t() {
    let samples = vec![vec![0.0, 100.0], vec![1000.0, 80.0], vec![2000.0, 50.0]];
    assert_eq!(health_at(&samples, 500), 100.0);
    assert_eq!(health_at(&samples, 1500), 80.0);
    assert_eq!(health_at(&samples, 5000), 50.0);
}

#[test]
fn health_at_returns_first_when_before_first_sample() {
    let samples = vec![vec![1000.0, 80.0]];
    assert_eq!(health_at(&samples, 0), 80.0);
}
