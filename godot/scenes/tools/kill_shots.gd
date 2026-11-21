extends Node
## Kill sequences in pictures (prompt 105), started by `main.gd`:
##
##     godot --path godot --fixed-fps 30 -- --scenario=kills.toml --kill-shots=<dir> [--kill-prefix=after]
##
## With the fog of war off it destroys the tank, the halftrack, the truck and
## the towed gun of `kills.toml` one after the other, for the presentation
## only (a `destroyed` combat event and the wreck tint; the units stay alive
## in the simulation, so no crew bails out), and saves every second frame of
## KILL_SECONDS per vehicle as `<prefix>_<n>_<kind>/frame_<k>.png`, plus a
## still at fixed times (`<prefix>_<kind>_<ms>.png`). The last vehicle also
## gets a hit that bounces off (prompt 105, ricochet tracer) before it is
## destroyed. `just kill-shots` turns the frames into
## `<prefix>_kills.mp4` with ffmpeg.
##
## Before the kills it saves `<prefix>_marks_ingame.png` (the four vehicles
## with made-up states, so every kind of state mark shows over a unit) and
## `<prefix>_marks_sheet.png` (every state mark large with its meaning).

const SHOT_SIZE := Vector2i(1280, 720)
const HudIcons := preload("res://scenes/hud/hud_icons.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const StateMarks := preload("res://scenes/hud/state_marks.gd")
## Made-up states of the marks picture, per vehicle kind.
const SHOW_STATES: Dictionary[String, Array] = {
	"tank": ["immobile", "track"], "halftrack": ["burning", "suppressed"],
	"truck": ["no_fuel", "service"], "gun": ["no_ammo"],
}
const SHEET_TEXT: Dictionary[String, String] = {
	"burning": "Burning / brennt", "immobile": "Immobilized / fahrunfähig", "track": "Track off / Kette ab",
	"suppressed": "Suppressed / niedergehalten", "no_ammo": "Out of ammunition / ohne Munition",
	"no_fuel": "Out of fuel / ohne Sprit", "service": "Being repaired or resupplied / wird versorgt",
}
## Real seconds recorded per kill (at --fixed-fps 30).
const KILL_SECONDS := 7.0
## Stills per kill, ms after the destroyed event.
const STILLS_MS: Array[int] = [100, 600, 1500, 3000, 5000]
const DISTANCE := 34.0
const KINDS: Dictionary[String, String] = {
	"de_pz4_h": "tank", "de_sdkfz251_1": "halftrack", "de_opel_blitz": "truck", "de_pak40": "gun",
}

var main: Node3D
var _dir := ""
var _prefix := "kill"


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--kill-shots="):
			_dir = arg.trim_prefix("--kill-shots=")
		if arg.begins_with("--kill-prefix="):
			_prefix = arg.trim_prefix("--kill-prefix=")
	await _shots()
	get_tree().quit()


func _shots() -> void:
	DirAccess.make_dir_recursive_absolute(_dir)
	get_window().size = SHOT_SIZE
	main.effects.shake_enabled = false
	main.fog_of_war.set_fog_enabled(false)
	# No ground tooltip in the pictures.
	Settings.values["ground_tooltip"] = "off"
	await _frames(20)
	var sim_host: SimHost = main.sim_host
	var ids := sim_host.unit_ids()
	var types := sim_host.unit_type_ids()
	await _marks(ids, types)
	var n := 0
	for i in ids.size():
		var kind: String = KINDS.get(types[i], "")
		if kind.is_empty():
			continue
		var id := ids[i]
		var node: Node3D = main._unit_nodes[id]
		var at := node.global_position
		main.camera_rig.snap_to(at, DISTANCE, node.rotation.y + PI * 0.6)
		await _frames(30)
		n += 1
		var folder := "%s/%s_%d_%s" % [_dir, _prefix, n, kind]
		DirAccess.make_dir_recursive_absolute(folder)
		if kind == "gun":
			# A round that bounces off the shield first (ricochet tracer).
			var from := at + Vector3(-60.0, 1.5, -40.0)
			main.combat_view.combat_event.emit({"type": "hit", "projectile": -1, "shooter": -1,
				"position": at + Vector3.UP * 1.0, "surface": "unit", "target": id, "face": "front",
				"turret": false, "penetrated": false, "damage": 0, "caliber_mm": 45.0, "from": from,
				"tick": sim_host.current_tick()})
			await _record(folder, 0, 20, kind, false)
		var tick := sim_host.current_tick()
		main.combat_view.combat_event.emit({"type": "hit", "projectile": -1, "shooter": -1,
			"position": at + Vector3.UP * 1.2, "surface": "unit", "target": id, "face": "side",
			"turret": false, "penetrated": true, "damage": 100, "caliber_mm": 75.0, "tick": tick})
		main.combat_view.combat_event.emit({"type": "destroyed", "unit": id, "by": -1, "position": at, "tick": tick})
		main.combat_view._make_wreck(node, false)
		await _record(folder, 20, int(KILL_SECONDS * 30.0), kind, true)
	print("kill-shots: %d kills recorded in %s" % [n, ProjectSettings.globalize_path(_dir)])


## The state marks over the four vehicles (made-up states) and the sheet.
func _marks(ids: PackedInt64Array, types: PackedStringArray) -> void:
	var marks: Control = main.combat_view.state_marks
	var center := Vector3.ZERO
	var count := 0
	for i in ids.size():
		var kind: String = KINDS.get(types[i], "")
		if not kind.is_empty():
			center += (main._unit_nodes[ids[i]] as Node3D).global_position
			count += 1
	main.camera_rig.snap_to(center / maxi(count, 1), 70.0, 0.0)
	main.selection.set_selection([])
	await _frames(20)
	marks._scan_left = 1000.0
	for i in ids.size():
		var kind: String = KINDS.get(types[i], "")
		if not kind.is_empty():
			marks.states[ids[i]] = SHOW_STATES[kind]
	await _frames(3)
	get_viewport().get_texture().get_image().save_png("%s/%s_marks_ingame.png" % [_dir, _prefix])
	marks._scan_left = 0.0
	var layer := CanvasLayer.new()
	layer.layer = 100
	var sheet := Control.new()
	sheet.set_anchors_preset(Control.PRESET_FULL_RECT)
	sheet.draw.connect(func() -> void: _draw_sheet(sheet))
	layer.add_child(sheet)
	add_child(layer)
	await _frames(3)
	get_viewport().get_texture().get_image().save_png("%s/%s_marks_sheet.png" % [_dir, _prefix])
	layer.queue_free()
	await _frames(2)


func _draw_sheet(sheet: Control) -> void:
	sheet.draw_rect(Rect2(Vector2.ZERO, sheet.size), Color(0.13, 0.14, 0.12))
	var font := ThemeDB.fallback_font
	var y := 60.0
	for kind: String in StateMarks.ORDER:
		for k in 2:
			# Large and at the size of the game.
			var size := 48.0 if k == 0 else StateMarks.SIZE
			var rect := Rect2(Vector2(80.0 + k * 90.0, y + (48.0 - size) * 0.5), Vector2(size, size))
			sheet.draw_circle(rect.get_center(), size * 0.5 + size * 0.1, Color(0, 0, 0, 0.62))
			HudIcons.draw(sheet, "state_" + kind, rect.grow(-size * 0.1), StateMarks.COLORS[kind], 1.4 * size / StateMarks.SIZE)
		var enemy := "  (also over enemies)" if kind in StateMarks.ENEMY_MARKS else "  (own units only)"
		sheet.draw_string(font, Vector2(260.0, y + 32.0), SHEET_TEXT[kind] + enemy, HORIZONTAL_ALIGNMENT_LEFT, -1, 24, Color(0.92, 0.9, 0.84))
		y += 82.0


## Records `count` frames into `folder` (every second frame, numbered from
## `first`), with stills at STILLS_MS when `stills`.
func _record(folder: String, first: int, count: int, kind: String, stills: bool) -> void:
	var still_frames: Dictionary[int, int] = {}
	for ms in STILLS_MS:
		still_frames[roundi(ms * 30.0 / 1000.0)] = ms
	for k in count:
		await _frame()
		var image: Image = null
		if k % 2 == 0:
			image = get_viewport().get_texture().get_image()
			image.save_png("%s/frame_%04d.png" % [folder, (first + k) / 2])
		if stills and still_frames.has(k):
			if image == null:
				image = get_viewport().get_texture().get_image()
			image.save_png("%s/%s_%s_%04d.png" % [_dir, _prefix, kind, still_frames[k]])


func _frames(count: int) -> void:
	for i in count:
		await _frame()


func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
	await get_tree().process_frame
