extends Node3D
## Own units easier to see (prompt 164). Presentation only.
##
## - **Under the cursor:** units within `hover_radius()` of the ground point
##   under the mouse show their ring in their relation color (own, ally,
##   enemy, `relation_colors.gd`), the one nearest the cursor stronger; own
##   units also light up (an additive overlay on their models, a squad as a
##   whole).
## - **Ground markers:** a small flat ring in a light own color on a dark
##   rim under every own soldier (one MultiMesh for all; the rim keeps it
##   readable on grass of the own color, on snow and sand), faint; bright
##   for the squads under the cursor and while the show-all key is held.
## - **Show all** (`show_units`, Tab, rebindable): while held, every own
##   unit shows its ring and its soldiers' markers bright.
##
## Setting `unit_markers` (`settings.gd`): `off` (none of it), `hover` (the
## cursor and the key only), `always` (also the faint markers; default).
##
## **Hidden units** (prompt 165): what of an own (and, by the setting
## `hidden_units`, allied) unit is hidden behind a house, a tree, a hill or
## another vehicle shows as a flat silhouette in its relation color
## (`unit_xray.gdshader`, a material overlay; how it decides what is hidden:
## the shader's header). Soldiers show their body (not the gear), vehicles
## and guns every mesh. None for enemy units, base buildings, units in a
## building or a vehicle pit, soldiers in dug works (trench, foxhole) and
## dead soldiers, and none for squads farther than `XRAY_SOLDIERS_M` from the
## camera (draw calls); none while the HUD is hidden or in photo mode. Selected
## units and those near the cursor are brighter. The hover light-up shares
## the overlay slot: hovered own units get the light with the silhouette as
## its next pass, allies the ally tint with it.
## Runs after `main.gd` placed the units and set the selection rings (it is
## added after them; children process after their parent).

const Settings := preload("res://scenes/menu/settings.gd")
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const XRAY_SHADER := preload("res://assets/shaders/unit_xray.gdshader")
const BaseView := preload("res://scenes/world/base_view.gd")

## Hover radius in meters: a share of the camera distance, clamped.
const HOVER_SHARE := 0.12
const HOVER_MIN_M := 20.0
const HOVER_MAX_M := 80.0
## Marker ring under a soldier (meters, light ring and its dark rim) and
## its height above his feet.
const MARKER_RADIUS := 0.62
const RIM_RADIUS := 0.78
const MARKER_LIFT := 0.06
## Share of white mixed into the own color of the light ring; opacity of
## the dark rim relative to the ring.
const MARKER_LIGHTEN := 0.4
const RIM_ALPHA := 0.55
## Marker opacity: faint (always), bright (hovered or show-all).
const MARKER_FAINT := 0.38
const MARKER_BRIGHT := 0.95
## Ring opacity of hovered units; the nearest one is drawn opaque.
const RING_HOVER := 0.55
## Brightening of hovered own models (additive white).
const LIGHT_UP := 0.12
## Silhouette of hidden units (prompt 165): opacity, and for selected units
## and those near the cursor a higher opacity and a share of white.
const XRAY_ALPHA := 0.55
const XRAY_BRIGHT_ALPHA := 0.75
const XRAY_LIGHTEN := 0.3
## Soldiers at a cover object whose catalog id contains one of these show
## no silhouette (dug in: the earth around them is not "in front").
const DUG_WORKS: Array[String] = ["trench", "foxhole", "dugout", "nest", "pit"]
## Squads farther than this from the camera show no silhouette (meters;
## on the lowest graphics level the nearer value: vehicles and guns keep
## theirs at any distance).
const XRAY_SOLDIERS_M := 300.0
const XRAY_SOLDIERS_LOW_M := 150.0
## Soldier exclusions (dead, dug in) are collected every this many ticks.
const EXCLUDE_TICKS := 4

