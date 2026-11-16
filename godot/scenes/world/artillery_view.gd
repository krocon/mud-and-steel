extends Node3D
## Indirect fire and smoke (prompt 032): shells in their arc, impact
## markers for the local player's own shells, and smoke clouds.
##
## Presentation only: everything comes from SimHost (`shell_*`,
## `smoke_*`); the simulation decides where shells land and which cells the
## smoke hides. Muzzle flashes and explosions come from the ordinary `shot`
## and `hit` events (`effects.gd`). Sound (prompt 029) can hook into
## `shell_incoming` for the whistle of a shell about to land. Godot
## randomness only places smoke puffs and never reaches the simulation.

## Emitted once per shell shortly before it lands (whistle).
signal shell_incoming(position: Vector3, caliber_mm: float, smoke: bool)

const EffectTextures := preload("res://scenes/world/effect_textures.gd")
const Effects := preload("res://scenes/world/effects.gd")
## Seconds before the impact at which `shell_incoming` fires.
const WHISTLE_SECONDS := 1.5
## Drawn shell size per mm of caliber (exaggerated so it can be seen).
const SHELL_SIZE_PER_MM := 0.012
const SHELL_COLOR := Color(0.12, 0.12, 0.1)
const MARKER_COLOR := Color(1.0, 0.35, 0.2, 0.8)
const MARKER_SMOKE_COLOR := Color(0.85, 0.85, 0.85, 0.8)
## Puffs per cloud and their look.
const PUFFS := 9
## Smoke trail of vehicle rockets (prompt 062): puffs this far apart (s)
## while the motor burns, the first seconds of the flight.
const TRAIL_INTERVAL_S := 0.08
const TRAIL_BURN_S := 1.2
const SMOKE_COLOR := Color(0.78, 0.78, 0.76)
## Fire smoke (prompt 041): dark, oily.
const FIRE_SMOKE_COLOR := Color(0.16, 0.15, 0.14)
const SMOKE_OPACITY := 0.85

var main: Node3D
var sim_host: SimHost
## Shells drawn in the last frame and smoke clouds shown (self-test).
var shells_drawn := 0
var clouds_drawn := 0
var incoming := 0

