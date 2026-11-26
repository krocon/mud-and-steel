extends Node3D
## Grass fire (prompt 082). Presentation only: nothing here is read back or
## reaches the simulation. A burning meadow hurts nobody, blocks no sight
## and changes no rule; it is what a heavy blast leaves in tall grass.
##
## Tall grass (terrain `tall_grass`) catches fire under the scorch mark of
## a big blast (`ignite`, called by `effects.gd` with every scorch mark:
## rockets, bombs, heavy shells, exploding vehicles; tank shells are too
## small). From there a fire front eats through the stand over a raster of
## 4 m cells: every burning cell sets its eight neighbours on fire after the
## time the front needs to reach them (faster with the wind, slower against
## it, a little chance in every step), burns for `BURN_S` and is then burnt.
## The front stops where the tall grass ends: short grass, tracks, roads,
## water, woods. Burnt cells are stamped into the track raster
## (`grass_trails.gd`), which draws the plants as black stubble.
##
## The fire runs on game time (the simulation's tick), so it pauses with
## the game and is as far after a fast run-up as after a slow one. Its own
## random generator has a fixed seed: the same blasts give the same fire.
##
## Drawn with a few emitters of the effect library that hop over the
## burning cells every frame (their particles stay where they were born):
## flames along the whole front and a blanket of dark smoke for the price
## of a dozen particle systems. Created by `main.gd`; built only on maps
## with tall grass.

const EffectLibrary := preload("res://scenes/world/effect_library.gd")
const GraphicsQuality := preload("res://scenes/world/graphics_quality.gd")
## Terrain id of tall grass (data/terrain.toml).
const TALL_GRASS := 8
## Edge of a raster cell in meters.
const CELL_M := 4.0
## Speed of the front in calm air (m per game second) and how much the
## wind adds downwind or takes upwind (share at full wind).
const SPREAD_MPS := 1.4
const WIND_SHARE := 0.55
## A neighbour catches fire with this chance per burning neighbour.
const CATCH_CHANCE := 0.9
## A cell burns this long (game seconds).
const BURN_S := 9.0
## Scorch marks at least this wide (m) start a fire: 4.2 m is a 132 mm
## rocket, 3.3 m an 8.8 cm shell.
const IGNITE_MIN_M := 4.0
## Flame and smoke emitters per quality level.
const EMITTERS: Dictionary[String, Vector2i] = {
	"low": Vector2i(4, 2), "medium": Vector2i(10, 5), "high": Vector2i(18, 10),
}
const FLAME_SCALE := 1.9
const SMOKE_SCALE := 2.6
## Flames of burning grass: deeper orange than the fire of a wreck and
## less glow, so a long front does not read as a string of lamps.
const FLAME_COLORS: Array[Color] = [Color(1.0, 0.62, 0.2, 0.95), Color(0.95, 0.32, 0.06, 0.85), Color(0.4, 0.08, 0.02, 0.0)]
const FLAME_OFFSETS: Array[float] = [0.0, 0.45, 1.0]
const FLAME_EMISSION := 0.7
## Burning cells one flame emitter covers at full strength.
const CELLS_PER_FLAME := 6.0
const CELLS_PER_SMOKE := 14.0
## Sound: the fire roars (`fire_large` of the battle sound) where it burns
## nearest the listener and at a second place at least this far from there;
## placed again every `SOUND_SECONDS`, louder with the number of burning
## cells (0 dB at `SOUND_CELLS`).
const SOUND_APART_M := 60.0
const SOUND_SECONDS := 0.5
const SOUND_CELLS := 60.0
## Loudest (dB over the loop's own level): a meadow on fire (1500 cells)
## roars 14 dB louder than a burning barn; the sample is a quiet one.
const SOUND_MAX_DB := 14.0

enum { UNKNOWN, FUEL, SCHEDULED, BURNING, BURNT, BARE }

var main: Node3D
var sim_host: Object
## Cells that burn or have burnt since the start (self-test, tools).
var ignited := 0

var _columns := 0
var _rows := 0
var _origin := Vector2.ZERO
var _state := PackedByteArray()
## Game second a scheduled cell catches fire, or a burning cell did.
var _at := PackedFloat32Array()
var _scheduled: Array[int] = []
var _burning: Array[int] = []
var _flames: Array[GPUParticles3D] = []
var _smokes: Array[GPUParticles3D] = []
var _quality := ""
var _sound_wait := 0.0
var _rng := RandomNumberGenerator.new()


func setup(main_node: Node3D) -> void:
	main = main_node
	sim_host = main.sim_host
	if int(sim_host.terrain_count(TALL_GRASS)) == 0:
		set_process(false)
		return
	var size: Vector2 = sim_host.map_size()
	_origin = -size * 0.5
	_columns = ceili(size.x / CELL_M)
	_rows = ceili(size.y / CELL_M)
	_state.resize(_columns * _rows)
	_at.resize(_columns * _rows)
	_rng.seed = 82


