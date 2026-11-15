//! What a computer player may know about the match: the fog-of-war view of
//! its player. The AI reads the simulation only through [`PlayerView`].

use sim::{FireMode, PlayerId, Resources, Sim, StructureView, SupplyView, UnitId, Vec2};

/// An own unit: everything the player knows about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnUnit {
    pub id: UnitId,
    /// Index into `SimConfig::unit_types`.
    pub unit_type: u32,
    pub position: Vec2,
    /// Current move target (formation slot), `None` when idle.
    pub target: Option<Vec2>,
    pub hp: u32,
    pub max_hp: u32,
    pub fire_mode: FireMode,
}

impl OwnUnit {
    pub const fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// An enemy unit the player sees right now: what one can see from outside
/// (not its orders or fire mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeenUnit {
    pub id: UnitId,
    pub owner: PlayerId,
    pub unit_type: u32,
    pub position: Vec2,
    /// Hull direction.
    pub facing: Vec2,
    /// Turret direction.
    pub aim: Vec2,
    /// Hit points: a hit tank visibly burns, a squad visibly shrinks.
    pub hp: u32,
    pub max_hp: u32,
}

impl SeenUnit {
    pub const fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// The match as `player` sees it after a tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerView {
    pub player: PlayerId,
    pub tick: u64,
    /// Own units (also destroyed ones), by id.
    pub own: Vec<OwnUnit>,
    /// Enemy units the player sees (wrecks are seen by everyone), by id.
    pub enemies: Vec<SeenUnit>,
    /// The match is decided.
    pub decided: bool,
    /// Resources of the player (prompt 023).
    pub account: Resources,
    /// Own buildings, construction sites and fortifications, by unit id.
    pub structures: Vec<StructureView>,
    /// Supply points of the map and who holds them (public).
    pub supply: Vec<SupplyView>,
    /// For every structure type the player can build right now: the first
    /// free spot around its headquarters (or its start), by type index.
    pub sites: Vec<(u32, Vec2)>,
}

impl PlayerView {
    /// Builds the view from the simulation through its fog-of-war filter
    /// ([`Sim::snapshot_for`]). This is the only place the AI touches the
    /// simulation's state.
    pub fn new(sim: &Sim, player: PlayerId) -> Self {
        let snapshot = sim.snapshot_for(player);
        let mut own = Vec::new();
        let mut enemies = Vec::new();
        for u in snapshot.units {
            if u.owner == player {
                own.push(OwnUnit {
                    id: u.id,
                    unit_type: u.unit_type,
                    position: u.position,
                    target: u.target,
                    hp: u.hp,
                    max_hp: u.max_hp,
                    fire_mode: u.fire_mode,
                });
            } else {
                enemies.push(SeenUnit {
                    id: u.id,
                    owner: u.owner,
                    unit_type: u.unit_type,
                    position: u.position,
                    facing: u.facing,
                    aim: u.aim,
                    hp: u.hp,
                    max_hp: u.max_hp,
                });
            }
        }
        let structures: Vec<StructureView> = sim
            .structures()
            .into_iter()
            .filter(|s| s.owner == player)
            .collect();
        let base = structures
            .first()
            .and_then(|s| sim.unit_position(s.unit))
            .or_else(|| own.first().map(|u| u.position))
            .unwrap_or_default();
        let sites = sim
            .build_options(player)
            .into_iter()
            .filter(|o| o.blocked.is_none())
            .filter_map(|o| Some((o.unit_type, sim.find_site(player, o.unit_type, base)?)))
            .collect();
        Self {
            player,
            tick: snapshot.tick,
            own,
            enemies,
            decided: sim.outcome().is_some(),
            account: sim.account(player).unwrap_or_default(),
            structures,
            supply: sim.supply_points(),
            sites,
        }
    }
}
