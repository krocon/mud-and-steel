extends Node3D
## Combat presentation (prompt 017): tracers for projectiles in flight,
## health bars, wrecks and turrets turned by the simulation's aim.
##
## Everything comes from SimHost; nothing here decides anything. Created by
## `main.gd`. The event stream (`SimHost.take_combat_events`) is read every
## frame and kept as counters and `recent_events`, so effects (prompt 019)
## and sound can hook in through the `combat_event` signal.

signal combat_event(event: Dictionary)

const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const Settings := preload("res://scenes/menu/settings.gd")

## How fast turrets follow the simulation's aim (presentation only).
const TURRET_TURN_RATE := 10.0
## Tracer length per m/s of projectile speed (seconds of flight shown).
const TRACER_SECONDS := 0.012
const TRACER_MIN_LENGTH := 2.0
## Tracer thickness per mm of caliber, meters.
const TRACER_WIDTH_PER_MM := 0.004
const TRACER_COLOR := Color(1.0, 0.75, 0.3)
## Darkening over wrecks.
const WRECK_TINT := Color(0.05, 0.04, 0.03, 0.72)
const RECENT_EVENTS := 64
## Tracers start at the model's muzzle (prompt 077): the drawn shell is
## shifted from the simulation's muzzle to the socket by an offset that
## fades over this flight distance (m); the simulated path is untouched.
const MUZZLE_BLEND_M := 40.0

## Time of the last `_process` in microseconds, effects of the events
## included (benchmark, prompt 016).
var process_usec := 0
## Turret node per unit id (null: none).
var _turrets: Dictionary[int, Node3D] = {}
var main: Node3D
var sim_host: SimHost
## Counters of all events since the start, for tools and the self-test.
var shots := 0
var hits := 0
var penetrations := 0
var destroyed := 0
var recent_events: Array[Dictionary] = []
## Ids of units drawn as wrecks (or hidden, for infantry).
var wrecks: Dictionary[int, bool] = {}
var health_bars: Control

## Tracers drawn from a model's muzzle since the start (self-test).
var tracers_from_muzzle := 0

var _tracers: Array[MeshInstance3D] = []
## Projectile id -> {from (socket), offset (socket - simulation muzzle),
## sim (simulation muzzle), frame (last seen), seen} (prompt 077).
var _muzzles: Dictionary[int, Dictionary] = {}
var _tracer_mesh: BoxMesh
var _wreck_material: StandardMaterial3D


func setup(main_node: Node3D, hud: CanvasLayer) -> void:
	main = main_node
	sim_host = main.sim_host
	_tracer_mesh = BoxMesh.new()
	_tracer_mesh.size = Vector3(1, 1, 1)
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = TRACER_COLOR
	material.emission_enabled = true
	material.emission = TRACER_COLOR
	material.emission_energy_multiplier = 4.0
	_tracer_mesh.material = material
	_wreck_material = StandardMaterial3D.new()
	_wreck_material.albedo_color = WRECK_TINT
	_wreck_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	health_bars = HealthBars.new()
	health_bars.view = self
	health_bars.name = "HealthBars"
	hud.add_child(health_bars)
	hud.move_child(health_bars, 0)


func _process(delta: float) -> void:
	if sim_host == null:
		return
	var began := Time.get_ticks_usec()
	_read_events()
	_draw_tracers()
	_update_units(delta)
	process_usec = Time.get_ticks_usec() - began


func _read_events() -> void:
	for event: Dictionary in sim_host.take_combat_events():
		match event["type"]:
			"shot":
				shots += 1
			"hit":
				hits += 1
				if event["penetrated"] and event["surface"] == "unit":
					penetrations += 1
			"destroyed":
				destroyed += 1
		recent_events.append(event)
		if recent_events.size() > RECENT_EVENTS:
			recent_events.pop_front()
		combat_event.emit(event)


## The shell `projectile` leaves the model's muzzle at `from`, the
## simulation fired it at `sim_from` (effects.gd, prompt 077).
func note_muzzle(projectile: int, from: Vector3, sim_from: Vector3) -> void:
	_muzzles[projectile] = {"from": from, "offset": from - sim_from, "sim": sim_from,
		"frame": Engine.get_process_frames(), "seen": false}


## One stretched box per projectile, from its position along its flight,
## advanced by the time since the last tick; it starts at the model's
## muzzle and never reaches back behind it.
func _draw_tracers() -> void:
	var positions := sim_host.projectile_positions()
	var velocities := sim_host.projectile_velocities()
	var calibers := sim_host.projectile_calibers()
	var ids := sim_host.projectile_ids() if not _muzzles.is_empty() else PackedInt64Array()
	var frame := Engine.get_process_frames()
	var ahead := sim_host.interpolation_alpha() / float(sim_host.tick_rate())
	while _tracers.size() < positions.size():
		var tracer := MeshInstance3D.new()
		tracer.mesh = _tracer_mesh
		tracer.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(tracer)
		_tracers.append(tracer)
	for i in _tracers.size():
		var tracer := _tracers[i]
		tracer.visible = i < positions.size()
		if not tracer.visible:
			continue
		var velocity := velocities[i]
		var speed := velocity.length()
		if speed < 0.001:
			tracer.visible = false
			continue
		var head := positions[i] + velocity * ahead
		var length := maxf(TRACER_MIN_LENGTH, speed * TRACER_SECONDS)
		var width := maxf(0.03, calibers[i] * TRACER_WIDTH_PER_MM)
		var direction := velocity / speed
		if i < ids.size() and _muzzles.has(ids[i]):
			var muzzle: Dictionary = _muzzles[ids[i]]
			if not muzzle.seen:
				muzzle.seen = true
				tracers_from_muzzle += 1
			muzzle.frame = frame
			var flown: float = (head - muzzle.sim).length()
			head += (muzzle.offset as Vector3) * clampf(1.0 - flown / MUZZLE_BLEND_M, 0.0, 1.0)
			length = minf(length, maxf(0.05, (head - (muzzle.from as Vector3)).dot(direction)))
		var up := Vector3.UP if absf(direction.y) < 0.99 else Vector3.RIGHT
		var basis := Basis.looking_at(direction, up)
		tracer.transform = Transform3D(basis.scaled_local(Vector3(width, width, length)), head - direction * length * 0.5)
	# Shells that are gone (hit or out of range) drop their muzzle.
	if frame % 30 == 0:
		for id: int in _muzzles.keys():
			if frame - int(_muzzles[id].frame) > 30:
				_muzzles.erase(id)


