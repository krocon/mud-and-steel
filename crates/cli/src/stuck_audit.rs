//! `cli stuck-audit` (prompt 173): how often units get stuck on the maps.
//!
//! For every map, groups of typical units (`data/stuck_audit.toml`) drive
//! routes across it: start to start, supply point to start, and over every
//! larger bridge in both directions. Each group drives all routes of a map
//! in one headless simulation (one player, no combat). A unit counts as
//! stuck when it follows a route and has not moved `stall_m` for `stall_s`
//! seconds, or when it stopped short of the end of its planned route. AI
//! games on a few maps add the units of real matches. Deterministic: the
//! same maps and settings give the same numbers.
//!
//! Output: a table per map, a CSV of every case (`--csv`), and the worst
//! places.
//!
//! Tracing (stderr): `STUCK_TRACE=1` prints every stalled unit once a
//! second, destroyed units and the AI games' stuck events;
//! `STUCK_ROUTE="<route label>"` the members of that route (and units
//! within 25 m) every second; `STUCK_AI_UNIT=<id>` one unit of the AI
//! games every second.

use ai::Difficulty;
use ai::game::Game;
use serde::Deserialize;
use sim::path::{PathGrid, SizeClass};
use sim::{
    Command, Fixed, Map, PlayerCommand, PlayerId, RouteView, Sim, SimConfig, Spawn, UnitId, Vec2,
};
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    pub stall_s: u32,
    pub stall_m: i32,
    pub long_s: u32,
    pub arrive_m: i32,
    pub max_s: u32,
    pub max_bridges: usize,
    pub approach_m: i32,
    #[serde(rename = "group")]
    pub groups: Vec<GroupKind>,
    pub ai: AiSettings,
    /// `--march`: formation orders that drive together (prompt 187).
    #[serde(skip)]
    pub march: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GroupKind {
    pub name: String,
    pub units: Vec<String>,
    #[serde(default)]
    pub tow: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AiSettings {
    pub games: u64,
    pub max_s: u32,
    pub maps: Vec<String>,
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// One route: a group drives from `from` to `to`.
#[derive(Clone, Debug)]
struct Route {
    label: String,
    from: Vec2,
    to: Vec2,
}

/// One stuck unit.
#[derive(Clone, Debug)]
pub struct Case {
    pub map: String,
    /// Group kind or `ai`.
    pub group: String,
    pub route: String,
    pub unit_type: String,
    pub at: Vec2,
    /// `stall` (no progress), `gave up` (stopped short) or `timeout`.
    pub what: &'static str,
    /// Longest stall in seconds (0 for a give-up without a stall).
    pub stall_s: u32,
    pub cause: String,
    pub replans: u32,
    pub reachable: bool,
}

/// Counts of one map.
#[derive(Clone, Debug, Default)]
pub struct MapResult {
    pub name: String,
    /// Group orders and units given one.
    pub orders: u32,
    pub units: u32,
    /// Orders with at least one stuck unit.
    pub stuck_orders: u32,
    pub stuck_units: u32,
    pub arrived_units: u32,
    /// Units destroyed or immobilized on the way (minefields of a map).
    pub lost_units: u32,
    /// Units still on their way, without a stall, when the run ended.
    pub unfinished_units: u32,
    /// "Stuck" events of the simulation (prompt 173, `Sim::move_events`)
    /// in the route runs and in the AI games.
    pub reported: u32,
    pub ai_reported: u32,
    /// Units of AI games watched, and stuck ones among them.
    pub ai_units: u32,
    pub ai_stuck: u32,
    pub cases: Vec<Case>,
}

impl MapResult {
    fn top_cause(&self) -> String {
        let mut counts: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
        for c in &self.cases {
            *counts.entry(c.cause.as_str()).or_default() += 1;
        }
        counts
            .into_iter()
            .max_by_key(|&(name, n)| (n, std::cmp::Reverse(name)))
            .map_or_else(|| "-".to_string(), |(name, n)| format!("{name} ({n})"))
    }
}

/// Watches one unit.
#[derive(Clone, Debug)]
struct Watch {
    id: UnitId,
    order: usize,
    group: String,
    route: String,
    /// Where it stood when the current stall window began.
    anchor: Vec2,
    anchor_tick: u64,
    /// End of its planned route (last waypoint seen).
    planned_end: Option<Vec2>,
    was_following: bool,
    replans: u32,
    /// Longest stall so far (ticks) and where it began.
    worst: u64,
    worst_at: Vec2,
    /// The current stall was already counted as a case.
    reported: bool,
    reachable: bool,
    /// Lost a track on a mine on the way.
    mined: bool,
    profile: usize,
    type_name: String,
    /// Where a long stall happened, filled when the stall ends.
    stall_case: Option<Case>,
    done: bool,
}

impl Watch {
    fn new(id: UnitId, at: Vec2, order: usize, group: &str, route: &str) -> Self {
        Self {
            id,
            order,
            group: group.to_string(),
            route: route.to_string(),
            anchor: at,
            anchor_tick: 0,
            planned_end: None,
            was_following: false,
            replans: 0,
            worst: 0,
            worst_at: at,
            reported: false,
            reachable: true,
            mined: false,
            profile: 0,
            type_name: String::new(),
            stall_case: None,
            done: false,
        }
    }
}

/// The audit of all maps (or one).
pub fn run(
    settings: &Settings,
    maps: &[(String, Arc<Map>)],
    jobs: usize,
) -> Result<Vec<MapResult>, String> {
    let jobs = jobs.max(1);
    let mut results: Vec<Option<Result<MapResult, String>>> = vec![None; maps.len()];
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out = std::sync::Mutex::new(&mut results);
    std::thread::scope(|scope| {
        for _ in 0..jobs.min(maps.len()) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some((name, map)) = maps.get(i) else {
                        break;
                    };
                    let result = audit_map(settings, name, map);
                    if let Ok(mut out) = out.lock() {
                        out[i] = Some(result);
                    }
                }
            });
        }
    });
    results
        .into_iter()
        .map(|r| r.unwrap_or_else(|| Err("not run".into())))
        .collect()
}

