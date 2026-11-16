//! Computer players for Mud and Steel (prompt 024).
//!
//! An [`Ai`] is a normal player (rule 5 of `CLAUDE.md`): it reads the match
//! only through the fog-of-war filtered [`PlayerView`] of its player and
//! answers with ordinary [`Command`]s, which the caller sends through the
//! lockstep session like a human's. It runs on one machine only (single
//! player: locally; LAN: on the host), so it is not part of the simulation,
//! the state hash or the replay format; replays contain its commands.
//!
//! The crate follows the simulation's rules anyway (no floats, no hash
//! iteration, no clocks, its own seeded [`Rng`]) so that tournaments give
//! the same results on every platform. See `docs/AI.md`.

mod air;
mod artillery;
mod bridgelayer;
mod commander;
mod economy;
mod engineers;
pub mod game;
mod intel;
mod scout;
mod strike;
mod supply;
mod surrender;
mod transport;
mod view;

pub use view::{OwnUnit, PlayerView, SeenUnit};

use serde::{Deserialize, Serialize};
use sim::{
    Command, Engage, FireMode, Fixed, Formation, Map, PlayerId, Rng, SimConfig, UnitId, Vec2,
};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// How well a computer player plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Difficulty {
    /// Issues no commands: units stand and fire at will. Baseline for
    /// tests and tournaments; the enemy of the matches before prompt 024.
    Passive,
    Easy,
    #[default]
    Normal,
    Hard,
    /// `Hard` without the commander of prompt 051 (the behavior of prompts
    /// 024–055), kept for tournaments that measure the commander against
    /// it. Not offered in the game's menus.
    HardOld,
}

impl Difficulty {
    /// The levels of the game's menus and of the tournament matrix.
    pub const ALL: [Self; 4] = [Self::Passive, Self::Easy, Self::Normal, Self::Hard];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Passive => "passive",
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
            Self::HardOld => "hard-old",
        }
    }

    /// The level whose rules apply (`HardOld` plays by `Hard`'s).
    pub const fn base(self) -> Self {
        match self {
            Self::HardOld => Self::Hard,
            other => other,
        }
    }

    /// Whether the commander of prompt 051 leads the army (situation
    /// picture, staged attacks, withdrawal, counter-production, surrender).
    pub const fn commander(self) -> bool {
        matches!(self, Self::Normal | Self::Hard)
    }

    const fn params(self) -> Params {
        match self.base() {
            Self::Passive => Params {
                think_ticks: 0,
                focus: Focus::None,
                regroup: false,
                retreat_hp_pct: 0,
                veteran_retreat_hp_pct: 0,
                patience_ticks: 0,
                attack_move: false,
            },
            Self::Easy => Params {
                think_ticks: 80,
                focus: Focus::Nearest,
                regroup: false,
                retreat_hp_pct: 0,
                veteran_retreat_hp_pct: 0,
                patience_ticks: 1600,
                attack_move: false,
            },
            Self::Normal => Params {
                think_ticks: 20,
                focus: Focus::Nearest,
                regroup: true,
                retreat_hp_pct: 0,
                veteran_retreat_hp_pct: 40,
                patience_ticks: 800,
                attack_move: true,
            },
            Self::Hard | Self::HardOld => Params {
                think_ticks: 10,
                focus: Focus::Weakest,
                regroup: true,
                retreat_hp_pct: 30,
                veteran_retreat_hp_pct: 50,
                patience_ticks: 800,
                attack_move: true,
            },
        }
    }
}

impl fmt::Display for Difficulty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Difficulty {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .chain([Self::HardOld])
            .find(|d| d.name().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| {
                format!("unknown AI difficulty '{s}' (passive, easy, normal, hard, hard-old)")
            })
    }
}

/// How a group picks a common target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Focus {
    /// No attack orders (passive).
    None,
    /// Everybody attacks the visible enemy nearest to the group.
    Nearest,
    /// Everybody attacks the weakest enemy in range (fewest hit points),
    /// else the nearest.
    Weakest,
}

/// Behavior switches of a difficulty (table in `docs/AI.md`).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Params {
    /// The AI acts every this many ticks (reaction time); 0 = never.
    think_ticks: u64,
    focus: Focus,
    /// Gather stragglers before advancing.
    regroup: bool,
    /// Units below this share of their hit points pull back behind the
    /// group while enemies are near (0 = never).
    retreat_hp_pct: u32,
    /// Veterans and better (prompt 050) pull back already below this share
    /// of their hit points (0 = like everybody else): experienced crews
    /// and squads are worth more than their hit points.
    veteran_retreat_hp_pct: u32,
    /// Ticks without damage to the enemy after which a fighting group
    /// moves round to the enemy's flank (wrecks and comrades in the line of
    /// fire, a crest in between).
    patience_ticks: u64,
    /// Advances as attack-move in an automatic formation, facing the
    /// enemy (prompt 046); otherwise plain `MoveTo`.
    attack_move: bool,
}

/// Role of a unit; every role is commanded as its own group. Infantry
/// (prompt 022) and builders (prompt 023) get their own rules here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum Role {
    /// Armed vehicles.
    Armor,
    /// Infantry squads.
    Infantry,
    /// Mortars and self-propelled guns (prompt 032, Hard only): barrages
    /// on enemy groups and smoke before the attack.
    Artillery,
    /// Units without weapons: stay at home.
    Unarmed,
    /// Buildings and fortifications (prompt 023): never commanded as a
    /// group; enemy ones are attacked last.
    Structure,
    /// Trucks and halftracks (prompt 034): carry infantry, tow guns.
    Transport,
    /// Towed guns (prompt 034): fetched by a transport, set up behind the
    /// army, fire at will.
    Gun,
    /// Engineer squads (prompt 035): lay mines at chokepoints, clear the
    /// enemy's.
    Engineer,
    /// Reconnaissance vehicles under the commander (prompt 051): scout.
    Scout,
}

