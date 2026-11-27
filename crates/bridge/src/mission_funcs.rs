//! `SimHost` functions for missions and the tutorial (prompt 039): starting
//! a mission file, its state for the HUD, and the signals of the player's
//! presentation (camera moved, units selected).

use crate::map_editor::MapEditor;
use crate::{SimHost, ground_to_godot, load_map_dir};
use godot::prelude::*;
use sim::mission::{Level, POINTS_FILE, mission_header, parse_mission_at};
use sim::scenario::{BUILTIN_MAP_NAME, builtin_map};
use sim::{Catalog, Command, Scale, TerrainRules};
use std::path::Path;
use std::sync::Arc;

#[godot_api(secondary)]
impl SimHost {
    /// Starts the mission file `mission_path` (absolute,
    /// `<maps>/<map>/missions/<id>.toml`) in single player, on its map with
    /// the map's `mission_points.toml` and the mission's computer players.
    /// Returns `false` (and logs why) if the mission is invalid.
    #[func]
    fn start_mission(&mut self, seed: i64, mission_path: GString) -> bool {
        self.start_mission_file(seed, &mission_path.to_string(), None, Level::Normal)
    }

    /// Like `start_mission` at a difficulty `level` (`easy`, `normal`,
    /// `hard`; prompt 181): the script's `[difficulty.<level>]` values
    /// apply. Unknown levels are Normal.
    #[func]
    fn start_mission_level(&mut self, seed: i64, mission_path: GString, level: GString) -> bool {
        let level = Level::parse(&level.to_string()).unwrap_or_default();
        self.start_mission_file(seed, &mission_path.to_string(), None, level)
    }

    /// Like `start_mission`, for a script that does not lie next to its map:
    /// `mission_path` (absolute) runs on the map in `map_dir`. For scripted
    /// recordings (`data/trailer/`, prompt 070), which are no missions of
    /// the menu.
    #[func]
    fn start_mission_on(&mut self, seed: i64, mission_path: GString, map_dir: GString) -> bool {
        self.start_mission_file(
            seed,
            &mission_path.to_string(),
            Some(Path::new(&map_dir.to_string())),
            Level::Normal,
        )
    }

    /// Every text of a mission file the player reads (title, briefing,
    /// objectives, messages), the keys of its translations (prompt 043);
    /// empty if the file cannot be read.
    #[func]
    fn mission_texts(mission_path: GString) -> PackedStringArray {
        std::fs::read_to_string(mission_path.to_string())
            .ok()
            .and_then(|text| sim::mission::mission_texts(&text).ok())
            .map(|texts| texts.iter().map(|t| GString::from(t.as_str())).collect())
            .unwrap_or_default()
    }

    /// Header of a mission file for the menu: `ok`, `error`, `id`, `map`,
    /// `title`, `kind` (`mission`, `tutorial`), `order`, `faction`,
    /// `briefing`, `biome`; prompt 190: `seats` (player ids of further
    /// human seats, empty for single-player missions) and `seat_names`.
    #[func]
    fn mission_header(mission_path: GString) -> VarDictionary {
        let path = mission_path.to_string();
        let file = Path::new(&path);
        let id = file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let map = file
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = std::fs::read_to_string(file).map_err(|e| e.to_string());
        let seats = text
            .as_ref()
            .ok()
            .and_then(|t| sim::mission::mission_seats(t).ok())
            .unwrap_or_default();
        let header = text.and_then(|text| mission_header(&text).map_err(|e| e.to_string()));
        let seat_players: PackedInt64Array = seats.iter().map(|(p, _)| i64::from(*p)).collect();
        let seat_names: PackedStringArray = seats
            .iter()
            .map(|(_, n)| GString::from(n.as_str()))
            .collect();
        match header {
            Ok(h) => {
                let mut d = vdict! {
                    "ok" => true,
                    "error" => "",
                    "id" => id,
                    "map" => map,
                    "title" => h.title,
                    "kind" => h.kind,
                    "order" => i64::from(h.order),
                    "faction" => h.faction,
                    "briefing" => h.briefing,
                    "biome" => h.biome,
                };
                d.set("seats", &seat_players);
                d.set("seat_names", &seat_names);
                d
            }
            Err(e) => vdict! { "ok" => false, "error" => e, "id" => id, "map" => map },
        }
    }

