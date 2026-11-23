//! Acceptance tests of the unit catalog in `data/catalog/`.
//!
//! `Catalog::builtin` already checks required fields, unique ids, plausible
//! ranges and sources per entry; these tests add the cross-entry rules.

use sim::catalog::{Catalog, Category, Faction, Group};
use sim::{Scale, UnitType};

fn catalog() -> Catalog {
    match Catalog::builtin() {
        Ok(catalog) => catalog,
        Err(e) => panic!("{e}"),
    }
}

/// Core units of prompt 006.
const TIER_1: &[(&str, Category)] = &[
    ("de_inf_rifle_squad", Category::InfRifle),
    ("de_inf_mg42_team", Category::InfLmg),
    ("de_pz4_h", Category::TankMedium),
    ("de_sdkfz251_1", Category::Halftrack),
    ("de_pak40", Category::GunAt),
    ("su_inf_rifle_squad", Category::InfRifle),
    ("su_inf_mg_team", Category::InfLmg),
    ("su_t34_76_1943", Category::TankMedium),
    ("su_zis5", Category::Truck),
    ("su_zis3", Category::GunAt),
    ("us_inf_rifle_squad", Category::InfRifle),
    ("us_inf_mg_team", Category::InfLmg),
    ("us_m4a3_75", Category::TankMedium),
    ("us_m3_halftrack", Category::Halftrack),
    ("us_57mm_m1", Category::GunAt),
    // Prompt 117: Great Britain.
    ("uk_inf_rifle_squad", Category::InfRifle),
    ("uk_inf_mg_team", Category::InfLmg),
    ("uk_cromwell", Category::TankMedium),
    ("uk_universal_carrier", Category::Halftrack),
    ("uk_6pdr", Category::GunAt),
];

#[test]
fn builtin_catalog_is_valid() {
    let catalog = catalog();
    assert!(catalog.entries.len() > 100, "{}", catalog.entries.len());
}

#[test]
fn tier_1_entries_exist_with_sources() {
    let catalog = catalog();
    for (id, category) in TIER_1 {
        let entry = catalog
            .get(id)
            .unwrap_or_else(|| panic!("missing tier 1 entry '{id}'"));
        assert_eq!(entry.tier, 1, "{id}");
        assert_eq!(entry.category, *category, "{id}");
        assert!(!entry.source.is_empty(), "{id} has no source");
    }
    let tier_1_faction_entries = catalog
        .entries
        .iter()
        .filter(|e| e.tier == 1 && e.faction != Faction::Neutral)
        .count();
    assert_eq!(tier_1_faction_entries, TIER_1.len());
}

#[test]
fn every_faction_covers_every_military_category() {
    let catalog = catalog();
    let military: Vec<Category> = Category::ALL
        .iter()
        .copied()
        .filter(|c| c.group() != Group::Environment)
        .collect();
    for faction in [Faction::De, Faction::Su, Faction::Us, Faction::Uk] {
        for category in &military {
            // Trenches, sandbags, obstacles and wire are shared in neutral.toml.
            let shared = matches!(
                category,
                Category::FortTrench
                    | Category::FortSandbag
                    | Category::FortObstacle
                    | Category::FortWire
            );
            // Bridge-laying tanks only where one is documented (prompt 055):
            // the US Army fielded none.
            if *category == Category::Bridgelayer && faction == Faction::Us {
                continue;
            }
            // Prompt 117: the British rocket launcher of 1944, the "Land
            // Mattress", was a towed projector, not a vehicle.
            if *category == Category::RocketLauncher && faction == Faction::Uk {
                continue;
            }
            // Prompt 119: boats for de, su and us; the British boats (LCA,
            // Goatley) follow with a later prompt, and the Soviet army of the
            // catalog has an assault boat but no landing craft.
            if category.is_boat()
                && (faction == Faction::Uk
                    || (faction == Faction::Su && *category == Category::BoatLanding))
            {
                continue;
            }
            let present = catalog
                .entries
                .iter()
                .any(|e| e.faction == faction && e.category == *category);
            if shared {
                assert!(
                    catalog
                        .entries
                        .iter()
                        .any(|e| e.faction == Faction::Neutral && e.category == *category),
                    "neutral.toml has no {}",
                    category.code()
                );
            }
            assert!(
                present || shared,
                "{} has no {}",
                faction.code(),
                category.code()
            );
        }
    }
}

#[test]
fn all_mobile_entries_convert_to_unit_types() {
    let catalog = catalog();
    let scale = Scale::builtin().unwrap();
    for entry in &catalog.entries {
        // Towed guns move as fast as their crew pushes them (prompt 034).
        let mobile = matches!(
            entry.category.group(),
            Group::Infantry | Group::Vehicle | Group::Aircraft | Group::Gun
        );
        let unit_type = UnitType::from_catalog(entry, &scale);
        assert_eq!(unit_type.is_some(), mobile, "{}", entry.id);
        if let Some(t) = unit_type {
            assert!(t.speed_m_per_s > 0, "{}", entry.id);
        }
    }
}
