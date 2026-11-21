extends Node3D
## Heavy rocket strike (prompt 054): what everybody sees and hears of a
## called strike, the call button of the local player, the battery
## position's launch frames, and the turrets of exploding tanks.
##
## - Coloured (red) smoke rises on the marked target and a ring shows the
##   area the rockets will hit; a siren wails while the strike is under way
##   (both public, like in the simulation).
## - The battery position of an enemy strike is marked (beam and ring) and
##   the HUD counts down: destroy it in time and the strike is called off.
## - The call bar ("Rockets", Shift+6) waits for a left click on the map,
##   like the air bar (prompt 033).
## - `launchers()` builds the launch frames that `base_view.gd` puts on the
##   battery position's borrowed model.
## - `StrikeEvent::AmmoExplosion` goes to `blast_effects.gd` (turret off).
##
## Presentation only: everything comes from `SimHost.strikes`,
## `strike_option` and `take_strike_events`; the rockets themselves are
## shells (`artillery_view.gd`) and explode with the ordinary effects plus
## the big blasts of `blast_effects.gd`. [provisional] colours, keys, siren.

const FogOfWar := preload("res://scenes/world/fog_of_war.gd")
const MenuTheme := preload("res://scenes/menu/menu_theme.gd")
const EffectLibrary := preload("res://scenes/world/effect_library.gd")
const EffectTextures := preload("res://scenes/world/effect_textures.gd")
const GroundPick := preload("res://scenes/camera/ground_pick.gd")
const Picker := preload("res://scenes/hud/picker.gd")
const Settings := preload("res://scenes/menu/settings.gd")

const READ_INTERVAL := 0.2
## Red marker smoke (WW2 coloured smoke grenades and candles). [provisional]
const SMOKE_COLOR := Color(0.72, 0.07, 0.05)
const RelationColors := preload("res://scenes/hud/relation_colors.gd")
const OWN_COLOR := Color(1.0, 0.75, 0.25, 0.75)
const ENEMY_COLOR := Color(1.0, 0.2, 0.15, 0.8)
## Seconds the smoke keeps rising after the last rocket.
const SMOKE_LINGER := 8.0
## Siren: one rise and fall (s), lowest and highest pitch (Hz), volume.
const SIREN_SECONDS := 4.0
const SIREN_LOW_HZ := 260.0
const SIREN_HIGH_HZ := 620.0
const SIREN_DB := -9.0
const LAUNCHER_COLOR := Color(0.22, 0.24, 0.18)

var main: Node3D
var sim_host: SimHost
var blast_effects: Node3D
var bar: StrikeBar
## Strike id -> {smoke, area, beam, owner, warning}; for the self-test.
var markers: Dictionary[int, Dictionary] = {}
## Events handled per type (self-test).
var counts: Dictionary[String, int] = {}

var _read_left := 0.0
var _hud: Control
var _countdown: Label
var _siren: AudioStreamPlayer
var _fading: Array[Dictionary] = []
var _area_mesh: TorusMesh
var _beam_mesh: CylinderMesh