## Every own unit shows as with the show-all key (the overview, prompt 103).
var force_show_all := false
var main: Node3D
var sim_host: SimHost
## Time of the last `_process` in microseconds (`perf_bench.gd`).
var process_usec := 0
## Unit ids lit up (overlay set) in the last frame.
var lit: Dictionary[int, bool] = {}
## Unit ids whose ring this script showed in the last frame (self-test).
var hovered: Dictionary[int, bool] = {}
## Unit nearest the cursor within the radius, or -1.
var nearest := -1
## Markers drawn in the last frame, and how many of them bright.
var markers_drawn := 0
var markers_bright := 0
## Ground point under the cursor (null: none); the self-test may set
## `forced_point` instead of moving the mouse.
var cursor_point: Variant = null
var forced_point: Variant = null
## Unit ids drawing a silhouette where hidden in the last frame (self-test).
var xray_units: Dictionary[int, bool] = {}

var _markers: MultiMeshInstance3D
## What the markers were last built for: tick, hovered ids, key, setting.
var _marker_key := []
var _ring_materials := {}
var _light_material: StandardMaterial3D
## Silhouette alone, the hover light and the ally tint with it as next pass.
var _xray: ShaderMaterial
var _light_xray: StandardMaterial3D
var _ally_xray: StandardMaterial3D
## Per unit id: what its overlays were last set for (light, silhouette,
## bright, soldier count as bits) and the mask of its excluded soldiers;
## what the overlays were last looked at for, and with which palette.
var _applied: Dictionary[int, int] = {}
var _applied_mask: Dictionary[int, int] = {}
var _overlay_frame_key := []
var _overlay_palette := ""
## Per unit id: a base building (no silhouette).
var _structure: Dictionary[int, bool] = {}
## Soldier keys (unit id * 256 + index) without a silhouette, a mask of
## them per unit id (bit index % 62), when they were collected.
var _excluded: Dictionary[int, bool] = {}
var _excluded_by_unit: Dictionary[int, int] = {}
var _excluded_tick := -1000
## Cover object index -> dug work (`DUG_WORKS`).
var _dug_cover: Dictionary[int, bool] = {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	var ring := _marker_mesh()
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.vertex_color_use_as_albedo = true
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	ring.surface_set_material(0, material)
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_colors = true
	multimesh.mesh = ring
	_markers = MultiMeshInstance3D.new()
	_markers.name = "SoldierMarkers"
	_markers.multimesh = multimesh
	_markers.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_markers)
	_light_material = StandardMaterial3D.new()
	_light_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_light_material.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
	_light_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_light_material.albedo_color = Color(1.0, 1.0, 1.0, LIGHT_UP)
	_xray = ShaderMaterial.new()
	_xray.shader = XRAY_SHADER
	_light_xray = _light_material.duplicate()
	_light_xray.next_pass = _xray
	if main._ally_material != null:
		_ally_xray = main._ally_material.duplicate()
		_ally_xray.next_pass = _xray


## A flat marker of radius 1: a light ring in the own color (inner part)
## on a dark rim (outer part), as vertex colors; the instance color scales
## the opacity.
static func _marker_mesh() -> ArrayMesh:
	var light := RelationColors.own().lerp(Color.WHITE, MARKER_LIGHTEN)
	var dark := Color(0.0, 0.0, 0.0, RIM_ALPHA)
	var inner := MARKER_RADIUS / RIM_RADIUS * 0.66
	var middle := MARKER_RADIUS / RIM_RADIUS
	# Bands: light from `inner` to `middle`, dark from `middle` to 1.
	var bands := [[inner, middle, light], [middle, 1.0, dark]]
	var vertices := PackedVector3Array()
	var colors := PackedColorArray()
	var segments := 24
	for band: Array in bands:
		for k in segments:
			var a0 := TAU * k / segments
			var a1 := TAU * (k + 1) / segments
			var r0: float = band[0]
			var r1: float = band[1]
			var p0 := Vector3(cos(a0) * r0, 0.0, sin(a0) * r0)
			var p1 := Vector3(cos(a0) * r1, 0.0, sin(a0) * r1)
			var p2 := Vector3(cos(a1) * r1, 0.0, sin(a1) * r1)
			var p3 := Vector3(cos(a1) * r0, 0.0, sin(a1) * r0)
			vertices.append_array([p0, p1, p2, p0, p2, p3])
			for i in 6:
				colors.append(band[2])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_COLOR] = colors
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