fn audit_map(settings: &Settings, name: &str, map: &Arc<Map>) -> Result<MapResult, String> {
    let mut result = MapResult {
        name: name.to_string(),
        ..MapResult::default()
    };
    for kind in &settings.groups {
        drive_routes(settings, name, map, kind, &mut result)?;
    }
    if settings.ai.maps.iter().any(|m| m == name) {
        for game in 0..settings.ai.games {
            watch_ai_game(settings, name, map, game, &mut result)?;
        }
    }
    Ok(result)
}

/// Unit types of a kind, the towed guns after the vehicles.
fn type_ids(kind: &GroupKind) -> Vec<String> {
    let mut ids: Vec<String> = kind.units.clone();
    for t in &kind.tow {
        if !ids.contains(t) {
            ids.push(t.clone());
        }
    }
    let mut unique = Vec::new();
    for id in ids {
        if !unique.contains(&id) {
            unique.push(id);
        }
    }
    unique
}

fn base_config(map: &Map, ids: &[String], spawns: Vec<Spawn>) -> Result<SimConfig, String> {
    let e = |e: &dyn std::fmt::Display| e.to_string();
    let catalog = sim::Catalog::builtin().map_err(|x| e(&x))?;
    let scale = sim::Scale::builtin().map_err(|x| e(&x))?;
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let unit_types = sim::unit_types_from_catalog(&catalog, &scale, &refs).map_err(|x| e(&x))?;
    Ok(SimConfig {
        tick_hz: SimConfig::DEFAULT_TICK_HZ,
        unit_types,
        spawns,
        map: map.reference(),
        terrain: sim::TerrainRules::builtin().map_err(|x| e(&x))?,
        combat: sim::CombatRules::builtin().map_err(|x| e(&x))?,
        vision: sim::VisionRules::builtin().map_err(|x| e(&x))?,
        infantry: sim::InfantryRules::builtin().map_err(|x| e(&x))?,
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    })
}

