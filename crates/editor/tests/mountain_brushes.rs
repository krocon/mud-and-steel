//! Prompt 060: the mountain brushes of the map editor (ridge and valley,
//! terraces, a road with embankments and a grade limit).

use editor::EditMap;
use editor::brush::{Brush, ROAD_SIDE_PML};
use sim::Biome;

const ROAD: u8 = 2;

fn m(meters: i32) -> i32 {
    meters * 1000
}

fn r(meters: u32) -> u32 {
    meters * 1000
}

/// Height (mm) of the sample nearest to `(x, y)` meters on a map whose
/// samples lie on even meters around the origin.
fn height(map: &EditMap, x: i32, y: i32) -> i32 {
    let (i, j) = sample(map, x, y);
    map.heights()[(j * map.samples().0 + i) as usize]
}

fn sample(map: &EditMap, x: i32, y: i32) -> (u32, u32) {
    let (x0, y0) = map.sample_position(0, 0);
    (((m(x) - x0) / 2000) as u32, ((m(y) - y0) / 2000) as u32)
}

#[test]
fn ridge_and_valley_have_a_sharp_profile() {
    let mut map = EditMap::new("ridge", 200, 200, Biome::Forest).unwrap();
    for x in (-40..=40).step_by(4) {
        map.ridge(Brush::new(m(x), 0, r(30)), 2000);
    }
    let crest = height(&map, 0, 0);
    assert!(crest > 12_000, "crest {crest}");
    assert!(height(&map, 0, 10) < crest && height(&map, 0, 20) < height(&map, 0, 10));
    // One dab is a cone: linear falloff, a pointed top (the smooth raise
    // brush is flat on top).
    map.ridge(Brush::new(m(-60), m(60), r(30)), 3000);
    let cone: Vec<i32> = [0, 10, 20, 30].map(|d| height(&map, -60, 60 + d)).to_vec();
    for (h, want) in cone.iter().zip([3000, 2000, 1000, 0]) {
        assert!((h - want).abs() <= 3, "{cone:?}");
    }
    assert_eq!(height(&map, 0, 40), 0);
    // A valley is a negative ridge.
    map.ridge(Brush::new(m(60), m(60), r(20)), -5000);
    assert_eq!(height(&map, 60, 60), -5000);
    assert!(height(&map, 60, 70) > -5000 && height(&map, 60, 70) < 0);
}

#[test]
fn terraces_turn_a_slope_into_benches() {
    let mut map = EditMap::new("terrace", 200, 200, Biome::Forest).unwrap();
    // A smooth bump 30 m high.
    map.raise(Brush::new(0, 0, r(80)), 30_000);
    for _ in 0..20 {
        map.terrace(Brush::new(0, 0, r(60)), 5000, 1000);
    }
    // Close to the center every sample sits on a multiple of 5 m.
    for x in (-20..=20).step_by(2) {
        assert_eq!(height(&map, x, 3) % 5000, 0, "x = {x}");
    }
    // Neighboring benches differ by a whole step: risers, no ramps.
    let levels: Vec<i32> = (0..=20).step_by(2).map(|x| height(&map, x, 0)).collect();
    assert!(levels.windows(2).all(|w| w[0] >= w[1]));
    assert!(levels[0] > levels[10]);
}

#[test]
fn a_road_keeps_its_grade_and_gets_embankments() {
    let mut map = EditMap::new("road", 400, 400, Biome::Forest).unwrap();
    // A steep hill: 60 m over about 100 m.
    map.raise(Brush::new(0, 0, r(150)), 60_000);
    // Drive a road from the foot up the hill as the map editor does: every
    // dab follows the untouched ground one radius ahead, at most 12 %
    // above the last.
    let grade_pml = 120;
    let mut level = height(&map, -150, 0);
    let mut last = -150;
    let mut previous = level;
    for x in (-150..=0).step_by(3) {
        let ground = height(&map, x + 8, 0);
        let reach = (x - last) * grade_pml;
        level = ground.clamp(level - reach, level + reach);
        map.road(
            Brush::new(m(x), 0, r(8)),
            level,
            (m(last), 0, previous),
            ROAD,
        )
        .unwrap();
        last = x;
        previous = level;
    }
    // The road bed is painted and flat across.
    for x in (-140..=-10).step_by(10) {
        let (i, j) = sample(&map, x, 0);
        assert_eq!(map.terrain()[(j * map.samples().0 + i) as usize], ROAD);
        // (Up to one grade step: the next dab's flat core reaches one of
        // the two samples only.)
        let across = (height(&map, x, 2) - height(&map, x, 0)).abs();
        assert!(across <= 3 * grade_pml, "x = {x}: {across}");
    }
    // Along the road the grade stays within the limit (plus rounding).
    for x in (-148..0).step_by(2) {
        let rise = (height(&map, x + 2, 0) - height(&map, x, 0)).abs();
        assert!(rise <= 2 * grade_pml + 50, "x = {x}: {rise} mm per 2 m");
    }
    // Beside the bed the ground is cut or filled to the side grade: 6 m
    // from the axis, 2 m beyond the bed (half of the 8 m radius); the hill
    // there stands meters above the road.
    for x in (-140..=-10).step_by(10) {
        let bed = height(&map, x, 0);
        let side = height(&map, x, 6);
        let wall = 2 * i32::try_from(ROAD_SIDE_PML).unwrap();
        assert!(
            (side - bed).abs() <= wall + 3 * grade_pml,
            "x = {x}: {bed} {side}"
        );
    }
    // The road ends below the top: it could not climb the whole hill.
    assert!(height(&map, 0, 0) < 60_000);
    assert!(map.road(Brush::new(0, 0, r(8)), 0, (0, 0, 0), 99).is_err());
}
