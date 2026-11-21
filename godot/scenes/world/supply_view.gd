extends Node3D
## Supply points (prompts 023 and 107). Presentation only: everything comes
## from `SimHost.supply_points`; nothing here reaches the simulation.
##
## - A flag pole per point with a cloth in the holder's color and a ring
##   that grows with the capture progress (in the taker's color).
## - The border of the capture area (`radius` from `[supply] radius_m` of
##   `data/economy.toml`) as a calm line on the ground in the holder's
##   color, following the terrain and drawn above the fog layer (prompt
##   098). Shapes differ besides the color: neutral dashed and pale, own
##   and allied solid, enemy solid with short ticks inwards. A contested
##   point (hostile players both in the area) blinks.
##   Setting `supply_border`: `strong`, `subtle` or `off`; the flags with
##   `show_supply_points` (prompt 103).
## - What the local player knows (`known`): a point's live state while its
##   center is in sight (or the fog is off, or it is held by the player or an
##   ally: the own income shows it anyway); otherwise the holder last seen,
##   without taker or progress. Flags, border, minimap marks and the ground
##   tooltip all read `known`, so nothing out of sight is given away.
## - Event feed messages "point taken", "point lost" and "point under
##   attack" (a hostile taker or a contested own point; out of view also a
##   minimap ping and the alarm sound, as `alerts.gd` does for hits).

const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const FogOfWar := preload("res://scenes/world/fog_of_war.gd")
const Settings := preload("res://scenes/menu/settings.gd")

## The state is read this often (s).
const READ_INTERVAL := 0.1
## Border line: width (m), height above the ground (m), segments.
const LINE_WIDTH := 0.7
const LINE_LIFT := 0.35
const SEGMENTS := 96
## Neutral border: this many segments drawn, then this many left out.
const DASH_ON := 3
const DASH_OFF := 2
## Enemy border: a tick inwards every TICK_EVERY segments, TICK_LENGTH m.
const TICK_EVERY := 8
const TICK_LENGTH := 1.6
## Strength of the border by `supply_border`.
const STRONG_ALPHA := 0.9
const SUBTLE_ALPHA := 0.35
## A contested border blinks this often per second.
const BLINK_HZ := 2.0
## "Under attack" for the same point at most this often (s).
const ATTACK_COOLDOWN := 20.0

var main: Node3D
var sim_host: SimHost
## Per point: `name`, `position`, `radius`, `income`, and what the local
## player knows: `owner`, `taker` (-1 none), `progress` (0..1),
## `contested`, `relation` ("neutral", "own", "ally", "enemy"), `seen`
## (the state is live).
var known: Array[Dictionary] = []
## Flag nodes, one per point (view toggles, trailer shots).
var flags: Array[Node3D] = []
## Border lines, one per point.
var borders: Array[MeshInstance3D] = []
## False: no flags and borders at all (trailer shots without interface).
var marks_enabled := true
## True: `known` is no longer read (pictures of `tools/supply_shots.gd`
## that set a state by hand).
var frozen := false
## Messages posted so far (self-test): kind → count.
var posted := {}

var _read_left := 0.0
var _time := 0.0
var _last_attack: Dictionary[int, float] = {}


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	name = "SupplyView"
	for p: Dictionary in sim_host.supply_points():
		var info := p.duplicate()
		info["relation"] = ""
		info["seen"] = true
		known.append(info)
		var flag := _make_flag()
		flag.position = p.position
		add_child(flag)
		flags.append(flag)
		var border := MeshInstance3D.new()
		border.name = "Border"
		border.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		var m := StandardMaterial3D.new()
		m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		m.cull_mode = BaseMaterial3D.CULL_DISABLED
		m.render_priority = FogOfWar.MARKER_PRIORITY  # above the fog layer (prompt 098)
		border.material_override = m
		add_child(border)
		borders.append(border)
	_read(false)


func _process(delta: float) -> void:
	if sim_host == null:
		return
	_time += delta
	_read_left -= delta
	if _read_left <= 0.0 and not frozen:
		_read_left = READ_INTERVAL
		_read(true)
	_apply_style()


## "neutral", "own", "ally" or "enemy" for a holder.
func relation_of(owner: int) -> String:
	if owner < 0:
		return "neutral"
	if owner == sim_host.local_player():
		return "own"
	return "ally" if sim_host.relation(owner) == 1 else "enemy"