/// What the AI knows about a unit type from the (public) config.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct TypeInfo {
    role: Role,
    /// Longest weapon range (game meters).
    range: Fixed,
    /// Sight range (game meters).
    sight: Fixed,
    /// Has a weapon of at least 20 mm (or a shaped charge): can hurt
    /// armor and buildings (prompt 023: AT squads).
    anti_tank: bool,
    /// Indirect weapon (prompt 032): minimum and maximum range, smoke.
    indirect: Option<(Fixed, Fixed, bool)>,
    /// Soldiers it carries (prompt 034, 0: none), soldiers of a squad.
    seats: u32,
    squad: u16,
    /// Can tow guns; engine power (hp) and mass (kg) for the towing rule.
    tows: bool,
    power_hp: u32,
    mass_kg: u32,
}

/// Where an enemy unit was last seen, and what it looked like then.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Sighting {
    position: Vec2,
    tick: u64,
    owner: PlayerId,
    unit_type: u32,
    hp: u32,
    max_hp: u32,
    /// Hull direction.
    facing: Vec2,
}

/// Base or start of one enemy player (prompt 037).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct EnemyHome {
    player: PlayerId,
    position: Vec2,
    searched: bool,
}

/// Share of the unit limit from which a base player stops waiting for
/// waves (percent).
const ALL_IN_PCT: usize = 85;

/// The last order a group got.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
enum Order {
    #[default]
    None,
    Move {
        target: Vec2,
        units: Vec<UnitId>,
    },
    Regroup {
        target: Vec2,
    },
    /// Targets with their attackers, by target.
    Attack(Vec<(UnitId, Vec<UnitId>)>),
}

/// Where a group is heading when no enemy is in sight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Objective {
    /// The last known position of this enemy unit.
    Sighting(UnitId),
    /// The start (or base) of the nearest undefeated enemy.
    EnemyHome,
    /// A supply point an enemy holds (end game, prompt 037).
    Supply(Vec2),
    /// A random point (everything known was searched).
    Explore(Vec2),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct GroupState {
    order: Order,
    /// Tick of the last command for `order`.
    issued: u64,
    explore: Option<Vec2>,
    /// Hit points of the visible enemies at the last think, and the tick in
    /// which they last went down (or new enemies appeared).
    enemy_hp: u64,
    progress_tick: u64,
    /// Tick of a running flank move.
    maneuver: Option<u64>,
    /// Side of the next flank move.
    flank_left: bool,
}

/// Distance at which a group counts as arrived (m).
const ARRIVED_M: i32 = 40;
/// A move order is repeated only if its target moved more than this (m).
const RETARGET_M: i32 = 40;
/// Rank index from which a unit counts as a veteran (prompt 050:
/// `data/experience.toml` ranks recruit, experienced, veteran, elite).
const VETERAN_RANK: u8 = 2;

/// Share of its hit points (percent) below which a unit of `rank` pulls
/// back: veterans earlier than recruits (prompt 050).
fn retreat_pct(base_pct: u32, veteran_pct: u32, rank: u8) -> u32 {
    if rank >= VETERAN_RANK {
        base_pct.max(veteran_pct)
    } else {
        base_pct
    }
}

/// Damaged units pull back this far behind the group (m).
const RETREAT_M: i32 = 80;
/// Orders are repeated after this many ticks: units drop attack orders on
/// targets they lose sight of, and moves can end early.
const REISSUE_TICKS: u64 = 100;
/// Groups engage enemies within this share of their sight range (percent).
const ENGAGE_SIGHT_PCT: i64 = 100;

