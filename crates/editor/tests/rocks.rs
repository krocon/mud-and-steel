//! Prompt 074: the object brush with rocks. Trees scattered over a rock
//! ridge stand between its pieces, never inside their boxes, and rock
//! pieces keep their whole footprint off roads.

use editor::brush::Brush;
use editor::{EditMap, PlacedObject};
use sim::objects::{MapObject, Shape};
use sim::{Biome, Fixed, ObjectRules, Vec2};

const ROAD: u8 = 2;

fn m(meters: i32) -> i32 {
    meters * 1000
}

fn kinds(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

/// The box of a placed rock as a simulation object.
fn rock_box(rules: &ObjectRules, o: &PlacedObject) -> (MapObject, Shape) {
    let kind = rules.find(&o.kind, &o.variant).unwrap();
    (
        MapObject {
            kind: 0,
            position: Vec2::from_millimeters(o.x_mm, o.y_mm),
            heading_deg: o.heading_deg,
        },
        kind.shape,
    )
}

#[test]
fn trees_grow_between_the_rocks_and_rocks_stay_off_the_road() {
    let rules = ObjectRules::builtin().unwrap();
    let mut map = EditMap::new("rock_test", 300, 300, Biome::Forest).unwrap();
    // A road from north to south through the middle, 8 m wide.
    for y in (-150..=150).step_by(3) {
        map.paint(Brush::new(0, m(y), 4000), ROAD).unwrap();
    }
    let ridge = kinds(&[
        ("env_rock_massif", "Ridge"),
        ("env_rock_massif", "Long"),
        ("env_rock_massif", "Short"),
    ]);
    let mut pieces = 0;
    for y in [-60, 0, 60] {
        pieces += map
            .scatter(Brush::new(m(10), m(y), 40_000), &ridge, 60, 9000)
            .unwrap();
    }
    assert!(pieces >= 5, "only {pieces} ridge pieces");
    let trees = kinds(&[("env_tree_conifer", "Pine"), ("env_tree_conifer", "Spruce")]);
    let mut planted = 0;
    for y in [-60, 0, 60] {
        planted += map
            .scatter(Brush::new(m(10), m(y), 40_000), &trees, 300, 4000)
            .unwrap();
    }
    assert!(planted > 20, "only {planted} trees");
    let objects = map.objects().to_vec();
    let rocks: Vec<_> = objects
        .iter()
        .filter(|o| o.kind == "env_rock_massif")
        .map(|o| rock_box(&rules, o))
        .collect();
    let tiny = Fixed::from_num(1) / 1000;
    for tree in objects.iter().filter(|o| o.kind == "env_tree_conifer") {
        let at = Vec2::from_millimeters(tree.x_mm, tree.y_mm);
        assert!(
            !rocks.iter().any(|(o, s)| o.overlaps_square(*s, at, tiny)),
            "tree at ({}, {}) stands inside a rock",
            tree.x_mm,
            tree.y_mm
        );
    }
    // No ridge piece covers the road (4 m either side of x = 0).
    for (o, s) in &rocks {
        for y in (-150..=150).step_by(2) {
            let at = Vec2::from_millimeters(0, m(y));
            assert!(
                !o.overlaps_square(*s, at, Fixed::from_num(1)),
                "a ridge piece at {:?} covers the road",
                o.position
            );
        }
    }
}
