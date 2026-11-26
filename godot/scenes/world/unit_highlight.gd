extends Node3D
## Own units easier to see (prompts 164, 165, 167). Presentation only.
##
## - **Under the pointer** (prompt 167): only the one unit whose model is
##   under the mouse pointer (`unit_pick.gd`, the same rule as a click) is
##   marked: a thin outline that follows its silhouette
##   (`unit_outline.gdshader`), in the hover blue for own units
##   (`own_mark_color`), red for enemies, the ally color for allies; a squad
##   as a whole (every soldier), fading in and out in `OUTLINE_FADE_S`.
##   Own units also show their health bar in that blue (`combat_view.gd`,
##   `HealthBars`). No rings and no lighting up on hover any more.
## - **Ground markers:** a small flat ring in a light own color on a dark
##   rim under every own soldier (one MultiMesh for all; the rim keeps it
##   readable on grass of the own color, on snow and sand), faint; bright
##   while the show-all key is held.
## - **Show all** (`show_units`, Tab, rebindable): while held, every own
##   unit shows its ring, its health bars and its soldiers' markers bright.
##
## Setting `unit_markers` (`settings.gd`): `off` (none of it), `hover` (the
## pointer and the key only), `always` (also the faint markers; default).
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
## units and the one under the pointer are brighter.
##
## The overlay slot of a mesh holds a chain: the ally / enemy tint (or
## nothing for own units), then the silhouette, then the outline, each as
## the next pass of the one before (`_set_overlay`).
## Runs after `main.gd` placed the units and set the selection rings (it is
## added after them; children process after their parent).

const Settings := preload("res://scenes/menu/settings.gd")
const RtsCamera := preload("res://scenes/camera/rts_camera.gd")
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const HudTheme := preload("res://scenes/hud/hud_theme.gd")
const XRAY_SHADER := preload("res://assets/shaders/unit_xray.gdshader")
const OUTLINE_SHADER := preload("res://assets/shaders/unit_outline.gdshader")
const BaseView := preload("res://scenes/world/base_view.gd")
const UnitPick := preload("res://scenes/world/unit_pick.gd")

## Marker ring under a soldier (meters, light ring and its dark rim) and
## its height above his feet.
const MARKER_RADIUS := 0.62
const RIM_RADIUS := 0.78
const MARKER_LIFT := 0.06
## Share of white mixed into the own color of the light ring; opacity of
## the dark rim relative to the ring.
const MARKER_LIGHTEN := 0.4
const RIM_ALPHA := 0.55
## Marker opacity: faint (always), bright (show-all).
const MARKER_FAINT := 0.38
const MARKER_BRIGHT := 0.95
## Outline of the unit under the pointer (prompt 167): width in screen
## pixels (scaled with the HUD), opacity, seconds to fade in or out.
const OUTLINE_PX := 2.5
const OUTLINE_ALPHA := 0.85
const OUTLINE_FADE_S := 0.15
## Own units under the pointer: outline and health bar in this blue in the
## standard palette (own units are green there and allies a lighter blue,
## `relation_colors.gd`); other palettes use their own color.
const OWN_MARK := Color(0.2, 0.42, 1.0)
## Silhouette of hidden units (prompt 165): opacity, and for selected units
## and the one under the pointer a higher opacity and a share of white.
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
## The unit whose model is under the pointer ({id: true}, at most one) and
## its id (-1: none).
var hovered: Dictionary[int, bool] = {}
var nearest := -1
## The show-all key is held (or forced) this frame.
var show_all := false
## Unit ids whose ring the show-all key showed in the last frame.
var ringed: Dictionary[int, bool] = {}
## Per unit id: opacity share (0..1) of its outline, fading in and out.
var outlined: Dictionary[int, float] = {}
## Markers drawn in the last frame, and how many of them bright.
var markers_drawn := 0
var markers_bright := 0
## The self-test may set `forced_point` (a world point, projected to the
## screen; off screen: nothing under the pointer) instead of moving the mouse.
var forced_point: Variant = null
## Unit ids drawing a silhouette where hidden in the last frame (self-test).
var xray_units: Dictionary[int, bool] = {}
## Hit test shared with `selection.gd`.
var pick: UnitPick