/// The routes of a map: start to start, supply point to start, and over
/// the longest bridges both ways. Origins closer than 40 m to an earlier
/// one are dropped (groups would start inside each other).
fn routes(settings: &Settings, map: &Map, grid: &PathGrid) -> Vec<Route> {
    let starts: Vec<Vec2> = map.starts().iter().map(|s| s.position).collect();
    let supplies: Vec<(String, Vec2)> = map
        .markers()
        .iter()
        .filter(|m| m.name.starts_with("supply"))
        .map(|m| (m.name.clone(), m.position))
        .collect();
    let mut all = Vec::new();
    let n = starts.len();
    if n >= 2 {
        for i in 0..n {
            all.push(Route {
                label: format!("start {i} -> start {}", (i + 1) % n),
                from: starts[i],
                to: starts[(i + 1) % n],
            });
        }
    }
    if n >= 1 {
        for (j, (name, at)) in supplies.iter().enumerate() {
            all.push(Route {
                label: format!("{name} -> start {}", j % n),
                from: *at,
                to: starts[j % n],
            });
        }
    }
    let mut bridges: Vec<(usize, &sim::Bridge)> = map
        .bridges()
        .iter()
        .enumerate()
        .filter(|(_, b)| b.intact && b.laid_by.is_none() && b.half_length >= Fixed::from_num(4))
        .collect();
    bridges.sort_by_key(|(i, b)| (std::cmp::Reverse(b.half_length), *i));
    let approach = Fixed::from_num(settings.approach_m);
    for (i, b) in bridges.into_iter().take(settings.max_bridges) {
        let a_end = b.center - b.forward.scale(b.half_length + approach);
        let b_end = b.center + b.forward.scale(b.half_length + approach);
        let a_far = b.center
            - b.forward
                .scale(b.half_length + approach + Fixed::from_num(20));
        let b_far = b.center
            + b.forward
                .scale(b.half_length + approach + Fixed::from_num(20));
        for (label, from, to) in [
            (format!("bridge {i} forward"), a_end, b_far),
            (format!("bridge {i} back"), b_end, a_far),
        ] {
            if let (Some(from), Some(to)) = (open_near(grid, map, from), open_near(grid, map, to)) {
                all.push(Route { label, from, to });
            }
        }
    }
    let mut kept: Vec<Route> = Vec::new();
    for r in all {
        if kept
            .iter()
            .all(|k| (k.from - r.from).length() >= Fixed::from_num(40))
        {
            kept.push(r);
        }
    }
    kept
}

/// The nearest point within 40 m (8 m steps around `p`) that is on the
/// map and open to vehicles.
fn open_near(grid: &PathGrid, map: &Map, p: Vec2) -> Option<Vec2> {
    for r in 0..=5i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let q = p + Vec2::from_meters(dx * 8, dy * 8);
                if map.contains(q) && grid.passable_at(SizeClass::Large, q) {
                    return Some(q);
                }
            }
        }
    }
    None
}

/// Free spawn spots for `count` units around `center` on cells open to
/// `class`, `spacing` meters apart, not in `used`.
fn spots(
    grid: &PathGrid,
    map: &Map,
    center: Vec2,
    classes: &[SizeClass],
    spacing: i32,
    used: &mut Vec<Vec2>,
) -> Option<Vec<Vec2>> {
    let mut out = Vec::new();
    for &class in classes {
        let mut found = None;
        'search: for r in 0..=12i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let q = center + Vec2::from_meters(dx * spacing, dy * spacing);
                    let free = used
                        .iter()
                        .all(|u| (*u - q).length() >= Fixed::from_num(spacing));
                    if free && map.contains(q) && grid.passable_at(class, q) {
                        found = Some(q);
                        break 'search;
                    }
                }
            }
        }
        let q = found?;
        used.push(q);
        out.push(q);
    }
    Some(out)
}

