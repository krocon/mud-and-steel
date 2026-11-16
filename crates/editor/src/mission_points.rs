//! Trigger areas and spawn points for mission scripts (prompt 039): the
//! map's `mission_points.toml`, edited with the map editor's "Trigger
//! area" and "Spawn point" tools. Whole meters, like start positions.
//!
//! The points are kept apart from the map files (and from undo): they do
//! not change the map's content hash, and the mission scripts that use them
//! are written by hand next to the map (`missions/<id>.toml`).

use sim::mission::{AreaEntry, MissionPoints, POINTS_FILE, SpawnEntry};
use std::path::Path;

/// The points of the map folder `dir`; none if it has no file.
pub fn load_points(dir: &Path) -> Result<MissionPoints, String> {
    let path = dir.join(POINTS_FILE);
    if !path.exists() {
        return Ok(MissionPoints::default());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    MissionPoints::parse(&text).map_err(|e| e.to_string())
}

/// Writes the points into `dir`; without points an existing file is
/// removed.
pub fn save_points(points: &MissionPoints, dir: &Path) -> Result<(), String> {
    let path = dir.join(POINTS_FILE);
    if points.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        return Ok(());
    }
    std::fs::write(&path, points.to_toml()).map_err(|e| format!("{}: {e}", path.display()))
}

/// `wanted` if it is a usable, unused name, else `<prefix>_<n>` with the
/// smallest unused `n`.
fn free_name(points: &MissionPoints, wanted: &str, prefix: &str) -> String {
    let used = |name: &str| {
        points.area.iter().any(|a| a.name == name) || points.spawn.iter().any(|s| s.name == name)
    };
    let wanted = wanted.trim();
    let valid = !wanted.is_empty()
        && wanted
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if valid && !used(wanted) {
        return wanted.to_owned();
    }
    (1..)
        .map(|n| format!("{prefix}_{n}"))
        .find(|name| !used(name))
        .unwrap_or_default()
}

/// Adds a trigger area; returns its name.
pub fn add_area(
    points: &mut MissionPoints,
    center_m: [i32; 2],
    radius_m: u32,
    name: &str,
) -> String {
    let name = free_name(points, name, "area");
    points.area.push(AreaEntry {
        name: name.clone(),
        center_m,
        radius_m: radius_m.max(1),
    });
    name
}

/// Adds a spawn point; returns its name.
pub fn add_spawn(points: &mut MissionPoints, position_m: [i32; 2], name: &str) -> String {
    let name = free_name(points, name, "spawn");
    points.spawn.push(SpawnEntry {
        name: name.clone(),
        position_m,
    });
    name
}

/// Removes the areas that contain the point and the spawn points within
/// `pick_m` of it; returns how many.
pub fn remove_near(points: &mut MissionPoints, at_m: [i32; 2], pick_m: u32) -> usize {
    let d2 = |p: [i32; 2]| {
        let (dx, dy) = (
            i64::from(p[0]) - i64::from(at_m[0]),
            i64::from(p[1]) - i64::from(at_m[1]),
        );
        dx * dx + dy * dy
    };
    let before = points.area.len() + points.spawn.len();
    points
        .area
        .retain(|a| d2(a.center_m) > i64::from(a.radius_m).pow(2));
    points
        .spawn
        .retain(|s| d2(s.position_m) > i64::from(pick_m).pow(2));
    before - points.area.len() - points.spawn.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_clicks_remove() {
        let mut p = MissionPoints::default();
        assert_eq!(add_area(&mut p, [0, 0], 30, ""), "area_1");
        assert_eq!(add_area(&mut p, [100, 0], 20, "bridge"), "bridge");
        assert_eq!(add_area(&mut p, [200, 0], 20, "bridge"), "area_2");
        assert_eq!(add_spawn(&mut p, [50, 50], "bad name!"), "spawn_1");
        assert_eq!(remove_near(&mut p, [10, 10], 5), 1, "inside area_1");
        assert_eq!(remove_near(&mut p, [52, 49], 5), 1, "the spawn");
        assert_eq!(p.area.len(), 2);
        assert!(p.spawn.is_empty());
    }

    #[test]
    fn saving_without_points_removes_the_file() {
        let dir = std::env::temp_dir().join(format!("ms-points-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut p = MissionPoints::default();
        add_spawn(&mut p, [1, 2], "s");
        save_points(&p, &dir).unwrap();
        assert_eq!(load_points(&dir).unwrap(), p);
        save_points(&MissionPoints::default(), &dir).unwrap();
        assert!(!dir.join(POINTS_FILE).exists());
        assert!(load_points(&dir).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