func setup(main_node: Node3D, blasts: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	blast_effects = blasts
	_area_mesh = TorusMesh.new()
	_area_mesh.inner_radius = 0.97
	_area_mesh.outer_radius = 1.0
	_area_mesh.rings = 48
	_area_mesh.ring_segments = 4
	_beam_mesh = CylinderMesh.new()
	_beam_mesh.top_radius = 0.6
	_beam_mesh.bottom_radius = 0.6
	_beam_mesh.height = 60.0
	_siren = AudioStreamPlayer.new()
	_siren.name = "Siren"
	_siren.stream = _siren_stream()
	_siren.volume_db = SIREN_DB
	if AudioServer.get_bus_index("Effects") >= 0:
		_siren.bus = &"Effects"
	add_child(_siren)


func setup_hud(hud: Control) -> void:
	_hud = hud
	bar = StrikeBar.new()
	hud.add_child(bar)
	bar.setup(main, hud.support, hud.hints)
	_countdown = Label.new()
	_countdown.name = "StrikeCountdown"
	_countdown.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_countdown.add_theme_font_size_override(&"font_size", MenuTheme.font(22))
	_countdown.add_theme_color_override(&"font_outline_color", Color.BLACK)
	_countdown.add_theme_constant_override(&"outline_size", 6)
	_countdown.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_countdown.visible = false
	hud.hints.add_child(_countdown)


func _process(delta: float) -> void:
	if sim_host == null:
		return
	for event in sim_host.take_strike_events():
		_on_event(event)
	_read_left -= delta
	if _read_left <= 0.0:
		_read_left = READ_INTERVAL
		_update_strikes()
	_pulse()
	_fade(delta)


func _on_event(event: Dictionary) -> void:
	var kind: String = event["type"]
	counts[kind] = counts.get(kind, 0) + 1
	var me := sim_host.local_player()
	var own := int(event.get("owner", -1)) == me
	var allied := own or sim_host.relation(int(event.get("owner", -1))) == 1
	match kind:
		"called":
			var text := tr("HUD_FEED_STRIKE_CALLED") % ceili(float(event["warning"])) if allied \
				else tr("HUD_FEED_ENEMY_STRIKE")
			_post("strike", text, event["target"])
			if not allied and _hud != null and _hud.get("minimap") != null:
				_hud.minimap.add_ping(event["from"])
		"cancelled":
			_post("strike", tr("HUD_FEED_STRIKE_OFF") if allied else tr("HUD_FEED_ENEMY_STRIKE_OFF"), event["target"])
		"launched":
			_post("strike", tr("HUD_FEED_ROCKETS_AWAY") if allied else tr("HUD_FEED_ROCKETS_INCOMING"), event["target"])
		"ammo_explosion":
			# The kill sequence cooks off with it (prompt 105).
			var effects: Node = main.get("effects") if main != null else null
			if effects != null and effects.has_method("note_ammo_explosion"):
				effects.note_ammo_explosion(int(event["unit"]), event["position"])
			else:
				blast_effects.ammo_explosion(int(event["unit"]), event["position"])


func _post(kind: String, text: String, at: Vector3) -> void:
	if _hud != null and _hud.get("event_feed") != null:
		_hud.event_feed.post(kind, text, at)


## Markers of every strike under way; the siren and the countdown.
func _update_strikes() -> void:
	var me := sim_host.local_player()
	var live: Dictionary[int, bool] = {}
	var warning := -1.0
	var enemy_warning := false
	for s: Dictionary in sim_host.strikes():
		var id: int = s["id"]
		live[id] = true
		var allied := int(s["owner"]) == me or sim_host.relation(int(s["owner"])) == 1
		var m: Dictionary = markers.get(id, {})
		if m.is_empty():
			m = _make_marker(s, allied)
			markers[id] = m
		m.warning = float(s["warning"])
		aim_launchers(int(s["battery"]), s["target"])
		(m.beam as Node3D).position = s["from"] + Vector3.UP * 30.0
		(m.beam as Node3D).visible = not allied and m.warning > 0.0
		(m.base_ring as Node3D).position = s["from"] + Vector3.UP * 0.4
		(m.base_ring as Node3D).visible = not allied and m.warning > 0.0
		if m.warning > 0.0 and (warning < 0.0 or m.warning < warning):
			warning = m.warning
			enemy_warning = not allied
	for id: int in markers.keys():
		if not live.has(id):
			_retire(markers[id])
			markers.erase(id)
	var sounding := not markers.is_empty()
	if sounding and not _siren.playing:
		_siren.play()
	elif not sounding and _siren.playing:
		_siren.stop()
	if _countdown != null:
		_countdown.visible = warning >= 0.0
		if warning >= 0.0:
			_countdown.text = tr("HUD_STRIKE_ENEMY_COUNTDOWN" if enemy_warning
				else "HUD_STRIKE_COUNTDOWN") % ceili(warning)
			_countdown.add_theme_color_override(&"font_color", Color(1, 0.35, 0.3) if enemy_warning else Color(1, 0.85, 0.4))


func _make_marker(s: Dictionary, allied: bool) -> Dictionary:
	var root := Node3D.new()
	root.name = "Strike%d" % int(s["id"])
	add_child(root)
	var target: Vector3 = s["target"]
	var smoke := _build_smoke(main.effects._settings.get("fx_particles", 1.0))
	root.add_child(smoke)
	smoke.global_position = target + Vector3.UP * 0.3
	smoke.emitting = true
	var area := MeshInstance3D.new()
	area.mesh = _area_mesh
	var r: float = s["radius"]
	area.scale = Vector3(r, 2.0, r)
	area.position = target + Vector3.UP * 0.5
	area.material_override = _flat(RelationColors.own_or(OWN_COLOR) if allied else RelationColors.enemy_or(ENEMY_COLOR))
	root.add_child(area)
	var beam := MeshInstance3D.new()
	beam.mesh = _beam_mesh
	beam.material_override = _flat(Color(RelationColors.enemy_or(ENEMY_COLOR), 0.35))
	root.add_child(beam)
	var base_ring := MeshInstance3D.new()
	base_ring.mesh = _area_mesh
	base_ring.scale = Vector3(9.0, 2.0, 9.0)
	base_ring.material_override = _flat(RelationColors.enemy_or(ENEMY_COLOR))
	root.add_child(base_ring)
	return {"root": root, "smoke": smoke, "area": area, "beam": beam, "base_ring": base_ring, "warning": 0.0, "radius": r}


## The strike is over: the smoke stops and drifts away, the rest goes.
func _retire(m: Dictionary) -> void:
	(m.area as Node).queue_free()
	(m.beam as Node).queue_free()
	(m.base_ring as Node).queue_free()
	_fading.append({"root": m.root, "smoke": m.smoke, "left": SMOKE_LINGER})


func _fade(delta: float) -> void:
	for i in range(_fading.size() - 1, -1, -1):
		var f: Dictionary = _fading[i]
		f.left -= delta
		var smoke: GPUParticles3D = f.smoke
		if f.left <= 0.0 and smoke.emitting:
			smoke.emitting = false
		if f.left <= -smoke.lifetime:
			(f.root as Node).queue_free()
			_fading.remove_at(i)


func _pulse() -> void:
	var t := Time.get_ticks_msec() * 0.006
	for m: Dictionary in markers.values():
		var s := 1.0 + 0.04 * sin(t)
		var r: float = m.radius
		(m.area as Node3D).scale = Vector3(r * (1.0 + 0.01 * sin(t)), 2.0, r * (1.0 + 0.01 * sin(t)))
		(m.base_ring as Node3D).scale = Vector3(9.0 * s, 2.0, 9.0 * s)


## Red smoke rising from a smoke candle on the target.
func _build_smoke(factor: float) -> GPUParticles3D:
	var p := GPUParticles3D.new()
	p.name = "MarkerSmoke"
	p.local_coords = false
	p.amount = maxi(16, int(round(64 * factor)))
	p.lifetime = 12.0
	p.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	p.visibility_aabb = AABB(Vector3(-60, -5, -60), Vector3(120, 80, 120))
	var pm := ParticleProcessMaterial.new()
	pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
	pm.emission_sphere_radius = 0.6
	pm.direction = Vector3.UP
	pm.spread = 12.0
	pm.initial_velocity_min = 2.0
	pm.initial_velocity_max = 3.5
	pm.damping_min = 0.2
	pm.damping_max = 0.4
	var wind: Vector2 = main.effects._wind()
	pm.gravity = Vector3(wind.x * 0.3, 0.15, wind.y * 0.3)
	pm.angle_min = 0.0
	pm.angle_max = 360.0
	pm.angular_velocity_min = -10.0
	pm.angular_velocity_max = 10.0
	pm.scale_curve = EffectLibrary._curve([Vector2(0, 0.4), Vector2(0.3, 1.8), Vector2(1, 5.0)])
	pm.color_ramp = EffectLibrary._ramp([
		Color(SMOKE_COLOR, 0.0), Color(SMOKE_COLOR, 1.0), Color(SMOKE_COLOR.lightened(0.12), 0.8),
		Color(SMOKE_COLOR.lightened(0.3), 0.0)], [0.0, 0.04, 0.5, 1.0])
	p.process_material = pm
	var quad := QuadMesh.new()
	quad.size = Vector2(3.2, 3.2)
	quad.material = EffectLibrary._material("lit", EffectTextures.smoke(), 0.5, main.effects._settings.get("fx_soft", false))
	p.draw_pass_1 = quad
	return p


## Three inclined launch frames with six rails each, on top of the crate
## stacks (a generic WW2 rocket rack, no particular weapon). [provisional:
## a real model is open]
static func launchers() -> Node3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = LAUNCHER_COLOR
	material.roughness = 0.8
	var root := Node3D.new()
	root.name = "Launchers"
	var rail := CylinderMesh.new()
	rail.top_radius = 0.09
	rail.bottom_radius = 0.09
	rail.height = 1.9
	rail.radial_segments = 8
	rail.material = material
	var frame := BoxMesh.new()
	frame.size = Vector3(1.4, 0.12, 1.3)
	frame.material = material
	var racks: Array[Node3D] = []
	for k in 3:
		var rack := Node3D.new()
		rack.position = Vector3(-2.0 + 2.0 * k, 1.55, 0.0)
		rack.rotation.x = deg_to_rad(-40.0)
		root.add_child(rack)
		racks.append(rack)
		var base := MeshInstance3D.new()
		base.mesh = frame
		rack.add_child(base)
		for j in 6:
			var tube := MeshInstance3D.new()
			tube.mesh = rail
			tube.position = Vector3(-0.55 + 0.22 * j, 0.15, 0.0)
			tube.rotation.x = PI * 0.5
			rack.add_child(tube)
	# Prompt 076: a socket on every rail where its rocket leaves, numbered
	# rail by rail across the three racks, so `effects.gd` fires them in
	# turn (one rail after the other over all racks).
	for j in 6:
		for k in 3:
			var socket := Node3D.new()
			socket.name = "Socket_Rocket_%02d" % (j * 3 + k + 1)
			socket.position = Vector3(-0.55 + 0.22 * j, 0.25, 0.4)
			racks[k].add_child(socket)
	return root


## Turns the launch frames of battery `battery` towards `target` (prompt
## 076): the rails point along +z and up.
func aim_launchers(battery: int, target: Vector3) -> void:
	var node: Node3D = main._unit_nodes.get(battery)
	var frames := node.get_node_or_null("Model/Launchers") as Node3D if node != null else null
	if frames == null:
		return
	var to := target - frames.global_position
	if Vector2(to.x, to.z).length() < 1.0:
		return
	frames.global_rotation = Vector3(0.0, atan2(to.x, to.z), 0.0)


static func _flat(color: Color) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.render_priority = FogOfWar.MARKER_PRIORITY  # above the fog layer (prompt 098)
	return m


## A rising and falling siren tone, looped (generated: no sound file).
static func _siren_stream() -> AudioStreamWAV:
	var rate := 22050
	var count := int(SIREN_SECONDS * rate)
	# Total phase of one loop, then the pitch is scaled so the loop ends on
	# a whole number of cycles (no click at the seam).
	var total := 0.0
	for i in count:
		var t := float(i) / rate
		total += _siren_hz(t) / rate
	var scale := roundf(total) / total
	var data := PackedByteArray()
	data.resize(count * 2)
	var phase := 0.0
	for i in count:
		var t := float(i) / rate
		phase += _siren_hz(t) * scale / rate
		var x := TAU * phase
		var v := 0.62 * sin(x) + 0.25 * sin(2.0 * x) + 0.1 * sin(3.0 * x)
		data.encode_s16(i * 2, int(clampf(v, -1.0, 1.0) * 26000.0))
	var wav := AudioStreamWAV.new()
	wav.format = AudioStreamWAV.FORMAT_16_BITS
	wav.mix_rate = rate
	wav.stereo = false
	wav.data = data
	wav.loop_mode = AudioStreamWAV.LOOP_FORWARD
	wav.loop_begin = 0
	wav.loop_end = count
	return wav


static func _siren_hz(t: float) -> float:
	var u := 0.5 - 0.5 * cos(TAU * t / SIREN_SECONDS)
	return SIREN_LOW_HZ + (SIREN_HIGH_HZ - SIREN_LOW_HZ) * u


## The call bar: one button with price, unlock time and cooldown.
class StrikeBar:
	extends Control
	## The rocket strike's tile in the support column of the HUD (prompt
	## 095): symbol, under it the price, the time until it unlocks or the
	## seconds of its cooldown. This node has no size of its own.

	const HudTheme := preload("res://scenes/hud/hud_theme.gd")
	const HudTile := preload("res://scenes/hud/hud_tile.gd")
	const AirBar := preload("res://scenes/hud/air_bar.gd")
	const REFRESH := 0.25
	const HINT := "HUD_HINT_STRIKE"

	var main: Node3D
	var sim_host: SimHost
	var button: HudTile
	## Waiting for the target click.
	var pending := false
	## Calls sent (self-test).
	var calls := 0
	var _hint: Label
	var _refresh_left := 0.0
	var _cooldown_full := 0.0

	func setup(main_node: Node3D, column: Control, hints: Control) -> void:
		main = main_node
		sim_host = main.sim_host
		name = "StrikeBar"
		mouse_filter = Control.MOUSE_FILTER_IGNORE
		button = HudTile.new("strike", AirBar.TILE_PX, true)
		button.name = "Strike"
		button.toggle_mode = true
		button.pressed.connect(func() -> void: set_pending(not pending))
		column.add_child(button)
		_hint = Label.new()
		_hint.name = "StrikeHint"
		_hint.text = HINT
		_hint.visible = false
		_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		_hint.add_theme_color_override(&"font_color", Color(1, 0.95, 0.6))
		_hint.add_theme_color_override(&"font_outline_color", Color.BLACK)
		_hint.add_theme_constant_override(&"outline_size", 5)
		hints.add_child(_hint)
		apply_scale()
		_refresh()

	func apply_scale() -> void:
		button.apply_scale()
		_hint.add_theme_font_size_override(&"font_size", HudTheme.font(14))

	func _process(delta: float) -> void:
		_refresh_left -= delta
		if _refresh_left <= 0.0:
			_refresh_left = REFRESH
			_refresh()

	func _refresh() -> void:
		if sim_host == null:
			return
		var o: Dictionary = sim_host.strike_option()
		# Only players who could ever call it see the tile: with a base.
		visible = not o.is_empty() and o["blocked"] != "defeated" and not sim_host.structures().is_empty()
		button.visible = visible
		if not visible:
			return
		var cost: PackedInt32Array = o["cost"]
		var blocked: String = o["blocked"]
		var line := "%d/%d/%d" % [cost[0], cost[1], cost[2]]
		button.footer_color = HudTheme.RED if blocked == "not enough resources" else HudTheme.GOLD
		button.cooldown = 0.0
		if float(o["unlock"]) > 0.0:
			line = tr("HUD_STRIKE_UNLOCK") % [int(o["unlock"]) / 60, int(o["unlock"]) % 60]
			button.footer_color = HudTheme.BLUE
		elif float(o["cooldown"]) > 0.0:
			_cooldown_full = maxf(_cooldown_full, float(o["cooldown"]))
			line = tr("COMMON_SECONDS") % ceili(float(o["cooldown"]))
			button.footer_color = HudTheme.BLUE
			button.cooldown = float(o["cooldown"]) / _cooldown_full
		elif blocked == "no rocket battery":
			line = "–"
		else:
			_cooldown_full = 0.0
		button.footer = line
		button.key_text = Settings.action_key_name("strike_rockets").replace("Shift+", "")
		button.disabled = not blocked.is_empty()
		var reason := Picker.blocked_text(blocked)
		button.tooltip_text = tr("HUD_TIP_STRIKE").format({
			"key": Settings.action_key_name("strike_rockets"), "rockets": int(o["rockets"]), "radius": int(o["radius"]), "warning": int(o["warning"]),
			"fuel": cost[0], "ammo": cost[1], "manpower": cost[2]}) \
			+ ("" if reason.is_empty() else "\n" + reason[0].to_upper() + reason.substr(1))
		button.set_pressed_no_signal(pending)
		if pending and button.disabled:
			set_pending(false)

	func set_pending(on: bool) -> void:
		pending = on
		_hint.visible = pending
		button.set_pressed_no_signal(pending)
		Input.set_default_cursor_shape(Input.CURSOR_CROSS if pending else Input.CURSOR_ARROW)
		var selection: Node = main.get("selection")
		if pending and selection != null:
			selection.set_command_mode("")

	## Calls the strike on the ground point under `screen_position`.
	func call_at(screen_position: Vector2) -> bool:
		var hit: Variant = GroundPick.from_screen(main.camera, screen_position)
		if hit == null or not pending:
			return false
		var point := hit as Vector3
		sim_host.call_strike(point.x, point.z)
		calls += 1
		set_pending(false)
		return true

	func _input(event: InputEvent) -> void:
		var key := event as InputEventKey
		if key != null and key.pressed and not key.echo and InputMap.has_action("strike_rockets") \
				and key.is_action_pressed("strike_rockets", false, true):
			if visible and not button.disabled:
				set_pending(not pending)
			get_viewport().set_input_as_handled()
			return
		if not pending:
			return
		if key != null and key.pressed and key.keycode == KEY_ESCAPE:
			set_pending(false)
			get_viewport().set_input_as_handled()
			return
		var mouse := event as InputEventMouseButton
		if mouse == null or not mouse.pressed:
			return
		var hud: Node = main.get("hud")
		if hud != null and hud.covers(mouse.position):
			return
		if mouse.button_index == MOUSE_BUTTON_LEFT:
			call_at(mouse.position)
			get_viewport().set_input_as_handled()
		elif mouse.button_index == MOUSE_BUTTON_RIGHT:
			set_pending(false)
			get_viewport().set_input_as_handled()