fn drive_routes(
    settings: &Settings,
    name: &str,
    map: &Arc<Map>,
    kind: &GroupKind,
    result: &mut MapResult,
) -> Result<(), String> {
    let ids = type_ids(kind);
    let index_of = |id: &str| ids.iter().position(|x| x == id).unwrap_or(0);
    // A first simulation without units gives the path grid for the routes
    // and the spawn spots.
    let probe = Sim::new(&base_config(map, &ids, Vec::new())?, map.clone(), 1)
        .map_err(|e| e.to_string())?;
    let grid = probe.path_grid();
    let routes = routes(settings, map, grid);
    if routes.is_empty() {
        return Ok(());
    }
    let class_of = |id: &str| grid.class_of(index_of(id));
    let mut used = Vec::new();
    let mut spawns = Vec::new();
    // Per route: unit indices (vehicles, then towed guns).
    let mut members: Vec<(usize, Vec<u32>, Vec<u32>)> = Vec::new();
    for (r, route) in routes.iter().enumerate() {
        let classes: Vec<SizeClass> = kind
            .units
            .iter()
            .chain(&kind.tow)
            .map(|u| class_of(u))
            .collect();
        let Some(at) = spots(grid, map, route.from, &classes, 10, &mut used) else {
            continue;
        };
        let first = u32::try_from(spawns.len()).unwrap_or(0);
        for (k, unit) in kind.units.iter().chain(&kind.tow).enumerate() {
            spawns.push(Spawn {
                owner: PlayerId(0),
                unit_type: unit.clone(),
                position: at[k],
                fire_mode: sim::FireMode::default(),
            });
        }
        let n = u32::try_from(kind.units.len()).unwrap_or(0);
        let t = u32::try_from(kind.tow.len()).unwrap_or(0);
        members.push((
            r,
            (first..first + n).collect(),
            (first + n..first + n + t).collect(),
        ));
    }
    let config = base_config(map, &ids, spawns)?;
    let mut sim = Sim::new(&config, map.clone(), 1).map_err(|e| e.to_string())?;
    let hz = u64::from(sim.tick_hz());

    // Orders: towing first, the move once the guns are hitched (or after a
    // minute).
    let mut watches: Vec<Watch> = Vec::new();
    let mut moves: Vec<(usize, PlayerCommand)> = Vec::new();
    let mut tows = Vec::new();
    for (order, (r, vehicles, guns)) in members.iter().enumerate() {
        let route = &routes[*r];
        let units: Vec<UnitId> = vehicles.iter().map(|&i| UnitId(i)).collect();
        for (k, &g) in guns.iter().enumerate() {
            if let Some(&v) = vehicles.get(k) {
                tows.push(PlayerCommand::new(
                    PlayerId(0),
                    Command::Tow {
                        units: vec![UnitId(v)],
                        gun: UnitId(g),
                    },
                ));
            }
        }
        moves.push((
            order,
            PlayerCommand::new(
                PlayerId(0),
                if settings.march {
                    Command::FormationMove {
                        units: units.clone(),
                        target: route.to,
                        formation: sim::Formation::Auto,
                        facing: None,
                        engage: sim::Engage::All,
                    }
                } else {
                    Command::MoveTo {
                        units: units.clone(),
                        target: route.to,
                    }
                },
            ),
        ));
        for &u in vehicles {
            let at = config.spawns[u as usize].position;
            let mut w = Watch::new(UnitId(u), at, order, &kind.name, &route.label);
            w.profile = index_of(&config.spawns[u as usize].unit_type);
            w.type_name.clone_from(&config.spawns[u as usize].unit_type);
            w.reachable = sim.path_grid().reachable(w.profile, at, route.to);
            watches.push(w);
        }
    }
    result.orders += u32::try_from(members.len()).unwrap_or(0);
    result.units += u32::try_from(watches.len()).unwrap_or(0);

    let longest = routes
        .iter()
        .map(|r| (r.to - r.from).length())
        .max()
        .unwrap_or(Fixed::ZERO);
    // Slowest member: about 1 m/s, a detour factor of 2.5, two minutes to
    // get going.
    let limit_s = (longest.to_num::<i64>() * 5 / 2 + 120).clamp(120, i64::from(settings.max_s));
    let limit = u64::try_from(limit_s).unwrap_or(0) * hz;
    let mut moved = false;
    let towing = !tows.is_empty();
    let hitch_deadline = 60 * hz;
    sim.step(&tows);
    for _ in 1..limit {
        let tick = sim.tick();
        let mut commands = Vec::new();
        if !moved {
            let hitched = !towing || {
                let snap = sim.snapshot();
                members.iter().all(|(_, _, guns)| {
                    guns.iter()
                        .all(|&g| unit_of(&snap, UnitId(g)).is_some_and(|u| u.carrier.is_some()))
                })
            };
            if hitched || tick >= hitch_deadline {
                commands = moves.iter().map(|(_, c)| c.clone()).collect();
                moved = true;
                for w in &mut watches {
                    w.anchor_tick = tick;
                }
            }
        }
        sim.step(&commands);
        if !moved {
            continue;
        }
        let tick = sim.tick();
        let snap = sim.snapshot();
        if let Ok(route) = std::env::var("STUCK_ROUTE")
            && tick.is_multiple_of(hz)
        {
            for w in watches.iter().filter(|w| w.route == route) {
                if let Some(u) = unit_of(&snap, w.id) {
                    for o in &snap.units {
                        if o.id != w.id && (o.position - u.position).length() < Fixed::from_num(25)
                        {
                            eprintln!(
                                "   near u{} ({:.1}, {:.1}) {:?}",
                                o.id.0,
                                o.position.x.to_num::<f64>(),
                                o.position.y.to_num::<f64>(),
                                sim.route_of(o.id).map(|r| matches!(r, RouteView::Idle))
                            );
                        }
                    }
                    eprintln!(
                        "route {} t{} u{} ({:.1}, {:.1}) {:?}",
                        kind.name,
                        tick / hz,
                        w.id.0,
                        u.position.x.to_num::<f64>(),
                        u.position.y.to_num::<f64>(),
                        match sim.route_of(w.id) {
                            Some(RouteView::Following(p)) => format!(
                                "next ({:.0},{:.0}) of {}",
                                p[0].x.to_num::<f64>(),
                                p[0].y.to_num::<f64>(),
                                p.len()
                            ),
                            other => format!("{other:?}"),
                        }
                    );
                }
            }
        }
        result.reported += u32::try_from(sim.move_events().len()).unwrap_or(0);
        for (id, _) in sim.tracks_off() {
            if let Some(w) = watches.iter_mut().find(|w| w.id == id) {
                w.mined = true;
            }
        }
        if std::env::var_os("STUCK_TRACE").is_some() {
            for e in sim.events() {
                if let sim::CombatEvent::Destroyed { unit, by, at } = e {
                    eprintln!(
                        "destroyed {} {} u{} by u{} at {at:?}",
                        kind.name, tick, unit.0, by.0
                    );
                }
            }
        }
        let mut busy = false;
        for w in &mut watches {
            if w.done {
                continue;
            }
            busy |= observe(&sim, &snap, w, tick, settings, name, map);
        }
        if !busy && tick > 2 {
            break;
        }
    }
    // End of the run: give-ups, timeouts and long stalls.
    let mut stuck_orders = vec![false; members.len()];
    let snap = sim.snapshot();
    for w in &mut watches {
        let snapshot_unit = unit_of(&snap, w.id);
        let Some(u) = snapshot_unit else { continue };
        if !u.alive() || u.stock.immobile || w.mined {
            // Lost or immobilized (mines on some maps), not stuck.
            result.lost_units += 1;
            continue;
        }
        let idle = sim.route_of(w.id) == Some(RouteView::Idle);
        let slack = Fixed::from_num(settings.arrive_m) + u.spread;
        let short = w
            .planned_end
            .is_some_and(|end| (end - u.position).length() > slack);
        let mut case = w.stall_case.take();
        if case.is_none() && w.reported {
            case = Some(make_case(&sim, &snap, map, w, name, "stall", u.position));
        }
        if idle && short {
            let mut c = make_case(&sim, &snap, map, w, name, "gave up", u.position);
            if let Some(stall) = &case {
                c.stall_s = stall.stall_s;
            }
            case = Some(c);
        } else if !idle && !w.done && w.reported {
            // Still stalled when the run ended.
            let mut c = make_case(&sim, &snap, map, w, name, "timeout", w.anchor);
            c.stall_s = c
                .stall_s
                .max(u32::try_from((sim.tick() - w.anchor_tick) / hz).unwrap_or(0));
            case = Some(c);
        }
        let unfinished = case.is_none() && !idle && !w.done;
        match case {
            Some(c) => {
                stuck_orders[w.order] = true;
                result.stuck_units += 1;
                result.cases.push(c);
            }
            None if unfinished => result.unfinished_units += 1,
            None => result.arrived_units += 1,
        }
    }
    result.stuck_orders += u32::try_from(stuck_orders.iter().filter(|&&s| s).count()).unwrap_or(0);
    Ok(())
}

