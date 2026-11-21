extends VBoxContainer
## Battle report on the result screen (prompt 179): why the match went the
## way it did, from `SimHost.match_stats()` (`losses`, `report`, `samples`).
## Presentation only.
##
## - Loss map: the map's square with a dot per own loss, colored by cause.
## - "What cost you most": the three costliest groups of losses (cause and
##   shooter type), with how many came from a shooter never seen and how
##   many vehicles were hit from the side or rear, and a hint from the
##   table `data/report_hints.toml` (`REPORT_HINT_*`).
## - Strength: living units of both sides from the 30-s samples; the
##   biggest own drops are buttons that open the replay viewer there.
##
## Shooters the player never saw are named: the report is shown after the
## match (decision of prompt 179; ARCHITECTURE.md).

const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const GameFiles := preload("res://scenes/menu/game_files.gd")

## Dot colors by cause; distinguishable in the color-weakness palettes by
## brightness as well as hue.
const CAUSE_COLORS := {
	"direct": Color("e8544a"), "indirect": Color("f2b33d"), "mine": Color("b06ad9"),
	"air": Color("5fb7e8"), "overrun": Color("f4f4f4"), "water": Color("3d6fe0"),
	"other": Color("8a8a8a"),
}
## Own drops shown at most.
const MAX_DROPS := 3

var main: Node3D
## Lines shown (self-test).
var lines: Array[String] = []
var drops: Array[int] = []

var _map: LossMap
var _text: Label
var _drops: HBoxContainer


func setup(main_node: Node3D) -> void:
	main = main_node
	name = "Report"
	_map = LossMap.new()
	add_child(_map)
	_text = MenuTheme.hint("")
	add_child(_text)
	_drops = HBoxContainer.new()
	add_child(_drops)


## Fills the report for player `me`.
func show_for(me: int) -> void:
	lines.clear()
	drops.clear()
	for child in _drops.get_children():
		child.queue_free()
	var stats: Array = main.sim_host.match_stats()
	var own: Dictionary = {}
	for player: Dictionary in stats:
		if int(player["id"]) == me:
			own = player
	if own.is_empty():
		_text.text = ""
		return
	_map.map_size = main.sim_host.map_size()
	_map.losses = own.get("losses", [])
	_map.colors = CAUSE_COLORS
	_map.queue_redraw()
	lines.append(tr("REPORT_COST_TITLE"))
	var report: Array = own.get("report", [])
	if report.is_empty():
		lines.append(tr("REPORT_NO_LOSSES"))
	for row: Dictionary in report:
		var line := tr("REPORT_LINE").format({
			"count": row["count"],
			"cause": tr("REPORT_CAUSE_" + str(row["cause"]).to_upper()),
			"by": (tr("REPORT_BY") % row["by"]) if not str(row["by"]).is_empty() else "",
		})
		var extra: PackedStringArray = []
		if int(row["unseen"]) > 0:
			extra.append(tr("REPORT_UNSEEN") % int(row["unseen"]))
		if int(row["flank"]) > 0:
			extra.append(tr("REPORT_FLANK") % int(row["flank"]))
		if not extra.is_empty():
			line += " (" + ", ".join(extra) + ")"
		lines.append(line)
		if not str(row["hint"]).is_empty():
			lines.append("    → " + tr(str(row["hint"])))
	# Biggest own drops of living units between two samples.
	var samples: Array = own.get("samples", [])
	var falls: Array = []
	for k in range(1, samples.size()):
		var lost := int(samples[k - 1]["units"]) - int(samples[k]["units"])
		if lost > 0:
			falls.append([lost, int(samples[k - 1]["seconds"])])
	falls.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0] or (a[0] == b[0] and a[1] < b[1]))
	if not falls.is_empty():
		lines.append(tr("REPORT_DROPS"))
	for fall: Array in falls.slice(0, MAX_DROPS):
		var seconds: int = fall[1]
		drops.append(seconds)
		var b := Button.new()
		b.text = tr("REPORT_DROP") % [seconds / 60, seconds % 60, fall[0]]
		b.tooltip_text = tr("REPORT_DROP_TIP")
		b.pressed.connect(_watch.bind(seconds, me))
		_drops.add_child(b)
	_text.text = "\n".join(lines)


## Saves the match as a replay and opens the viewer `seconds` into it.
func _watch(seconds: int, me: int) -> void:
	var path: String = main.save_replay()
	if path.is_empty() or not GameFiles.open_replay(path).is_empty():
		return
	GameFiles.watch_replay(seconds * 20, me)
	main.get_tree().change_scene_to_file("res://scenes/main.tscn")


## The map's square with the losses as dots.
class LossMap:
	extends Control

	var map_size := Vector2(1000, 1000)
	var losses: Array = []
	var colors: Dictionary = {}

	func _init() -> void:
		custom_minimum_size = Vector2(160, 160)

	func _draw() -> void:
		var side := minf(size.x, size.y)
		var area := Rect2(Vector2((size.x - side) * 0.5, 0), Vector2(side, side))
		draw_rect(area, Color(0.12, 0.14, 0.12, 0.85))
		draw_rect(area, Color(1, 1, 1, 0.25), false, 1.0)
		for l: Dictionary in losses:
			var u := Vector2(float(l["x"]) / maxf(map_size.x, 1.0) + 0.5, float(l["z"]) / maxf(map_size.y, 1.0) + 0.5)
			var p := area.position + u.clamp(Vector2.ZERO, Vector2.ONE) * side
			var color: Color = colors.get(str(l["cause"]), Color.GRAY)
			draw_circle(p, 4.0, Color(0, 0, 0, 0.7))
			draw_circle(p, 3.0, color)
