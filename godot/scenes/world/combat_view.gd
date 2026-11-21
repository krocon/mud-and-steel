extends Node3D
## Combat presentation (prompt 017): tracers for projectiles in flight,
## health bars, wrecks and turrets turned by the simulation's aim.
##
## Everything comes from SimHost; nothing here decides anything. Created by
## `main.gd`. The event stream (`SimHost.take_combat_events`) is read every
## frame and kept as counters and `recent_events`, so effects (prompt 019)
## and sound can hook in through the `combat_event` signal.
##
## Prompt 105: state marks over the units (`hud/state_marks.gd`, beside the
## health bars), and a round that bounces off armor flies away as a short
## tracer at the angle of reflection (shot direction from the shooter, the
## hit face of the target; a little upward kick and spread from the
## projectile id, the same in every replay).

signal combat_event(event: Dictionary)

const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const Settings := preload("res://scenes/menu/settings.gd")
const StateMarks := preload("res://scenes/hud/state_marks.gd")
const KillSequence := preload("res://scenes/world/kill_sequence.gd")

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
## A shell that hits within the tick it is fired (closer than one tick of
## flight: 75 m for a 750 m/s gun since prompt 097 halved the ranges) never
## appears in a snapshot; it is drawn as a tracer from the muzzle to the
## hit for this long (seconds).
const QUICK_TRACER_S := 0.07
## Ricochets (prompt 105): from this caliber (mm), flying this fast (m/s,
## drawn), for this long (s), with a tail of this length (m).
const RICOCHET_MIN_MM := 20.0
const RICOCHET_SPEED := 160.0
const RICOCHET_S := 0.35
const RICOCHET_TAIL_M := 4.0

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
## State marks over the units (prompt 105).
var state_marks: Control
## Ricochet tracers started since the start (self-test).
var ricochets := 0

## Tracers drawn from a model's muzzle since the start (self-test).
var tracers_from_muzzle := 0

var _tracers: Array[MeshInstance3D] = []
## Tracers of shells that hit in the tick they were fired:
## {node, from, to, width, until (msec)}.
var _quick: Array[Dictionary] = []
## Ricochet tracers: {node, from, dir, width, born (msec)}.
var _ricochets: Array[Dictionary] = []
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
	state_marks = StateMarks.new()
	state_marks.view = self
	state_marks.name = "StateMarks"
	hud.add_child(state_marks)
	hud.move_child(state_marks, 1)


func _process(delta: float) -> void:
	if sim_host == null:
		return
	var began := Time.get_ticks_usec()
	_read_events()
	_draw_tracers()
	_draw_quick_tracers()
	_draw_ricochets()
	_update_units(delta)
	process_usec = Time.get_ticks_usec() - began


func _read_events() -> void:
	for event: Dictionary in sim_host.take_combat_events():
		match event["type"]:
			"shot":
				shots += 1
			"hit":
				hits += 1
				_quick_tracer(event)
				if event["penetrated"] and event["surface"] == "unit":
					penetrations += 1
				elif event["surface"] == "unit":
					ricochet(event)
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


## A shell that hit before any snapshot showed it in flight: a short-lived
## tracer from the model's muzzle to the hit.
func _quick_tracer(event: Dictionary) -> void:
	var muzzle: Dictionary = _muzzles.get(int(event["projectile"]), {})
	if muzzle.is_empty() or muzzle.seen:
		return
	muzzle.seen = true
	tracers_from_muzzle += 1
	var tracer := MeshInstance3D.new()
	tracer.mesh = _tracer_mesh
	tracer.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(tracer)
	var width := maxf(0.03, float(event.get("caliber_mm", 0.0)) * TRACER_WIDTH_PER_MM)
	_quick.append({"node": tracer, "from": muzzle.from, "to": event["position"], "width": width,
		"born": Time.get_ticks_msec()})


func _draw_quick_tracers() -> void:
	var now := Time.get_ticks_msec()
	var keep: Array[Dictionary] = []
	for q in _quick:
		var age := float(now - int(q.born)) / 1000.0
		var tracer: MeshInstance3D = q.node
		if age >= QUICK_TRACER_S:
			tracer.queue_free()
			continue
		keep.append(q)
		# The shell runs from the muzzle to the hit over the drawing time.
		var from: Vector3 = q.from
		var to: Vector3 = q.to
		var span := to - from
		var total := span.length()
		if total < 0.01:
			tracer.visible = false
			continue
		var direction := span / total
		var head := from + span * clampf(age / QUICK_TRACER_S + 0.3, 0.0, 1.0)
		tracer.visible = _visible_at(head)
		if not tracer.visible:
			continue
		var length := minf(maxf(TRACER_MIN_LENGTH, total * 0.3), (head - from).length())
		var up := Vector3.UP if absf(direction.y) < 0.99 else Vector3.RIGHT
		var basis := Basis.looking_at(direction, up)
		var w: float = q.width
		tracer.transform = Transform3D(basis.scaled_local(Vector3(w, w, length)), head - direction * length * 0.5)
	_quick = keep