fn unit_of(snap: &sim::Snapshot, id: UnitId) -> Option<sim::UnitSnapshot> {
    // The snapshot is sorted by id.
    snap.units
        .binary_search_by_key(&id, |u| u.id)
        .ok()
        .map(|i| snap.units[i])
}

/// Updates the watch of one unit after a tick; returns whether it is still
/// busy (has a route).
fn observe(
    sim: &Sim,
    snap: &sim::Snapshot,
    w: &mut Watch,
    tick: u64,
    s: &Settings,
    map_name: &str,
    map: &Map,
) -> bool {
    let hz = u64::from(sim.tick_hz());
    let Some(position) = unit_of(snap, w.id)
        .filter(sim::UnitSnapshot::alive)
        .map(|u| u.position)
    else {
        w.done = true;
        return false;
    };
    let route = sim.route_of(w.id);
    match route {
        Some(RouteView::Following(waypoints)) => {
            w.planned_end = waypoints.last().copied().or(w.planned_end);
            w.was_following = true;
        }
        Some(RouteView::Waiting) => {
            if w.was_following {
                w.replans += 1;
            }
            w.was_following = false;
            // Waiting for a path is no stall.
            w.anchor = position;
            w.anchor_tick = tick;
            return true;
        }
        _ => {
            w.was_following = false;
        }
    }
    if sim.waits_for_group(w.id) {
        // Waiting in front of a bridge for its group (prompt 187) is no
        // stall either.
        w.anchor = position;
        w.anchor_tick = tick;
        return true;
    }
    let busy = matches!(route, Some(RouteView::Following(_)));
    if (position - w.anchor).length() > Fixed::from_num(s.stall_m) || !busy {
        // Moved on (or stopped): a stall that ended is a case.
        if w.reported {
            let stall = tick - w.anchor_tick;
            if stall >= w.worst {
                w.worst = stall;
                w.worst_at = w.anchor;
            }
            if w.stall_case
                .as_ref()
                .is_none_or(|c| u64::from(c.stall_s) * hz <= stall)
            {
                let mut c = make_case(sim, snap, map, w, map_name, "stall", w.anchor);
                c.stall_s = u32::try_from(stall / hz).unwrap_or(0);
                w.stall_case = Some(c);
            }
            w.reported = false;
        }
        w.anchor = position;
        w.anchor_tick = tick;
    } else if tick - w.anchor_tick >= u64::from(s.stall_s) * hz {
        if std::env::var_os("STUCK_TRACE").is_some() && (tick - w.anchor_tick).is_multiple_of(hz) {
            let u = unit_of(snap, w.id);
            eprintln!(
                "trace {map_name} {} {} t{tick} pos {:?} route {:?} stance {:?} spread {}",
                w.route,
                w.id.0,
                position,
                route,
                u.map(|u| u.stance),
                u.map_or(0.0, |u| u.spread.to_num::<f64>())
            );
        }
        w.reported = true;
        let stall = tick - w.anchor_tick;
        if stall > w.worst {
            w.worst = stall;
            w.worst_at = w.anchor;
        }
    }
    if !busy {
        w.done = true;
    }
    busy
}

