//! Determinism tests. The golden hash is the cross-platform proof: CI runs it on
//! macOS (ARM), Linux and Windows, and it must pass unchanged everywhere.

use sim::scenario::run_demo;

#[test]
fn same_seed_and_commands_give_same_hash() {
    let a = run_demo(42, 1000);
    let b = run_demo(42, 1000);
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.snapshot(), b.snapshot());
}

#[test]
fn different_seed_gives_different_hash() {
    assert_ne!(
        run_demo(1, 1000).state_hash(),
        run_demo(2, 1000).state_hash()
    );
}

/// Expected hash of the demo scenario with seed 42 after 1000 ticks.
///
/// Last changed by the decisions of prompt 045 (2026-11-16): the economy rules
/// gained the rate-of-fire factor of unit prices and the `[defeat]` rule
/// (a base match is lost with the base); both are hashed, the prices of
/// the match's unit types too, and so is the list of players who have had
/// a base. The demo has no buildings, its battle is unchanged. Before
/// (`0x164a_535e_3316_b34e`): prompt 045 (balancing): the economy rules (start
/// account 500/600/500, income of headquarters, depots and supply points,
/// the range factor of unit prices) and the resupply cost of
/// `data/logistics.toml` (25 % instead of 50 %) are part of the hash and
/// of the demo's accounts. The demo keeps 8 against 8 tanks (no faction is
/// chosen, so the new `[faction_count_pct]` of the scenario does not
/// apply). Before:
/// `0xf8f2_b8a4_aead_460d`, set by the merge of prompt 049 (wounds, buddy aid, medics,
/// bailed-out crews) with 047, 054 and 039: the care rules of
/// `data/care.toml` and every soldier's wound state enter the hash once
/// anything happened (the demo: at once); hits a man survives wound him
/// (heavily wounded men lie, do not fire and bleed out; aimed fire goes to
/// men who are up and low at lying ones), comrades drag and carry the
/// wounded, and the survivors of destroyed tanks bail out as crew squads
/// (catalog `*_inf_tank_crew`, part of the match's unit types) and
/// withdraw away from the tank that destroyed theirs. The demo battle is
/// decided at tick 1013 instead of 1017, same winner. 049 alone (before 047): `0x5f35_7247_d536_22a3`.
/// Before: `0x9ca9_6477_eff5_75ef`, the merge of prompt 047 (infantry tactics) with 054:
/// the rules of `data/tactics.toml` and every squad's tactical state
/// (suppression, pinning, contact, bounds, blind-fire memory, jams) are in
/// the hash, and the rules change the fight: suppression widens the
/// dispersion and pins squads down, squads in contact crouch and bound, jams
/// draw from the generator. The last enemy squad now holds out longer: the
/// demo battle is decided at tick 1017 instead of 412 (same winner, player
/// 0), after the 1000 ticks of the golden run (see
/// `demo_battle_is_decided`). 047 with 050/055 (before 054):
/// `0x3d9a_b414_771f_2276`. Before: `0x7681_c049_8fc9_19b3` (the merge of
/// prompt 054, below).
///
/// Last changed by the merge of prompt 054 (heavy strike) with main: the
/// new base building `base_rocket_battery` in `data/economy.toml` is part
/// of the hashed economy rules, and every unit type after it moves one
/// index up. Nothing else changes: no strike is called in the demo, and the
/// strike state is only hashed once one was (checked: with the old
/// `economy.toml` the hash stays the one before). 054 alone:
/// `0xeca6_c2de_63a8_f16d`. Before: `0x2022_32ed_b313_f95a`, set by
/// the merge of prompt 055:
///
/// The merge of prompt 055 (bridges, bridge-laying tanks,
/// camouflage, paratroopers): the hashed air rules gained the `airborne`
/// mission, the hit points of transport aircraft, the bomb carpet length
/// and the paratrooper rules; the air types of the match include the
/// transport aircraft and every player has a fifth cooldown. The crossing
/// rules and state enter the hash only once one of them acted (not in the
/// demo). The demo battle is unchanged (decided at tick 412). Prompt 055
/// alone: `0xef27_60b4_5add_ed7b`. Before: `0xb608_35fb_c1d7_09a8`, set by
/// prompt 050:
///
/// Prompt 050 (experience and ranks): the rules of
/// `data/experience.toml` and every unit's experience points and kills are
/// in the hash, and units that gain a rank shoot more accurately and
/// reload faster. The demo's tanks gain ranks from their first kill: the
/// battle runs differently and is decided at tick 412 instead of 467, same
/// winner (player 0). Before: `0x04d1_0b77_115d_59c5`, set by
/// the merge of prompt 059, see below.
///
/// The merge of prompt 059 (desert village: new entries
/// in the hashed rule tables of `data/destruction.toml`,
/// `data/infantry.toml` and `data/vision.toml`; the demo uses none of
/// them, battle unchanged) with 060. Before (`0x9b87_57bd_bec7_96f8`),
/// 059 with 048 alone `0x0773_c8ed_1eac_669d`:
/// the merge of prompt 060 (mountains): the downhill
/// sight rules (`[height]` in `data/vision.toml`) are in the hash. Nothing
/// else changes: `test_small` has no ground 20 m below any eye, so every
/// viewshed keeps its sight range (checked: without hashing the new
/// rules the hash stays the one before). Before:
/// `0x934d_db12_191d_9a20` (prompt 048, below); prompt 060 alone
/// `0x1fa2_761f_1d8f_77f6`.
///
/// Prompt 048 (knockdown): the `[knockdown]` rules of
/// `data/infantry.toml` and every soldier's knock-down state (ticks left,
/// blast point) are in the hash; blasts and tank ammunition explosions
/// knock soldiers down. The ammunition roll does not advance the shared
/// generator, so the demo battle is unchanged (decided at tick 467, player 0
/// wins). Before: `0x738f_0759_efe2_a7bd`, the merge of prompt 035 with 036.
/// The merge of prompt 035 (engineers, mines, obstacles,
/// positions) with 036 (supply and repair). Prompt 035: the rules of
/// `data/works.toml` and the (empty) field works, mines, engineer jobs,
/// thrown tracks and dug samples are in the hash, and the economy rules
/// (also hashed) list engineer squads for production; prompt 035 alone:
/// `0xfe4f_ccdf_7c02_97e0`. The demo has no engineers and no mines: its
/// battle is unchanged (decided at tick 467, player 0 wins).
/// Before: prompt 036 alone (`0x1971_0235_5cc0_0b3e`).
/// Prompt 036 (supply and repair): the rules of
/// `data/logistics.toml`, every unit's ammunition and fuel stock, building
/// repair orders and sold buildings are in the hash; units use up
/// ammunition and fuel. The demo battle is unchanged (decided at tick 467,
/// player 0 wins). Before: the merge of prompt 034 (transport and towing)
/// with 041 (fire) and 033 (air support), `0x0b1d_7230_9f70_e885`: the
/// state of all three is in the hash.
/// Prompt 034 alone (`0xe602_e2eb_a51e_1c8e`): the rules of
/// `data/transport.toml`, transport tasks, links of passengers and towed
/// guns and the hit points of loaded carriers are in the hash, and the
/// economy rules (also hashed) produce trucks, AT guns and howitzers in
/// the vehicle works instead of the AT gun position; the demo has no
/// carriers or guns and its battle was unchanged.
/// Before: the merge of 041 with 033 (`0xd8f0_f135_a14d_e016`).
/// That merge:
/// prompt 041: the fire rules of `data/fire.toml`, burning objects and
/// wrecks, burnt-out objects and fire smoke clouds are in the hash; new
/// catalog objects (town and log houses, town hall, warehouse, dumps) and
/// destruction classes for hay, firewood and dumps change the hashed object
/// and destruction rules. Destroyed tanks of the demo now burn as wrecks
/// and hits roll whether trees and buildings catch fire, which draws from
/// the match's random generator: the battle runs differently and is decided
/// at tick 467 instead of 480, same winner.
/// Before: prompt 033 (air support and anti-aircraft fire): the
/// rules of `data/air.toml`, the aircraft types of every faction (catalog
/// ids), every player's cooldowns and the (empty) sorties, aircraft, bombs
/// and flak reload times are in the hash, and the economy rules (also
/// hashed) list light and heavy flak as fortifications. The demo calls no
/// aircraft and has no anti-aircraft weapons: its battle is unchanged
/// (decided at tick 480, player 0 wins).
/// Before: prompt 032 (indirect fire and smoke): the rules of
/// `data/indirect.toml`, the indirect-fire state of units with mortars,
/// howitzers and self-propelled guns, shells in flight, smoke clouds and
/// the vision cells under smoke are in the hash, and the economy rules
/// (also hashed) list mortar teams and self-propelled guns for production.
/// The demo has no indirect-fire units: its battle is unchanged (decided
/// at tick 480, player 0 wins).
/// Before: the merge of prompt 023 (base building and resources)
/// with 021 (terrain deformation): the economy rules, unit prices,
/// accounts, structures, production queues and supply points (three
/// `supply_*` markers on `test_small`) are in the hash next to the
/// craters. The demo has no buildings and captures no point; its battle
/// is the one of prompt 021 (decided at tick 480, player 0 wins).
/// Before: prompt 021 (terrain deformation): ground hits of rounds
/// from 45 mm leave craters (`data/craters.toml`) that lower and raise the
/// map's samples, turn the bowl to mud, rebuild path and vision cells, and
/// give soldiers in them cover; the crater rules, count and a running
/// checksum of every changed sample are in the hash. The demo makes 9
/// craters; the battle is decided at tick 480 instead of 515, player 0
/// still wins. Prompt 023 alone: `0xc9ba_6118_2de5_7872`.
/// Before: the merge of prompt 022 (infantry) with 020
/// (destruction) and 024 (AI): the destruction state of 020 (its rules,
/// the two farmhouses `test_small` places as ruins acting as ruins from
/// the start, every changed object) and the new infantry building rules
/// (exposure in damaged buildings and ruins, losses when a building
/// collapses on its occupants) are in the hash; building conditions now
/// come from the destruction state. The demo battle is still decided at
/// tick 515 and player 0 wins.
/// Before: prompt 022: infantry. The demo has two rifle squads
/// per side behind the tanks; a squad is one unit with soldiers that have
/// their own positions, hit points and hit boxes (stance height), stances
/// (stand, kneel, prone: speed, hit box, dispersion), cover spots behind
/// objects with soft cover rolls, and buildings to occupy
/// (`data/infantry.toml`). Infantry rules, the squad state and per-unit
/// speed share, spread and eye elevation are in the hash. The demo battle
/// was decided at tick 515 instead of 645, and player 0 won.
/// Before: prompt 020: destruction. Map objects have hit points
/// and damage stages (`data/destruction.toml`); the rules and every object
/// that differs from its start are in the hash. The two farmhouses that
/// `test_small` places as ruins now act as ruins from the start (they block
/// tanks only and no longer block sight; the path grid version counts their
/// rebuild). The demo
/// battle destroys no object and is still decided at tick 645, same winner.
/// Before: prompt 018: vision and fog of war. Units keep viewsheds
/// on an 8 m vision grid (hills, forests, buildings and hedges block sight;
/// `data/vision.toml`), pick automatic targets and fire only in their own
/// line of sight, and follow attack targets out of sight to where they were
/// last seen; vision rules, viewshed anchors, the update queue and who sees
/// which unit are in the hash. The demo battle is now decided at tick 645
/// instead of 525, same winner.
/// Before: prompt 014: `test_small` was regenerated with a deeper
/// river profile, a stone and a timber bridge with ramps (bridge objects
/// with decks, `data/objects.toml`), a narrowed river at the bridges and
/// stones in the river; the map hash covers the bridges (objects near the
/// meadow moved with it, so the demo battle is now decided at tick 525
/// instead of 405, same winner). Units may not drive down grades steeper
/// than the climbing limit or over the side of a deck.
/// Before: prompt 027 (victory conditions: the demo battle is
/// decided at tick 405, player 0 loses its last tank, player 1 wins; from the
/// first defeat on, the standings and the result are part of the hash).
/// Before: prompt 017: combat. Units have hit points, a hull
/// direction, turrets, weapons and armor from the catalog, pick targets,
/// fire projectiles with ballistic flight and dispersion (simulation RNG),
/// penetrate armor by hit face and angle, and are destroyed (wrecks block
/// paths); combat rules (`data/combat.toml`), unit combat state and
/// projectiles are in the hash. The demo's tanks start 120 m apart and fight.
/// Before: prompt 013: `test_small` has map objects (woods,
/// hedges, a village; `objects.csv`), their shapes block path cells by size
/// class (`data/objects.toml`), and the map hash covers them.
/// Before: prompt 012 (units drive along A* paths on the 4 m path grid, one
/// path per group, formation offsets, separation, re-planning, path queue
/// with a node budget, unreachable targets snap to the nearest reachable
/// cell; routes, queue and grid version in the hash; wider fords), prompt 010 (map `test_small`, ground height, terrain- and
/// slope-dependent speed, map hash and terrain rules in the hash), prompt
/// 009 (two players with eight tanks each, group commands with grid
/// formation, `Stop`, owner/footprint in the hash).
///
/// If this test fails, the simulation is no longer deterministic across
/// platforms, or its rules changed. Only update the value for an intentional
/// rule change, and say so in the commit message.
const GOLDEN_HASH_SEED_42_TICKS_1000: u64 = 0x0b95_b29d_5ff1_d003;

