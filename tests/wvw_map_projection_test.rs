//! `Arena::project` — the one transform that turns raw world inches into
//! the map-pixel space `ui/map.rs` draws tiles and landmarks in.
//!
//! This is the migration's highest-risk surface. Elite Insights handed
//! the plugin positions ALREADY projected into that pixel space; axilog
//! hands over raw world coordinates on purpose, so the projection has to
//! live here now and has to land in the same space the landmark and tile
//! tables were authored against.

mod common;

use arcdps_axipulse::fight_data::{Arena, FightData};
use arcdps_axipulse::map::tiles::map_pixel_size;
use arcdps_axipulse::map::wvw::resolve_map_from_zone;

fn unit_arena() -> Arena {
    Arena {
        image_width: 100,
        image_height: 200,
        image_url: String::new(),
        world_min_x: -50.0,
        world_min_y: -100.0,
        world_max_x: 50.0,
        world_max_y: 100.0,
    }
}

#[test]
fn projects_the_corners_of_the_world_rect_onto_the_canvas() {
    let a = unit_arena();
    // World y grows northward, canvas y grows downward: the world's
    // BOTTOM-left corner lands at the canvas's TOP-left's opposite.
    assert_eq!(a.project(-50.0, 100.0, 100.0, 200.0), (0.0, 0.0));
    assert_eq!(a.project(50.0, -100.0, 100.0, 200.0), (100.0, 200.0));
    assert_eq!(a.project(0.0, 0.0, 100.0, 200.0), (50.0, 100.0));
}

/// The y flip is the easiest half of this to get backwards, and getting
/// it backwards mirrors every player about the map's horizontal axis
/// without any value going out of range — so pin it on its own.
#[test]
fn north_is_up() {
    let a = unit_arena();
    let (_, north_y) = a.project(0.0, 90.0, 100.0, 200.0);
    let (_, south_y) = a.project(0.0, -90.0, 100.0, 200.0);
    assert!(north_y < south_y, "north {north_y} should be above south {south_y}");
}

/// A degenerate rect must not emit NaN into a draw call.
#[test]
fn a_zero_width_rect_projects_to_the_origin_not_a_nan() {
    let a = Arena { world_max_x: -50.0, ..unit_arena() };
    assert_eq!(a.project(0.0, 0.0, 100.0, 200.0), (0.0, 0.0));
}

/// **The integration this migration had to get right.** Projecting the
/// fixture's real tracks into `map_pixel_size`'s canvas must put every
/// sample inside that canvas — which is only true if axilog's arena
/// world rect and this crate's own pixel-size table describe the same
/// map region. They are independent tables in independent repositories;
/// if they ever drift, the map silently draws players off the tiles.
#[test]
fn the_fixtures_tracks_project_inside_the_maps_pixel_canvas() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let arena = f.arena.as_ref().expect("fixture's map has an arena");
    let map = resolve_map_from_zone(&f.map_name).expect("fixture is a known WvW map");
    let (mw, mh) = map_pixel_size(map);

    let mut samples = 0usize;
    for p in f.players.iter().filter(|p| p.in_squad) {
        for (wx, wy) in &p.positions {
            let (px, py) = arena.project(*wx, *wy, mw, mh);
            assert!(
                (0.0..=mw).contains(&px) && (0.0..=mh).contains(&py),
                "{} projects to ({px:.1}, {py:.1}) outside the {mw}x{mh} canvas",
                p.account,
            );
            samples += 1;
        }
    }
    assert!(samples > 1000, "only {samples} samples projected -- check is thin");
}

/// The arena's world rect and the crate's pixel-size table must agree on
/// ASPECT, or the projection stretches the map relative to its tiles.
/// Measured on this fixture: arena 697x1000 (0.6970), Green Alpine
/// pixel_size 523x750 (0.6973) — the crate's table is the arena image
/// scaled 0.75x, so the aspects agree to 5e-4.
#[test]
fn the_arena_and_the_pixel_size_table_agree_on_aspect() {
    let n = common::native();
    let f = FightData::from_report(&n);
    let arena = f.arena.as_ref().expect("fixture's map has an arena");
    let map = resolve_map_from_zone(&f.map_name).expect("fixture is a known WvW map");
    let (mw, mh) = map_pixel_size(map);

    let arena_aspect = arena.image_width as f32 / arena.image_height as f32;
    let table_aspect = mw / mh;
    assert!(
        (arena_aspect - table_aspect).abs() < 0.001,
        "arena aspect {arena_aspect:.4} vs pixel-size table aspect {table_aspect:.4}",
    );
}