## The ring material of a relation (0 own, 1 ally, 2 enemy) and strength.
func _ring_material(relation: int, strong: bool) -> StandardMaterial3D:
	var key := relation * 2 + int(strong)
	if not _ring_materials.has(key):
		var color: Color = [RelationColors.own(), RelationColors.ally(), RelationColors.enemy()][relation]
		var m := StandardMaterial3D.new()
		m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		m.no_depth_test = true
		m.render_priority = 1
		m.albedo_color = Color(color.r, color.g, color.b, 1.0 if strong else RING_HOVER)
		_ring_materials[key] = m
	return _ring_materials[key]


func hover_radius() -> float:
	var camera: Camera3D = main.camera
	var target: Vector3 = main.camera_rig.global_position if main.get("camera_rig") != null else Vector3.ZERO
	var distance := camera.global_position.distance_to(target)
	return clampf(distance * HOVER_SHARE, HOVER_MIN_M, HOVER_MAX_M)


func _process(_delta: float) -> void:
	if sim_host == null:
		return
	var began := Time.get_ticks_usec()
	var mode: String = Settings.get_value("unit_markers")
	var show_all := mode != "off" and (force_show_all or InputMap.has_action("show_units") \
			and Input.is_action_pressed("show_units"))
	cursor_point = null
	if mode != "off":
		if forced_point != null:
			cursor_point = forced_point
		elif get_viewport().gui_get_hovered_control() == null:
			cursor_point = GroundPick.from_screen(main.camera, get_viewport().get_mouse_position())
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var positions := sim_host.current_positions()
	var me := sim_host.local_player()
	# Who is under the cursor.
	var near: Dictionary[int, bool] = {}
	nearest = -1
	if cursor_point != null:
		var point: Vector3 = cursor_point
		var radius := hover_radius()
		var best := INF
		for i in ids.size():
			if alive[i] == 0:
				continue
			var d := Vector2(positions[i].x - point.x, positions[i].z - point.z).length()
			if d <= radius:
				near[ids[i]] = true
				if d < best:
					best = d
					nearest = ids[i]
	_update_rings(ids, owners, alive, near, show_all, me)
	_update_overlays(ids, owners, alive, near, me)
	_update_markers(ids, owners, near, show_all, mode, me)
	process_usec = Time.get_ticks_usec() - began


## Rings of hovered units (and of every own unit while show-all is held),
## unless the unit is selected: `main.gd` keeps the selection ring.
func _update_rings(ids: PackedInt64Array, owners: PackedInt32Array, alive: PackedByteArray,
		near: Dictionary[int, bool], show_all: bool, me: int) -> void:
	var rings: Dictionary = main._rings
	var shown: Dictionary[int, bool] = {}
	for i in ids.size():
		var id := ids[i]
		if alive[i] == 0 or main.selection.is_selected(id):
			continue
		var own := owners[i] == me
		if not near.has(id) and not (show_all and own):
			continue
		var ring: MeshInstance3D = rings.get(id)
		if ring == null:
			continue
		var relation := 0 if own else (1 if sim_host.relation(owners[i]) == 1 else 2)
		ring.material_override = _ring_material(relation, id == nearest or (show_all and own))
		ring.visible = true
		shown[id] = true
	# Rings shown last frame and no longer: back to the selection look.
	for id in hovered:
		if not shown.has(id) and rings.has(id):
			var ring: MeshInstance3D = rings[id]
			ring.material_override = null
	for id in shown:
		var ring: MeshInstance3D = rings[id]
		if main.selection.is_selected(id):
			ring.material_override = null
	hovered = shown