## Turrets follow the aim, wrecks are darkened, dead infantry disappears.
func _update_units(delta: float) -> void:
	var ids := sim_host.unit_ids()
	var alive := sim_host.unit_alive()
	var facings := sim_host.unit_facing_yaws()
	var aims := sim_host.unit_aim_yaws()
	var turn := clampf(delta * TURRET_TURN_RATE, 0.0, 1.0)
	for i in ids.size():
		var node: Node3D = main._unit_nodes.get(ids[i])
		if node == null:
			continue
		if alive[i] == 0:
			if not wrecks.has(ids[i]):
				_make_wreck(node, (sim_host.unit_type_ids()[i] as String).contains("_inf_"))
				wrecks[ids[i]] = true
				main.selection.selected.erase(ids[i])
			continue
		var turret := _turret_of(ids[i], node)
		if turret != null:
			if not turret.has_meta(&"rest_yaw"):
				turret.set_meta(&"rest_yaw", turret.rotation.y)
			var target := float(turret.get_meta(&"rest_yaw")) + wrapf(aims[i] - facings[i], -PI, PI)
			turret.rotation.y = lerp_angle(turret.rotation.y, target, turn)


## The model's turret, searched once per unit: a recursive search every
## frame went through the hundreds of skeleton nodes of every squad
## (prompt 016).
func _turret_of(id: int, node: Node3D) -> Node3D:
	if not _turrets.has(id):
		_turrets[id] = node.find_child("Turret", true, false) as Node3D
	return _turrets[id]


func _make_wreck(node: Node3D, infantry: bool) -> void:
	(node.get_node("SelectionRing") as Node3D).visible = false
	# Fallen soldiers stay on the field (prompt 022, `infantry_view.gd`).
	if infantry:
		return
	_tint(node, _wreck_material)


func _tint(node: Node, material: Material) -> void:
	var mesh := node as MeshInstance3D
	if mesh != null and node.name != "SelectionRing":
		mesh.material_overlay = material
	for child in node.get_children():
		_tint(child, material)


## Health bars over damaged and selected units, drawn on the HUD; the
## setting "Health bars" limits them to selected units or hides them.
class HealthBars:
	extends Control

	const WIDTH := 34.0
	const HEIGHT := 4.0
	## Meters above the ground.
	const LIFT := 3.6

	var view: Node3D
	## Bars drawn in the last frame (for the self-test).
	var drawn := 0

	func _ready() -> void:
		mouse_filter = Control.MOUSE_FILTER_IGNORE
		set_anchors_preset(Control.PRESET_FULL_RECT)

	func _process(_delta: float) -> void:
		queue_redraw()

	func _draw() -> void:
		drawn = 0
		# Setting "Health bars" (prompt 031): damaged, selected or never.
		var mode: String = Settings.get_value("health_bars")
		if mode == "never":
			return
		var damaged_too := mode != "selected"
		var sim_host: SimHost = view.sim_host
		var camera: Camera3D = view.main.camera
		var selection: Control = view.main.selection
		var ids := sim_host.unit_ids()
		var health := sim_host.unit_health()
		var alive := sim_host.unit_alive()
		var owners := sim_host.unit_owners()
		var me := sim_host.local_player()
		for i in ids.size():
			if alive[i] == 0 or ((health[i] >= 1.0 or not damaged_too) and not selection.is_selected(ids[i])):
				continue
			var node: Node3D = view.main._unit_nodes.get(ids[i])
			if node == null:
				continue
			var world := node.global_position + Vector3.UP * LIFT
			if camera.is_position_behind(world):
				continue
			var at := camera.unproject_position(world) - Vector2(WIDTH * 0.5, 0)
			var color := RelationColors.own_or(Color(0.3, 0.9, 0.3)) if owners[i] == me else (
				RelationColors.ally_or(Color(0.35, 0.6, 1.0)) if sim_host.relation(owners[i]) == 1 else RelationColors.enemy_or(Color(0.95, 0.3, 0.2)))
			draw_rect(Rect2(at - Vector2(1, 1), Vector2(WIDTH + 2, HEIGHT + 2)), Color(0, 0, 0, 0.6))
			draw_rect(Rect2(at, Vector2(WIDTH * health[i], HEIGHT)), color)
			drawn += 1
