//! What a computer player may know about the match: the fog-of-war view of
//! its player. The AI reads the simulation only through [`PlayerView`].

use sim::{
    AirOption, FireMode, PlayerId, Resources, Sim, StockView, StructureView, SupplySource,
    SupplyView, UnitId, Vec2,
};

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
    /// Limits of automatic fire (prompt 176).
    pub fire_limits: sim::FireLimits,
    /// The vehicle it rides in or is towed by (prompt 034).
    pub carrier: Option<UnitId>,
    /// Soldiers riding in or on it.
    pub passengers: u16,
    /// The gun it tows.
    pub towing: Option<UnitId>,
    /// Ammunition, fuel and mobility (prompt 036).
    pub stock: StockView,
    /// Rank index (prompt 050; 0 = recruit).
    pub rank: u8,
    /// Hull direction (prompt 110).
    pub facing: Vec2,
    /// Where it remembers being fired at from (prompt 110).
    pub alert: Option<Vec2>,
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
    /// Units of allied players (prompt 037; always visible), by id.
    pub allies: Vec<SeenUnit>,
    /// Players of the match that are enemies of this one, ascending,
    /// with whether they are defeated (public: the result screen).
    pub enemy_players: Vec<(PlayerId, bool)>,
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
    /// The same, searched from 100 m in front of the headquarters towards
    /// the map center (prompt 051: forward depots).
    pub forward_sites: Vec<(u32, Vec2)>,
    /// Air missions the player can call (prompt 033).
    pub air: Vec<AirOption>,
    /// Enemy aircraft the player sees now.
    pub enemy_aircraft: usize,
    /// Own and allied suppliers and repairers (prompt 036).
    pub sources: Vec<SupplySource>,
    /// Mines the player's team knows (prompt 035): its own and its
    /// allies', and the hostile ones it found. Never the others.
    pub mines: Vec<sim::MineView>,
    /// Field works: finished ones of everybody, sites of the own team.
    pub works: Vec<sim::WorkView>,
    /// Own bridge-laying tanks (prompt 055).
    pub layers: Vec<sim::LayerView>,
    /// Whether every bridge of the map stands (public: one sees a bridge
    /// is down), laid bridges after the map's own.
    pub bridges_intact: Vec<bool>,
    /// Whether and when the player can call a heavy strike (prompt 054).
    pub strike: sim::StrikeOption,
    /// Every called strike (public: smoke, siren, revealed position).
    pub strikes: Vec<sim::StrikeView>,
}

impl PlayerView {
    /// Builds the view from the simulation through its fog-of-war filter
    /// ([`Sim::snapshot_for`]). This is the only place the AI touches the
    /// simulation's state.
    pub fn new(sim: &Sim, player: PlayerId) -> Self {
        let snapshot = sim.snapshot_for(player);
        let mut own = Vec::new();
        let mut enemies = Vec::new();
        let mut allies = Vec::new();
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
                    fire_limits: u.fire_limits,
                    carrier: u.carrier,
                    passengers: u.passengers,
                    towing: u.towing,
                    stock: u.stock,
                    rank: u.experience.rank,
                    facing: u.facing,
                    alert: u.alert,
                });
            } else {
                let seen = SeenUnit {
                    id: u.id,
                    owner: u.owner,
                    unit_type: u.unit_type,
                    position: u.position,
                    facing: u.facing,
                    aim: u.aim,
                    hp: u.hp,
                    max_hp: u.max_hp,
                };
                if sim.allied(player, u.owner) {
                    allies.push(seen);
                } else {
                    enemies.push(seen);
                }
            }
        }
        let enemy_players = sim
            .standings()
            .iter()
            .filter(|s| !sim.allied(player, s.player))
            .map(|s| (s.player, s.defeat.is_some()))
            .collect();
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
        let options: Vec<u32> = sim
            .build_options(player)
            .into_iter()
            .filter(|o| o.blocked.is_none())
            .map(|o| o.unit_type)
            .collect();
        let sites = options
            .iter()
            .filter_map(|&t| Some((t, sim.find_site(player, t, base)?)))
            .collect();
        let ahead = base.length();
        let front = if ahead > sim::Fixed::from_num(100) {
            base - base.scale(sim::Fixed::from_num(100) / ahead)
        } else {
            base
        };
        let forward_sites = if structures.is_empty() {
            Vec::new()
        } else {
            options
                .iter()
                .filter_map(|&t| Some((t, sim.find_site(player, t, front)?)))
                .collect()
        };
        Self {
            player,
            tick: snapshot.tick,
            own,
            enemies,
            allies,
            enemy_players,
            decided: sim.outcome().is_some(),
            account: sim.account(player).unwrap_or_default(),
            structures,
            supply: sim.supply_points(),
            sites,
            forward_sites,
            air: sim.air_options(player),
            enemy_aircraft: snapshot
                .aircraft
                .iter()
                .filter(|a| !sim.allied(player, a.owner) && !a.crashing)
                .count(),
            sources: sim.supply_sources(player),
            mines: sim.mines_for(player),
            works: sim.works_for(player),
            layers: sim
                .layers()
                .into_iter()
                .filter(|l| l.owner == player)
                .collect(),
            bridges_intact: sim.map().bridges().iter().map(|b| b.intact).collect(),
            strike: sim.strike_option(player),
            strikes: sim.strikes(),
        }
    }
}
