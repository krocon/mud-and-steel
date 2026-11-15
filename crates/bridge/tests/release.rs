//! Release packaging checks (prompt 044, docs/RELEASE.md).

const PROJECT: &str = include_str!("../../../godot/project.godot");
const EXTENSION: &str = include_str!("../../../godot/mud_and_steel.gdextension");
const PRESETS: &str = include_str!("../../../godot/export_presets.cfg");

/// The version has one source, Cargo.toml; project.godot (shown by Godot and
/// used by the export presets) must repeat it.
#[test]
fn godot_version_matches_cargo() {
    let line = format!("config/version=\"{}\"", env!("CARGO_PKG_VERSION"));
    assert!(
        PROJECT.lines().any(|l| l == line),
        "godot/project.godot needs `{line}` (the version of Cargo.toml)"
    );
}

/// Exported games load the staged release libraries and ship steam_api.
#[test]
fn release_libraries_come_from_target_export() {
    for platform in [
        "macos.release",
        "linux.release.x86_64",
        "windows.release.x86_64",
    ] {
        let library = EXTENSION
            .lines()
            .find(|l| l.starts_with(&format!("{platform} = \"")))
            .unwrap_or_else(|| panic!("no library for {platform}"));
        assert!(library.contains("res://../target/export/"), "{library}");
        let dependency = EXTENSION
            .lines()
            .filter(|l| l.starts_with(&format!("{platform} = {{")))
            .collect::<Vec<_>>();
        assert!(
            dependency.len() == 1
                && dependency[0].contains("target/export/")
                && dependency[0].contains("steam_api"),
            "{platform} must ship steam_api from target/export/: {dependency:?}"
        );
    }
}

/// One preset per system, with the names tools/release/export.sh uses.
#[test]
fn export_presets_exist() {
    for name in ["macOS", "Windows", "Linux"] {
        assert!(
            PRESETS.contains(&format!("name=\"{name}\"")),
            "preset {name} missing"
        );
    }
    assert_eq!(PRESETS.matches("include_filter=\"licenses/*\"").count(), 3);
}