/// A computer player.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ai {
    player: PlayerId,
    /// The level whose rules apply ([`Difficulty::base`]).
    difficulty: Difficulty,
    /// The commander of prompt 051 leads (Normal, Hard; not `HardOld`).
    commander: bool,
    /// Played as [`Difficulty::HardOld`].
    legacy: bool,
    params: Params,
    types: Vec<TypeInfo>,
    /// Center of the own start (m).
    home: Vec2,
    /// Start of the enemy the AI heads for: from the map, else mirrored
    /// through the center (prompt 037: the nearest undefeated enemy's).
    enemy_home: Vec2,
    enemy_home_searched: bool,
    /// Base or start of every enemy player, and whether it was searched
    /// (prompt 037: teams have several).
    enemy_homes: Vec<EnemyHome>,
    /// Living units besides buildings from which a base player attacks
    /// with everything instead of in waves (end game, prompt 037).
    all_in_units: usize,
    /// Half the map size (m).
    half: Vec2,
    rng: Rng,
    /// Last sightings of enemy units that are not visible now.
    memory: BTreeMap<UnitId, Sighting>,
    groups: BTreeMap<Role, GroupState>,
    /// Damaged units sent back (they keep firing at will).
    retreated: Vec<UnitId>,
    started: bool,
    /// Tick of the last view seen: a second call for the same tick (several
    /// frames for one tick after a raised input delay) does nothing.
    last_tick: Option<u64>,
    /// Ticks in which `think` made decisions, for statistics.
    thinks: u64,
    /// Unit type names of the match (prompt 023).
    names: Vec<String>,
    /// Price of every unit type, if it can be bought.
    costs: Vec<Option<sim::Resources>>,
    eco: economy::EcoState,
    /// Barrages and smoke of the artillery (prompt 032).
    fire: artillery::FireState,
    /// Air support calls and flak (prompt 033).
    air: air::AirState,
    /// Boarding and towing jobs (prompt 034).
    transport: transport::TransportState,
    /// Mines and clearing (prompt 035).
    #[serde(default)]
    engineers: engineers::EngineerState,
    /// Towing rule: engine hp per tonne of tractor and gun.
    tow_min_hp_per_tonne: u32,
    /// Supply trips and building repairs (prompt 036).
    supply: supply::SupplyState,
    /// Low ammunition and fuel in percent (`data/logistics.toml`).
    low_stock: (u8, u8),
    /// The river crossing and the bridge-laying tank (prompt 055).
    #[serde(default)]
    layers: bridgelayer::LayerPlan,
    /// Per unit type: a bridge-laying tank.
    #[serde(default)]
    is_layer: Vec<bool>,
    /// Heavy strikes: own calls and evading enemy ones (prompt 054).
    #[serde(default)]
    strike: strike::StrikeState,
    /// Situation picture: losses, known enemies (prompt 051).
    #[serde(default)]
    intel: intel::Intel,
    /// The army's plan against the current front (prompt 051).
    #[serde(default)]
    army: commander::ArmyPlan,
    /// Passable ground, coarse (prompt 051).
    #[serde(default)]
    ground: intel::Ground,
    /// Giving up when hopeless (prompt 051).
    #[serde(default)]
    surrender: surrender::SurrenderState,
    /// When the player last saw every part of the map (prompt 051).
    #[serde(default)]
    knowledge: scout::Knowledge,
}

impl Ai {
    /// The complete state of this computer player, for saved games
    /// (prompt 038, `net::HostedState`).
    pub fn save_state(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("serializing into a Vec cannot fail")
    }

    /// A computer player from [`save_state`](Self::save_state).
    pub fn load_state(bytes: &[u8]) -> Result<Self, String> {
        postcard::from_bytes(bytes).map_err(|e| format!("malformed computer player state: {e}"))
    }