var _markers: MultiMeshInstance3D
## What the markers were last built for: tick, key, setting.
var _marker_key := []
var _ring_material: StandardMaterial3D
## Overlay chains (`_set_overlay`).
var _xray: ShaderMaterial
var _outline: ShaderMaterial
var _xray_outline: ShaderMaterial
var _ally_xray: StandardMaterial3D
var _ally_outline: StandardMaterial3D
var _ally_xray_outline: StandardMaterial3D
var _enemy_outline: StandardMaterial3D
## Per unit id: what its overlays were last set for (silhouette, bright,
## outline, soldier count as bits) and the mask of its excluded soldiers;
## what the overlays were last looked at for, and with which palette.
var _applied: Dictionary[int, int] = {}
var _applied_mask: Dictionary[int, int] = {}
var _overlay_frame_key := []
var _overlay_palette := ""
## Per unit id: the meshes carrying its outline.
var _outline_meshes: Dictionary[int, Array] = {}
## Per unit id: a base building (no silhouette).
var _structure: Dictionary[int, bool] = {}
## Soldier keys (unit id * 256 + index) without a silhouette, a mask of
## them per unit id (bit index % 62), when they were collected.
var _excluded: Dictionary[int, bool] = {}
var _excluded_by_unit: Dictionary[int, int] = {}
var _excluded_tick := -1000
## Cover object index -> dug work (`DUG_WORKS`).
var _dug_cover: Dictionary[int, bool] = {}
## Hover pick cache (prompt 208): skip unit_at() when pointer, camera & tick are still.
var _pick_mode := ""
var _pick_screen: Variant = null
var _pick_cam := Transform3D()
var _pick_tick := -1
var _pick_result := -1
var _last_ally_alpha := -1.0
var _last_enemy_alpha := -1.0
var _last_outline_px := -1.0


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	pick = UnitPick.new(main)
	main.selection.model_hit = pick.unit_at
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
	_xray = ShaderMaterial.new()
	_xray.shader = XRAY_SHADER
	_outline = ShaderMaterial.new()
	_outline.shader = OUTLINE_SHADER
	# After the silhouettes: the outline keeps out of them (stencil).
	_outline.render_priority = 1
	_xray_outline = _xray.duplicate()
	_xray_outline.next_pass = _outline
	if main._ally_material != null:
		_ally_xray = _tint_with(main._ally_material, _xray)
		_ally_outline = _tint_with(main._ally_material, _outline)
		_ally_xray_outline = _tint_with(main._ally_material, _xray_outline)
	if main._enemy_material != null:
		_enemy_outline = _tint_with(main._enemy_material, _outline)


static func _tint_with(tint: StandardMaterial3D, next: Material) -> StandardMaterial3D:
	var copy := tint.duplicate() as StandardMaterial3D
	copy.next_pass = next
	return copy


## Outline and health bar color of own units under the pointer.
static func own_mark_color() -> Color:
	return OWN_MARK if RelationColors.palette == "standard" else RelationColors.own()


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


func _process(delta: float) -> void:
	if sim_host == null:
		return
	var began := Time.get_ticks_usec()
	var mode: String = Settings.get_value("unit_markers")
	show_all = mode != "off" and (force_show_all or InputMap.has_action("show_units") \
			and Input.is_action_pressed("show_units"))
	nearest = -1
	if mode != "off":
		var screen: Variant = null
		if forced_point != null:
			if not main.camera.is_position_behind(forced_point):
				var projected: Vector2 = main.camera.unproject_position(forced_point)
				if get_viewport().get_visible_rect().has_point(projected):
					screen = projected
		elif get_viewport().gui_get_hovered_control() == null:
			screen = get_viewport().get_mouse_position()
		if screen != null:
			var cam_xform: Transform3D = main.camera.global_transform
			var cur_tick := sim_host.current_tick()
			if mode != _pick_mode or screen != _pick_screen or cam_xform != _pick_cam or cur_tick != _pick_tick:
				_pick_mode = mode
				_pick_screen = screen
				_pick_cam = cam_xform
				_pick_tick = cur_tick
				_pick_result = pick.unit_at(screen, "any")
			nearest = _pick_result
		else:
			_pick_screen = null
			_pick_result = -1
	else:
		_pick_mode = "off"
		_pick_result = -1
	hovered = {}
	if nearest >= 0:
		hovered[nearest] = true
	_fade_outlines(delta)
	var ids := sim_host.unit_ids()
	var owners := sim_host.unit_owners()
	var alive := sim_host.unit_alive()
	var me := sim_host.local_player()
	_update_rings(ids, owners, alive, me)
	_update_overlays(ids, owners, alive, me)
	_color_outlines(ids, owners, me)
	_update_markers(ids, owners, show_all, mode, me)
	process_usec = Time.get_ticks_usec() - began


## The hovered unit's outline fades in, the others' out (dropped at 0).
func _fade_outlines(delta: float) -> void:
	var step := delta / OUTLINE_FADE_S
	for id: int in outlined.keys():
		if id != nearest:
			outlined[id] = float(outlined[id]) - step
			if outlined[id] <= 0.0:
				outlined.erase(id)
	if nearest >= 0:
		outlined[nearest] = minf(float(outlined.get(nearest, 0.0)) + step, 1.0)