fn make_case(
    sim: &Sim,
    snap: &sim::Snapshot,
    map: &Map,
    w: &Watch,
    map_name: &str,
    what: &'static str,
    at: Vec2,
) -> Case {
    let hz = u64::from(sim.tick_hz());
    Case {
        map: map_name.to_string(),
        group: w.group.clone(),
        route: w.route.clone(),
        unit_type: w.type_name.clone(),
        at,
        what,
        stall_s: u32::try_from(w.worst / hz).unwrap_or(0),
        cause: cause(sim, snap, map, w.id, w.profile, at),
        replans: w.replans,
        reachable: w.reachable,
    }
}

/// Why a unit may be stuck at `at`: the first that applies of bridge deck,
/// bridgehead, another unit in the way, and the blocked cells around it
/// (object kind, water, bank, slope).
fn cause(
    sim: &Sim,
    snap: &sim::Snapshot,
    map: &Map,
    id: UnitId,
    profile: usize,
    at: Vec2,
) -> String {
    if map.deck_at(at).is_some() {
        return "bridge deck".into();
    }
    let near_end = map.bridges().iter().any(|b| {
        b.intact && {
            let d = at - b.center;
            let along = (d.x * b.forward.x + d.y * b.forward.y).abs();
            let across = (d.x * b.right.x + d.y * b.right.y).abs();
            along <= b.half_length + Fixed::from_num(16)
                && across <= b.half_width + Fixed::from_num(12)
        }
    });
    if near_end {
        return "bridgehead".into();
    }
    let me = snap.units.iter().find(|u| u.id == id);
    let spread = me.map_or(Fixed::from_num(4), |u| u.spread);
    let crowd = snap.units.iter().any(|u| {
        u.id != id
            && u.alive()
            && u.carrier.is_none()
            && (u.position - at).length() < (u.spread + spread) * 3 / 4 + Fixed::ONE
    });
    let grid = sim.path_grid();
    let (cx, cy) = grid.cell_of(at);
    let (w, h) = grid.size();
    let mut reasons: Vec<&str> = Vec::new();
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let (x, y) = (i64::from(cx) + i64::from(dx), i64::from(cy) + i64::from(dy));
            if x < 0 || y < 0 || x >= i64::from(w) || y >= i64::from(h) {
                reasons.push("map edge");
                continue;
            }
            for r in grid.block_reasons(x as u32, y as u32) {
                if !reasons.contains(&r) {
                    reasons.push(r);
                }
            }
        }
    }
    let class = grid.class_of(profile);
    let object =
        |r: &&str| *r == "object" || (*r == "object (vehicles)" && class == SizeClass::Large);
    if reasons.iter().any(object) {
        return format!("object {}", nearest_object(map, at));
    }
    for r in [
        "obstacle",
        "field work",
        "water",
        "bank",
        "slope",
        "map edge",
    ] {
        if reasons.contains(&r) {
            return if r == "obstacle" {
                "wreck".into()
            } else {
                r.into()
            };
        }
    }
    if crowd {
        return "other unit".into();
    }
    "other".into()
}