    /// A computer player for `player` of the match `config` on `map` (both
    /// public knowledge). `seed` drives its random choices.
    pub fn new(
        player: PlayerId,
        difficulty: Difficulty,
        config: &SimConfig,
        map: &Map,
        seed: u64,
    ) -> Self {
        // Minimum ranges of indirect weapons (`data/indirect.toml`, public
        // like the config).
        let indirect_rules = sim::IndirectRules::builtin().ok();
        // Seats and towing (`data/transport.toml`, prompt 034).
        let transport_rules = sim::TransportRules::builtin().ok();
        // Sight ranges under the match's weather (prompt 040).
        let vision = config.effective_vision();
        // Engineers (`data/works.toml`, prompt 035).
        let works_rules = sim::WorksRules::builtin().ok();
        let types = config
            .unit_types
            .iter()
            .map(|t| {
                let range = t
                    .combat
                    .weapons
                    .iter()
                    .map(|w| w.range_m)
                    .max()
                    .unwrap_or(Fixed::ZERO);
                let indirect = t.combat.indirect.as_ref().map(|w| {
                    let pct = indirect_rules
                        .as_ref()
                        .map_or(10, |r| r.category(t.combat.category).min_range_pct);
                    (w.range_m * Fixed::from_num(pct) / 100, w.range_m, w.smoke)
                });
                let category = t.combat.category;
                let carrier = transport_rules
                    .as_ref()
                    .and_then(|r| r.carrier(category).copied());
                let seats = carrier.map_or(0, |c| {
                    if c.riders > 0 {
                        c.riders
                    } else {
                        u32::from(t.transport.passengers)
                    }
                });
                let gun = transport_rules
                    .as_ref()
                    .is_some_and(|r| r.gun(category).is_some());
                let tows = carrier.is_some_and(|c| c.tows) && t.transport.power_hp > 0;
                let truck = carrier.is_some_and(|c| c.riders == 0) && (seats > 0 || tows);
                let role = if !t.mobile() {
                    Role::Structure
                } else if gun {
                    Role::Gun
                } else if truck {
                    Role::Transport
                } else if indirect.is_some() && difficulty.base() == Difficulty::Hard {
                    Role::Artillery
                } else if t.combat.weapons.is_empty() && indirect.is_none() {
                    Role::Unarmed
                } else if works_rules
                    .as_ref()
                    .is_some_and(|r| r.is_engineer(category))
                {
                    Role::Engineer
                } else if t.combat.is_infantry() {
                    Role::Infantry
                } else if difficulty.commander()
                    && category == Some(sim::catalog::Category::ReconVehicle)
                {
                    Role::Scout
                } else {
                    Role::Armor
                };
                TypeInfo {
                    role,
                    range,
                    sight: vision.range_m(t.combat.category),
                    anti_tank: t.combat.weapons.iter().any(|w| w.caliber_mm_x10 >= 200),
                    indirect,
                    seats,
                    squad: t.combat.squad_size,
                    tows,
                    power_hp: t.transport.power_hp,
                    mass_kg: t.mass_kg,
                }
            })
            .collect();
        let own: Vec<Vec2> = config
            .spawns
            .iter()
            .filter(|s| s.owner == player)
            .map(|s| s.position)
            .collect();
        let home = centroid(own.iter().copied()).unwrap_or_default();
        // Every enemy's base (prompt 023: its first building), else its
        // start from the map, else the centroid of its spawns (public like
        // the map). Allies (prompt 037) are no enemies.
        let teams = &config.teams;
        let home_of = |enemy: PlayerId| {
            config
                .spawns
                .iter()
                .find(|s| {
                    s.owner == enemy
                        && config
                            .unit_types
                            .iter()
                            .any(|t| t.name == s.unit_type && !t.mobile())
                })
                .map(|s| s.position)
                .or_else(|| {
                    map.starts()
                        .iter()
                        .find(|s| s.player == enemy.0)
                        .map(|s| s.position)
                })
                .or_else(|| {
                    centroid(
                        config
                            .spawns
                            .iter()
                            .filter(|s| s.owner == enemy)
                            .map(|s| s.position),
                    )
                })
        };
        let enemy_homes: Vec<EnemyHome> = config
            .players()
            .into_iter()
            .filter(|&p| teams.hostile(player, p))
            .filter_map(|p| {
                Some(EnemyHome {
                    player: p,
                    position: home_of(p)?,
                    searched: false,
                })
            })
            .collect();
        // Mirror through the center as a last resort.
        let enemy_home = nearest_home(&enemy_homes, home, |_| true)
            .map_or(Vec2::new(-home.x, -home.y), |h| h.position);
        let all_in_units = sim::EconomyRules::builtin().map_or(usize::MAX, |r| {
            r.production.max_units as usize * ALL_IN_PCT / 100
        });
        let size = map.size();
        Self {
            player,
            difficulty: difficulty.base(),
            commander: difficulty.commander(),
            legacy: difficulty == Difficulty::HardOld,
            params: difficulty.params(),
            types,
            home,
            enemy_home,
            enemy_home_searched: false,
            enemy_homes,
            all_in_units,
            half: Vec2::new(size.x / 2, size.y / 2),
            rng: Rng::new(seed ^ (u64::from(player.0) << 56) ^ 0xA1_0024),
            memory: BTreeMap::new(),
            groups: BTreeMap::new(),
            retreated: Vec::new(),
            started: false,
            last_tick: None,
            thinks: 0,
            names: config.unit_types.iter().map(|t| t.name.clone()).collect(),
            costs: unit_costs(config),
            eco: economy::EcoState::default(),
            fire: artillery::FireState::default(),
            air: air::AirState::default(),
            transport: transport::TransportState::default(),
            engineers: engineers::EngineerState::new(map, home, enemy_home),
            layers: bridgelayer::LayerPlan::new(map, home, enemy_home),
            is_layer: config
                .unit_types
                .iter()
                .map(|t| {
                    sim::economy::builtin_catalog()
                        .and_then(|c| c.get(&t.name))
                        .is_some_and(|e| e.category == sim::catalog::Category::Bridgelayer)
                })
                .collect(),
            tow_min_hp_per_tonne: transport_rules
                .as_ref()
                .map_or(8, |r| r.towing.min_hp_per_tonne),
            supply: supply::SupplyState::default(),
            low_stock: sim::LogisticsRules::builtin().map_or((25, 20), |r| {
                (
                    u8::try_from(r.ammo.low_pct).unwrap_or(25),
                    u8::try_from(r.fuel.low_pct).unwrap_or(20),
                )
            }),
            strike: strike::StrikeState::default(),
            intel: intel::Intel::default(),
            army: commander::ArmyPlan::default(),
            surrender: surrender::SurrenderState::default(),
            knowledge: scout::Knowledge::default(),
            ground: if difficulty.commander() {
                intel::Ground::new(map)
            } else {
                intel::Ground::default()
            },
        }
    }

    pub const fn player(&self) -> PlayerId {
        self.player
    }

    pub const fn difficulty(&self) -> Difficulty {
        if self.legacy {
            Difficulty::HardOld
        } else {
            self.difficulty
        }
    }

    /// One line about the commander's plan, for tuning (`cli
    /// ai-tournament --trace`).
    pub fn status(&self) -> String {
        format!(
            "p{} {}: {:?} {:?} groups {:?} memory {} known {} losses {}",
            self.player.0,
            self.difficulty(),
            self.surrender.reason,
            self.army,
            self.groups
                .iter()
                .map(|(r, g)| format!("{r:?} {:?} {}", g.order, g.issued))
                .collect::<Vec<_>>(),
            self.memory.len(),
            self.intel.known.len(),
            self.intel.losses.len(),
        )
    }

    /// Number of ticks in which the AI made decisions.
    pub const fn decisions(&self) -> u64 {
        self.thinks
    }