## Burning cells right now (self-test, tools).
func burning() -> int:
	return _burning.size()


## A blast left a scorch mark `size_m` wide at `at`: tall grass under it
## catches fire if the mark is big enough.
func ignite(at: Vector3, size_m: float) -> void:
	if _state.is_empty() or size_m < IGNITE_MIN_M:
		return
	var now := _now()
	var radius := size_m * 0.5
	var reach := ceili(radius / CELL_M)
	var center := Vector2i(floori((at.x - _origin.x) / CELL_M), floori((at.z - _origin.y) / CELL_M))
	for dy in range(-reach, reach + 1):
		for dx in range(-reach, reach + 1):
			var cell := center + Vector2i(dx, dy)
			if cell.x < 0 or cell.y < 0 or cell.x >= _columns or cell.y >= _rows:
				continue
			if _center(cell.y * _columns + cell.x).distance_to(Vector2(at.x, at.z)) > radius + CELL_M * 0.5:
				continue
			var index := cell.y * _columns + cell.x
			if _fuel(index):
				_start(index, now)


func _now() -> float:
	return float(sim_host.current_tick()) / float(sim_host.tick_rate())


func _center(index: int) -> Vector2:
	@warning_ignore("integer_division")
	var row := index / _columns
	return _origin + Vector2((index % _columns) + 0.5, row + 0.5) * CELL_M


## Whether the cell can still catch fire (asks the map once per cell).
func _fuel(index: int) -> bool:
	if _state[index] == UNKNOWN:
		var p := _center(index)
		_state[index] = FUEL if int(sim_host.terrain_at(p.x, p.y)) == TALL_GRASS else BARE
	return _state[index] == FUEL or _state[index] == SCHEDULED


func _start(index: int, now: float) -> void:
	_state[index] = BURNING
	_at[index] = now
	_burning.append(index)
	ignited += 1
	var p := _center(index)
	if main.get("grass_trails") != null:
		# Stubble under the cell, off its center and of uneven size, so the
		# edge of the burnt ground does not follow the raster.
		var off := Vector2(_rng.randf_range(-1.5, 1.5), _rng.randf_range(-1.5, 1.5))
		main.grass_trails.burn(Vector3(p.x + off.x, 0.0, p.y + off.y), CELL_M * _rng.randf_range(0.65, 1.05))
	# The front reaches the neighbours after the way there, faster downwind.
	var wind := _wind()
	var wind_strength := minf(wind.length() / 2.5, 1.0)
	var wind_direction := wind.normalized() if wind.length() > 0.01 else Vector2.ZERO
	@warning_ignore("integer_division")
	var cell := Vector2i(index % _columns, index / _columns)
	for dy in range(-1, 2):
		for dx in range(-1, 2):
			if dx == 0 and dy == 0:
				continue
			var next := cell + Vector2i(dx, dy)
			if next.x < 0 or next.y < 0 or next.x >= _columns or next.y >= _rows:
				continue
			var n := next.y * _columns + next.x
			if not _fuel(n) or _rng.randf() > CATCH_CHANCE:
				continue
			var step := Vector2(dx, dy)
			var speed := SPREAD_MPS * (1.0 + WIND_SHARE * wind_strength * step.normalized().dot(wind_direction))
			var due := now + step.length() * CELL_M / speed * _rng.randf_range(0.7, 1.4)
			if _state[n] == SCHEDULED:
				_at[n] = minf(_at[n], due)
			else:
				_state[n] = SCHEDULED
				_at[n] = due
				_scheduled.append(n)


func _process(delta: float) -> void:
	_sound_wait -= delta
	if _sound_wait <= 0.0:
		_sound_wait = SOUND_SECONDS
		_sound()
	if _scheduled.is_empty() and _burning.is_empty():
		if not _flames.is_empty() and _flames[0].amount_ratio > 0.0:
			_place([], _flames, CELLS_PER_FLAME, 1.2)
			_place([], _smokes, CELLS_PER_SMOKE, 2.5)
		return
	var now := _now()
	# Cells whose time has come catch fire (which schedules more).
	var due: Array[int] = []
	var waiting: Array[int] = []
	for index in _scheduled:
		if _state[index] != SCHEDULED:
			continue
		if _at[index] <= now:
			due.append(index)
		else:
			waiting.append(index)
	_scheduled = waiting
	for index in due:
		_start(index, _at[index])
	var still: Array[int] = []
	for index in _burning:
		if now - _at[index] < BURN_S:
			still.append(index)
		else:
			_state[index] = BURNT
	_burning = still
	_build()
	_place(_burning, _flames, CELLS_PER_FLAME, 1.2)
	_place(_burning, _smokes, CELLS_PER_SMOKE, 2.5)