## Overlays of own and allied units: the hover light (own units near the
## cursor) and the silhouette of hidden parts (prompt 165). Looked at only
## when a tick arrived or the hover, the selection, the setting or the
## camera (10 m steps) changed, and set only for units whose needs changed
## (`_applied`): checking every unit every frame cost about 0.6 ms.
func _update_overlays(ids: PackedInt64Array, owners: PackedInt32Array, alive: PackedByteArray,
		near: Dictionary[int, bool], me: int) -> void:
	var mode: String = Settings.get_value("hidden_units")
	var toggles: Node = main.get("view_toggles")
	var photo: Node = main.get("photo_mode")
	var off: bool = mode == "off" or (toggles != null and toggles.hidden) \
			or (photo != null and photo.active)
	# The copy follows the tint switch (`view_toggles.gd`).
	if _ally_xray != null:
		_ally_xray.albedo_color.a = main._ally_material.albedo_color.a
	var camera_position: Vector3 = main.camera.global_position
	var near_ids := near.keys()
	near_ids.sort()
	var frame_key := [sim_host.current_tick(), near_ids, main.selection.selected.hash(), mode, off,
			Vector3i(camera_position / 10.0)]
	if frame_key == _overlay_frame_key:
		return
	_overlay_frame_key = frame_key
	if RelationColors.palette != _overlay_palette:
		_overlay_palette = RelationColors.palette
		_applied.clear()
	var view: Node = main.get("infantry_view")
	if not off and sim_host.current_tick() >= _excluded_tick + EXCLUDE_TICKS:
		_collect_excluded()
	var garrisons := sim_host.unit_garrisons()
	var dug := {}
	if not off:
		for id in sim_host.dug_in_units():
			dug[id] = true
	var atmosphere: Node = main.get("atmosphere")
	var low: bool = atmosphere != null and atmosphere.get("quality") == "low"
	var soldier_range := XRAY_SOLDIERS_LOW_M if low else XRAY_SOLDIERS_M
	var relations := {}
	var want_lit: Dictionary[int, bool] = {}
	var seen: Dictionary[int, bool] = {}
	xray_units = {}
	for i in ids.size():
		var id := ids[i]
		var owner := owners[i]
		var own := owner == me
		if not own:
			if not relations.has(owner):
				relations[owner] = sim_host.relation(owner) == 1
			if not relations[owner]:
				continue
		var node: Node3D = main._unit_nodes.get(id)
		if node == null:
			continue
		seen[id] = true
		var light := own and near.has(id)
		if light:
			want_lit[id] = true
		var xray: bool = not off and (own or mode == "own_allies") and alive[i] != 0 \
				and garrisons[i] < 0 and not dug.has(id) and node.visible and not _is_structure(id, i)
		var soldiers: Array = view._by_unit.get(id, []) if view != null else []
		# Far squads: no silhouette (a few pixels tall; its draw calls cost
		# more than it shows).
		if xray and not soldiers.is_empty() \
				and node.global_position.distance_to(camera_position) > soldier_range:
			xray = false
		if xray:
			xray_units[id] = true
		var bright: bool = xray and (near.has(id) or main.selection.is_selected(id))
		var mask: int = _excluded_by_unit.get(id, 0)
		var code := int(light) | int(xray) << 1 | int(bright) << 2 | soldiers.size() << 3
		if _applied.get(id, -1) == code and _applied_mask.get(id, 0) == mask:
			continue
		_applied[id] = code
		_applied_mask[id] = mask
		_apply_overlays(node, own, light, xray, bright, soldiers, view)
	lit = want_lit
	if _applied.size() > seen.size():
		for id: int in _applied.keys():
			if not seen.has(id):
				_applied.erase(id)
				_applied_mask.erase(id)


func _is_structure(id: int, index: int) -> bool:
	if not _structure.has(id):
		_structure[id] = BaseView.is_building(sim_host.unit_type_ids()[index])
	return _structure[id]


## Dead soldiers and those in dug works (a trench is cut into the ground:
## the earth would show them all the time).
func _collect_excluded() -> void:
	_excluded_tick = sim_host.current_tick()
	_excluded.clear()
	_excluded_by_unit.clear()
	var units := sim_host.soldier_units()
	var indices := sim_host.soldier_indices()
	var hp := sim_host.soldier_hp()
	var covers := sim_host.soldier_cover_objects()
	for k in units.size():
		if hp[k] > 0 and (covers[k] < 0 or not _is_dug_cover(covers[k])):
			continue
		var key := units[k] * 256 + indices[k]
		_excluded[key] = true
		_excluded_by_unit[units[k]] = int(_excluded_by_unit.get(units[k], 0)) | 1 << (indices[k] % 62)


func _is_dug_cover(cover: int) -> bool:
	if not _dug_cover.has(cover):
		var info: Dictionary = sim_host.cover_object_info(cover)
		var id := str(info.get("id", ""))
		_dug_cover[cover] = DUG_WORKS.any(func(work: String) -> bool: return id.contains(work))
	return _dug_cover[cover]


