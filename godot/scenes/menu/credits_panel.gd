extends PanelContainer
## Credits and licenses of the main menu (prompt 044).
##
## Tabs: the game's credits (`CREDITS.md`), its license (`LICENSE.txt`), the
## third-party licenses of the Rust crates and the Steamworks library
## (`THIRD_PARTY_LICENSES.txt`, made by `tools/release/third_party_licenses.py`)
## and Godot's license with its third-party components (from the engine
## itself). An exported game has the files in `res://licenses/` (copied there
## by the release script); in the repository they are read from the root.

signal closed

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Language := preload("res://scenes/menu/language.gd")

const LICENSE_DIR := "res://licenses"
## Tab title keys (also the tab node names) with their files.
const FILES: Dictionary[String, String] = {
	"CREDITS_TAB_CREDITS": "CREDITS.md",
	"CREDITS_TAB_LICENSE": "LICENSE.txt",
	"CREDITS_TAB_THIRD_PARTY": "THIRD_PARTY_LICENSES.txt",
}

var tabs: TabContainer
var back_button: Button


func _ready() -> void:
	custom_minimum_size = Vector2(900, 620)
	var box := VBoxContainer.new()
	add_child(box)
	box.add_child(MenuTheme.title("MENU_CREDITS", MenuTheme.HEADING_SIZE))
	tabs = TabContainer.new()
	tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	box.add_child(tabs)
	for title in FILES:
		_add_tab(title, read_text(FILES[title]))
	_add_tab("CREDITS_TAB_GODOT", godot_licenses())
	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(buttons)
	back_button = Button.new()
	back_button.text = "COMMON_BACK"
	back_button.custom_minimum_size = Vector2(140, 44)
	back_button.pressed.connect(func() -> void: closed.emit())
	buttons.add_child(back_button)


## Text of a license file, from the export or the repository root.
static func read_text(file_name: String) -> String:
	var exported := LICENSE_DIR.path_join(file_name)
	if FileAccess.file_exists(exported):
		return FileAccess.get_file_as_string(exported)
	var repo := ProjectSettings.globalize_path("res://..").path_join(file_name).simplify_path()
	if not GamePaths.is_exported() and FileAccess.file_exists(repo):
		return FileAccess.get_file_as_string(repo)
	if file_name == FILES["CREDITS_TAB_THIRD_PARTY"]:
		return Language.t("CREDITS_THIRD_PARTY_MISSING")
	return Language.t("CREDITS_FILE_MISSING") % file_name


## Godot's license, copyright notices of its components and their license texts.
static func godot_licenses() -> String:
	var lines: PackedStringArray = [
		Language.t("CREDITS_GODOT_LICENSE"), "",
		Engine.get_license_text(), "", Language.t("CREDITS_GODOT_COMPONENTS"), "",
	]
	for info: Dictionary in Engine.get_copyright_info():
		lines.append(info["name"])
		for part: Dictionary in info["parts"]:
			for line: String in part["copyright"]:
				lines.append("    © " + line)
			lines.append("    " + Language.t("CREDITS_LICENSE_LINE") % part["license"])
		lines.append("")
	var licenses := Engine.get_license_info()
	for license_name: String in licenses:
		lines.append("--- %s ---" % license_name)
		lines.append(licenses[license_name])
		lines.append("")
	return "\n".join(lines)


func _add_tab(title: String, text: String) -> void:
	var view := TextEdit.new()
	view.name = title
	view.text = text
	view.editable = false
	view.wrap_mode = TextEdit.LINE_WRAPPING_BOUNDARY
	view.size_flags_vertical = Control.SIZE_EXPAND_FILL
	tabs.add_child(view)
