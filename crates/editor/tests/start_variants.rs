//! Start variants (prompt 182): a second start layout is added in the
//! editor, saved in `map.toml`, loaded by the simulation, checked like the
//! standard layout, and a match can be set up in it.

use editor::validate::validate_files;
use editor::{EditMap, MapFiles};
use sim::Biome;

#[test]
fn variants_are_edited_saved_loaded_and_checked() {
    let mut map = EditMap::new("variants_test", 400, 400, Biome::Forest).unwrap();
    let standard: Vec<(u8, [i32; 2])> = map.meta().starts.clone();
    assert_eq!(map.add_variant("flanks", "The flanks"), Ok(1));
    assert!(map.add_variant("flanks", "").is_err(), "names are unique");
    assert!(map.add_variant("random", "").is_err(), "reserved name");
    assert!(map.add_variant("Bad Name", "").is_err());
    map.select_variant(1);
    map.set_start(0, -120_000, 0);
    map.set_start(1, 120_000, 0);
    map.set_start_team(1, Some(1));
    // The standard layout is untouched.
    assert_eq!(map.meta().starts, standard);
    assert_eq!(map.active_starts().0, &[(0, [-120, 0]), (1, [120, 0])]);
    let files = map.files();
    assert!(
        files
            .map_toml
            .contains("[[variant]]\nname = \"flanks\"\ntitle = \"The flanks\""),
        "{}",
        files.map_toml
    );
    let loaded = files.load().unwrap();
    assert_eq!(loaded.variant_names(), ["standard", "flanks"]);
    let v = &loaded.variants()[0];
    assert_eq!(v.title, "The flanks");
    assert_eq!(v.starts.len(), 2);
    assert_eq!(v.starts[1].team, Some(1));
    let report = validate_files(&files);
    assert!(report.ok(), "{report:?}");

    // Read back into the editor, the variant is still there.
    let dir = std::env::temp_dir().join(format!("variants_test_{}", std::process::id()));
    files.write_dir(&dir).unwrap();
    let again = EditMap::from_files(&MapFiles::read_dir(&dir).unwrap()).unwrap();
    assert_eq!(again.meta().variants, map.meta().variants);
    let _ = std::fs::remove_dir_all(&dir);

    // A start in water is found in the variant too.
    map.set_start(0, 199_000, 199_000);
    map.set_start(1, 199_000, 199_000);
    let report = validate_files(&map.files());
    assert!(
        report
            .warnings
            .iter()
            .chain(&report.errors)
            .any(|m| m.starts_with("start variant 'flanks':")),
        "{report:?}"
    );

    // Removing it leaves the standard layout.
    map.remove_variant();
    assert!(map.meta().variants.is_empty());
    assert_eq!(map.active_variant(), 0);
}

#[test]
fn a_match_is_set_up_in_a_variant_and_random_follows_the_seed() {
    let mut map = EditMap::new("variants_match", 400, 400, Biome::Forest).unwrap();
    map.add_variant("west_east", "").unwrap();
    map.set_start(0, -150_000, 0);
    map.set_start(1, 150_000, 0);
    let loaded = map.files().load().unwrap();
    let build = |m: &sim::Map| {
        sim::scenario::parse_scenario(
            sim::scenario::DEMO_SCENARIO,
            &sim::Catalog::builtin().unwrap(),
            &sim::Scale::builtin().unwrap(),
            &sim::TerrainRules::builtin().unwrap(),
            m,
        )
    };
    let config = sim::scenario::build_in_variant(&loaded, "west_east", 1, build).unwrap();
    assert_eq!(
        config.variant.as_ref().map(|v| v.name.as_str()),
        Some("west_east")
    );
    let p0 = config
        .spawns
        .iter()
        .find(|s| s.owner == sim::PlayerId(0))
        .unwrap();
    assert!(
        p0.position.x < sim::Fixed::from_num(-100),
        "{:?}",
        p0.position
    );
    let standard = sim::scenario::build_in_variant(&loaded, "", 1, build).unwrap();
    assert_eq!(standard.variant, None);
    assert!(sim::scenario::build_in_variant(&loaded, "nope", 1, build).is_err());
    // `random` gives both layouts over a few seeds, the same for a seed.
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..32 {
        let a = sim::scenario::build_in_variant(&loaded, "random", seed, build).unwrap();
        let b = sim::scenario::build_in_variant(&loaded, "random", seed, build).unwrap();
        assert_eq!(a, b);
        seen.insert(a.variant.map(|v| v.name));
    }
    assert_eq!(seen.len(), 2, "{seen:?}");
    // The match starts.
    assert!(sim::Sim::new(&config, std::sync::Arc::new(loaded), 1).is_ok());
}

#[test]
fn a_variant_uses_only_its_supply_points() {
    use sim::map::{MapLayout, MapVariant, Marker, StartPosition};
    let n = 201usize;
    let at = |x: i32, y: i32| sim::Vec2::from_meters(x, y);
    let start = |player, x, y| StartPosition {
        player,
        position: at(x, y),
        team: None,
    };
    let layout = MapLayout {
        name: "supply_variants".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: vec![start(0, 0, 150), start(1, 0, -150)],
        markers: ["supply_a", "supply_b"]
            .iter()
            .zip([-100, 100])
            .map(|(name, x)| Marker {
                name: (*name).into(),
                position: at(x, 0),
            })
            .collect(),
    };
    let mut map = sim::Map::from_rasters(layout, vec![0; n * n], vec![0; n * n]).unwrap();
    let variant = |supply: Option<Vec<String>>| MapVariant {
        name: "one_point".into(),
        title: String::new(),
        starts: vec![start(0, -150, 0), start(1, 150, 0)],
        supply,
    };
    let bad = map.set_variants(vec![variant(Some(vec!["supply_x".into()]))]);
    assert!(bad.is_err(), "unknown marker");
    map.set_variants(vec![variant(Some(vec!["supply_a".into()]))])
        .unwrap();
    let build = |m: &sim::Map| {
        sim::scenario::parse_scenario(
            sim::scenario::DEMO_SCENARIO,
            &sim::Catalog::builtin().unwrap(),
            &sim::Scale::builtin().unwrap(),
            &sim::TerrainRules::builtin().unwrap(),
            m,
        )
    };
    let map = std::sync::Arc::new(map);
    let points = |choice: &str| {
        let config = sim::scenario::build_in_variant(&map, choice, 1, build).unwrap();
        let sim = sim::Sim::new(&config, map.clone(), 1).unwrap();
        sim.supply_points()
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(points("standard"), ["supply_a", "supply_b"]);
    assert_eq!(points("one_point"), ["supply_a"]);
}
