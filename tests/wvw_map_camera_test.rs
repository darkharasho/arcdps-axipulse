use arcdps_axipulse::ui::map::{tile_zoom_for_scale, zoom_at_point};

#[test]
fn tile_zoom_picks_higher_levels_as_scale_grows() {
    assert_eq!(tile_zoom_for_scale(0.5), 4);
    assert_eq!(tile_zoom_for_scale(1.0), 4);
    assert_eq!(tile_zoom_for_scale(1.99), 4);
    assert_eq!(tile_zoom_for_scale(2.0), 5);
    assert_eq!(tile_zoom_for_scale(3.5), 5);
    assert_eq!(tile_zoom_for_scale(4.0), 6);
    assert_eq!(tile_zoom_for_scale(7.5), 6);
    assert_eq!(tile_zoom_for_scale(8.0), 7);
    assert_eq!(tile_zoom_for_scale(100.0), 7);
}

#[test]
fn zoom_at_point_keeps_cursor_pinned() {
    let (s, px, py) = zoom_at_point(1.0, 0.0, 0.0, 2.0, 50.0, 30.0);
    assert_eq!(s, 2.0);
    assert_eq!(px, -50.0);
    assert_eq!(py, -30.0);
}

#[test]
fn zoom_at_point_with_existing_pan() {
    let (s, px, py) = zoom_at_point(1.0, 10.0, -5.0, 2.0, 20.0, 10.0);
    assert_eq!(s, 2.0);
    assert_eq!(px, 0.0);
    assert_eq!(py, -20.0);
}

#[test]
fn zoom_at_point_zoom_out_inverse() {
    let (s, px, py) = zoom_at_point(2.0, -50.0, -30.0, 1.0, 50.0, 30.0);
    assert_eq!(s, 1.0);
    assert!((px - 0.0).abs() < 1e-4, "expected px ≈ 0, got {}", px);
    assert!((py - 0.0).abs() < 1e-4, "expected py ≈ 0, got {}", py);
}