## A round bounced off a unit (`hit` event, not penetrated): a short tracer
## flies away at the angle of reflection off the hit face. Infantry and
## small arms give none; nothing where the player does not see.
func ricochet(event: Dictionary) -> void:
	var caliber := float(event.get("caliber_mm", 0.0))
	var at: Vector3 = event["position"]
	if caliber < RICOCHET_MIN_MM or not _visible_at(at):
		return
	var ids := sim_host.unit_ids()
	var target := ids.find(int(event.get("target", -1)))
	if target < 0 or (sim_host.unit_type_ids()[target] as String).contains("_inf_"):
		return
	var shooter := ids.find(int(event.get("shooter", -1)))
	var from: Vector3 = sim_host.current_positions()[shooter] + Vector3.UP * 1.8 if shooter >= 0 \
		else event.get("from", at - Vector3.FORWARD * 50.0)
	var projectile := int(event.get("projectile", 0))
	var tick := int(event.get("tick", 0))
	var direction := ricochet_direction(at - from, str(event.get("face", "")), sim_host.unit_facing_yaws()[target],
		KillSequence.roll(projectile, tick, 7), KillSequence.roll(projectile, tick, 8))
	var tracer := MeshInstance3D.new()
	tracer.mesh = _tracer_mesh
	tracer.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	tracer.visible = false
	add_child(tracer)
	_ricochets.append({"node": tracer, "from": at, "dir": direction,
		"width": maxf(0.03, caliber * TRACER_WIDTH_PER_MM * 0.8), "born": Time.get_ticks_msec()})
	ricochets += 1


## Where a round coming along `incoming` flies after bouncing off face
## `face` ("front", "side", "rear", "top") of a hull turned to `yaw`
## (Godot yaw of a model facing -Z): mirrored at the face, kicked upwards
## and spread a little by the rolls `a` and `b` (0..1).
static func ricochet_direction(incoming: Vector3, face: String, yaw: float, a: float, b: float) -> Vector3:
	var d := incoming.normalized() if incoming.length() > 0.001 else Vector3.FORWARD
	var forward := Vector3(-sin(yaw), 0.0, -cos(yaw))
	var normal := Vector3.UP
	match face:
		"front":
			normal = forward
		"rear":
			normal = -forward
		"side":
			normal = forward.cross(Vector3.UP)
	if normal.dot(d) > 0.0:
		normal = -normal
	var out := d - 2.0 * d.dot(normal) * normal
	out.y = absf(out.y) + 0.15 + 0.3 * a
	return out.normalized().rotated(Vector3.UP, (b - 0.5) * 0.6)


func _draw_ricochets() -> void:
	var now := Time.get_ticks_msec()
	for i in range(_ricochets.size() - 1, -1, -1):
		var r: Dictionary = _ricochets[i]
		var age := float(now - int(r.born)) / 1000.0
		var tracer: MeshInstance3D = r.node
		if age >= RICOCHET_S:
			tracer.queue_free()
			_ricochets.remove_at(i)
			continue
		var direction: Vector3 = r.dir
		var flown := RICOCHET_SPEED * age
		var length := minf(RICOCHET_TAIL_M, maxf(flown, 0.05))
		var head: Vector3 = r.from + direction * flown
		var up := Vector3.UP if absf(direction.y) < 0.99 else Vector3.RIGHT
		var w: float = r.width
		tracer.visible = true
		tracer.transform = Transform3D(Basis.looking_at(direction, up).scaled_local(Vector3(w, w, length)),
			head - direction * length * 0.5)


## Whether the local player sees `at` (fog of war, `effects.visible_at`).
func _visible_at(at: Vector3) -> bool:
	var effects: Node = main.get("effects")
	return effects == null or effects.visible_at.call(at)


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
		# Only where the local player sees (prompt 098): a tracer in the
		# fog would give the shooter away.
		if not _visible_at(head):
			tracer.visible = false
			continue
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