## Color for a relation (`relation_colors.gd`, also the palettes).
static func color_of(relation: String, alpha := 1.0) -> Color:
	match relation:
		"own":
			return RelationColors.own(alpha)
		"ally":
			return RelationColors.ally(alpha)
		"enemy":
			return RelationColors.enemy(alpha)
	return RelationColors.neutral(alpha)


## Index of the point whose area holds the ground point `at`, or -1.
func point_at(at: Vector3) -> int:
	for i in known.size():
		var p: Dictionary = known[i]
		var c: Vector3 = p.position
		if Vector2(at.x - c.x, at.z - c.z).length() <= float(p.radius):
			return i
	return -1


## Whether `owner` is the local player or an ally.
func _friendly(owner: int) -> bool:
	return owner >= 0 and relation_of(owner) != "enemy"


## Whether the local player sees the point's center now (or has no fog).
func _sees(at: Vector3) -> bool:
	if sim_host.is_reveal_all():
		return true
	var fog: Node = main.get("fog_of_war")
	if fog == null or not fog.enabled:
		return true
	var state: int = fog.state_at(at)
	return state < 0 or state == 255


func _read(report: bool) -> void:
	var points := sim_host.supply_points()
	for i in mini(points.size(), known.size()):
		var live: Dictionary = points[i]
		var k := known[i]
		var before_owner: int = k.owner
		var before_taker: int = k.taker
		var before_contested: bool = k.contested
		var seen := _sees(k.position) or _friendly(live.owner) or _friendly(before_owner)
		k.seen = seen
		if seen:
			k.owner = live.owner
			k.taker = live.taker
			k.progress = live.progress
			k.contested = live.contested
		else:
			k.taker = -1
			k.progress = 0.0
			k.contested = false
		k.relation = relation_of(k.owner)
		if report:
			_report(i, before_owner, before_taker, before_contested)


## Feed messages for point `i` after a read.
func _report(i: int, before_owner: int, before_taker: int, before_contested: bool) -> void:
	var alerts: Node = _hud_part("alerts")
	var feed: Node = _hud_part("event_feed")
	if feed == null or alerts == null or not alerts.active():
		return
	var k := known[i]
	var me := sim_host.local_player()
	var at: Vector3 = k.position
	if int(k.owner) == me and before_owner != me:
		_post(feed, "point_taken", tr("HUD_FEED_POINT_TAKEN"), at)
	elif before_owner == me and int(k.owner) != me:
		_post(feed, "point_lost", tr("HUD_FEED_POINT_LOST"), at)
		_alarm(alerts, feed, at, "ui_unit_lost")
	elif int(k.owner) == me:
		var attacked := (int(k.taker) >= 0 and int(k.taker) != before_taker and relation_of(k.taker) == "enemy") \
			or (bool(k.contested) and not before_contested)
		if attacked and _time - float(_last_attack.get(i, -1e9)) >= ATTACK_COOLDOWN:
			_last_attack[i] = _time
			_post(feed, "point_attack", tr("HUD_FEED_POINT_ATTACK"), at)
			_alarm(alerts, feed, at, "ui_under_attack")


func _post(feed: Node, kind: String, text: String, at: Vector3) -> void:
	feed.post(kind, text, at)
	posted[kind] = int(posted.get(kind, 0)) + 1


## Out of view: a minimap ping and the alarm sound.
func _alarm(alerts: Node, feed: Node, at: Vector3, sound_name: String) -> void:
	if alerts.in_view(at):
		return
	feed.add_ping(at)
	var sound: Node = main.get("battle_sound")
	if sound != null:
		sound.notice(sound_name, true)


func _hud_part(part: String) -> Node:
	var hud: Node = main.get("hud")
	return hud.get(part) if hud != null else null


## Strength of the border lines now (0: hidden).
func border_alpha() -> float:
	if not marks_enabled:
		return 0.0
	var toggles: Node = main.get("view_toggles")
	if toggles != null and not toggles.shows("show_supply_points"):
		return 0.0
	match Settings.get_value("supply_border"):
		"off":
			return 0.0
		"subtle":
			return SUBTLE_ALPHA
	return STRONG_ALPHA