var _shells: Array[MeshInstance3D] = []
var _markers: Array[MeshInstance3D] = []
var _shell_mesh: SphereMesh
var _marker_mesh: TorusMesh
var _marker_material: StandardMaterial3D
var _marker_smoke_material: StandardMaterial3D
var _puff_mesh: QuadMesh
var _dark_puff_mesh: QuadMesh
## Cloud id -> its puffs (MeshInstance3D) and their offsets (x, y, z in
## units of the radius).
var _clouds: Dictionary[int, Array] = {}
var _whistled: Dictionary[int, bool] = {}
## Rocket id -> time (ms) of its last trail puff (prompt 062).
var _trail_last: Dictionary[int, int] = {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_shell_mesh = SphereMesh.new()
	_shell_mesh.radius = 0.5
	_shell_mesh.height = 1.6
	_shell_mesh.radial_segments = 8
	_shell_mesh.rings = 4
	var shell_material := StandardMaterial3D.new()
	shell_material.albedo_color = SHELL_COLOR
	_shell_mesh.material = shell_material
	_marker_mesh = TorusMesh.new()
	_marker_mesh.inner_radius = 0.85
	_marker_mesh.outer_radius = 1.0
	_marker_mesh.rings = 24
	_marker_mesh.ring_segments = 4
	_marker_material = _flat_material(MARKER_COLOR)
	_marker_smoke_material = _flat_material(MARKER_SMOKE_COLOR)
	_puff_mesh = QuadMesh.new()
	_puff_mesh.size = Vector2(1, 1)
	var puff := StandardMaterial3D.new()
	puff.albedo_texture = EffectTextures.smoke()
	puff.albedo_color = SMOKE_COLOR
	puff.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	puff.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	puff.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	puff.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
	_puff_mesh.material = puff
	_dark_puff_mesh = QuadMesh.new()
	_dark_puff_mesh.size = Vector2(1, 1)
	var dark := puff.duplicate() as StandardMaterial3D
	dark.albedo_color = FIRE_SMOKE_COLOR
	_dark_puff_mesh.material = dark


func _flat_material(color: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.emission_enabled = true
	m.emission = Color(color.r, color.g, color.b)
	return m


func _process(_delta: float) -> void:
	if sim_host == null:
		return
	_draw_shells()
	_draw_smoke()


## Shells along their arc (advanced by the time since the last tick) and,
## for the local player's shells, a ring where they will land.
func _draw_shells() -> void:
	var positions := sim_host.shell_positions()
	var velocities := sim_host.shell_velocities()
	var targets := sim_host.shell_targets()
	var owners := sim_host.shell_owners()
	var ids := sim_host.shell_ids()
	var left := sim_host.shell_seconds_left()
	var calibers := sim_host.shell_calibers()
	var me := sim_host.local_player()
	var ahead := sim_host.interpolation_alpha() / float(sim_host.tick_rate())
	_grow(_shells, positions.size(), _shell_mesh)
	_grow(_markers, positions.size(), _marker_mesh)
	var live: Dictionary[int, bool] = {}
	var pulse := 0.85 + 0.15 * sin(Time.get_ticks_msec() * 0.01)
	for i in _shells.size():
		var shown := i < positions.size()
		_shells[i].visible = shown
		_markers[i].visible = shown and owners[i] == me
		if not shown:
			continue
		var caliber := absf(calibers[i])
		var smoke := calibers[i] < 0.0
		var velocity := velocities[i]
		var at := positions[i] + velocity * ahead
		var size := maxf(0.4, caliber * SHELL_SIZE_PER_MM)
		var up := Vector3.UP if absf(velocity.normalized().y) < 0.99 else Vector3.RIGHT
		var basis := Basis.looking_at(velocity.normalized(), up) if velocity.length() > 0.01 else Basis.IDENTITY
		_shells[i].transform = Transform3D((basis * Basis(Vector3.RIGHT, PI * 0.5)).scaled_local(Vector3.ONE * size), at)
		if _markers[i].visible:
			var radius := maxf(2.0, caliber * 0.1)
			_markers[i].material_override = _marker_smoke_material if smoke else _marker_material
			_markers[i].transform = Transform3D(Basis.IDENTITY.scaled(Vector3(radius, 0.2, radius) * pulse), targets[i] + Vector3.UP * 0.3)
		live[ids[i]] = true
		_rocket_trail(ids[i], at, velocity)
		if left[i] <= WHISTLE_SECONDS and not _whistled.has(ids[i]):
			_whistled[ids[i]] = true
			incoming += 1
			shell_incoming.emit(targets[i], caliber, smoke)
	for id in _whistled.keys():
		if not live.has(id):
			_whistled.erase(id)
	for id in _trail_last.keys():
		if not live.has(id):
			_trail_last.erase(id)
	shells_drawn = positions.size()


## Smoke trail of a rocket of a vehicle launcher while its motor burns
## (prompt 062; the launch is marked by `effects.gd`). Prompt 076 builds
## the real rocket look.
func _rocket_trail(id: int, at: Vector3, velocity: Vector3) -> void:
	var effects := main.get("effects") as Effects if main != null else null
	if effects == null or effects.rocket_age(id) > TRAIL_BURN_S:
		return
	var now := Time.get_ticks_msec()
	if now - int(_trail_last.get(id, -100000)) < int(TRAIL_INTERVAL_S * 1000.0):
		return
	_trail_last[id] = now
	effects.rocket_trail(at, velocity)


func _grow(pool: Array[MeshInstance3D], count: int, mesh: Mesh) -> void:
	while pool.size() < count:
		var node := MeshInstance3D.new()
		node.mesh = mesh
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(node)
		pool.append(node)


## Every cloud as a few billboard puffs that drift with it and fade out at
## the end of its life.
func _draw_smoke() -> void:
	var clouds := sim_host.smoke_clouds()
	var ids := sim_host.smoke_ids()
	var life := sim_host.smoke_life()
	var dark := sim_host.smoke_dark()
	var live: Dictionary[int, bool] = {}
	for i in clouds.size():
		var id := ids[i]
		live[id] = true
		if not _clouds.has(id):
			_clouds[id] = _make_cloud(id, i < dark.size() and dark[i] != 0)
		var c := clouds[i]
		var center := Vector3(c.x, c.y, c.z)
		var radius := c.w
		var fade := clampf(life[i] * 4.0, 0.0, 1.0)
		for puff: Array in _clouds[id]:
			var node: MeshInstance3D = puff[0]
			var offset: Vector3 = puff[1]
			node.position = center + Vector3(offset.x * radius, (0.35 + offset.y) * radius, offset.z * radius)
			node.scale = Vector3.ONE * radius * (1.1 + 0.3 * offset.y)
			node.transparency = 1.0 - SMOKE_OPACITY * fade
	for id in _clouds.keys():
		if not live.has(id):
			for puff: Array in _clouds[id]:
				(puff[0] as Node).queue_free()
			_clouds.erase(id)
	clouds_drawn = clouds.size()


func _make_cloud(id: int, is_dark := false) -> Array:
	var rng := RandomNumberGenerator.new()
	rng.seed = id * 7919 + 17
	var puffs: Array = []
	for k in PUFFS:
		var node := MeshInstance3D.new()
		node.mesh = _dark_puff_mesh if is_dark else _puff_mesh
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(node)
		var angle := rng.randf() * TAU
		var distance := sqrt(rng.randf()) * 0.6 if k > 0 else 0.0
		puffs.append([node, Vector3(cos(angle) * distance, rng.randf() * 0.4, sin(angle) * distance)])
	return puffs
