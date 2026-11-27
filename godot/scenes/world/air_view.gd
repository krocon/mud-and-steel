extends Node3D
## Air support (prompt 033): aircraft with their catalog models, shadows on
## the ground, spinning propellers, smoke trails of damaged aircraft, flak
## bursts and tracers, strafing tracers, falling bombs, wrecks of crashed
## aircraft, the local player's own sorties (route and time to target), the
## "enemy aircraft" warning, and the call bar (`hud/air_bar.gd`). Sortie
## labels (mission and seconds to target) show only for a few seconds after
## the call and just before the aircraft arrive; nearby sorties of one
## mission share one label.
##
## Presentation only: everything comes from SimHost (`aircraft_*`,
## `bomb_*`, `air_sorties`, `take_air_events`). The simulation decides
## where aircraft fly, what flak hits and where bombs land; explosions and
## dust come from the ordinary `hit` events (`effects.gd`). Godot
## randomness only scatters tracers and smoke puffs. Tracers are light
## streaks like those of ground fire (prompt 199, `light_streak.gd`).

## Every air event as read (sound, prompt 053: flak, strafing, crashes).
signal air_event(event: Dictionary)

const FogOfWar := preload("res://scenes/world/fog_of_war.gd")
const LightStreak := preload("res://scenes/world/light_streak.gd")
## Tracer of flak and strafing (prompt 199): real width, held on screen
## between TRACER_MIN_PX and TRACER_MAX_PX, at most TRACER_LENGTH_M long.
const TRACER_WIDTH_M := 0.12
const TRACER_LENGTH_M := 25.0
const TRACER_MIN_PX := 1.4
const TRACER_MAX_PX := 3.0
const TRACER_COLOR := Color(1.0, 0.85, 0.45)
const TRACER_ENERGY := 3.5
const EffectTextures := preload("res://scenes/world/effect_textures.gd")
const AirBar := preload("res://scenes/hud/air_bar.gd")
const MODELS_DIR := "res://assets/models/"
## Propeller turns per second (drawn).
const PROPELLER_TURNS := 22.0
## Damaged aircraft below this share of their hit points trail smoke.
const SMOKE_BELOW := 0.5
## Banked turns (circling recon): the bank of a coordinated turn,
## atan(speed × turn rate / g), drawn this many times steeper so a wide
## circle still reads as a turn, at most MAX_BANK, eased at BANK_EASE per
## second.
const BANK_EXAGGERATION := 2.0
const MAX_BANK := 0.7
const BANK_EASE := 3.0
const PUFF_SECONDS := 2.5
const TRAIL_INTERVAL := 0.06
const TRACER_SECONDS := 0.15
## A new enemy aircraft warns at most this often.
const WARN_SECONDS := 15.0
## Wrecks burn and smoke this long.
const WRECK_SECONDS := 90.0
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const ENEMY_TINT := Color(0.85, 0.2, 0.15, 0.18)
const ROUTE_COLOR := Color(0.55, 0.85, 1.0, 0.85)
const SHADOW_COLOR := Color(0.0, 0.0, 0.0, 0.45)
## Mission names: translation key per mission code.
const MISSION_NAMES := {
	"recon": "HUD_AIR_MISSION_RECON", "strafe": "HUD_AIR_MISSION_STRAFE", "attack": "HUD_AIR_MISSION_ATTACK",
	"bomb": "HUD_AIR_MISSION_BOMB", "airborne": "HUD_AIR_MISSION_AIRBORNE",
	"recon_loiter": "HUD_AIR_MISSION_RECON_LOITER",
}

var main: Node3D
var sim_host: SimHost
var air_bar: AirBar
## Counters for the self-test.
var aircraft_drawn := 0
var bombs_drawn := 0
var flak_bursts := 0
var strafe_bursts := 0
var crashes := 0
var warnings := 0
## Sortie labels shown (one per group of nearby targets).
var sortie_labels := 0
## Sortie id -> `_now` when it was first drawn (age of its label).
var _sortie_seen: Dictionary[int, float] = {}

