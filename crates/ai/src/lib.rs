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

pub mod game;
mod view;

pub use view::{OwnUnit, PlayerView, SeenUnit};

use sim::{Command, FireMode, Fixed, Map, PlayerId, Rng, SimConfig, UnitId, Vec2};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// How well a computer player plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difficulty {
    /// Issues no commands: units stand and fire at will. Baseline for
    /// tests and tournaments; the enemy of the matches before prompt 024.
    Passive,
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Self; 4] = [Self::Passive, Self::Easy, Self::Normal, Self::Hard];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Passive => "passive",
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }

    const fn params(self) -> Params {
        match self {
            Self::Passive => Params {
                think_ticks: 0,
                focus: Focus::None,
                regroup: false,
                retreat_hp_pct: 0,
                patience_ticks: 0,
            },
            Self::Easy => Params {
                think_ticks: 80,
                focus: Focus::Nearest,
                regroup: false,
                retreat_hp_pct: 0,
                patience_ticks: 1600,
            },
            Self::Normal => Params {
                think_ticks: 20,
                focus: Focus::Nearest,
                regroup: true,
                retreat_hp_pct: 0,
                patience_ticks: 800,
            },
            Self::Hard => Params {
                think_ticks: 10,
                focus: Focus::Weakest,
                regroup: true,
                retreat_hp_pct: 30,
                patience_ticks: 800,
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
            .find(|d| d.name().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| format!("unknown AI difficulty '{s}' (passive, easy, normal, hard)"))
    }
}

/// How a group picks a common target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug)]
struct Params {
    /// The AI acts every this many ticks (reaction time); 0 = never.
    think_ticks: u64,
    focus: Focus,
    /// Gather stragglers before advancing.
    regroup: bool,
    /// Units below this share of their hit points pull back behind the
    /// group while enemies are near (0 = never).
    retreat_hp_pct: u32,
    /// Ticks without damage to the enemy after which a fighting group
    /// moves round to the enemy's flank (wrecks and comrades in the line of
    /// fire, a crest in between).
    patience_ticks: u64,
}

/// Role of a unit; every role is commanded as its own group. Infantry
/// (prompt 022) and builders (prompt 023) get their own rules here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    /// Armed vehicles.
    Armor,
    /// Infantry squads.
    Infantry,
    /// Units without weapons: stay at home.
    Unarmed,
}

/// What the AI knows about a unit type from the (public) config.
#[derive(Clone, Copy, Debug)]
struct TypeInfo {
    role: Role,
    /// Longest weapon range (game meters).
    range: Fixed,
    /// Sight range (game meters).
    sight: Fixed,
}

/// Where an enemy unit was last seen.
#[derive(Clone, Copy, Debug)]
struct Sighting {
    position: Vec2,
    tick: u64,
}

/// The last order a group got.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Order {
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
    /// The enemy's start position.
    EnemyHome,
    /// A random point (everything known was searched).
    Explore(Vec2),
}

#[derive(Clone, Debug)]
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
/// Damaged units pull back this far behind the group (m).
const RETREAT_M: i32 = 80;
/// Orders are repeated after this many ticks: units drop attack orders on
/// targets they lose sight of, and moves can end early.
const REISSUE_TICKS: u64 = 100;
/// Groups engage enemies within this share of their sight range (percent).
const ENGAGE_SIGHT_PCT: i64 = 100;

/// A computer player.
#[derive(Clone, Debug)]
pub struct Ai {
    player: PlayerId,
    difficulty: Difficulty,
    params: Params,
    types: Vec<TypeInfo>,
    /// Center of the own start (m).
    home: Vec2,
    /// Start of the enemy: from the map, else mirrored through the center.
    enemy_home: Vec2,
    enemy_home_searched: bool,
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
}

