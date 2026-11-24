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

/// Prompt 202, group D: prickly pear, agave and vine rows (`env_bush_*`)
/// extend the hashed object and infantry cover rules. The demo uses none of
/// them: every unit ends where it did (`cli run --verbose` against the old
/// rules); before `0x8f56_95cd_a0bb_808c`.
/// Prompt 202, group C: seven buildings (brownstone, tenement, loft, quay
/// warehouse, large church, memorial arch, thatched house) extend the same
/// hashed rules; again every unit of the demo ends where it did; before
/// `0x398b_2a96_2df7_b1e0`.
/// Prompt 202, group B: the coast defences (`fort_casemate_gun`,
/// `fort_coast_gun`, `fort_battery_concrete`, `fort_fire_control`) and the
/// barracks hut as a map object extend the hashed object, destruction, fire,
/// vision and infantry rules. The demo uses none of them: every unit ends
/// where it did (`cli run --verbose` against the old rules); before
/// `0xbcff_0d3b_95a7_bebf`.
/// Prompt 202, group A: the US vehicle list (economy rules, in the hash)
/// gained the jeep `us_jeep`; before `0x84f9_dcaa_edb2_e015`.
/// Expected hash of the demo scenario with seed 42 after 1000 ticks.
///
/// Last changed by prompt 127 (castle rock map), an intended rule change:
/// the castle pieces (curtain wall and tower, gatehouse, bastion, castle
/// range, chapel) extend the hashed object, destruction, vision and
/// infantry rules. The demo uses none of them: every unit ends where it did
/// (checked with `cli run --verbose` against the old rules).
///
/// Before (`0xef5b_97be_bb7c_c0a5`), prompt 120 (Mediterranean biome), an intended rule
/// change: new map objects (stone pine, cypress, olive, macchia, the
/// Mediterranean houses, church, watch tower, town hall, field-stone and
/// terrace walls) extend the hashed object, destruction, fire, vision and
/// infantry rules. The demo uses none of them: every unit ends where it did
/// (checked with `cli run --verbose` against the old rules).
///
/// Before (`0x4106_6072_af0c_a67c`), prompt 198 (infantry advances behind the tanks of its
/// group), an intended rule change: the hashed tactics rules gained
/// `[escort]`, and squads following a tank hash it in their drill (only
/// when used). The demo's battle is unchanged: with the section off, or
/// with it on but its rules left out of the hash, the old value comes back
/// (both checked); its groups never put squads behind tanks.
///
/// Before (`0xb225_67a4_c1b5_7320`), prompt 196 (running in the assault and in bounds,
/// hitting the dirt under close fire), an intended rule change: the hashed
/// tactics rules gained `[rush]` and `[drop]`, squads hash their running
/// state and breath and soldiers the time they lie (only when used: with
/// both sections off the hash is the old value, checked). The demo's
/// squads run in their bounds and throw themselves down under blasts; its
/// battle is still decided at tick 715.
///
/// Before (`0x9e1b_4f2f_d781_54eb`), prompt 194 (big blasts throw soldiers),
/// an intended rule change: the hashed knock-down rules gained `throw_mm_per_mm`,
/// `throw_max_mm` and `ammo_throw_mm`, and every soldier hashes his last
/// throw (start point, tick, flight time). The demo has no infantry:
/// nothing else changes (checked: hashing without them gives the old
/// value); its battle is unchanged.
///
/// Before (`0xb352_1dda_3679_b5ab`), the circling recon (`recon_loiter`), an intended rule
/// change: a sixth air mission in the hashed air rules (with `loiter_s`
/// and `loiter_radius_m`), its aircraft types, a sixth cooldown per player
/// and a loiter marker per aircraft. Nothing else changes (checked: hashing
/// without them gives the old value); the demo battle is unchanged.
///
/// Before (`0x9b38_bd33_19c4_47f6`), the Sturmtiger (`de_sturmtiger`), an
/// intended rule change: the German vehicle works' list in the hashed
/// economy rules ends with it. Nothing else changes (checked: without the
/// list entry the old value comes out); the demo battle is unchanged.
///
/// Before (`0x35b9_524a_728b_ca8d`), prompt 187 (groups drive together), an intended rule
/// change: the hashed tactics rules gained `[march]`, and squads no
/// longer bound by fire teams on and near bridge decks. The demo gives
/// plain moves only (no march): its battle is unchanged (tick 715,
/// player 0).
///
/// Before (`0x1336_c665_a6ad_e5b4`), prompt 177 (gun crews), an intended rule change: the
/// hashed combat rules gained `[gun_crew]` (the demo has no gun, its
/// battle is unchanged: tick 715, player 0).
///
/// Before (`0x54a8_dffa_7753_37b2`), prompt 188 (target choice by score), an intended rule
/// change: automatic targets are chosen by a score (effect of my best hit
/// on the face it shows me, its threat to me, role, focus of other own
/// units, staying on the current target, distance) from the new hashed
/// `[targeting]` rules, and indirect weapons without an order skip the edge
/// of their range and squads of fewer than three men. The demo battle is
/// decided at tick 715 instead of 1519, still by player 0.
///
/// Before (`0xf927_8410_5bb5_50d8`), prompt 174 (formation slots along the way), an
/// intended rule change: `MoveTo`, `QueueMove` and `Patrol` assign their
/// grid slots in ranks along the way to the target and swap slots of units
/// whose straight lines to them cross (`formation::untangle`). The demo's groups take
/// other slots: the battle is decided at tick 1519 instead of 436, still
/// by player 0.
///
/// Before (`0x0be4_14e3_d70b_7823`), prompt 176 (fire discipline), an intended rule change:
/// the hashed tactics rules have the new `[discipline]` section (return
/// fire, close range, unarmed vehicles drive away when fired at), fire
/// modes hash as a code (0 at will, 1 hold, 2 return fire: unchanged for
/// the old two), limits and remembered attackers only when set. The demo
/// has no unarmed vehicle and nobody uses the new modes: its battle is
/// unchanged (decided at tick 436 by player 0).
///
/// Before (`0xfc84_38d4_649c_5305`), prompt 173 (stuck audit), an intended rule change in
/// the movement: getting closer to a waypoint by less than 0.2 m is no
/// progress (creeping units skip it after 2 s), slides shorter than a
/// quarter step do not count, units of a column queue (push only along
/// their line) at bridges and when they head for the same waypoint,
/// squads count 4 m wide near bridges, and the direct step must stay on
/// cells passable for the unit's class; every unit hashes its progress
/// (a unit without progress for 20 s stops and is reported). The demo
/// battle is decided at tick 436 instead of 603, still by player 0.
///
/// Before (`0xd68a_2529_06b0_2b43`), prompt 110 (guns and standing tanks react to fire from
/// the flank), an intended rule change: the hashed tactics rules have the
/// new `[flank]` section, and units that remember where fire came from
/// hash that alert after the drills. Standing units without a target now
/// turn towards hits; the demo battle is still decided at tick 603 by
/// player 0.
///
/// Before (`0x7aed_4dec_7cbe_20b2`), prompt 106 (heavy vehicles push trees over), an
/// intended rule change: the hashed destruction rules have per-variant
/// crush limits (`crush_variants`) and the dense-stand check
/// (`stand_radius_mm`, `stand_min_objects`) of the tree class. The demo
/// fells no tree: without these fields in the hash it is unchanged.
///
/// Before (`0xab17_f1b1_96c9_e323`), prompt 163 (faster mounting and dismounting), an
/// intended rule change: the hashed transport rules have `climb_ms` (one
/// soldier climbing aboard) instead of `embark_ms` and shorter
/// `disembark_ms`, and a mounting task hashes the soldiers already up. The
/// demo mounts nobody: its battle is unchanged (decided at tick 603).
///
/// Before (`0x51f2_adfb_a7d6_82f8`), prompt 097 (vehicle speed and weapon range after the
/// reference video), an intended rule change: ground vehicles drive their
/// real speed (`[speed] vehicle = 1/1` in `data/scale.toml` instead of the
/// time factor 2), direct fire and sight are compressed 1/8 instead of 1/4
/// with a minimum game range of 15 m, anti-aircraft fire keeps 1/4, and the
/// starts of `test_small` (and the demo's groups) moved from 120 m to 60 m
/// apart, so the tanks start inside the 87.5 m of their guns as they
/// started inside 175 m before. The hashed unit types, vision and price
/// rules change with the factors; the demo battle is decided at tick 603
/// instead of 556, same winner (player 0).
///
/// Before (`0x61a2_ce50_2027_ba7c`), prompt 093 (tanks against walls,
/// infantry and buildings), an intended rule change:
/// - Hashed rules and state: `[crush]` and the crush limits of mud and
///   dry-stone walls (`data/destruction.toml`), `[overrun]` and
///   `[vehicle_mg]` (`data/combat.toml`), every unit's slow-down after
///   breaking a wall, the machine guns' targets, shelling orders. Checked:
///   with the machine guns and the overrun switched off in the data
///   (`max_caliber_mm = 0`, `min_mass_kg = 0`) the demo battle is the one
///   from before (decided at tick 1871, player 0 wins; hash
///   `0x67b5_8f6f_dcc2_d1ea`), also with the walls not crushable
///   (`0x7ca6_3535_d3dc_8049`): `test_small` makes no tank break a wall.
/// - The tanks' secondary machine guns (two MG 34 per Panzer IV, two DT per
///   T-34) now exist in the simulation and pick their own infantry targets:
///   the rifle squads behind the tanks die much earlier and the battle is
///   decided at tick 556 instead of 1871, same winner (player 0). Their
///   bursts roll dispersion with a generator derived from the shared one
///   without advancing it, so the gun duels keep their rolls until the
///   first machine gun hit changes something.
/// - Heavy vehicles run over enemy soldiers and no longer swerve around
///   enemy squads. Alone (machine guns off) this decides the battle after
///   tick 1900 instead of 1871; with the machine guns on it does not move
///   the decision (556 with and without).
///
/// Before (`0x9516_95b0_336f_27c0`), the fix for tanks stuck at crossings
/// (after prompt 088):
/// a unit on a path cell blocked for it (pushed into the margin of an
/// obstacle or onto a bank) no longer plans from that cell but drives first
/// to the nearest passable cell it can reach in a straight line
/// (`PathGrid::lead_in`). Demo units pushed into margins take these lead-ins,
/// so the battle runs differently: still decided by elimination, player 0
/// wins, at tick 1871 instead of 1834. Checked: without the blocked-cell
/// lead-in (the deck edge lead-in, 45° steps and the no-halt-on-decks rule
/// kept) the old value comes out.
/// Before (`0x6883_ba0f_7cbe_d521`), prompt 088 (fields): the hashed terrain and weather
/// rules gained `field_grain`, `field_green` and `field_ploughed` with the
/// rules of grass. Checked: with the old `data/terrain.toml` and
/// `data/weather.toml` the old value comes out (`test_small` has no fields).
/// Before (`0xd58a_97ab_9472_eeb7`), prompt 085 (trees under heavy fire): the hashed
/// destruction rules gained `[blast]` (area damage of big explosions) and
/// per class `blast`, `damaged_clear`, `damaged_height_pct`; trees now have
/// a bare stage (`damaged_pct = 40`). Checked: with the old
/// `data/destruction.toml` and without the new hashed fields the old value
/// comes out, and the demo battle is still decided at tick 1834 by player 0.
/// Before (`0xa074_f11f_4471_6147`), prompt 078 (apartment blocks): the hashed object,
/// vision, destruction, infantry and fire rules gained the three
/// `env_block_*` entries. Checked: with the old five data files the old
/// value comes out, so the demo plays exactly as before (`test_small` has
/// no blocks).
/// Before (`0xa09f_7812_095c_68e3`), prompt 074 (rocks): the hashed vision rules list the new
/// solid objects `env_rock_outcrop` and `env_rock_massif`. Checked: with
/// the old `data/vision.toml` the old value comes out, so the demo plays
/// exactly as before (`test_small` has no new rocks).
/// Before (`0x5ffd_458e_4ba8_005a`), prompt 073 (tall grass): the hashed terrain rules gained
/// the terrain type `tall_grass` (id 8, the same rules as grass) and the
/// hashed weather rules its ground softness (the same as grass). Checked:
/// with both new entries left out of the hash, the old value comes out, so
/// the demo plays exactly as before (`test_small` has no tall grass).
/// Before (`0xbd96_60f1_5924_2388`), prompt 062 (rocket launchers): the hashed indirect-fire
/// rules gained the `rocket_launcher` category and the per-category
/// `ripple_ms` and `mrad_x10`, every gun's salvo state is hashed, and the
/// hashed economy rules produce the new units. Checked: the demo's
/// snapshot after 1000 ticks is unchanged (no rocket launchers in it).
/// Before (`0x3ad2_633d_388d_e4f8`), prompt 067 (typical combat distances): direct fire uses
/// the catalog's `range_typical_m` instead of `range_effective_m` (Panzer IV
/// 175 m instead of 375 m, T-34 175 m instead of 250 m, rifles 75 m instead
/// of 125 m), so the demo battle runs differently (decided at tick 1834
/// instead of 1013, same winner); the unit prices changed too (range
/// reference 175 m, the new velocity factor, both hashed economy rules).
/// Before (`0x0b95_b29d_5ff1_d003`), the decisions of prompt 045
/// (2026-11-16): the economy rules
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
const GOLDEN_HASH_SEED_42_TICKS_1000: u64 = 0xb2f5_c679_c302_8f78;

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
/// holds out until about tick 1000 (1013 with the wounds of prompt 049).
/// With the typical combat distances of prompt 067 the rifle squads start
/// out of each other's range (176 m apart, rifles reach 75 m) and the last
/// Soviet squad falls at tick 1834 (1871 since units on blocked path cells
/// drive to a lead-in point first, after prompt 088), so the run is 1900
/// ticks long (the golden hash above stays at 1000 ticks). Since prompt 093
/// the tanks' machine guns fire at the squads: the last Soviet squad falls
/// at tick 556. Since prompt 097 (vehicles at real speed, direct fire and
/// sight 1/8, tanks starting 60 m apart) at tick 603. Since prompt 173
/// (progress needs 0.2 m, no creeping slides, columns queue at bridges and
/// shared waypoints, direct steps stay on passable cells) at tick 436.
/// Since prompt 174 (slots along the way, no crossing ways) at tick 1519,
/// since prompt 188 (target choice by score) at tick 715.
#[test]
fn demo_battle_is_decided() {
    let end = run_demo(42, 1900);
    let outcome = end.outcome().expect("the demo battle is decided");
    assert_eq!(outcome.winner, Some(sim::PlayerId(0)));
    assert_eq!(outcome.tick, 715);
    assert_eq!(end.alive_units(sim::PlayerId(1)), 0);
    assert_eq!(
        end.standings()[1].defeat,
        Some((sim::DefeatReason::Eliminated, 715))
    );
}