    /// Call once per simulated tick with the view after it; returns the
    /// commands to send for this player (often none).
    pub fn think(&mut self, view: &PlayerView) -> Vec<Command> {
        let interval = self.params.think_ticks;
        if interval == 0
            || view.decided
            || view.player != self.player
            || self.last_tick == Some(view.tick)
        {
            return Vec::new();
        }
        self.last_tick = Some(view.tick);
        // Spread the work of several AIs over different ticks.
        if !(view.tick + u64::from(self.player.0) * 3).is_multiple_of(interval) {
            return Vec::new();
        }
        self.thinks += 1;
        let mut out = Vec::new();
        self.remember(view);
        if !self.legacy {
            self.update_intel(view);
            if self.commander {
                self.update_knowledge(view);
            }
            self.think_surrender(view, &mut out);
            if self.surrender.done {
                return out;
            }
        }
        self.choose_enemy_home(view);

        if !self.started {
            self.started = true;
            let hold: Vec<UnitId> = view
                .own
                .iter()
                .filter(|u| u.alive() && u.fire_mode == FireMode::HoldFire)
                .map(|u| u.id)
                .collect();
            if !hold.is_empty() {
                out.push(Command::SetFireMode {
                    units: hold,
                    mode: FireMode::FireAtWill,
                });
            }
        }

        self.think_economy(view, &mut out);
        self.think_air(view, &mut out);
        self.think_strike(view, &mut out);
        self.command_transport(view, &mut out);
        self.command_supply(view, &mut out);
        self.command_engineers(view, &mut out);
        self.command_layers(view, &mut out);
        let mut roles: Vec<Role> = Vec::new();
        for u in view.own.iter().filter(|u| u.alive()) {
            let role = self.type_info(u.unit_type).role;
            if role != Role::Structure && !roles.contains(&role) {
                roles.push(role);
            }
        }
        roles.sort_unstable();
        for role in roles {
            // Passengers and squads on their way to a carrier are left to
            // the transport (prompt 034).
            let units: Vec<OwnUnit> = view
                .own
                .iter()
                .filter(|u| u.alive() && self.type_info(u.unit_type).role == role)
                .filter(|u| u.carrier.is_none() && !self.transport.boarding(u.id, view.tick))
                .filter(|u| !self.supply.busy(u.id))
                .filter(|u| !self.strike.busy(u.id, view.tick))
                .copied()
                .collect();
            if units.is_empty() {
                continue;
            }
            match role {
                Role::Armor | Role::Infantry => self.command_group(role, &units, view, &mut out),
                Role::Artillery => self.command_artillery(&units, view, &mut out),
                Role::Scout => self.command_scouts(&units, view, &mut out),
                Role::Unarmed | Role::Structure | Role::Transport | Role::Gun | Role::Engineer => {}
            }
        }
        out
    }

    /// The enemy home at `reached` was searched: head for the next.
    fn enemy_home_searched_at(&mut self, reached: Vec2, view: &PlayerView) {
        for h in self
            .enemy_homes
            .iter_mut()
            .filter(|h| h.position == reached)
        {
            h.searched = true;
        }
        if self.enemy_homes.is_empty() {
            self.enemy_home_searched = true;
        } else {
            self.choose_enemy_home(view);
        }
    }

    /// Heads for the nearest home of an undefeated enemy that was not
    /// searched yet (prompt 037: with teams there are several).
    fn choose_enemy_home(&mut self, view: &PlayerView) {
        let alive = |p: PlayerId| {
            view.enemy_players
                .iter()
                .any(|&(q, defeated)| q == p && !defeated)
        };
        let next = nearest_home(&self.enemy_homes, self.home, |h| {
            alive(h.player) && !h.searched
        })
        .or_else(|| nearest_home(&self.enemy_homes, self.home, |h| alive(h.player)));
        if let Some(h) = next {
            self.enemy_home = h.position;
        }
        if !self.enemy_homes.is_empty() {
            self.enemy_home_searched = self
                .enemy_homes
                .iter()
                .filter(|h| alive(h.player))
                .all(|h| h.searched);
        }
    }

    fn type_info(&self, unit_type: u32) -> TypeInfo {
        self.types
            .get(unit_type as usize)
            .copied()
            .unwrap_or(TypeInfo {
                role: Role::Unarmed,
                range: Fixed::ZERO,
                sight: Fixed::ZERO,
                anti_tank: false,
                indirect: None,
                seats: 0,
                squad: 0,
                tows: false,
                power_hp: 0,
                mass_kg: 0,
            })
    }

    /// Soldiers of a squad type (0: no squad).
    fn squad_size(&self, unit_type: u32) -> u16 {
        self.type_info(unit_type).squad
    }

    /// Whether a vehicle of type `tractor` can pull a gun of type `gun`
    /// (`data/transport.toml`: power per tonne of both).
    fn can_pull(&self, tractor: u32, gun: u32) -> bool {
        let (t, g) = (self.type_info(tractor), self.type_info(gun));
        let min = self.tow_min_hp_per_tonne;
        t.tows
            && u64::from(t.power_hp) * 1000
                >= u64::from(min) * (u64::from(t.mass_kg) + u64::from(g.mass_kg))
    }

    /// Updates the memory of enemy positions: visible units are recorded,
    /// wrecks forgotten, and so are sightings an own unit is close to without
    /// seeing the enemy there any more.
    fn remember(&mut self, view: &PlayerView) {
        for e in &view.enemies {
            if e.alive() {
                self.memory.insert(
                    e.id,
                    Sighting {
                        position: e.position,
                        tick: view.tick,
                        owner: e.owner,
                        unit_type: e.unit_type,
                        hp: e.hp,
                        max_hp: e.max_hp,
                        facing: e.facing,
                    },
                );
            } else {
                self.memory.remove(&e.id);
            }
        }
        let visible: Vec<UnitId> = view.enemies.iter().map(|e| e.id).collect();
        let own: Vec<(Vec2, Fixed)> = view
            .own
            .iter()
            .filter(|u| u.alive())
            .map(|u| (u.position, self.type_info(u.unit_type).sight / 4))
            .collect();
        self.memory.retain(|id, s| {
            visible.contains(id) || !own.iter().any(|&(p, r)| dist(p, s.position) <= r)
        });
        // The commander forgets where mobile units stood long ago (prompt
        // 051): they have moved; scouting finds them again.
        if self.commander {
            let tick = view.tick;
            let types = &self.types;
            self.memory.retain(|_, s| {
                let structure = types
                    .get(s.unit_type as usize)
                    .is_some_and(|t| t.role == Role::Structure);
                intel::freshness(tick.saturating_sub(s.tick), structure) > 0
            });
        }
    }