## Aircraft id -> its node (child "Model"; meta "props", "bombs", "shadow").
var _planes: Dictionary[int, Node3D] = {}
var _bombs: Array[MeshInstance3D] = []
## [node, age, life, growth]
var _puffs: Array = []
## [node, age]
var _tracers: Array = []
## [node, age, next puff]
var _wrecks: Array = []
var _trail_left: Dictionary[int, float] = {}
## Aircraft id -> [last heading (Vector3), drawn bank (rad)].
var _bank: Dictionary[int, Array] = {}
var _seen_enemies: Dictionary[int, bool] = {}
var _last_warning := -1e9
var _now := 0.0
var _puff_mesh: QuadMesh
var _puff_materials: Dictionary[Color, StandardMaterial3D] = {}
var _flash_mesh: SphereMesh
var _bomb_mesh: CapsuleMesh
var _tracer_mesh: QuadMesh
var _tracer_material: ShaderMaterial
var _shadow_mesh: QuadMesh
var _ring_mesh: TorusMesh
var _enemy_material: StandardMaterial3D
var _wreck_material: StandardMaterial3D
var _route: MeshInstance3D
var _route_mesh: ImmediateMesh
var _rings: Array[MeshInstance3D] = []
var _labels: Array[Label3D] = []
## Rings, route lines and labels of the sorties (interface; off in the
## trailer recordings).
var marks_enabled := true


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	_puff_mesh = QuadMesh.new()
	_puff_mesh.size = Vector2(1, 1)
	var puff := StandardMaterial3D.new()
	puff.albedo_texture = EffectTextures.smoke()
	puff.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	puff.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED
	puff.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	puff.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
	_puff_mesh.material = puff
	_flash_mesh = SphereMesh.new()
	_flash_mesh.radius = 0.5
	_flash_mesh.height = 1.0
	_flash_mesh.radial_segments = 8
	_flash_mesh.rings = 4
	_flash_mesh.material = _glow(Color(1.0, 0.75, 0.35))
	_bomb_mesh = CapsuleMesh.new()
	_bomb_mesh.radius = 0.35
	_bomb_mesh.height = 2.0
	var bomb_material := StandardMaterial3D.new()
	bomb_material.albedo_color = Color(0.16, 0.17, 0.14)
	_bomb_mesh.material = bomb_material
	_tracer_mesh = LightStreak.mesh()
	_tracer_material = LightStreak.material(TRACER_COLOR, TRACER_ENERGY, TRACER_MIN_PX, TRACER_MAX_PX, 0.35)
	_tracer_material.render_priority = FogOfWar.MARKER_PRIORITY  # above the fog layer (prompt 098)
	_shadow_mesh = QuadMesh.new()
	_shadow_mesh.size = Vector2(1, 1)
	_shadow_mesh.orientation = PlaneMesh.FACE_Y
	var shadow := StandardMaterial3D.new()
	shadow.albedo_texture = EffectTextures.smoke()
	shadow.albedo_color = SHADOW_COLOR
	shadow.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	shadow.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	shadow.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
	_shadow_mesh.material = shadow
	_ring_mesh = TorusMesh.new()
	_ring_mesh.inner_radius = 0.92
	_ring_mesh.outer_radius = 1.0
	_ring_mesh.rings = 32
	_ring_mesh.ring_segments = 4
	_ring_mesh.material = _glow(ROUTE_COLOR)
	_enemy_material = StandardMaterial3D.new()
	_enemy_material.albedo_color = RelationColors.enemy_or(ENEMY_TINT)
	_enemy_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_wreck_material = StandardMaterial3D.new()
	_wreck_material.albedo_color = Color(0.05, 0.04, 0.03, 0.75)
	_wreck_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_route_mesh = ImmediateMesh.new()
	_route = MeshInstance3D.new()
	_route.mesh = _route_mesh
	_route.material_override = _glow(ROUTE_COLOR)
	_route.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_route)


## Puts the call tiles into the support column (call after the HUD exists).
func setup_hud(hud: Control) -> void:
	air_bar = AirBar.new()
	hud.add_child(air_bar)
	air_bar.setup(main, hud.support, hud.hints)