func _apply_style() -> void:
	var alpha := border_alpha()
	var blink := 0.5 + 0.5 * cos(_time * TAU * BLINK_HZ)
	for i in known.size():
		var k := known[i]
		var flag := flags[i]
		var relation: String = k.relation
		var cloth := flag.get_node("Cloth") as MeshInstance3D
		if cloth.get_meta(&"relation", "") != relation:
			cloth.set_meta(&"relation", relation)
			cloth.material_override = _flat(color_of(relation))
		var ring := flag.get_node("Ring") as MeshInstance3D
		var taker: int = k.taker
		ring.visible = taker >= 0
		if taker >= 0:
			var key := relation_of(taker)
			if ring.get_meta(&"relation", "") != key:
				ring.set_meta(&"relation", key)
				ring.material_override = _flat(color_of(key, 0.6))
			var r := 2.0 + (float(k.radius) - 2.0) * float(k.progress)
			ring.scale = Vector3(r, 1.0, r)
		var border := borders[i]
		border.visible = alpha > 0.0
		if not border.visible:
			continue
		if border.get_meta(&"relation", "") != relation:
			border.set_meta(&"relation", relation)
			border.mesh = _border_mesh(k.position, float(k.radius), relation)
		var a := alpha
		if relation == "neutral":
			a *= 0.6
		if k.contested:
			a *= 0.25 + 0.75 * blink
		(border.material_override as StandardMaterial3D).albedo_color = color_of(relation, a)


## The border ring of a point: a band LINE_WIDTH wide on the ground, in
## world coordinates (the node stays at the origin).
func _border_mesh(center: Vector3, radius: float, relation: String) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var half := LINE_WIDTH * 0.5
	for s in SEGMENTS:
		if relation == "neutral" and s % (DASH_ON + DASH_OFF) >= DASH_ON:
			continue
		var a0 := TAU * s / SEGMENTS
		var a1 := TAU * (s + 1) / SEGMENTS
		var o0 := _ground(center, a0, radius + half)
		var o1 := _ground(center, a1, radius + half)
		var i0 := _ground(center, a0, radius - half)
		var i1 := _ground(center, a1, radius - half)
		for v in [o0, i0, o1, o1, i0, i1]:
			st.add_vertex(v)
		if relation == "enemy" and s % TICK_EVERY == 0:
			var w := 0.5 * (a1 - a0)
			var t0 := _ground(center, a0 - w * 0.5, radius - half)
			var t1 := _ground(center, a0 + w * 0.5, radius - half)
			var tip := _ground(center, a0, radius - half - TICK_LENGTH)
			for v in [t0, t1, tip]:
				st.add_vertex(v)
	return st.commit()


func _ground(center: Vector3, angle: float, r: float) -> Vector3:
	var x := center.x + cos(angle) * r
	var z := center.z + sin(angle) * r
	return Vector3(x, sim_host.height_at(x, z) + LINE_LIFT, z)


func _make_flag() -> Node3D:
	var flag := Node3D.new()
	var pole := MeshInstance3D.new()
	var pole_mesh := CylinderMesh.new()
	pole_mesh.top_radius = 0.08
	pole_mesh.bottom_radius = 0.1
	pole_mesh.height = 7.0
	pole.mesh = pole_mesh
	pole.position.y = 3.5
	pole.material_override = _flat(Color(0.35, 0.3, 0.25))
	flag.add_child(pole)
	var cloth := MeshInstance3D.new()
	cloth.name = "Cloth"
	var cloth_mesh := BoxMesh.new()
	cloth_mesh.size = Vector3(2.4, 1.5, 0.05)
	cloth.mesh = cloth_mesh
	cloth.position = Vector3(1.25, 6.1, 0.0)
	cloth.material_override = _flat(RelationColors.neutral())
	flag.add_child(cloth)
	var ring := MeshInstance3D.new()
	ring.name = "Ring"
	var ring_mesh := TorusMesh.new()
	ring_mesh.inner_radius = 0.92
	ring_mesh.outer_radius = 1.0
	ring.mesh = ring_mesh
	ring.position.y = 0.3
	ring.visible = false
	flag.add_child(ring)
	return flag


static func _flat(color: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	if color.a < 1.0:
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	return m