    /// State of the running mission: `active` (false without one); with a
    /// mission `id`, `title`, `tutorial`, `player`, `message` and
    /// `message_tick` (-1 without), `camera` (ground point or null) and
    /// `camera_tick`, `objectives` (array of `id`, `text`, `state`
    /// (`hidden`, `active`, `done`, `failed`), `position` (ground point or
    /// null), `radius` (m), `unit` (catalog id or "")), `wanted` (signals
    /// armed triggers wait for), `fired` and `triggers`, `arrivals` (columns
    /// of reinforcements that drove in, oldest first: `owner`, `entry`, `to`
    /// (positions), `tick`; prompt 108), `level` (`easy`, `normal`, `hard`)
    /// and `hopeless_tick` (-1, or when the script said the mission can
    /// hardly be won; prompt 181) with its text `hopeless`. Every objective
    /// also has a `goal` (prompt 180): what completes it, `when` (the
    /// script's condition name, "" if no trigger completes it) with
    /// `unit`, `stance`, `signal`, `area` (ground point or null),
    /// `area_radius`, `group` (living unit ids) and `by` where the
    /// condition has them. Prompt 189: every objective has `optional`,
    /// `timer_s` (seconds until its timer trigger fires, -1 without) and
    /// `progress` / `progress_of` (its count, -1 without); `rating` is
    /// empty until the player won, then `stars` (0..3), `in_time`,
    /// `few_losses`, `optional_done`, `lost`, `units`, `optional_won`,
    /// `optional_total`, `par_s` and `losses_pct`.
    #[func]
    fn mission_info(&self) -> VarDictionary {
        let Some(game) = &self.game else {
            return vdict! { "active" => false };
        };
        let sim = &game.sim;
        let Some(mission) = sim.mission() else {
            return vdict! { "active" => false };
        };
        let map = game.map();
        let mut objectives = VarArray::new();
        for (k, o) in sim.mission_objectives().into_iter().enumerate() {
            let mut d = vdict! {
                "id" => o.id.as_str(),
                "text" => o.text.as_str(),
                "state" => o.state.name(),
                "radius" => o.area.map_or(0.0, |(_, r)| r.to_num::<f32>()),
                "unit" => o.unit.clone().unwrap_or_default(),
            };
            match o.area {
                Some((c, _)) => d.set("position", ground_to_godot(c, map.height_at(c))),
                None => d.set("position", &Variant::nil()),
            }
            d.set("goal", &goal_of(sim, mission, k, map));
            // Prompt 189: side objective, timer, count.
            let hz = u64::from(sim.tick_hz().max(1));
            d.set("optional", o.optional);
            d.set(
                "timer_s",
                o.timer_ticks
                    .map_or(-1, |t| i64::try_from(t.div_ceil(hz)).unwrap_or(i64::MAX)),
            );
            d.set("progress", o.progress.map_or(-1, |(n, _)| i64::from(n)));
            d.set("progress_of", o.progress.map_or(-1, |(_, n)| i64::from(n)));
            objectives.push(&d.to_variant());
        }
        let (message, message_tick) = sim
            .mission_message()
            .map_or((String::new(), -1), |(t, tick)| {
                (t.to_owned(), i64::try_from(tick).unwrap_or(i64::MAX))
            });
        let mut info = vdict! {
            "active" => true,
            "id" => mission.id.as_str(),
            "title" => mission.title.as_str(),
            "tutorial" => mission.tutorial,
            "player" => i64::from(mission.player.0),
            "message" => message,
            "message_tick" => message_tick,
            "level" => mission.level.name(),
            "hopeless" => sim.mission_hopeless().map_or(String::new(), |(t, _)| t.to_owned()),
            "hopeless_tick" => sim
                .mission_hopeless()
                .map_or(-1, |(_, tick)| i64::try_from(tick).unwrap_or(i64::MAX)),
            "fired" => sim.mission_progress().0 as i64,
            "triggers" => sim.mission_progress().1 as i64,
        };
        info.set("objectives", &objectives);
        match sim.mission_camera() {
            Some((p, tick)) => {
                info.set("camera", ground_to_godot(p, map.height_at(p)));
                info.set("camera_tick", i64::try_from(tick).unwrap_or(i64::MAX));
            }
            None => {
                info.set("camera", &Variant::nil());
                info.set("camera_tick", -1i64);
            }
        }
        let wanted: PackedStringArray = sim
            .mission_wanted_signals()
            .into_iter()
            .map(GString::from)
            .collect();
        info.set("wanted", &wanted);
        // Prompt 108: columns of reinforcements that drove in.
        let mut arrivals = VarArray::new();
        for a in sim.mission_arrivals() {
            let d = vdict! {
                "owner" => i64::from(a.owner.0),
                "entry" => ground_to_godot(a.entry, map.height_at(a.entry)),
                "to" => ground_to_godot(a.to, map.height_at(a.to)),
                "tick" => i64::try_from(a.tick).unwrap_or(i64::MAX),
            };
            arrivals.push(&d.to_variant());
        }
        info.set("arrivals", &arrivals);
        // Prompt 189: the rating once the mission is won.
        let mut rating = VarDictionary::new();
        if let Some(r) = sim.mission_rating() {
            let hz = u64::from(sim.tick_hz().max(1));
            rating.set("stars", i64::from(r.stars));
            rating.set("in_time", r.in_time);
            rating.set("few_losses", r.few_losses);
            rating.set("optional_done", r.optional_done);
            rating.set("lost", i64::from(r.lost));
            rating.set("units", i64::from(r.units));
            rating.set("optional_won", i64::from(r.optional.0));
            rating.set("optional_total", i64::from(r.optional.1));
            rating.set(
                "par_s",
                i64::try_from(mission.rating.par_ticks / hz).unwrap_or(0),
            );
            rating.set("losses_pct", i64::from(mission.rating.losses_pct));
        }
        info.set("rating", &rating);
        info
    }