func _glow(color: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.emission_enabled = true
	m.emission = Color(color.r, color.g, color.b)
	m.render_priority = FogOfWar.MARKER_PRIORITY  # above the fog layer (prompt 098)
	return m


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_now += delta
	for event in sim_host.take_air_events():
		_on_event(event)
	_draw_planes(delta)
	_draw_bombs()
	_draw_sorties()
	_age_effects(delta)


# --- Aircraft ---------------------------------------------------------------


func _draw_planes(delta: float) -> void:
	var ids := sim_host.aircraft_ids()
	var positions := sim_host.aircraft_positions()
	var velocities := sim_host.aircraft_velocities()
	var owners := sim_host.aircraft_owners()
	var types := sim_host.aircraft_types()
	var health := sim_host.aircraft_health()
	var me := sim_host.local_player()
	var ahead := sim_host.interpolation_alpha() / float(sim_host.tick_rate())
	var live: Dictionary[int, bool] = {}
	for i in ids.size():
		var id := ids[i]
		live[id] = true
		if not _planes.has(id):
			_planes[id] = _make_plane(types[i], owners[i] != me)
			if owners[i] != me and not _seen_enemies.has(id):
				_seen_enemies[id] = true
				_warn(positions[i])
		var node := _planes[id]
		var velocity := velocities[i]
		var at := positions[i] + velocity * ahead
		var heading := velocity.normalized() if velocity.length() > 0.5 else Vector3.FORWARD
		var up := Vector3.UP if absf(heading.y) < 0.98 else Vector3.RIGHT
		var basis := Basis.looking_at(heading, up)
		var falling := health[i] < 0.0
		if falling:
			basis = basis * Basis(Vector3(0, 0, 1), _now * 4.0)
		else:
			basis = basis * Basis(Vector3(0, 0, 1), _bank_of(id, heading, velocity.length(), delta))
		node.global_transform = Transform3D(basis, at)
		for prop: Node3D in node.get_meta(&"props", []):
			prop.rotate_object_local(Vector3(0, 0, 1), TAU * PROPELLER_TURNS * delta)
		_place_shadow(node, at, heading, falling)
		if falling or health[i] < SMOKE_BELOW:
			_trail_left[id] = _trail_left.get(id, 0.0) - delta
			if _trail_left[id] <= 0.0:
				_trail_left[id] = TRAIL_INTERVAL
				var color := Color(0.12, 0.11, 0.1) if falling else Color(0.35, 0.34, 0.33)
				_puff(at - heading * 4.0, 2.0 if falling else 1.4, color, 0.8)
				if falling:
					_flash(at - heading * 2.0, 1.5, 0.08)
	for id in _planes.keys():
		if not live.has(id):
			(_planes[id] as Node).queue_free()
			_planes.erase(id)
			_trail_left.erase(id)
			_bank.erase(id)
	aircraft_drawn = ids.size()


## Bank (roll about the local Z axis, positive: left wing down) of aircraft
## `id` from how fast its heading turns about the vertical.
func _bank_of(id: int, heading: Vector3, speed: float, delta: float) -> float:
	var state: Array = _bank.get(id, [heading, 0.0])
	var last: Vector3 = state[0]
	var flat_last := Vector3(last.x, 0.0, last.z).normalized()
	var flat_now := Vector3(heading.x, 0.0, heading.z).normalized()
	var rate := 0.0
	if delta > 0.0 and flat_last.length() > 0.5 and flat_now.length() > 0.5:
		rate = flat_last.signed_angle_to(flat_now, Vector3.UP) / delta
	var target := clampf(atan(speed * rate / 9.81) * BANK_EXAGGERATION, -MAX_BANK, MAX_BANK)
	var bank := lerpf(float(state[1]), target, clampf(BANK_EASE * delta, 0.0, 1.0))
	_bank[id] = [heading, bank]
	return bank


func _make_plane(type_id: String, enemy: bool) -> Node3D:
	var root := Node3D.new()
	root.name = "Aircraft"
	add_child(root)
	var faction := type_id.get_slice("_", 0)
	var path := "%s%s/%s.glb" % [MODELS_DIR, faction, type_id]
	var model: Node3D
	if not type_id.is_empty() and ResourceLoader.exists(path):
		model = (load(path) as PackedScene).instantiate() as Node3D
		UnitCamo.apply(model, UnitCamo.palette(faction, main.atmosphere.preset.camo_biome))
	else:
		var box := MeshInstance3D.new()
		var mesh := BoxMesh.new()
		mesh.size = Vector3(10, 1.2, 8)
		box.mesh = mesh
		model = box
	model.name = "Model"
	root.add_child(model)
	var props: Array[Node3D] = []
	var bombs: Array[Node3D] = []
	var extras: Variant = model.get_meta(&"extras", {})
	var retract := extras is Dictionary and bool((extras as Dictionary).get("gear_retractable", false))
	for child in model.find_children("*", "Node3D", true, false):
		var n := str(child.name)
		if n.begins_with("Propeller"):
			props.append(child)
		elif n.begins_with("Bomb_"):
			bombs.append(child)
		elif n.begins_with("Gear_") and retract:
			(child as Node3D).visible = false
		elif n == "ShadowProxy" and child is GeometryInstance3D:
			(child as GeometryInstance3D).cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY
	if enemy:
		for mesh in model.find_children("*", "MeshInstance3D", true, false):
			(mesh as MeshInstance3D).material_overlay = _enemy_material
	root.set_meta(&"props", props)
	root.set_meta(&"bombs", bombs)
	var span := sim_host.aircraft_wingspan(type_id)
	var shadow := MeshInstance3D.new()
	shadow.mesh = _shadow_mesh
	shadow.top_level = true
	shadow.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	shadow.set_meta(&"span", span if span > 0.0 else 10.0)
	root.add_child(shadow)
	root.set_meta(&"shadow", shadow)
	return root


## A soft dark patch on the ground where the sun throws the aircraft's
## shadow (directional shadows do not reach that far up).
func _place_shadow(node: Node3D, at: Vector3, heading: Vector3, falling: bool) -> void:
	var shadow: MeshInstance3D = node.get_meta(&"shadow")
	var light := Vector3.DOWN
	var sun: DirectionalLight3D = main.get("sun")
	if sun != null:
		light = -sun.global_transform.basis.z
	if light.y > -0.1:
		light = Vector3.DOWN
	var ground := sim_host.height_at(at.x, at.z)
	var t := (ground - at.y) / light.y
	var point := at + light * t
	point.y = sim_host.height_at(point.x, point.z) + 0.25
	var span: float = shadow.get_meta(&"span", 10.0)
	var height := maxf(at.y - point.y, 0.0)
	var blur := 1.0 + height / 250.0
	var yaw := atan2(-heading.x, -heading.z)
	shadow.global_transform = Transform3D(
		Basis(Vector3.UP, yaw).scaled(Vector3(span * blur, 1.0, span * 0.8 * blur)), point)
	shadow.transparency = clampf(height / 700.0, 0.0, 0.8) if not falling else 0.3


## Enemy aircraft came into sight: a message in the event feed and the
## spoken "Enemy aircraft sighted!" of the player's faction (prompt 244,
## `battle_sound.announce`; the alarm sound where it has no recording), at
## most every WARN_SECONDS.
func _warn(at: Vector3) -> void:
	if _now - _last_warning < WARN_SECONDS:
		return
	_last_warning = _now
	warnings += 1
	var hud: Node = main.get("hud")
	if hud != null and hud.get("event_feed") != null:
		hud.event_feed.post("air", tr("HUD_FEED_ENEMY_AIRCRAFT"), Vector3(at.x, sim_host.height_at(at.x, at.z), at.z))
	var sound: Node = main.get("battle_sound")
	if sound != null:
		sound.announce("enemy_aircraft")


# --- Bombs and sorties ------------------------------------------------------


func _draw_bombs() -> void:
	var positions := sim_host.bomb_positions()
	var velocities := sim_host.bomb_velocities()
	var ahead := sim_host.interpolation_alpha() / float(sim_host.tick_rate())
	while _bombs.size() < positions.size():
		var node := MeshInstance3D.new()
		node.mesh = _bomb_mesh
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(node)
		_bombs.append(node)
	for i in _bombs.size():
		var shown := i < positions.size()
		_bombs[i].visible = shown
		if not shown:
			continue
		var v := velocities[i]
		var dir := v.normalized() if v.length() > 0.1 else Vector3.DOWN
		var up := Vector3.UP if absf(dir.y) < 0.98 else Vector3.RIGHT
		# The capsule's axis is Y: turn it along the flight path.
		var basis := Basis.looking_at(dir, up) * Basis(Vector3.RIGHT, PI * 0.5)
		_bombs[i].transform = Transform3D(basis, positions[i] + v * ahead)
		# Only where the local player sees (prompt 098).
		var visible_at: Callable = main.effects.visible_at if main.get("effects") != null else Callable()
		if visible_at.is_valid() and not visible_at.call(positions[i]):
			_bombs[i].visible = false
	bombs_drawn = positions.size()


## The local player's sorties: a line from where the aircraft enter to the
## target, a ring on the target and the time until they arrive.
## Sorties of one mission whose targets lie this close (m) share one ring
## and one label ("Circling recon ×2: 155 s", the soonest arrival), so
## their labels never overlap.
const SORTIE_MERGE_M := 60.0
## A sortie label shows only briefly: LABEL_SECONDS after the call (then
## fades out over LABEL_FADE), and again over the last LABEL_ARRIVAL
## seconds before the aircraft reach the target. Ring and route stay.
const LABEL_SECONDS := 4.0
const LABEL_FADE := 1.0
const LABEL_ARRIVAL := 5.0


## Own sorties grouped by mission and nearby target: per group
## {mission, target, count, seconds (soonest arrival still ahead, 0: one
## is there)}.
func _sortie_groups(sorties: Array) -> Array[Dictionary]:
	var groups: Array[Dictionary] = []
	for s: Dictionary in sorties:
		var target: Vector3 = s.target
		var seconds: float = s.seconds
		var group: Dictionary = {}
		for g in groups:
			var at: Vector3 = g.target
			if g.mission == s.mission and Vector2(at.x - target.x, at.z - target.z).length() <= SORTIE_MERGE_M:
				group = g
				break
		var id: int = s.get("id", -1)
		if not _sortie_seen.has(id):
			_sortie_seen[id] = _now
		var age := _now - _sortie_seen[id]
		if group.is_empty():
			groups.append({"mission": s.mission, "target": target, "count": 1, "seconds": seconds, "age": age})
			continue
		group.count += 1
		group.age = minf(group.age, age)
		# The soonest arrival still ahead; 0 only while none is on its way.
		var best: float = group.seconds
		group.seconds = minf(best, seconds) if best > 0.0 and seconds > 0.0 else maxf(best, seconds)
	return groups


## Opacity of a sortie label: the youngest call of its group is `age`
## seconds old, the soonest aircraft `seconds` from the target.
func _label_alpha(age: float, seconds: float) -> float:
	var fresh := clampf((LABEL_SECONDS + LABEL_FADE - age) / LABEL_FADE, 0.0, 1.0)
	var arriving := 1.0 if seconds > 0.0 and seconds <= LABEL_ARRIVAL else 0.0
	return maxf(fresh, arriving)


func _draw_sorties() -> void:
	var sorties := sim_host.air_sorties()
	var groups := _sortie_groups(sorties)
	sortie_labels = 0
	_route_mesh.clear_surfaces()
	while _rings.size() < groups.size():
		var ring := MeshInstance3D.new()
		ring.mesh = _ring_mesh
		ring.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(ring)
		_rings.append(ring)
		var label := Label3D.new()
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		label.fixed_size = true
		label.pixel_size = 0.0012
		label.font_size = 28
		label.outline_size = 8
		label.modulate = ROUTE_COLOR
		label.no_depth_test = true
		# Drawn after every transparent surface (water, smoke), else the
		# river paints over it.
		label.render_priority = FogOfWar.MARKER_PRIORITY + 10
		add_child(label)
		_labels.append(label)
	for i in _rings.size():
		var shown := i < groups.size()
		_rings[i].visible = shown
		_labels[i].visible = shown
		if not shown:
			continue
		var g: Dictionary = groups[i]
		var at: Vector3 = g.target
		at.y = sim_host.height_at(at.x, at.z)
		var pulse := 0.9 + 0.1 * sin(_now * 6.0)
		_rings[i].transform = Transform3D(Basis.IDENTITY.scaled(Vector3(14, 0.3, 14) * pulse), at + Vector3.UP * 0.5)
		var title := tr(MISSION_NAMES.get(g.mission, str(g.mission)))
		if g.count > 1:
			title += " ×%d" % g.count
		var soonest: float = g.seconds
		_labels[i].text = title + ": " + tr("COMMON_SECONDS") % ceili(soonest) if soonest > 0.0 else title
		_labels[i].position = at + Vector3.UP * 12.0
		var alpha := _label_alpha(g.age, soonest)
		_labels[i].visible = alpha > 0.0
		_labels[i].modulate.a = ROUTE_COLOR.a * alpha
		_labels[i].outline_modulate.a = alpha
		if alpha > 0.0:
			sortie_labels += 1
	if not marks_enabled:
		_route_mesh.clear_surfaces()
		for i in _rings.size():
			_rings[i].visible = false
			_labels[i].visible = false
		sortie_labels = 0
	for id: int in _sortie_seen.keys():
		if not sorties.any(func(s: Dictionary) -> bool: return s.get("id", -1) == id):
			_sortie_seen.erase(id)
	var lines := false
	for s: Dictionary in sorties:
		var target: Vector3 = s.target
		target.y = sim_host.height_at(target.x, target.z)
		var seconds: float = s.seconds
		if seconds <= 0.0:
			continue
		if not lines:
			_route_mesh.surface_begin(Mesh.PRIMITIVE_LINES)
			lines = true
		var entry: Vector3 = s.entry
		var steps := 24
		for k in steps:
			if k % 2 == 1:
				continue
			for p in [k, k + 1]:
				var q: Vector3 = entry.lerp(target, float(p) / steps)
				q.y = sim_host.height_at(q.x, q.z) + 3.0
				_route_mesh.surface_add_vertex(q)
	if lines:
		_route_mesh.surface_end()


# --- Events -----------------------------------------------------------------


func _on_event(event: Dictionary) -> void:
	air_event.emit(event)
	match event["type"]:
		"flak":
			_flak(event)
		"strafe":
			_strafe(event)
		"released":
			var node: Node3D = _planes.get(int(event["aircraft"]))
			if node != null:
				for bomb: Node3D in node.get_meta(&"bombs", []):
					if bomb.visible:
						bomb.visible = false
						break
		"shot_down":
			var at: Vector3 = event["position"]
			if _planes.has(int(event["aircraft"])):
				_flash(at, 4.0, 0.15)
				_puff(at, 6.0, Color(0.1, 0.09, 0.08), 1.6)
			if int(event["owner"]) == sim_host.local_player():
				_post("lost", tr("HUD_FEED_AIRCRAFT_DOWN"), at)
			else:
				_post("air", tr("HUD_FEED_ENEMY_AIRCRAFT_DOWN"), at)
		"crashed":
			_crash(event)


func _flak(event: Dictionary) -> void:
	if not _planes.has(int(event["aircraft"])):
		return
	flak_bursts += 1
	var at: Vector3 = event["position"]
	match str(event["class"]):
		"heavy":
			_flash(at, 3.0, 0.08)
			_puff(at, 7.0, Color(0.08, 0.08, 0.08), 3.0)
		"light":
			_flash(at, 1.2, 0.06)
			_puff(at, 2.5, Color(0.3, 0.3, 0.3), 1.5)
			_tracer(_gun_position(int(event["unit"]), at), at)
		_:
			_tracer(_gun_position(int(event["unit"]), at), at)


func _gun_position(unit: int, fallback: Vector3) -> Vector3:
	var ids := sim_host.unit_ids()
	for i in ids.size():
		if ids[i] == unit:
			return sim_host.current_positions()[i] + Vector3.UP * 2.0
	return fallback + Vector3.DOWN * 100.0


func _strafe(event: Dictionary) -> void:
	if not _planes.has(int(event["aircraft"])):
		return
	strafe_bursts += 1
	var muzzle: Vector3 = event["muzzle"]
	var from: Vector3 = event["from"]
	var to: Vector3 = event["to"]
	for k in 4:
		var p := from.lerp(to, randf())
		p += Vector3(randf_range(-3, 3), 0.2, randf_range(-3, 3))
		_tracer(muzzle, p)


## A crashed aircraft: its model on the ground, dark, tilted, burning.
func _crash(event: Dictionary) -> void:
	crashes += 1
	var at: Vector3 = event["position"]
	var visible_at: Callable = main.effects.visible_at if main.get("effects") != null else Callable()
	if visible_at.is_valid() and not visible_at.call(at):
		return
	var type_id: String = event["aircraft_type"]
	var faction := type_id.get_slice("_", 0)
	var path := "%s%s/%s.glb" % [MODELS_DIR, faction, type_id]
	if type_id.is_empty() or not ResourceLoader.exists(path):
		return
	var model := (load(path) as PackedScene).instantiate() as Node3D
	add_child(model)
	model.global_transform = Transform3D(
		Basis(Vector3.UP, float(event["yaw"])) * Basis(Vector3(0, 0, 1), 0.35) * Basis(Vector3.RIGHT, -0.12),
		at + Vector3.DOWN * 0.6)
	for mesh in model.find_children("*", "MeshInstance3D", true, false):
		(mesh as MeshInstance3D).material_overlay = _wreck_material
	for child in model.find_children("Propeller*", "Node3D", true, false):
		(child as Node3D).visible = false
	_wrecks.append([model, 0.0, 0.0])


func _post(kind: String, text: String, at: Vector3) -> void:
	var hud: Node = main.get("hud")
	if hud != null and hud.get("event_feed") != null:
		hud.event_feed.post(kind, text, Vector3(at.x, sim_host.height_at(at.x, at.z), at.z))


# --- Short-lived effects ----------------------------------------------------


func _puff(at: Vector3, size: float, color: Color, life_scale: float) -> void:
	var node := MeshInstance3D.new()
	node.mesh = _puff_mesh
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(node)
	node.position = at
	node.scale = Vector3.ONE * size
	if not _puff_materials.has(color):
		var material := (_puff_mesh.material as StandardMaterial3D).duplicate() as StandardMaterial3D
		material.albedo_color = color
		_puff_materials[color] = material
	node.material_override = _puff_materials[color]
	_puffs.append([node, 0.0, PUFF_SECONDS * life_scale, size * 0.6])


func _flash(at: Vector3, size: float, life: float) -> void:
	var node := MeshInstance3D.new()
	node.mesh = _flash_mesh
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(node)
	node.position = at
	node.scale = Vector3.ONE * size
	_puffs.append([node, 0.0, life, 0.0])


func _tracer(from: Vector3, to: Vector3) -> void:
	var length := from.distance_to(to)
	if length < 1.0:
		return
	var node := LightStreak.node(_tracer_mesh, _tracer_material)
	add_child(node)
	# A short glowing streak that runs from the gun to the target.
	node.set_meta(&"from", from)
	node.set_meta(&"to", to)
	_place_tracer(node, from, to, 0.0)
	_tracers.append([node, 0.0])


## The streak's head at share `k` of the way, its tail at most
## TRACER_LENGTH_M behind and never behind the gun.
func _place_tracer(node: MeshInstance3D, from: Vector3, to: Vector3, k: float) -> void:
	var head := from.lerp(to, k)
	var dir := (to - from).normalized()
	var length := minf(TRACER_LENGTH_M, maxf(from.distance_to(head), 0.5))
	node.transform = LightStreak.transform(head, head - dir * length, TRACER_WIDTH_M)


func _age_effects(delta: float) -> void:
	var keep: Array = []
	for p in _puffs:
		var node: MeshInstance3D = p[0]
		p[1] += delta
		if p[1] >= p[2]:
			node.queue_free()
			continue
		var k: float = p[1] / p[2]
		node.scale += Vector3.ONE * float(p[3]) * delta
		node.position += Vector3.UP * 1.5 * delta
		node.transparency = clampf(k, 0.0, 1.0)
		keep.append(p)
	_puffs = keep
	var tracers: Array = []
	for t in _tracers:
		var node: MeshInstance3D = t[0]
		t[1] += delta
		if t[1] >= TRACER_SECONDS:
			node.queue_free()
			continue
		var from: Vector3 = node.get_meta(&"from")
		var to: Vector3 = node.get_meta(&"to")
		_place_tracer(node, from, to, t[1] / TRACER_SECONDS)
		tracers.append(t)
	_tracers = tracers
	var wrecks: Array = []
	for w in _wrecks:
		var node: Node3D = w[0]
		w[1] += delta
		if w[1] >= WRECK_SECONDS:
			wrecks.append(w)
			continue
		w[2] -= delta
		if w[2] <= 0.0:
			w[2] = 0.4
			var fire: bool = w[1] < WRECK_SECONDS * 0.4
			_puff(node.global_position + Vector3(randf_range(-2, 2), 2.0, randf_range(-2, 2)),
				3.0, Color(0.15, 0.14, 0.13) if fire else Color(0.3, 0.3, 0.3), 2.0)
			if fire:
				_flash(node.global_position + Vector3.UP * 1.0, 1.5, 0.3)
		wrecks.append(w)
	_wrecks = wrecks