    fn command_group(
        &mut self,
        role: Role,
        units: &[OwnUnit],
        view: &PlayerView,
        out: &mut Vec<Command>,
    ) {
        let Some(center) = centroid(units.iter().map(|u| u.position)) else {
            return;
        };
        let range = units
            .iter()
            .map(|u| self.type_info(u.unit_type).range)
            .max()
            .unwrap_or(Fixed::ZERO);
        let sight = units
            .iter()
            .map(|u| self.type_info(u.unit_type).sight)
            .max()
            .unwrap_or(Fixed::ZERO);
        let enemies: Vec<SeenUnit> = view.enemies.iter().filter(|e| e.alive()).copied().collect();
        let mut state = self.groups.remove(&role).unwrap_or(GroupState {
            order: Order::None,
            issued: 0,
            explore: None,
            enemy_hp: 0,
            progress_tick: view.tick,
            maneuver: None,
            flank_left: self.rng.below(2) == 0,
        });
        let stale = view.tick >= state.issued + REISSUE_TICKS;

        // Damaged units pull back once (Hard), veterans also on Normal and
        // earlier (prompt 050).
        let mut fighting: Vec<OwnUnit> = units.to_vec();
        if enemies.is_empty() {
            // Out of the fight: the damaged follow the group again.
            self.retreated.clear();
        }
        let (base_pct, veteran_pct) = (
            self.params.retreat_hp_pct,
            self.params.veteran_retreat_hp_pct,
        );
        if (base_pct > 0 || veteran_pct > 0)
            && let Some(enemy_center) = centroid(enemies.iter().map(|e| e.position))
        {
            let damaged = |u: &OwnUnit| {
                let threshold = u64::from(retreat_pct(base_pct, veteran_pct, u.rank));
                u64::from(u.hp) * 100 < u64::from(u.max_hp) * threshold
            };
            let healthy = units.iter().filter(|u| !damaged(u)).count();
            for u in units.iter().filter(|u| damaged(u)) {
                if healthy == 0 || self.retreated.contains(&u.id) {
                    continue;
                }
                let away = toward(enemy_center, center, Fixed::from_num(RETREAT_M));
                let rear = self.clamp_to_map(away);
                out.push(Command::MoveTo {
                    units: vec![u.id],
                    target: rear,
                });
                self.retreated.push(u.id);
            }
            if healthy > 0 {
                fighting.retain(|u| !self.retreated.contains(&u.id));
            }
        }
        let ids: Vec<UnitId> = fighting.iter().map(|u| u.id).collect();
        if ids.is_empty() {
            self.groups.insert(role, state);
            return;
        }

        // Engage what is in sight.
        let engage = sight * ENGAGE_SIGHT_PCT / 100;
        let near: Vec<&SeenUnit> = enemies
            .iter()
            .filter(|e| dist(e.position, center) <= engage)
            .collect();
        // Infantry (prompt 022): tanks fight tanks first, squads only what
        // their small arms can hurt. Squads facing nothing but armor stop
        // where they are instead of walking into it.
        let armor = |e: &&SeenUnit| self.type_info(e.unit_type).role == Role::Armor;
        let structure = |e: &&SeenUnit| self.type_info(e.unit_type).role == Role::Structure;
        let armor_near = near.iter().any(armor);
        // Buildings (prompt 023) come last: armor shoots them when nothing
        // else is near, infantry cannot hurt them.
        let mobile_near = near.iter().any(|e| !structure(e));
        // Squads with AT weapons (built in base matches) take on armor and
        // buildings once no enemy infantry is left near.
        let anti_tank = units.iter().any(|u| self.type_info(u.unit_type).anti_tank);
        let soft_near = near.iter().any(|e| !armor(e) && !structure(e));
        // Raids (prompt 051): with no enemy armor near, the commander's
        // armor shoots the enemy's trucks and halftracks first.
        let carrier = |e: &&SeenUnit| self.type_info(e.unit_type).role == Role::Transport;
        let carrier_near = self.commander && near.iter().any(carrier);
        let near: Vec<&SeenUnit> = match role {
            Role::Armor if armor_near => near.into_iter().filter(armor).collect(),
            Role::Armor if carrier_near => near.into_iter().filter(carrier).collect(),
            Role::Armor if mobile_near => near.into_iter().filter(|e| !structure(e)).collect(),
            Role::Infantry if anti_tank && !soft_near => {
                if mobile_near {
                    near.into_iter().filter(|e| !structure(e)).collect()
                } else {
                    near
                }
            }
            Role::Infantry => near
                .into_iter()
                .filter(|e| !armor(e) && !structure(e))
                .collect(),
            _ => near,
        };
        if self.commander
            && self.command_army(role, &fighting, center, range, &near, view, &mut state, out)
        {
            self.groups.insert(role, state);
            return;
        }
        if role == Role::Infantry && near.is_empty() && armor_near {
            if fighting.iter().any(|u| u.target.is_some()) {
                out.push(Command::Stop { units: ids });
            }
            state.order = Order::None;
            state.maneuver = None;
            self.groups.insert(role, state);
            return;
        }
        if near.is_empty() {
            state.maneuver = None;
        } else {
            let enemy_hp: u64 = near.iter().map(|e| u64::from(e.hp)).sum();
            if enemy_hp != state.enemy_hp {
                state.progress_tick = view.tick;
                state.enemy_hp = enemy_hp;
            }
            if let Some(started) = state.maneuver {
                let idle = fighting.iter().all(|u| u.target.is_none());
                if (idle && view.tick > started + REISSUE_TICKS / 2)
                    || view.tick > started + 3 * REISSUE_TICKS
                {
                    state.maneuver = None;
                    state.order = Order::None;
                    state.progress_tick = view.tick;
                } else {
                    self.groups.insert(role, state);
                    return;
                }
            } else if self.params.patience_ticks > 0
                && view.tick > state.progress_tick + self.params.patience_ticks
            {
                let enemy_center =
                    centroid(near.iter().map(|e| e.position)).unwrap_or(near[0].position);
                let side = flank_point(center, enemy_center, range, state.flank_left);
                state.flank_left = !state.flank_left;
                let side = self.clamp_to_map(side);
                out.push(Command::MoveTo {
                    units: ids,
                    target: side,
                });
                state.order = Order::Move {
                    target: side,
                    units: Vec::new(),
                };
                state.maneuver = Some(view.tick);
                state.issued = view.tick;
                self.groups.insert(role, state);
                return;
            }
            let mut targets: Vec<(UnitId, Vec<UnitId>)> = Vec::new();
            match self.params.focus {
                Focus::None => {}
                focus => {
                    let target = pick_target(&near, center, range, focus, &state.order);
                    targets.push((target, ids));
                }
            }
            let order = Order::Attack(targets);
            if state.order != order || stale {
                if let Order::Attack(targets) = &order {
                    for (target, units) in targets {
                        out.push(Command::Attack {
                            units: units.clone(),
                            target: *target,
                        });
                    }
                }
                state.order = order;
                state.issued = view.tick;
            }
            self.groups.insert(role, state);
            return;
        }

        // Base building (prompt 023): wait for the wave near the base.
        if let Some(hold) = self.hold_point(role, ids.len(), center, view) {
            if dist(center, hold) > Fixed::from_num(ARRIVED_M) {
                let idle = fighting.iter().all(|u| u.target.is_none());
                let facing = Some(self.enemy_home - hold);
                self.move_group(
                    &mut state,
                    &ids,
                    (hold, facing),
                    stale && idle,
                    view.tick,
                    out,
                );
            }
            self.groups.insert(role, state);
            return;
        }

        // Nothing in sight: gather, then head for the objective.
        if self.params.regroup {
            let spacing = units
                .iter()
                .map(|u| u.position)
                .map(|p| dist(p, center))
                .max()
                .unwrap_or(Fixed::ZERO);
            let n = i32::try_from(ids.len()).unwrap_or(i32::MAX);
            let allowed = Fixed::from_num(60 + 8 * n);
            let regrouping = matches!(state.order, Order::Regroup { .. });
            let all_idle = fighting.iter().all(|u| u.target.is_none());
            if spacing > allowed && !(regrouping && (all_idle || stale)) {
                if !regrouping {
                    out.push(Command::MoveTo {
                        units: ids,
                        target: center,
                    });
                    state.order = Order::Regroup { target: center };
                    state.issued = view.tick;
                }
                self.groups.insert(role, state);
                return;
            }
        }

        let objective = self.objective(&mut state, center, view);
        let target = match objective {
            Objective::Sighting(id) => self.memory[&id].position,
            Objective::EnemyHome => self.enemy_home,
            Objective::Supply(p) | Objective::Explore(p) => p,
        };
        if dist(center, target) <= Fixed::from_num(ARRIVED_M) {
            match objective {
                Objective::Sighting(id) => {
                    self.memory.remove(&id);
                }
                Objective::EnemyHome => {
                    let reached = self.enemy_home;
                    self.enemy_home_searched_at(reached, view);
                }
                // Units at the point take it over: wait there.
                Objective::Supply(_) => {}
                Objective::Explore(_) => state.explore = None,
            }
        } else {
            let idle = fighting.iter().all(|u| u.target.is_none());
            self.move_group(
                &mut state,
                &ids,
                (target, None),
                stale && idle,
                view.tick,
                out,
            );
        }
        self.groups.insert(role, state);
    }