    /// A campaign file (`data/campaigns/<id>.toml`, prompt 189) for the
    /// menu: `ok`, `error`, `id`, `title`, `briefing`, `order` and
    /// `missions` (array of `id`, `map`, `intro`, `outro` in play order).
    #[func]
    fn campaign_header(campaign_path: GString) -> VarDictionary {
        let path = campaign_path.to_string();
        let id = Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parsed = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| sim::campaign::Campaign::parse(&t).map_err(|e| e.to_string()));
        match parsed {
            Ok(c) => {
                let mut missions = VarArray::new();
                for m in &c.missions {
                    let d = vdict! {
                        "id" => m.id.as_str(),
                        "map" => m.map.as_str(),
                        "intro" => m.intro.as_str(),
                        "outro" => m.outro.as_str(),
                        "date_label" => m.date_label.as_str(),
                        "tagline" => m.tagline.as_str(),
                    };
                    missions.push(&d.to_variant());
                }
                let mut d = vdict! {
                    "ok" => true,
                    "error" => "",
                    "id" => id,
                    "title" => c.campaign.title,
                    "briefing" => c.campaign.briefing,
                    "order" => i64::from(c.campaign.order),
                };
                d.set("missions", &missions);
                d
            }
            Err(e) => vdict! { "ok" => false, "error" => e, "id" => id },
        }
    }

    /// Reports `name` (`camera`, `select`) to the mission script of the
    /// running match (prompt 039); see `docs/MISSIONS.md`.
    #[func]
    fn mission_signal(&mut self, name: GString) {
        if self.game.is_some() {
            self.pending.push(Command::MissionSignal {
                name: name.to_string(),
            });
        }
    }
}