/// Id of the map object nearest to `at` (kind ids name trees, hedges,
/// buildings).
fn nearest_object(map: &Map, at: Vec2) -> String {
    let kinds = map.object_kinds();
    map.objects()
        .iter()
        .min_by_key(|o| (o.position - at).length())
        .and_then(|o| kinds.get(usize::from(o.kind)))
        .map_or_else(|| "?".to_string(), |k| k.id.clone())
}

/// One AI game on the map (normal against normal, demo battle), every
/// unit watched with the same stall rule while it follows a route.
fn watch_ai_game(
    s: &Settings,
    name: &str,
    map: &Arc<Map>,
    game_index: u64,
    result: &mut MapResult,
) -> Result<(), String> {
    let e = |e: &dyn std::fmt::Display| e.to_string();
    let config = sim::scenario::parse_scenario_with_factions(
        sim::scenario::DEMO_SCENARIO,
        &[],
        &sim::Catalog::builtin().map_err(|x| e(&x))?,
        &sim::Scale::builtin().map_err(|x| e(&x))?,
        &sim::TerrainRules::builtin().map_err(|x| e(&x))?,
        map,
    )
    .map_err(|x| x.0)?;
    let levels: Vec<(PlayerId, Difficulty)> = config
        .players()
        .into_iter()
        .map(|p| (p, Difficulty::Normal))
        .collect();
    let mut game = Game::new(&config, map.clone(), 1 + game_index, &levels)?;
    let hz = u64::from(game.sim().tick_hz());
    let max = u64::from(s.ai.max_s) * hz;
    let mut watches: std::collections::BTreeMap<u32, Watch> = std::collections::BTreeMap::new();
    while game.sim().outcome().is_none() && game.sim().tick() < max {
        let decisions = game.think();
        game.step(decisions);
        let sim = game.sim();
        result.ai_reported += u32::try_from(sim.move_events().len()).unwrap_or(0);
        if std::env::var_os("STUCK_TRACE").is_some() {
            for e in sim.move_events() {
                let sim::MoveEvent::Stuck { unit, .. } = e;
                let u = unit_of(&sim.snapshot(), *unit);
                eprintln!(
                    "ai move event t{}: {e:?} {:?} stance {:?}",
                    sim.tick(),
                    u.map(|u| config.unit_types[u.unit_type as usize].name.clone()),
                    u.map(|u| u.stance)
                );
            }
        }
        let tick = sim.tick();
        let snap = sim.snapshot();
        if let Some(id) = std::env::var("STUCK_AI_UNIT")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            && tick.is_multiple_of(hz)
            && let Some(u) = unit_of(&snap, UnitId(id))
        {
            eprintln!(
                "ai t{} u{id} ({:.1}, {:.1}) {:?} stance {:?} changing {} route {:?}",
                tick / hz,
                u.position.x.to_num::<f64>(),
                u.position.y.to_num::<f64>(),
                u.target.map(|t| (t.x.to_num::<i32>(), t.y.to_num::<i32>())),
                u.stance,
                u.changing_stance,
                match sim.route_of(u.id) {
                    Some(RouteView::Following(p)) => format!(
                        "{} left, next ({:.0},{:.0})",
                        p.len(),
                        p[0].x.to_num::<f64>(),
                        p[0].y.to_num::<f64>()
                    ),
                    other => format!("{other:?}"),
                }
            );
        }
        for u in &snap.units {
            if !u.alive() || u.carrier.is_some() || u.loading || u.changing_stance {
                watches.remove(&u.id.0);
                continue;
            }
            let following = matches!(sim.route_of(u.id), Some(RouteView::Following(_)));
            let w = watches.entry(u.id.0).or_insert_with(|| {
                let mut w = Watch::new(u.id, u.position, 0, "ai", "ai game");
                w.profile = u.unit_type as usize;
                w.type_name.clone_from(&config.unit_types[w.profile].name);
                w.anchor_tick = tick;
                w
            });
            if !following {
                if w.reported {
                    let mut c = make_case(sim, &snap, map, w, name, "stall", w.anchor);
                    c.stall_s = u32::try_from((tick - w.anchor_tick) / hz).unwrap_or(0);
                    result.cases.push(c);
                    result.ai_stuck += 1;
                }
                w.reported = false;
                w.anchor = u.position;
                w.anchor_tick = tick;
                continue;
            }
            if (u.position - w.anchor).length() > Fixed::from_num(s.stall_m) {
                if w.reported {
                    let mut c = make_case(sim, &snap, map, w, name, "stall", w.anchor);
                    c.stall_s = u32::try_from((tick - w.anchor_tick) / hz).unwrap_or(0);
                    result.cases.push(c);
                    result.ai_stuck += 1;
                }
                w.reported = false;
                w.anchor = u.position;
                w.anchor_tick = tick;
            } else if tick - w.anchor_tick >= u64::from(s.stall_s) * hz {
                w.reported = true;
            }
        }
    }
    result.ai_units += u32::try_from(game.sim().snapshot().units.len()).unwrap_or(0);
    Ok(())
}

