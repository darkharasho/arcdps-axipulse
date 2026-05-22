use arcdps_axipulse::ui::map::lerp_position;

fn pos(x: f64, y: f64) -> Vec<f64> { vec![x, y] }

#[test]
fn at_zero_returns_first_sample() {
    let samples = vec![pos(10.0, 20.0), pos(110.0, 220.0)];
    assert_eq!(lerp_position(&samples, 0, 500), Some((10.0, 20.0)));
}

#[test]
fn at_polling_rate_returns_second_sample() {
    let samples = vec![pos(10.0, 20.0), pos(110.0, 220.0)];
    assert_eq!(lerp_position(&samples, 500, 500), Some((110.0, 220.0)));
}

#[test]
fn between_samples_lerps_linearly() {
    let samples = vec![pos(0.0, 0.0), pos(100.0, 200.0)];
    assert_eq!(lerp_position(&samples, 250, 500), Some((50.0, 100.0)));
}

#[test]
fn past_last_sample_clamps_to_last() {
    let samples = vec![pos(10.0, 20.0), pos(110.0, 220.0)];
    assert_eq!(lerp_position(&samples, 5000, 500), Some((110.0, 220.0)));
}

#[test]
fn empty_samples_returns_none() {
    let samples: Vec<Vec<f64>> = vec![];
    assert_eq!(lerp_position(&samples, 0, 500), None);
}

#[test]
fn single_sample_returns_it() {
    let samples = vec![pos(7.0, 8.0)];
    assert_eq!(lerp_position(&samples, 1234, 500), Some((7.0, 8.0)));
}

#[test]
fn zero_polling_rate_returns_first_sample() {
    let samples = vec![pos(1.0, 2.0), pos(3.0, 4.0)];
    assert_eq!(lerp_position(&samples, 100, 0), Some((1.0, 2.0)));
}

#[test]
fn malformed_sample_returns_none() {
    let samples = vec![pos(1.0, 2.0), vec![3.0]];
    assert_eq!(lerp_position(&samples, 500, 500), None);
}