/// Whole meters of a Godot coordinate (presentation input).
fn whole_m(v: f32) -> i32 {
    let r = v.round();
    if r.is_finite() {
        r.clamp(-1.0e6, 1.0e6) as i32
    } else {
        0
    }
}

/// Map editor (prompt 028): trigger areas and spawn points for mission
/// scripts (prompt 039), saved as the map's `mission_points.toml`.
#[godot_api(secondary)]
impl MapEditor {
    /// Adds a trigger area of `radius` m around `(x, z)`; returns its name
    /// (`name` if free and valid, else `area_<n>`).
    #[func]
    fn add_mission_area(&mut self, x: f32, z: f32, radius: f32, name: GString) -> GString {
        let radius = u32::try_from(whole_m(radius).max(1)).unwrap_or(1);
        GString::from(
            editor::mission_points::add_area(
                &mut self.points,
                [whole_m(x), whole_m(z)],
                radius,
                &name.to_string(),
            )
            .as_str(),
        )
    }

    /// Adds a spawn point at `(x, z)`; returns its name.
    #[func]
    fn add_mission_spawn(&mut self, x: f32, z: f32, name: GString) -> GString {
        GString::from(
            editor::mission_points::add_spawn(
                &mut self.points,
                [whole_m(x), whole_m(z)],
                &name.to_string(),
            )
            .as_str(),
        )
    }

    /// Removes the areas containing `(x, z)` and the spawn points within
    /// `pick` m; returns how many.
    #[func]
    fn remove_mission_points(&mut self, x: f32, z: f32, pick: f32) -> i64 {
        let pick = u32::try_from(whole_m(pick).max(0)).unwrap_or(0);
        i64::try_from(editor::mission_points::remove_near(
            &mut self.points,
            [whole_m(x), whole_m(z)],
            pick,
        ))
        .unwrap_or(0)
    }

    /// The areas (`name`, `center` as a ground point, `radius`) and spawn
    /// points (`name`, `position`; `radius` 0), areas first.
    #[func]
    fn mission_points(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        for a in &self.points.area {
            let (x, z) = (a.center_m[0] as f32, a.center_m[1] as f32);
            out.push(&vdict! {
                "kind" => "area",
                "name" => a.name.as_str(),
                "position" => Vector3::new(x, self.height_at(x, z), z),
                "radius" => a.radius_m as f32,
            });
        }
        for s in &self.points.spawn {
            let (x, z) = (s.position_m[0] as f32, s.position_m[1] as f32);
            out.push(&vdict! {
                "kind" => "spawn",
                "name" => s.name.as_str(),
                "position" => Vector3::new(x, self.height_at(x, z), z),
                "radius" => 0.0f32,
            });
        }
        out
    }
}