    /// Where to go without enemies in sight: the freshest sighting, the
    /// enemy start (the nearest undefeated enemy's), then the nearest
    /// supply point an enemy holds (hunting the last units, prompt 037),
    /// then random points.
    fn objective(&mut self, state: &mut GroupState, center: Vec2, view: &PlayerView) -> Objective {
        let freshest = self
            .memory
            .iter()
            .max_by(|a, b| {
                a.1.tick
                    .cmp(&b.1.tick)
                    // Ties: the nearer one, then the lower id.
                    .then(dist(b.1.position, center).cmp(&dist(a.1.position, center)))
                    .then(b.0.cmp(a.0))
            })
            .map(|(&id, _)| id);
        if let Some(id) = freshest {
            return Objective::Sighting(id);
        }
        if !self.enemy_home_searched {
            return Objective::EnemyHome;
        }
        let held = view
            .supply
            .iter()
            .filter(|p| {
                p.owner.is_some_and(|o| {
                    view.enemy_players
                        .iter()
                        .any(|&(q, defeated)| q == o && !defeated)
                })
            })
            .min_by_key(|p| (dist(p.position, center), p.name.clone()));
        if let Some(p) = held {
            return Objective::Supply(p.position);
        }
        let point = match state.explore {
            Some(p) => p,
            None => {
                let hx = (self.half.x * 8 / 10).to_num::<i32>();
                let hy = (self.half.y * 8 / 10).to_num::<i32>();
                let p = Vec2::from_meters(self.rng.range_i32(-hx, hx), self.rng.range_i32(-hy, hy));
                state.explore = Some(p);
                p
            }
        };
        Objective::Explore(point)
    }