impl Ai {
    /// A computer player for `player` of the match `config` on `map` (both
    /// public knowledge). `seed` drives its random choices.
    pub fn new(
        player: PlayerId,
        difficulty: Difficulty,
        config: &SimConfig,
        map: &Map,
        seed: u64,
    ) -> Self {
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
                let role = if t.combat.weapons.is_empty() {
                    Role::Unarmed
                } else if t.combat.is_infantry() {
                    Role::Infantry
                } else {
                    Role::Armor
                };
                TypeInfo {
                    role,
                    range,
                    sight: config.vision.range_m(t.combat.category),
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
        let enemy_home = map
            .starts()
            .iter()
            .find(|s| s.player != player.0)
            .map(|s| s.position)
            .or_else(|| {
                centroid(
                    config
                        .spawns
                        .iter()
                        .filter(|s| s.owner != player)
                        .map(|s| s.position),
                )
                // Without starts in the map, spawns are the scenario's: the
                // enemy's are public like the map. Mirror as a last resort.
            })
            .unwrap_or(Vec2::new(-home.x, -home.y));
        let size = map.size();
        Self {
            player,
            difficulty,
            params: difficulty.params(),
            types,
            home,
            enemy_home,
            enemy_home_searched: false,
            half: Vec2::new(size.x / 2, size.y / 2),
            rng: Rng::new(seed ^ (u64::from(player.0) << 56) ^ 0xA1_0024),
            memory: BTreeMap::new(),
            groups: BTreeMap::new(),
            retreated: Vec::new(),
            started: false,
            last_tick: None,
            thinks: 0,
        }
    }

    pub const fn player(&self) -> PlayerId {
        self.player
    }

    pub const fn difficulty(&self) -> Difficulty {
        self.difficulty
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

        let mut roles: Vec<Role> = Vec::new();
        for u in view.own.iter().filter(|u| u.alive()) {
            let role = self.type_info(u.unit_type).role;
            if !roles.contains(&role) {
                roles.push(role);
            }
        }
        roles.sort_unstable();
        for role in roles {
            let units: Vec<OwnUnit> = view
                .own
                .iter()
                .filter(|u| u.alive() && self.type_info(u.unit_type).role == role)
                .copied()
                .collect();
            match role {
                Role::Armor | Role::Infantry => self.command_group(role, &units, view, &mut out),
                Role::Unarmed => {}
            }
        }
        out
    }

    fn type_info(&self, unit_type: u32) -> TypeInfo {
        self.types
            .get(unit_type as usize)
            .copied()
            .unwrap_or(TypeInfo {
                role: Role::Unarmed,
                range: Fixed::ZERO,
                sight: Fixed::ZERO,
            })
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

        // Damaged units pull back once (Hard).
        let mut fighting: Vec<OwnUnit> = units.to_vec();
        if enemies.is_empty() {
            // Out of the fight: the damaged follow the group again.
            self.retreated.clear();
        }
        if self.params.retreat_hp_pct > 0
            && let Some(enemy_center) = centroid(enemies.iter().map(|e| e.position))
        {
            let threshold = u64::from(self.params.retreat_hp_pct);
            let damaged = |u: &OwnUnit| u64::from(u.hp) * 100 < u64::from(u.max_hp) * threshold;
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

        let objective = self.objective(&mut state, center);
        let target = match objective {
            Objective::Sighting(id) => self.memory[&id].position,
            Objective::EnemyHome => self.enemy_home,
            Objective::Explore(p) => p,
        };
        if dist(center, target) <= Fixed::from_num(ARRIVED_M) {
            match objective {
                Objective::Sighting(id) => {
                    self.memory.remove(&id);
                }
                Objective::EnemyHome => self.enemy_home_searched = true,
                Objective::Explore(_) => state.explore = None,
            }
        } else {
            let idle = fighting.iter().all(|u| u.target.is_none());
            self.move_group(&mut state, &ids, target, stale && idle, view.tick, out);
        }
        self.groups.insert(role, state);
    }

    /// Where to go without enemies in sight: the freshest sighting, the
    /// enemy start, then random points.
    fn objective(&mut self, state: &mut GroupState, center: Vec2) -> Objective {
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

    /// Sends the group to `target` unless it already drives there.
    fn move_group(
        &self,
        state: &mut GroupState,
        ids: &[UnitId],
        target: Vec2,
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
        out.push(Command::MoveTo {
            units: ids.to_vec(),
            target,
        });
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