impl SimHost {
    /// Starts the mission file `path` on the map in `map_dir`, or on the map
    /// the file lies next to (`<map>/missions/<id>.toml`).
    fn start_mission_file(
        &mut self,
        seed: i64,
        path: &str,
        map_dir: Option<&Path>,
        level: Level,
    ) -> bool {
        let result = (|| -> Result<_, String> {
            let file = Path::new(path);
            let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
            let id = file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let map_dir = match map_dir {
                Some(dir) => dir,
                // <map>/missions/<id>.toml
                None => file
                    .parent()
                    .and_then(Path::parent)
                    .ok_or("a mission lies in <map>/missions/")?,
            };
            let map_name = map_dir
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let map = if map_name == BUILTIN_MAP_NAME {
                builtin_map()
            } else {
                Arc::new(load_map_dir(map_dir)?)
            };
            let points = std::fs::read_to_string(map_dir.join(POINTS_FILE)).ok();
            let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
            let scale = Scale::builtin().map_err(|e| e.to_string())?;
            let terrain = TerrainRules::builtin().map_err(|e| e.to_string())?;
            let config = parse_mission_at(
                &id,
                &text,
                points.as_deref(),
                &catalog,
                &scale,
                &terrain,
                &map,
                level,
            )
            .map_err(|e| e.to_string())?;
            Ok((map, config))
        })();
        let (map, config) = match result {
            Ok(started) => started,
            Err(e) => {
                godot_error!("cannot start mission {path}: {e}");
                return false;
            }
        };
        let mission = config.mission.clone().unwrap_or_else(|| unreachable!());
        self.computers.clear();
        for (player, difficulty) in &mission.computers {
            match difficulty.parse() {
                Ok(d) => self.computers.push((*player, d)),
                Err(e) => {
                    godot_error!("{e}; player {} gets no orders", player.0);
                }
            }
        }
        // Players without a computer give no orders (training targets).
        self.ai_opponent = GString::new();
        self.start_config(seed, map, config)
    }
}

/// The `goal` of objective `k` for `mission_info` (prompt 180).
fn goal_of(
    sim: &sim::Sim,
    mission: &sim::mission::Mission,
    k: usize,
    map: &sim::Map,
) -> VarDictionary {
    use sim::mission::Condition;
    let mut d = VarDictionary::new();
    let Some(c) = sim.mission_goal(k) else {
        d.set("when", "");
        return d;
    };
    d.set("when", c.name());
    let area = |d: &mut VarDictionary, a: Option<u16>| {
        if let Some(a) = a.and_then(|a| mission.areas.get(usize::from(a))) {
            d.set("area", ground_to_godot(a.center, map.height_at(a.center)));
            d.set("area_radius", a.radius.to_num::<f32>());
        }
    };
    let group = |d: &mut VarDictionary, g: u16| {
        let name = mission
            .groups
            .get(usize::from(g))
            .cloned()
            .unwrap_or_default();
        let ids: PackedInt64Array = sim
            .mission_group(&name)
            .into_iter()
            .filter(|&id| sim.unit_alive(id))
            .map(|id| i64::from(id.0))
            .collect();
        d.set("group", &ids);
    };
    let stance_name = |s: sim::Stance| match s {
        sim::Stance::Stand => "stand",
        sim::Stance::Kneel => "kneel",
        sim::Stance::Prone => "prone",
    };
    match c {
        Condition::Area { area: a, unit, .. } => {
            area(&mut d, Some(*a));
            d.set("unit", unit.clone().unwrap_or_default());
        }
        Condition::Destroyed { group: g } | Condition::Pinned { group: g, .. } => group(&mut d, *g),
        Condition::Spotted { group: g, .. } => group(&mut d, *g),
        Condition::Signal { name } => d.set("signal", name.as_str()),
        Condition::Units { unit, .. } => d.set("unit", unit.as_str()),
        Condition::Stance {
            unit,
            area: a,
            stance,
            ..
        } => {
            area(&mut d, *a);
            d.set("unit", unit.clone().unwrap_or_default());
            d.set("stance", stance_name(*stance));
        }
        Condition::Garrison { unit, area: a, .. } => {
            area(&mut d, *a);
            d.set("unit", unit.clone().unwrap_or_default());
        }
        Condition::Unseen {
            unit, area: a, by, ..
        } => {
            area(&mut d, *a);
            d.set("unit", unit.clone().unwrap_or_default());
            d.set("by", i64::from(by.0));
        }
        Condition::Smoke { area: a } | Condition::BridgeDown { area: a } => area(&mut d, Some(*a)),
        Condition::Captured { point, .. } => d.set("point", point.as_str()),
        Condition::Reach { area: a, unit, .. } => {
            area(&mut d, Some(*a));
            d.set("unit", unit.clone().unwrap_or_default());
        }
        _ => {}
    }
    d
}