    /// Sends the group to `target` unless it already drives there; with
    /// attack-move (prompt 046) in an automatic formation facing `facing`
    /// (default: the direction of travel).
    fn move_group(
        &self,
        state: &mut GroupState,
        ids: &[UnitId],
        (target, facing): (Vec2, Option<Vec2>),
        repeat: bool,
        tick: u64,
        out: &mut Vec<Command>,
    ) {
        let same = !repeat
            && match &state.order {
                Order::Move {
                    target: t,
                    units: u,
                } => u == ids && dist(*t, target) <= Fixed::from_num(RETARGET_M),
                _ => false,
            };
        if same {
            return;
        }
        let target = self.clamp_to_map(target);
        if self.params.attack_move {
            out.push(Command::FormationMove {
                units: ids.to_vec(),
                target,
                formation: Formation::Auto,
                facing,
                engage: Engage::All,
            });
        } else {
            out.push(Command::MoveTo {
                units: ids.to_vec(),
                target,
            });
        }
        state.order = Order::Move {
            target,
            units: ids.to_vec(),
        };
        state.issued = tick;
    }

    fn clamp_to_map(&self, p: Vec2) -> Vec2 {
        let margin = Fixed::from_num(20);
        let hx = (self.half.x - margin).max(Fixed::ZERO);
        let hy = (self.half.y - margin).max(Fixed::ZERO);
        Vec2::new(p.x.clamp(-hx, hx), p.y.clamp(-hy, hy))
    }

    /// Center of the own start.
    pub const fn home(&self) -> Vec2 {
        self.home
    }
}

/// The common target of a group: keeps the current one while it is in
/// sight, else picks by `focus`.
fn pick_target(
    near: &[&SeenUnit],
    center: Vec2,
    range: Fixed,
    focus: Focus,
    current: &Order,
) -> UnitId {
    if let Order::Attack(targets) = current
        && let [(target, _)] = targets.as_slice()
        && near.iter().any(|e| e.id == *target)
    {
        return *target;
    }
    let nearest = |e: &&&SeenUnit| (dist(e.position, center), e.id);
    let in_range: Vec<&&SeenUnit> = near
        .iter()
        .filter(|e| dist(e.position, center) <= range)
        .collect();
    let pick = match focus {
        Focus::Weakest if !in_range.is_empty() => in_range
            .into_iter()
            .min_by_key(|e| (e.hp, dist(e.position, center), e.id)),
        _ => near.iter().min_by_key(nearest),
    };
    pick.map_or(near[0].id, |e| e.id)
}

/// A point beside the enemy: turned 70 degrees from the line between the
/// group and the enemy, at 60 % of the group's range (at least 60 m).
fn flank_point(center: Vec2, enemy: Vec2, range: Fixed, left: bool) -> Vec2 {
    let d = center - enemy;
    let len = d.length();
    if len == Fixed::ZERO {
        return center;
    }
    let reach = (range * 6 / 10).max(Fixed::from_num(60));
    let u = d.scale(reach / len);
    // cos 70° ≈ 0.342, sin 70° ≈ 0.940.
    let (c, s) = (Fixed::from_num(342) / 1000, Fixed::from_num(940) / 1000);
    let s = if left { s } else { -s };
    enemy + Vec2::new(u.x * c - u.y * s, u.x * s + u.y * c)
}

/// Price of every unit type of the match that can be bought (prompt 023).
/// The home of `homes` nearest to `from` among those `wanted` accepts
/// (ties: the lower player id).
fn nearest_home(
    homes: &[EnemyHome],
    from: Vec2,
    wanted: impl Fn(&EnemyHome) -> bool,
) -> Option<EnemyHome> {
    homes
        .iter()
        .filter(|h| wanted(h))
        .min_by_key(|h| (dist(h.position, from), h.player))
        .copied()
}

fn unit_costs(config: &SimConfig) -> Vec<Option<sim::Resources>> {
    let rules = sim::EconomyRules::builtin().ok();
    let catalog = sim::economy::builtin_catalog();
    config
        .unit_types
        .iter()
        .map(|t| {
            let (rules, catalog) = (rules.as_ref()?, catalog?);
            let cost = if rules.structure(&t.name).is_some() {
                rules.structure_cost(&t.name, catalog, &t.combat)?
            } else {
                rules.unit_cost(catalog.get(&t.name)?, &t.combat)
            };
            Some(cost.resources)
        })
        .collect()
}

fn dist(a: Vec2, b: Vec2) -> Fixed {
    (a - b).length()
}

/// Mean of the points, `None` for none.
fn centroid(points: impl Iterator<Item = Vec2>) -> Option<Vec2> {
    let mut sum = Vec2::default();
    let mut n = 0i64;
    for p in points {
        sum = sum + p;
        n += 1;
    }
    (n > 0).then(|| Vec2::new(sum.x / n, sum.y / n))
}

/// The point `distance` beyond `to`, seen from `from`.
fn toward(from: Vec2, to: Vec2, distance: Fixed) -> Vec2 {
    let d = to - from;
    let len = d.length();
    if len == Fixed::ZERO {
        return to;
    }
    to + d.scale(distance / len)
}