#[test]
fn golden_hash() {
    let hash = run_demo(42, 1000).state_hash();
    assert_eq!(
        hash, GOLDEN_HASH_SEED_42_TICKS_1000,
        "golden hash mismatch: got {hash:#018x}"
    );
}

#[test]
fn demo_moves_units_of_both_players() {
    let start = sim::scenario::demo_config();
    let end = run_demo(42, 1000).snapshot();
    for player in start.players() {
        let moved = start
            .spawns
            .iter()
            .zip(&end.units)
            .filter(|(spawn, unit)| unit.owner == player && spawn.position != unit.position)
            .count();
        assert!(moved > 0, "no unit of player {} moved", player.0);
    }
}

#[test]
fn demo_is_a_battle() {
    let end = run_demo(42, 1000);
    let snapshot = end.snapshot();
    let destroyed = snapshot.units.iter().filter(|u| !u.alive()).count();
    assert!(destroyed >= 8, "only {destroyed} units destroyed");
    let start = sim::scenario::demo_config();
    for player in start.players() {
        // Bailed-out crews (prompt 049) do not keep a player alive.
        let alive = snapshot
            .units
            .iter()
            .filter(|u| u.owner == player && u.alive() && !end.is_bailed_crew(u.id))
            .count();
        assert_eq!(end.alive_units(player), alive);
    }
}

/// The demo battle ends by elimination (prompt 027): the result is part of
/// the golden state above. Since prompt 047 the last suppressed squad
/// holds out until about tick 1000 (1013 with the wounds of prompt 049),
/// so the run is 1100 ticks long (the golden hash above stays at 1000
/// ticks).
#[test]
fn demo_battle_is_decided() {
    let end = run_demo(42, 1100);
    let outcome = end.outcome().expect("the demo battle is decided");
    assert_eq!(outcome.winner, Some(sim::PlayerId(0)));
    assert_eq!(outcome.tick, 1013);
    assert_eq!(end.alive_units(sim::PlayerId(1)), 0);
    assert_eq!(
        end.standings()[1].defeat,
        Some((sim::DefeatReason::Eliminated, 1013))
    );
}