## Puts every emitter on a burning cell the player sees (another one each
## frame) and thins it out while only a few cells burn.
func _place(cells: Array[int], emitters: Array[GPUParticles3D], cells_per_emitter: float, lift: float) -> void:
	var strength := clampf(float(cells.size()) / (cells_per_emitter * maxf(emitters.size(), 1.0)), 0.0, 1.0)
	if not cells.is_empty():
		strength = maxf(strength, 0.03)
	var visible_at: Callable = main.effects.visible_at if main.effects != null else Callable()
	for emitter in emitters:
		var found := false
		for attempt in 4:
			if cells.is_empty():
				break
			var p := _center(cells[_rng.randi() % cells.size()]) \
				+ Vector2(_rng.randf_range(-0.5, 0.5), _rng.randf_range(-0.5, 0.5)) * CELL_M
			var at := Vector3(p.x, float(sim_host.height_at(p.x, p.y)) + lift, p.y)
			if visible_at.is_valid() and not visible_at.call(at):
				continue
			emitter.global_position = at
			found = true
			break
		emitter.amount_ratio = strength if found else 0.0


## Tells the battle sound where the fire burns nearest the listener.
func _sound() -> void:
	var sound: Node = main.get("battle_sound")
	if sound == null or sound.get("listener") == null:
		return
	var points: Array[Vector3] = []
	if not _burning.is_empty():
		var hear: Vector3 = sound.listener.global_position
		var ear := Vector2(hear.x, hear.z)
		var first := Vector2.INF
		var second := Vector2.INF
		for index in _burning:
			var p := _center(index)
			if p.distance_squared_to(ear) < first.distance_squared_to(ear):
				first = p
		for index in _burning:
			var p := _center(index)
			if p.distance_to(first) >= SOUND_APART_M and p.distance_squared_to(ear) < second.distance_squared_to(ear):
				second = p
		for p: Vector2 in [first, second]:
			if p != Vector2.INF:
				points.append(Vector3(p.x, float(sim_host.height_at(p.x, p.y)) + 1.0, p.y))
	var gain := clampf(10.0 * log(maxf(_burning.size(), 1.0) / SOUND_CELLS) / log(10.0), -8.0, SOUND_MAX_DB)
	sound.set_grass_fires(points, gain)


## The emitters of the quality level (built with the first fire, rebuilt
## when the level changes).
func _build() -> void:
	var atmosphere: Node = main.atmosphere
	var quality: String = atmosphere.quality if atmosphere != null else GraphicsQuality.DEFAULT
	if quality == _quality:
		return
	_quality = quality
	for emitter in _flames + _smokes:
		emitter.queue_free()
	_flames.clear()
	_smokes.clear()
	var settings := GraphicsQuality.settings(quality)
	EffectLibrary.use_clouds(bool(settings.get("fx_clouds", true)))
	var counts: Vector2i = EMITTERS.get(quality, EMITTERS["medium"])
	var wind := _wind()
	for k in counts.x + counts.y:
		var flame := k < counts.x
		var emitter := EffectLibrary.build("fire" if flame else "smoke_column", settings.fx_particles,
			settings.fx_soft, wind)
		# Flames as thick as on a burning barn; the particles are spread
		# over many cells, so there are more of them per emitter.
		emitter.amount = emitter.amount * (6 if flame else 2)
		emitter.scale = Vector3.ONE * (FLAME_SCALE if flame else SMOKE_SCALE)
		if flame:
			var process := emitter.process_material.duplicate() as ParticleProcessMaterial
			process.color_ramp = EffectLibrary._ramp(FLAME_COLORS, FLAME_OFFSETS)
			emitter.process_material = process
			# Prompt 208: EffectLibrary shares QuadMesh/ShaderMaterial across emitters;
			# duplicate before overriding emission for grass flames.
			var quad := (emitter.draw_pass_1 as QuadMesh).duplicate() as QuadMesh
			var mat := (quad.material as ShaderMaterial).duplicate() as ShaderMaterial
			mat.set_shader_parameter("emission", FLAME_EMISSION)
			quad.material = mat
			emitter.draw_pass_1 = quad
		# The emitter hops over the whole fire: never cull by its place.
		emitter.visibility_aabb = AABB(Vector3(-4000, -100, -4000), Vector3(8000, 600, 8000))
		emitter.amount_ratio = 0.0
		add_child(emitter)
		emitter.emitting = true
		(_flames if flame else _smokes).append(emitter)


func _wind() -> Vector2:
	var atmosphere: Node = main.atmosphere
	if atmosphere == null or atmosphere.preset == null:
		return Vector2.ZERO
	var direction: Vector2 = atmosphere.preset.wind_direction
	return direction.normalized() * (0.5 + 2.0 * atmosphere.preset.wind_strength)
