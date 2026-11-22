//! Support from the second line (prompt 195): an AI attack with a
//! Sturmtiger, two Panzer IV and two rifle squads on the village of
//! `test_small`, held by passive Soviet rifle squads, an MG team and an
//! anti-tank gun, in sight from the start.
//! The table (distance to the nearest defender of the Sturmtiger and of the
//! foremost escort, every 5 s) with `--nocapture`.

use ai::Difficulty;
use ai::game::Game;
use sim::scenario::builtin_map;
use sim::{FireMode, PlayerId, SimConfig, Spawn, UnitId, Vec2};

const UNITS: [&str; 6] = [
    "de_sturmtiger",
    "de_pz4_h",
    "de_inf_rifle_squad",
    "su_inf_rifle_squad",
    "su_zis3",
    "su_inf_mg_team",
];

/// One sample: tick, the Sturmtiger's distance to the nearest defender, the
/// foremost living escort's, living escorts, the Sturmtiger's ammunition
/// (percent).
type Row = (u64, i64, Option<i64>, usize, u8);

/// `tanks`: two Panzer IV with the squads; without, the two squads alone
/// escort the Sturmtiger (and fall).
fn village_attack(difficulty: Difficulty, seed: u64, tanks: bool) -> Vec<Row> {
    let map = builtin_map();
    let village = map.marker("village").expect("test_small has a village");
    let catalog = sim::Catalog::builtin().unwrap();
    let scale = sim::Scale::builtin().unwrap();
    let unit_types = sim::unit_types_from_catalog(&catalog, &scale, &UNITS).unwrap();
    let at = |dx: i32, dy: i32| village + Vec2::from_meters(dx, dy);
    let spawn = |owner: u8, unit: &str, p: Vec2| Spawn {
        owner: PlayerId(owner),
        unit_type: unit.into(),
        position: p,
        fire_mode: FireMode::FireAtWill,
    };
    // The Sturmtiger starts in front of its escorts: the AI has to put it
    // behind them.
    let mut spawns = vec![
        spawn(0, "de_sturmtiger", at(140, 0)),
        spawn(0, "de_inf_rifle_squad", at(165, 15)),
        spawn(0, "de_inf_rifle_squad", at(175, 0)),
    ];
    if tanks {
        spawns.push(spawn(0, "de_pz4_h", at(165, -15)));
        spawns.push(spawn(0, "de_pz4_h", at(175, -30)));
    }
    let own = u32::try_from(spawns.len()).unwrap();
    spawns.extend([
        // Beside the church and a farm house at the east edge, the
        // gun deeper in the village.
        spawn(1, "su_inf_rifle_squad", at(26, -4)),
        spawn(1, "su_inf_rifle_squad", at(20, 17)),
        spawn(1, "su_inf_mg_team", at(-3, 24)),
        spawn(1, "su_zis3", at(-20, -8)),
    ]);
    let config = SimConfig {
        tick_hz: 20,
        unit_types,
        spawns,
        map: map.reference(),
        terrain: sim::TerrainRules::builtin().unwrap(),
        combat: sim::CombatRules::builtin().unwrap(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    let mut game = Game::new(&config, map, seed, &[(PlayerId(0), difficulty)]).unwrap();
    let mut rows = Vec::new();
    for _ in 0..60 {
        for _ in 0..100 {
            let decisions = game.think();
            game.step(decisions);
        }
        let snap = game.sim().snapshot();
        // Distance to the nearest living defender (the village center
        // once they are all gone).
        let defenders: Vec<Vec2> = snap
            .units
            .iter()
            .filter(|u| u.owner == PlayerId(1) && u.hp > 0)
            .map(|u| u.position)
            .collect();
        let d = |id: u32| {
            snap.units
                .iter()
                .find(|u| u.id == UnitId(id) && u.hp > 0)
                .map(|u| {
                    defenders
                        .iter()
                        .map(|&p| (u.position - p).length())
                        .min()
                        .unwrap_or_else(|| (u.position - village).length())
                        .to_num::<i64>()
                })
        };
        let Some(st) = d(0) else { break };
        let escorts: Vec<i64> = (1..own).filter_map(d).collect();
        let ammo = snap
            .units
            .iter()
            .find(|u| u.id == UnitId(0))
            .and_then(|u| u.stock.ammo_pct)
            .unwrap_or(0);
        rows.push((
            game.sim().tick(),
            st,
            escorts.iter().copied().min(),
            escorts.len(),
            ammo,
        ));
        if game.sim().outcome().is_some() {
            break;
        }
    }
    rows
}

/// With at least two escorts the Sturmtiger is never nearer the defenders
/// than the foremost of them while they are in contact (within
/// [`CONTACT_M`]); with fewer it does not drive on towards them. It fires
/// in at least one of the games.
#[test]
fn sturmtiger_stays_behind_its_escorts() {
    let mut fired = false;
    for d in [Difficulty::Normal, Difficulty::Hard] {
        for seed in 1..=2 {
            let rows = village_attack(d, seed, true);
            println!("{d} seed {seed}: tick, Sturmtiger m, foremost escort m, escorts, ammo %");
            for r in &rows {
                let lead = r.2.unwrap_or(-1);
                println!("  {:>5} {:>4} {:>4} {} {:>3}", r.0, r.1, lead, r.3, r.4);
            }
            fired |= rows.iter().any(|r| r.4 < 100);
            // Ten seconds to back off from its start in front; a sample
            // right after an escort fell may catch it before its next
            // decision (the front jumps to the next enemy).
            for w in rows.windows(2).filter(|w| w[1].0 >= 200) {
                let (tick, st, lead, n, _) = w[1];
                if n >= 2 && n == w[0].3 {
                    let lead = lead.unwrap();
                    if lead > CONTACT_M {
                        continue;
                    }
                    assert!(
                        st + 3 >= lead,
                        "{d} seed {seed} tick {tick}: {st} m vs {lead} m"
                    );
                }
            }
            for w in rows.windows(2) {
                if w[0].3 < 2 && w[1].3 < 2 && w[0].0 >= 200 {
                    assert!(
                        w[1].1 + 3 >= w[0].1,
                        "{d} seed {seed}: advanced alone {w:?}"
                    );
                }
            }
        }
    }
    assert!(fired, "the Sturmtiger never fired");
}

/// Escorts this close to a defender are in contact (m); farther the army
/// heads elsewhere and the nearest defender is no measure of "in front".
const CONTACT_M: i64 = 100;

/// Two squads alone escort the Sturmtiger into the village and fall: once
/// fewer than two are left it drives back (at least 40 m farther from the
/// defenders within 30 s) instead of on.
#[test]
fn sturmtiger_pulls_back_when_its_escorts_fall() {
    let mut pulled_back = 0;
    for d in [Difficulty::Normal, Difficulty::Hard] {
        for seed in 1..=2 {
            let rows = village_attack(d, seed, false);
            println!(
                "{d} seed {seed} (squads only): tick, Sturmtiger m, foremost escort m, escorts, ammo %"
            );
            for r in &rows {
                let lead = r.2.unwrap_or(-1);
                println!("  {:>5} {:>4} {:>4} {} {:>3}", r.0, r.1, lead, r.3, r.4);
            }
            let Some(i) = rows.iter().position(|r| r.3 < 2) else {
                continue;
            };
            let at = rows[i].1;
            let later = rows[i..].iter().take(7).map(|r| r.1).max().unwrap();
            assert!(
                later >= at + 40 || at >= 150,
                "{d} seed {seed}: {:?}",
                &rows[i..]
            );
            pulled_back += 1;
        }
    }
    assert!(pulled_back > 0, "the escorts never fell");
}