/// The report: a table per map, the long stalls and the worst places.
pub fn report(settings: &Settings, results: &[MapResult]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "stuck-audit: stall = no {} m of progress for {} s while following a route; gave up = stopped more than {} m (plus own width) short of the planned end",
        settings.stall_m, settings.stall_s, settings.arrive_m
    );
    let _ = writeln!(
        out,
        "{:<20} {:>6} {:>6} {:>7} {:>6} {:>4} {:>6} {:>7} {:>5} {:>8} {:>10}  most common cause",
        "map",
        "orders",
        "units",
        "arrived",
        "on way",
        "lost",
        "stuck",
        "orders%",
        "long",
        "reported",
        "ai s/r/all"
    );
    let (mut orders, mut stuck_orders, mut units, mut stuck, mut on_way, mut lost) =
        (0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
    for r in results {
        let long = r
            .cases
            .iter()
            .filter(|c| c.group != "ai" && (c.stall_s >= settings.long_s || c.what != "stall"))
            .count();
        let pct = f64::from(r.stuck_orders) * 100.0 / f64::from(r.orders.max(1));
        let ai = if r.ai_units > 0 {
            format!("{}/{}/{}", r.ai_stuck, r.ai_reported, r.ai_units)
        } else {
            "-".into()
        };
        let _ = writeln!(
            out,
            "{:<20} {:>6} {:>6} {:>7} {:>6} {:>4} {:>6} {:>6.1}% {:>5} {:>8} {:>10}  {}",
            r.name,
            r.orders,
            r.units,
            r.arrived_units,
            r.unfinished_units,
            r.lost_units,
            r.stuck_units,
            pct,
            long,
            r.reported,
            ai,
            r.top_cause()
        );
        orders += r.orders;
        stuck_orders += r.stuck_orders;
        units += r.units;
        stuck += r.stuck_units;
        on_way += r.unfinished_units;
        lost += r.lost_units;
    }
    let _ = writeln!(
        out,
        "{:<20} {:>6} {:>6} {:>7} {:>6} {:>4} {:>6} {:>6.1}%",
        "all",
        orders,
        units,
        units - stuck - on_way - lost,
        on_way,
        lost,
        stuck,
        f64::from(stuck_orders) * 100.0 / f64::from(orders.max(1))
    );
    let mut worst: Vec<&Case> = results.iter().flat_map(|r| &r.cases).collect();
    worst.sort_by_key(|c| {
        (
            std::cmp::Reverse(u32::from(c.what != "stall")),
            std::cmp::Reverse(c.stall_s),
        )
    });
    let _ = writeln!(out, "worst places:");
    for c in worst.iter().take(10) {
        let _ = writeln!(
            out,
            "  {} {} ({}, {}): {} {} s at ({:.0}, {:.0}), {}, {} re-plans{}",
            c.map,
            c.unit_type,
            c.group,
            c.route,
            c.what,
            c.stall_s,
            c.at.x.to_num::<f64>(),
            c.at.y.to_num::<f64>(),
            c.cause,
            c.replans,
            if c.reachable {
                ""
            } else {
                ", target not reachable"
            }
        );
    }
    out
}

/// Every case as CSV.
pub fn csv(results: &[MapResult]) -> String {
    let mut out =
        String::from("map,group,route,unit_type,what,stall_s,x_m,y_m,cause,replans,reachable\n");
    for c in results.iter().flat_map(|r| &r.cases) {
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{:.1},{:.1},{},{},{}",
            c.map,
            c.group,
            c.route,
            c.unit_type,
            c.what,
            c.stall_s,
            c.at.x.to_num::<f64>(),
            c.at.y.to_num::<f64>(),
            c.cause,
            c.replans,
            c.reachable
        );
    }
    out
}