## Rings of every own unit while show-all is held, unless the unit is
## selected: `main.gd` keeps the selection ring. Hovering shows no ring.
func _update_rings(ids: PackedInt64Array, owners: PackedInt32Array, alive: PackedByteArray,
		me: int) -> void:
	var rings: Dictionary = main._rings
	var shown: Dictionary[int, bool] = {}
	if show_all:
		if _ring_material == null:
			_ring_material = StandardMaterial3D.new()
			_ring_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
			_ring_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
			_ring_material.no_depth_test = true
			_ring_material.render_priority = 1
		_ring_material.albedo_color = RelationColors.own()
		for i in ids.size():
			var id := ids[i]
			if alive[i] == 0 or owners[i] != me or main.selection.is_selected(id):
				continue
			var ring: MeshInstance3D = rings.get(id)
			if ring == null:
				continue
			ring.material_override = _ring_material
			ring.visible = true
			shown[id] = true
	# Rings shown last frame and no longer: back to the selection look.
	for id in ringed:
		if not shown.has(id) and rings.has(id):
			var ring: MeshInstance3D = rings[id]
			ring.material_override = null
	ringed = shown


## Overlays of own and allied units (silhouette of hidden parts, prompt
## 165) and of the outlined units (prompt 167). Looked at only when a tick
## arrived or the outlined units, the selection, the setting or the camera
## (10 m steps) changed, and set only for units whose needs changed
## (`_applied`): checking every unit every frame cost about 0.6 ms.
func _update_overlays(ids: PackedInt64Array, owners: PackedInt32Array, alive: PackedByteArray,
		me: int) -> void:
	var mode: String = Settings.get_value("hidden_units")
	var toggles: Node = main.view_toggles
	var photo: Node = main.photo_mode
	var off: bool = mode == "off" or (toggles != null and toggles.hidden) \
			or (photo != null and photo.active)
	# Prompt 208: update overlay material alphas & outline width only when changed.
	var ally_a: float = main._ally_material.albedo_color.a
	if ally_a != _last_ally_alpha:
		_last_ally_alpha = ally_a
		if _ally_xray != null:
			_ally_xray.albedo_color.a = ally_a
		if _ally_outline != null:
			_ally_outline.albedo_color.a = ally_a
		if _ally_xray_outline != null:
			_ally_xray_outline.albedo_color.a = ally_a
	var enemy_a: float = main._enemy_material.albedo_color.a
	if enemy_a != _last_enemy_alpha:
		_last_enemy_alpha = enemy_a
		if _enemy_outline != null:
			_enemy_outline.albedo_color.a = enemy_a
	var outline_px := HudTheme.px(OUTLINE_PX)
	if outline_px != _last_outline_px:
		_last_outline_px = outline_px
		_outline.set_shader_parameter(&"width_px", outline_px)
	var camera_position: Vector3 = main.camera.global_position
	var outline_ids: Array = outlined.keys()
	if outline_ids.size() > 1:
		outline_ids.sort()
	var frame_key := [sim_host.current_tick(), outline_ids, nearest, main.selection.selected.hash(),
			mode, off, Vector3i(camera_position / 10.0)]
	if frame_key == _overlay_frame_key:
		return
	_overlay_frame_key = frame_key
	if RelationColors.palette != _overlay_palette:
		_overlay_palette = RelationColors.palette
		_applied.clear()
	var view: Node = main.infantry_view
	if not off and sim_host.current_tick() >= _excluded_tick + EXCLUDE_TICKS:
		_collect_excluded()
	var garrisons := sim_host.unit_garrisons()
	var dug := {}
	if not off:
		for id in sim_host.dug_in_units():
			dug[id] = true
	var atmosphere: Node = main.atmosphere
	var low: bool = atmosphere != null and atmosphere.quality == "low"
	var soldier_range := XRAY_SOLDIERS_LOW_M if low else XRAY_SOLDIERS_M
	# By the size on the screen, as the infantry view (prompt 217).
	soldier_range /= RtsCamera.detail_distance_factor(main.camera)
	var relations := {}
	var seen: Dictionary[int, bool] = {}
	xray_units = {}
	for i in ids.size():
		var id := ids[i]
		var owner := owners[i]
		var relation := 0
		if owner != me:
			if not relations.has(owner):
				relations[owner] = 1 if sim_host.relation(owner) == 1 else 2
			relation = relations[owner]
		var outline := outlined.has(id)
		# Enemies only while outlined (and once more to take it off).
		if relation == 2 and not outline and not _applied.has(id):
			continue
		var node: Node3D = main._unit_nodes.get(id)
		if node == null:
			continue
		seen[id] = true
		var xray: bool = relation != 2 and not off and (relation == 0 or mode == "own_allies") \
				and alive[i] != 0 and garrisons[i] < 0 and not dug.has(id) and node.visible \
				and not _is_structure(id, i)
		var soldiers: Array = view._by_unit.get(id, []) if view != null else []
		# Far squads: no silhouette (a few pixels tall; its draw calls cost
		# more than it shows).
		if xray and not soldiers.is_empty() \
				and node.global_position.distance_to(camera_position) > soldier_range:
			xray = false
		if xray:
			xray_units[id] = true
		var bright: bool = xray and (id == nearest or main.selection.is_selected(id))
		var mask: int = _excluded_by_unit.get(id, 0)
		var code := int(xray) | int(bright) << 1 | int(outline) << 2 | soldiers.size() << 3
		if _applied.get(id, -1) == code and _applied_mask.get(id, 0) == mask:
			continue
		_applied[id] = code
		_applied_mask[id] = mask
		_apply_overlays(id, node, relation, xray, bright, outline, soldiers, view)
		if relation == 2 and not outline:
			_applied.erase(id)
			_applied_mask.erase(id)
	if _applied.size() > seen.size():
		for id: int in _applied.keys():
			if not seen.has(id):
				_applied.erase(id)
				_applied_mask.erase(id)
				_outline_meshes.erase(id)