## Sets the overlay of every mesh of a unit: soldiers (top-level nodes of
## the squad model, `infantry_view.gd`) the light on every mesh and the
## silhouette on the body; other units both on every mesh.
func _apply_overlays(node: Node3D, own: bool, light: bool, xray: bool, bright: bool,
		soldiers: Array, view: Node) -> void:
	var color := RelationColors.own() if own else RelationColors.ally()
	if bright:
		color = color.lerp(Color.WHITE, XRAY_LIGHTEN)
	color.a = XRAY_BRIGHT_ALPHA if bright else XRAY_ALPHA
	if soldiers.is_empty():
		for mesh: Node in node.find_children("*", "MeshInstance3D", true, false):
			if mesh.name != "SelectionRing":
				_set_overlay(mesh, own, light, xray, color)
		return
	for key: int in soldiers:
		var s: Dictionary = view._soldiers.get(key, {})
		if s.is_empty():
			continue
		var soldier_xray := xray and not _excluded.has(key)
		for mesh: GeometryInstance3D in s["meshes"]:
			if mesh is MeshInstance3D:
				_set_overlay(mesh, own, light, soldier_xray and mesh.name == "Body", color)


func _set_overlay(mesh: MeshInstance3D, own: bool, light: bool, xray: bool, color: Color) -> void:
	var material: Material
	if own:
		material = (_light_xray if xray else _light_material) if light else (_xray if xray else null)
	else:
		material = _ally_xray if xray and _ally_xray != null else main._ally_material
	mesh.material_overlay = material
	if xray:
		mesh.set_instance_shader_parameter(&"xray_color", color)


## One marker ring per living own soldier on the ground. Rebuilt only when
## a new tick arrived or the hover, the key or the setting changed: at the
## half-way point between the two ticks, so a marker is at most a few
## centimeters from the soldier drawn between them (rebuilding every frame
## cost about 1.2 ms with 1100 soldiers).
func _update_markers(ids: PackedInt64Array, owners: PackedInt32Array,
		near: Dictionary[int, bool], show_all: bool, mode: String, me: int) -> void:
	var multimesh := _markers.multimesh
	if mode == "off":
		multimesh.visible_instance_count = 0
		markers_drawn = 0
		markers_bright = 0
		_marker_key = []
		return
	var near_ids := near.keys()
	near_ids.sort()
	var key := [sim_host.current_tick(), near_ids, show_all, mode]
	if key == _marker_key:
		return
	_marker_key = key
	markers_drawn = 0
	markers_bright = 0
	var own_units := {}
	var garrisons := sim_host.unit_garrisons()
	for i in ids.size():
		if owners[i] == me and garrisons[i] < 0:
			own_units[ids[i]] = true
	var units := sim_host.soldier_units()
	var current := sim_host.soldier_positions()
	var previous := sim_host.soldier_previous_positions()
	var hp := sim_host.soldier_hp()
	# 12 floats of transform (basis rows with origin) and 4 of color per
	# instance; the color only scales the opacity of the marker mesh.
	var buffer := PackedFloat32Array()
	buffer.resize(units.size() * 16)
	var n := 0
	for k in units.size():
		var unit := units[k]
		if hp[k] <= 0 or not own_units.has(unit):
			continue
		var bright := show_all or near.has(unit)
		if not bright and mode != "always":
			continue
		var at := previous[k].lerp(current[k], 0.5)
		var o := n * 16
		buffer[o] = RIM_RADIUS
		buffer[o + 3] = at.x
		buffer[o + 5] = 1.0
		buffer[o + 7] = at.y + MARKER_LIFT
		buffer[o + 10] = RIM_RADIUS
		buffer[o + 11] = at.z
		buffer[o + 12] = 1.0
		buffer[o + 13] = 1.0
		buffer[o + 14] = 1.0
		buffer[o + 15] = MARKER_BRIGHT if bright else MARKER_FAINT
		n += 1
		markers_drawn += 1
		if bright:
			markers_bright += 1
	if multimesh.instance_count < n:
		multimesh.instance_count = maxi(n, multimesh.instance_count * 2)
	buffer.resize(multimesh.instance_count * 16)
	multimesh.buffer = buffer
	multimesh.visible_instance_count = n
