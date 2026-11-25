extends Node3D
## Map editor (prompt 028): shape the ground, paint terrain and water, place
## objects and start positions, check and save a map in the format of
## docs/MAPS.md. Opened from the main menu ("Map editor") or with
## `just map-editor` (`-- --edit=<name>` opens data/maps/<name>/).
##
## All map data lives in the `MapEditor` node (Rust, crates/editor): brushes,
## undo, saving and validation. This script turns mouse strokes into brush
## dabs and redraws what changed. The world is drawn by the same nodes as a
## match: `terrain.gd` (rebuilt per dab in the changed rectangle),
## `map_objects.gd` and `water.gd` (rebuilt when a stroke ends).
##
## Mouse: left button paints with the current tool; the camera works as in
## a match (WASD, Q/E, wheel, middle button). Keys: Ctrl+Z undo, Ctrl+Y or
## Ctrl+Shift+Z redo, [ and ] change the radius, Ctrl+S saves.
## New and Open reload the scene with `pending` set (like the match setup).

const Atmosphere := preload("res://scenes/world/atmosphere.gd")
const MapObjects := preload("res://scenes/world/map_objects.gd")
const Water := preload("res://scenes/world/water.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const MatchSetup := preload("res://scenes/menu/match_setup.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const EditorPanel := preload("res://scenes/editor/editor_panel.gd")
const MapPreview := preload("res://scenes/menu/map_preview.gd")
const CameraGizmo := preload("res://scenes/hud/camera_gizmo.gd")
const EditorData := preload("res://scenes/editor/editor_data.gd")
const SCENE := "res://scenes/editor/map_editor.tscn"
## Maps the editor refuses to overwrite: the golden hash and the benchmarks
## depend on them (regenerate them with `just test-maps`).
const PROTECTED_MAPS: Array[String] = ["test_small", "test_large"]
## Raise/lower speed at full strength (meters per second at the center).
const RAISE_RATE := 3.0
## Smooth/flatten strength per second at full strength.
const LEVEL_RATE := 6.0
## River depth at strength 0 and 1 (meters).
const RIVER_DEPTH := Vector2(0.6, 3.0)
## Spacing of dabs of the stamp tools, relative to the radius.
const STAMP_SPACING := 0.3
## Mountain tools (prompt 060): terrace height and the steepest grade of a
## road on a slope [provisional].
const TERRACE_STEP := 5.0
const ROAD_GRADE := 0.12

## The open map editor and the next one to open: `{"new": [name, size,
## biome]}` or `{"load": name}`; empty = a new default map.
static var pending := {}

@onready var editor: MapEditor = $MapEditor
@onready var camera_rig: Node3D = $RtsCamera
@onready var camera: Camera3D = $RtsCamera/Camera3D
@onready var sun: DirectionalLight3D = $Sun
@onready var terrain: Node3D = $Terrain

var map_objects: Node3D
var water: Node3D
var atmosphere: Atmosphere
var panel: EditorPanel
var gizmo: CameraGizmo
## Strongpoints drawn (self-test).
var strongpoint_outlines := 0
## Current tool (key of TOOLS) and brush settings.
var tool := "raise"
var radius := 12.0
var strength := 0.5
var density := 300.0
var terrain_id := 2
var scatter_preset := "Deciduous wood"  # l10n-ignore (preset id)
var place_kind := "env_house_farm:Intact"
var place_heading := 0
## Compound of the "compound" tool (prompt 059, data/compounds.toml).
var compound_name := "hut_yard"
## Name of the next trigger area or spawn point (prompt 039; empty: numbered).
var mission_point_name := ""
## Trigger areas and spawn points drawn (self-test).
var mission_points_drawn := 0
## Counters for the self-test.
var strokes := 0
var world_rebuilds := 0
## Result of the last `validate_map` (MapEditor.validate()).
var last_report := {}

var _stroke := false
var _stroke_level := 0.0
var _last_dab := Vector3.INF
var _cursor: MeshInstance3D
var _cursor_mesh: ImmediateMesh
var _start_markers: Array[MeshInstance3D] = []
## Outlines of the strongpoint markers (prompt 052).
var _strongpoint_mesh: MeshInstance3D
## Trigger areas and spawn points with their names (prompt 039).
var _mission_root: Node3D
var _hover: Variant = null
## Road tool (prompt 060): the previous dab and the road level there.
var _road_from := Vector3.ZERO
var _road_level := 0.0


func _ready() -> void:
	Settings.ensure_loaded()
	var request := pending
	pending = {}
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--edit=") and request.is_empty():
			request = {"load": arg.trim_prefix("--edit=")}
	var ok := false
	if request.has("load"):
		ok = editor.load_map(GamePaths.map_dir(request["load"]))
	elif request.has("new"):
		var spec: Array = request["new"]
		ok = editor.new_map(spec[0], spec[1], spec[1], spec[2])
	if not ok:
		var error := editor.last_error()
		editor.new_map("new_map", 500, 500, "forest")
		if not error.is_empty():
			print("Map editor: %s" % error)
	print("Map editor: map '%s' (%s m)" % [editor.map_name(), editor.map_size()])
	GroundPick.height_source = editor
	var size := editor.map_size()
	camera_rig.bounds = Rect2(-size * 0.5, size)
	camera_rig.max_distance = maxf(camera_rig.max_distance, minf(size.x, 1500.0))
	camera_rig.snap_to(Vector3.ZERO, minf(size.x * 0.45, 400.0))
	Settings.apply_camera(camera_rig, false)
	terrain.build(editor)
	atmosphere = Atmosphere.new()
	atmosphere.name = "Atmosphere"
	add_child(atmosphere)
	atmosphere.setup($WorldEnvironment, sun, camera_rig, terrain)
	atmosphere.quality = Settings.quality()
	atmosphere.set_biome(Atmosphere.biome_from(editor.map_biome()))
	_rebuild_world()
	_build_cursor()
	panel = EditorPanel.new()
	panel.name = "EditorPanel"
	panel.main = self
	$Hud.add_child(panel)
	# The camera control of the match HUD (prompt 096), bottom right.
	gizmo = CameraGizmo.new()
	$Hud.add_child(gizmo)
	gizmo.setup(camera_rig)
	gizmo.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	gizmo.offset_left = -gizmo.size.x - 12.0
	gizmo.offset_top = -gizmo.size.y - 12.0
	gizmo.offset_right = -12.0
	gizmo.offset_bottom = -12.0
	gizmo.visible = bool(Settings.get_value("camera_gizmo"))
	_update_starts()
	set_status(tr("EDITOR_STATUS_OPENED") % editor.map_name())


func _exit_tree() -> void:
	if GroundPick.height_source == editor:
		GroundPick.height_source = null


## Absolute folder a map is saved to: data/maps/ in the repository, the
## player's own maps (user://maps) in an exported game (see GamePaths).
static func map_dir(map_name: String) -> String:
	return GamePaths.user_maps_dir().path_join(map_name)


# --- Strokes ------------------------------------------------------------


## Starts a stroke of the current tool at a ground point.
func stroke_begin(point: Vector3) -> void:
	if _stroke:
		stroke_end()
	_stroke = true
	_stroke_level = editor.height_at(point.x, point.z)
	_last_dab = Vector3.INF
	_road_from = point
	_road_level = _stroke_level
	editor.begin_stroke()
	match tool:
		"place":
			var placed: Variant = editor.place_object(place_kind, point.x, point.z, place_heading)
			if placed == null:
				set_status(editor.last_error(), true)
		"strongpoint":
			# A click on a strongpoint removes it, elsewhere adds one.
			if editor.remove_strongpoints(point.x, point.z, EditorData.STRONGPOINT_PICK_M) == 0:
				editor.add_strongpoint(point.x, point.z)
			_update_strongpoints()
		"mission_area", "mission_spawn":
			_mission_point_click(point)
		"compound":
			if editor.place_compound(compound_name, point.x, point.z, place_heading) < 0:
				set_status(editor.last_error(), true)
		"village", "village_med":
			var n: int = editor.village(point.x, point.z, radius) if tool == "village" \
				else editor.village_of(point.x, point.z, radius, "mediterranean")
			if n < 0:
				set_status(editor.last_error(), true)
			else:
				set_status(tr("EDITOR_COMPOUNDS_PLACED") % n)
		"start0", "start1", "start2", "start3", "start4", "start5":
			var player := int(tool.right(1))
			var new_start: bool = not editor.start_positions().has(player)
			editor.set_start(player, point.x, point.z)
			# A new start gets the usual team: even players against odd
			# ones (prompt 037); the panel changes it.
			if new_start and not editor.start_teams().has(player):
				editor.set_start_team(player, player % 2)
				_update_starts()
		_:
			stroke_to(point, 1.0 / 60.0)


## Continues the stroke at `point`; `delta` (seconds) scales the rate of the
## height tools.
func stroke_to(point: Vector3, delta: float) -> void:
	if not _stroke:
		return
	var rect := Rect2i()
	match tool:
		"raise", "lower":
			var amount := RAISE_RATE * strength * delta * (1.0 if tool == "raise" else -1.0)
			rect = editor.raise(point.x, point.z, radius, amount)
		"smooth":
			rect = editor.smooth(point.x, point.z, radius, clampf(strength * LEVEL_RATE * delta, 0.0, 1.0))
		"flatten":
			rect = editor.flatten(point.x, point.z, radius, _stroke_level, clampf(strength * LEVEL_RATE * delta, 0.0, 1.0))
		"ridge", "valley":
			var amount := RAISE_RATE * strength * delta * (1.0 if tool == "ridge" else -1.0)
			rect = editor.ridge(point.x, point.z, radius, amount)
		"terrace":
			rect = editor.terrace(point.x, point.z, radius, TERRACE_STEP, clampf(strength * LEVEL_RATE * delta, 0.0, 1.0))
		"road":
			if _last_dab != Vector3.INF and _flat_distance(point, _last_dab) < radius * STAMP_SPACING:
				return
			_last_dab = point
			rect = _road_dab(point)
		"paint", "river", "scatter", "erase":
			if _last_dab != Vector3.INF and _flat_distance(point, _last_dab) < radius * STAMP_SPACING:
				return
			_last_dab = point
			match tool:
				"paint":
					rect = editor.paint(point.x, point.z, radius, terrain_id)
				"river":
					var depth := lerpf(RIVER_DEPTH.x, RIVER_DEPTH.y, strength)
					rect = editor.river(point.x, point.z, radius, _stroke_level, depth)
				"scatter":
					var preset: Dictionary = EditorData.SCATTER_PRESETS[scatter_preset]
					editor.scatter(point.x, point.z, radius, PackedStringArray(preset.kinds),
						density * preset.get("density", 1.0), preset.spacing)
					if preset.has("then"):
						var then: Dictionary = preset.then
						editor.scatter(point.x, point.z, radius, PackedStringArray(then.kinds),
							density * then.get("density", 1.0), then.spacing)
				"erase":
					editor.erase_objects(point.x, point.z, radius, "")
	if rect.has_area():
		terrain.heights_changed(rect)


## One dab of the road tool (prompt 060): the level follows the untouched
## ground one radius ahead, but changes by at most ROAD_GRADE per meter
## since the previous dab.
func _road_dab(point: Vector3) -> Rect2i:
	var step := Vector3(point.x - _road_from.x, 0.0, point.z - _road_from.z)
	var dist := step.length()
	var ahead := point + step.normalized() * radius if dist > 0.01 else point
	var ground := editor.height_at(ahead.x, ahead.z)
	var from_level := _road_level
	_road_level = clampf(ground, from_level - ROAD_GRADE * dist, from_level + ROAD_GRADE * dist)
	var id := terrain_id if terrain_id in [1, 2] else 2
	var rect := editor.road(point.x, point.z, radius, _road_level, _road_from.x, _road_from.z, from_level, id)
	_road_from = point
	return rect


## Ends the stroke (one undo step) and redraws objects and water.
func stroke_end() -> void:
	if not _stroke:
		return
	_stroke = false
	var changed := editor.end_stroke()
	if changed["any"]:
		strokes += 1
		_after_change(changed, false)
	panel.update_history()


## Draws a whole stroke along `points` (self-test, scripts).
func stroke(points: Array, delta := 1.0 / 60.0) -> void:
	stroke_begin(points[0])
	for p: Vector3 in points.slice(1):
		stroke_to(p, delta)
	stroke_end()


func undo() -> void:
	_stroke = false
	_after_change(editor.undo(), true)
	panel.update_history()


func redo() -> void:
	_stroke = false
	_after_change(editor.redo(), true)
	panel.update_history()


## Redraws after a stroke, undo or redo. `redraw_terrain`: the rectangle
## was not drawn dab by dab.
func _after_change(changed: Dictionary, redraw_terrain: bool) -> void:
	if not changed["any"]:
		return
	var rect: Rect2i = changed["rect"]
	if redraw_terrain and rect.has_area():
		terrain.heights_changed(rect)
	if rect.has_area() or changed["objects"]:
		_rebuild_world()
	if changed["meta"]:
		_update_starts()


## Objects, decoration and water of the current map, built anew.
func _rebuild_world() -> void:
	for node in [map_objects, water]:
		if node != null:
			remove_child(node)
			node.queue_free()
	map_objects = MapObjects.new()
	map_objects.name = "MapObjects"
	add_child(map_objects)
	map_objects.biome = atmosphere.preset.env_biome
	map_objects.atmosphere = atmosphere
	map_objects.terrain = terrain
	map_objects.show_minefields = true
	map_objects.build(editor, camera)
	water = Water.new()
	water.name = "Water"
	add_child(water)
	water.build(editor)
	water.apply_biome(atmosphere.preset)
	terrain.refresh_water()
	world_rebuilds += 1


# --- Saving and checking --------------------------------------------------


## Checks the map; shows and returns the report.
func validate_map() -> Dictionary:
	last_report = editor.validate()
	var lines: PackedStringArray = []
	for e: String in last_report.get("errors", []):
		lines.append(tr("EDITOR_ERROR_LINE") % e)
	for w: String in last_report.get("warnings", []):
		lines.append(tr("EDITOR_WARNING_LINE") % w)
	if last_report.get("ok", false):
		lines.insert(0, tr("EDITOR_PLAYABLE") % last_report["map_hash"])
	set_status("\n".join(lines), not last_report.get("ok", false))
	return last_report


## Saves under `map_name` (see `map_dir`); returns whether it worked.
func save_as(map_name: String) -> bool:
	map_name = map_name.strip_edges()
	if map_name in PROTECTED_MAPS:
		set_status(tr("EDITOR_PROTECTED_MAP") % map_name, true)
		return false
	if not editor.set_map_name(map_name):
		set_status(editor.last_error(), true)
		return false
	if not editor.save_map(map_dir(map_name)):
		set_status(editor.last_error(), true)
		return false
	# The map picker builds its preview again (prompt 201).
	MapPreview.invalidate(map_name)
	var report := validate_map()
	set_status(tr("EDITOR_SAVED").format({"path": map_dir(map_name), "result": tr("EDITOR_SAVED_PLAYABLE" if report["ok"] else "EDITOR_SAVED_NOT_PLAYABLE")}), not report["ok"])
	panel.update_title()
	return true


## Opens a new map or another map (reloads the scene).
func open_new(map_name: String, size: int, biome: String) -> void:
	pending = {"new": [map_name, size, biome]}
	get_tree().reload_current_scene()


func open_map(map_name: String) -> void:
	pending = {"load": map_name}
	get_tree().reload_current_scene()


func back_to_menu() -> void:
	get_tree().change_scene_to_file(MatchSetup.MENU_SCENE)


func set_status(text: String, error := false) -> void:
	print("Map editor: %s" % text.replace("\n", " | "))
	if panel != null:
		panel.set_status(text, error)


# --- Input ---------------------------------------------------------------


func _unhandled_input(event: InputEvent) -> void:
	var key := event as InputEventKey
	if key != null and key.pressed and not key.echo:
		if key.ctrl_pressed or key.meta_pressed:
			match key.keycode:
				KEY_Z:
					if key.shift_pressed:
						redo()
					else:
						undo()
				KEY_Y:
					redo()
				KEY_S:
					save_as(editor.map_name())
				_:
					return
			get_viewport().set_input_as_handled()
			return
		match key.keycode:
			KEY_BRACKETLEFT:
				panel.set_radius(radius / 1.25)
			KEY_BRACKETRIGHT:
				panel.set_radius(radius * 1.25)
	var button := event as InputEventMouseButton
	if button != null and button.button_index == MOUSE_BUTTON_LEFT:
		if button.pressed:
			var point: Variant = GroundPick.from_screen(camera, button.position)
			if point != null:
				stroke_begin(point)
				get_viewport().set_input_as_handled()
		elif _stroke:
			stroke_end()
			get_viewport().set_input_as_handled()


func _process(delta: float) -> void:
	_hover = GroundPick.from_screen(camera, get_viewport().get_mouse_position()) if not DisplayServer.get_name() == "headless" else null
	if _stroke and _hover != null:
		stroke_to(_hover, delta)
	_draw_cursor()


func _flat_distance(a: Vector3, b: Vector3) -> float:
	return Vector2(a.x - b.x, a.z - b.z).length()


# --- Cursor and markers -----------------------------------------------------


func _build_cursor() -> void:
	_cursor_mesh = ImmediateMesh.new()
	_cursor = MeshInstance3D.new()
	_cursor.name = "BrushCursor"
	_cursor.mesh = _cursor_mesh
	_cursor.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = MenuTheme.ACCENT
	material.no_depth_test = true
	_cursor.material_override = material
	add_child(_cursor)


func _draw_cursor() -> void:
	_cursor_mesh.clear_surfaces()
	if _hover == null:
		return
	var center: Vector3 = _hover
	var r := radius if tool not in ["place", "compound", "mission_spawn"] + EditorData.START_TOOLS else 3.0
	_cursor_mesh.surface_begin(Mesh.PRIMITIVE_LINE_STRIP)
	for k in 49:
		var angle := TAU * k / 48.0
		var x := center.x + cos(angle) * r
		var z := center.z + sin(angle) * r
		_cursor_mesh.surface_add_vertex(Vector3(x, editor.height_at(x, z) + 0.3, z))
	_cursor_mesh.surface_end()


## Edits the start layout `index` of `editor.variants()` (prompt 182).
func select_variant(index: int) -> void:
	editor.select_variant(index)
	_update_starts()
	if panel != null:
		panel.update_variants()


## Adds a start variant with a copy of the standard starts
## (`variant_<n>`) and edits it; returns its name ("" on failure).
func add_variant() -> String:
	var n := editor.variants().size()
	var name := "variant_%d" % n
	while name in editor.variants():
		n += 1
		name = "variant_%d" % n
	var error := editor.add_variant(name, "")
	if not error.is_empty():
		push_warning(error)
		return ""
	_update_starts()
	if panel != null:
		panel.update_variants()
	return name


## Removes the edited start variant (not the standard layout).
func remove_variant() -> void:
	editor.remove_variant()
	_update_starts()
	if panel != null:
		panel.update_variants()


## Team of `player`'s start, -1 without (prompt 037).
func start_team_of(player: int) -> int:
	return int(editor.start_teams().get(player, -1))


## Sets the team of the start the current start tool places (-1: none).
func set_start_team(team: int) -> void:
	if tool in EditorData.START_TOOLS:
		editor.set_start_team(int(tool.right(1)), team)
		_update_starts()


## Columns at the start positions, colored by team (prompt 037; without
## teams blue for player 1, red for the others).
func _update_starts() -> void:
	for marker in _start_markers:
		marker.queue_free()
	_start_markers.clear()
	var starts := editor.start_positions()
	var teams := editor.start_teams()
	var team_colors: Array[Color] = [Color(0.3, 0.55, 1.0), Color(1.0, 0.35, 0.25), Color(0.35, 0.9, 0.4)]
	for player: int in starts:
		var marker := MeshInstance3D.new()
		var mesh := CylinderMesh.new()
		mesh.top_radius = 1.2
		mesh.bottom_radius = 1.2
		mesh.height = 12.0
		var material := StandardMaterial3D.new()
		material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		var team: int = teams.get(player, -1)
		if team >= 0:
			material.albedo_color = team_colors[team % team_colors.size()]
		else:
			material.albedo_color = Color(0.3, 0.55, 1.0) if player == 0 else Color(1.0, 0.35, 0.25)
		mesh.material = material
		marker.mesh = mesh
		marker.name = "Start%d" % player
		marker.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		marker.position = starts[player] + Vector3(0, 6.0, 0)
		add_child(marker)
		_start_markers.append(marker)
	_update_strongpoints()
	_update_mission_points()


## Outlines of the strongpoints the map's markers make in a match (prompt
## 052): trenches brown, wire grey, nests and dugouts as rings.
func _update_strongpoints() -> void:
	if _strongpoint_mesh == null:
		_strongpoint_mesh = MeshInstance3D.new()
		_strongpoint_mesh.name = "Strongpoints"
		_strongpoint_mesh.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		var material := StandardMaterial3D.new()
		material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		material.vertex_color_use_as_albedo = true
		_strongpoint_mesh.material_override = material
		add_child(_strongpoint_mesh)
	var mesh := ImmediateMesh.new()
	var any := false
	for sp: Dictionary in editor.strongpoints():
		for piece: Dictionary in sp.pieces:
			var from: Vector3 = piece.from
			var to: Vector3 = piece.to
			var color := Color(0.45, 0.3, 0.15)
			match piece.kind:
				"wire":
					color = Color(0.7, 0.7, 0.7)
				"comm_trench":
					color = Color(0.6, 0.45, 0.25)
			if not any:
				mesh.surface_begin(Mesh.PRIMITIVE_LINES)
				any = true
			if from.distance_to(to) < 0.1:
				for k in 12:
					var a := TAU * k / 12.0
					var b := TAU * (k + 1) / 12.0
					mesh.surface_set_color(Color(0.9, 0.8, 0.4))
					mesh.surface_add_vertex(from + Vector3(cos(a) * 1.5, 0.5, sin(a) * 1.5))
					mesh.surface_add_vertex(from + Vector3(cos(b) * 1.5, 0.5, sin(b) * 1.5))
			else:
				mesh.surface_set_color(color)
				mesh.surface_add_vertex(from + Vector3(0, 0.5, 0))
				mesh.surface_add_vertex(to + Vector3(0, 0.5, 0))
	if any:
		mesh.surface_end()
	_strongpoint_mesh.mesh = mesh
	strongpoint_outlines = editor.strongpoints().size()


# --- Mission places (prompt 039) ---------------------------------------------


## A click of a mission tool: removes the areas under the point and the
## spawn points near it, or adds one (saved with the map as
## `mission_points.toml`; not part of undo [provisional]).
func _mission_point_click(point: Vector3) -> void:
	if editor.remove_mission_points(point.x, point.z, EditorData.MISSION_PICK_M) > 0:
		set_status(tr("EDITOR_MISSION_REMOVED"))
	elif tool == "mission_area":
		var added: String = editor.add_mission_area(point.x, point.z, radius, mission_point_name)
		set_status(tr("EDITOR_MISSION_AREA_ADDED").format({"name": added, "radius": roundi(radius)}))
	else:
		var added: String = editor.add_mission_spawn(point.x, point.z, mission_point_name)
		set_status(tr("EDITOR_MISSION_SPAWN_ADDED").format({"name": added}))
	_update_mission_points()


## Rings for trigger areas (yellow), small rings for spawn points (green),
## each with its name.
func _update_mission_points() -> void:
	if _mission_root != null:
		_mission_root.queue_free()
	_mission_root = Node3D.new()
	_mission_root.name = "MissionPoints"
	add_child(_mission_root)
	var mesh := ImmediateMesh.new()
	var points: Array = editor.mission_points()
	for p: Dictionary in points:
		var at: Vector3 = p.position
		var area: bool = p.kind == "area"
		var color := Color(0.95, 0.8, 0.3) if area else Color(0.4, 0.95, 0.45)
		var r: float = p.radius if area else 2.0
		mesh.surface_begin(Mesh.PRIMITIVE_LINE_STRIP)
		for k in 49:
			var angle := TAU * k / 48.0
			var x := at.x + cos(angle) * r
			var z := at.z + sin(angle) * r
			mesh.surface_set_color(color)
			mesh.surface_add_vertex(Vector3(x, editor.height_at(x, z) + 0.4, z))
		mesh.surface_end()
		var label := Label3D.new()
		label.text = p.name
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		label.no_depth_test = true
		label.pixel_size = 0.02
		label.modulate = color
		label.position = at + Vector3(0, 4.0, 0)
		_mission_root.add_child(label)
	var lines := MeshInstance3D.new()
	lines.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.vertex_color_use_as_albedo = true
	lines.material_override = material
	if not points.is_empty():
		lines.mesh = mesh
	_mission_root.add_child(lines)
	mission_points_drawn = points.size()