## Outline color and fade of every outlined unit, every frame (a few meshes).
func _color_outlines(ids: PackedInt64Array, owners: PackedInt32Array, me: int) -> void:
	if outlined.is_empty():
		return
	for id: int in outlined:
		var meshes: Array = _outline_meshes.get(id, [])
		if meshes.is_empty():
			continue
		var index := ids.find(id)
		if index < 0:
			continue
		var color: Color
		if owners[index] == me:
			color = own_mark_color()
		elif sim_host.relation(owners[index]) == 1:
			color = RelationColors.ally()
		else:
			color = RelationColors.enemy()
		color.a = OUTLINE_ALPHA * float(outlined[id])
		for mesh: MeshInstance3D in meshes:
			if is_instance_valid(mesh):
				mesh.set_instance_shader_parameter(&"outline_color", color)


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
## the squad model, `infantry_view.gd`) the silhouette and the outline on
## the body, the tint on every mesh; other units all on every mesh.
func _apply_overlays(id: int, node: Node3D, relation: int, xray: bool, bright: bool,
		outline: bool, soldiers: Array, view: Node) -> void:
	var color := RelationColors.own() if relation == 0 else RelationColors.ally()
	if bright:
		color = color.lerp(Color.WHITE, XRAY_LIGHTEN)
	color.a = XRAY_BRIGHT_ALPHA if bright else XRAY_ALPHA
	var outline_meshes: Array = []
	if soldiers.is_empty():
		for mesh: Node in node.find_children("*", "MeshInstance3D", true, false):
			if mesh.name != "SelectionRing":
				_set_overlay(mesh, relation, xray, outline, color)
				if outline:
					outline_meshes.append(mesh)
	else:
		for key: int in soldiers:
			var s: Object = view._soldiers.get(key)
			if s == null:
				continue
			var soldier_xray := xray and not _excluded.has(key)
			for mesh: GeometryInstance3D in s.meshes:
				if mesh is MeshInstance3D:
					var body := mesh.name == "Body"
					_set_overlay(mesh, relation, soldier_xray and body, outline and body, color)
					if outline and body:
						outline_meshes.append(mesh)
	if outline:
		_outline_meshes[id] = outline_meshes
	else:
		_outline_meshes.erase(id)


## The overlay chain of a mesh: tint (allies, enemies), silhouette,
## outline.
func _set_overlay(mesh: MeshInstance3D, relation: int, xray: bool, outline: bool, color: Color) -> void:
	var material: Material
	match relation:
		0:
			material = (_xray_outline if xray else _outline) if outline else (_xray if xray else null)
		1:
			if _ally_xray == null:
				material = main._ally_material
			elif outline:
				material = _ally_xray_outline if xray else _ally_outline
			else:
				material = _ally_xray if xray else main._ally_material
		_:
			material = _enemy_outline if outline and _enemy_outline != null else main._enemy_material
	mesh.material_overlay = material
	if xray:
		mesh.set_instance_shader_parameter(&"xray_color", color)


## One marker ring per living own soldier on the ground, bright while the
## show-all key is held. Rebuilt only when a new tick arrived or the key or
## the setting changed: at the
## half-way point between the two ticks, so a marker is at most a few
## centimeters from the soldier drawn between them (rebuilding every frame
## cost about 1.2 ms with 1100 soldiers).
func _update_markers(ids: PackedInt64Array, owners: PackedInt32Array, show_all: bool,
		mode: String, me: int) -> void:
	var multimesh := _markers.multimesh
	if mode == "off":
		multimesh.visible_instance_count = 0
		markers_drawn = 0
		markers_bright = 0
		_marker_key = []
		return
	var key := [sim_host.current_tick(), show_all, mode]
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
		var bright := show_all
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
