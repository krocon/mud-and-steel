extends Node
## Map object benchmark and shots (prompt 013), started by `main.gd`:
##
##     godot --path godot --always-on-top -- --objects-bench --map=test_large
##
## Finds the 128 m chunk with the most trees and measures frame rate, draw
## calls, triangles and instance counts there while panning slowly: near
## (15 m), middle (120 m) and overview (600 m), the overview once more
## without objects and decoration, and the first village from 150 m.
## `just objects-bench` runs it.
##
## With `--objects-shots` (and `--write-movie <dir>/f.png --fixed-fps 30`) it
## holds fixed views for SHOT_FRAMES frames each instead; frame
## `n * SHOT_FRAMES + SHOT_FRAMES / 2` shows view n. `just objects-shots <dir>`.
##
## `--rocks` (prompt 074, on a map with grown rock such as `rock_hills`)
## shoots the rock selection instead: slab, outcrop and ridge piece nearest
## to the map center at 15, 35, 110 and 350 m, the ridge piece on the
## steepest slope, and three views like the reference pictures, in all
## three biomes (`--daytime=evening` for low sun). `--rocks-bench` measures
## frame rate over the densest ridge field instead of the wood views.
##
## `--roofs-bench` (prompt 080) measures the frame rate over the house
## nearest to the village marker at 35, 110 and 350 m instead.
##
## `--roofs` (prompt 075, `just roof-shots`) shoots the roof views: the
## map's village (marker `village`, else `town_square`) at 15, 35, 110 and
## 350 m and from about 90 m like the reference picture, and with the map
## objects hidden three line-ups placed at the village: every roof covering
## side by side (also from 15 m each), twenty equal farm houses, and damaged,
## burning (charred 0.6) and burnt-out houses; in the forest by day, at low
## sun and at night, in the desert and in winter (`--biomes=`,
## `--daytimes=` narrow it, `--only=<view,...>` picks views).
## With `--snow` (prompt 080, `just snow-shots`, map `winter_village`) the
## same views in winter only: by day, at low sun, at night and in falling
## snow, with more coverings in the line-up (log house, town house, depot,
## the apartment blocks), a close view of two houses like the reference
## picture, and the day views again with the snow amount set to each of
## `--amounts=` (default 0.3, 0.6, 0.85, 1.0; files `winter-day-a<amount>-`).
##
## `--blocks` (prompt 078, `just block-shots`, map `city_blocks`) shoots the
## apartment blocks: the quarter at 15, 35, 110 and 350 m, two views like
## the reference pictures (overview from about 160 m, a block from about
## 45 m), every block in all three damage levels side by side and a burning
## (charred 0.6) damaged block, in the desert by day, at low sun and at
## night, in the forest and in winter (`--biomes=`, `--daytimes=`,
## `--only=`). `--blocks-bench` measures the frame rate over the quarter
## at 35, 110 and 350 m. `--blocks-fight` (`just block-fight`) plays it:
## an own rifle squad occupies the long block nearest to the quarter, two
## own tanks shell it until it is damaged and a ruin, the other units
## cross the quarter's streets; a picture every few seconds and at every
## change of the block.

const WARMUP_FRAMES := 90
const MEASURE_FRAMES := 240
const SHOT_FRAMES := 30
## Views relative to the densest wood chunk ("wood") or the village marker.
const VIEWS: Array[Dictionary] = [
	{"name": "near", "at": "edge", "distance": 15.0},
	{"name": "near-nodeco", "at": "edge", "distance": 15.0, "deco": false},
	{"name": "near-noshadow", "at": "edge", "distance": 15.0, "shadows": false},
	{"name": "near-noobj", "at": "edge", "distance": 15.0, "objects": false},
	{"name": "middle", "at": "edge", "distance": 120.0},
	{"name": "middle-noshadow", "at": "edge", "distance": 120.0, "shadows": false},
	{"name": "overview", "at": "wood", "distance": 600.0},
	{"name": "bare", "at": "wood", "distance": 600.0, "hide": true},
	{"name": "village", "at": "village", "distance": 150.0},
]
const SHOTS: Array[Dictionary] = [
	{"name": "wood-near", "at": "edge", "distance": 25.0, "yaw": 0.5},
	{"name": "wood-middle", "at": "edge", "distance": 120.0, "yaw": 0.5},
	{"name": "wood-overview", "at": "wood", "distance": 600.0, "yaw": 0.5},
	{"name": "village", "at": "village", "distance": 110.0, "yaw": 0.9},
	{"name": "village-near", "at": "village-edge", "distance": 45.0, "yaw": 2.2},
	{"name": "meadow-near", "at": "meadow", "distance": 15.0, "yaw": 0.4},
	{"name": "hedgerow", "at": "hedge", "distance": 60.0, "yaw": 1.2},
]

var main: Node3D
var forced_draws := 0
var _points: Dictionary[String, Vector3] = {}


func _ready() -> void:
	_run()


const ROCK_DISTANCES: Array[float] = [15.0, 35.0, 110.0, 350.0]
const ROCK_KINDS: Array[String] = ["env_rock_slab", "env_rock_outcrop", "env_rock_massif"]


func _run() -> void:
	get_window().size = Vector2i(1920, 1080)
	main.camera_rig.edge_scroll_enabled = false
	var args := OS.get_cmdline_user_args()
	if "--roofs" in args:
		await _roof_shots()
		get_tree().quit()
		return
	if "--blocks" in args:
		await _block_shots()
		get_tree().quit()
		return
	if "--blocks-fight" in args:
		await _block_fight()
		get_tree().quit()
		return
	if "--blocks-bench" in args:
		_find_block_points()
		var views: Array[Dictionary] = []
		for d in [35.0, 110.0, 350.0]:
			views.append({"name": "blocks-%d" % int(d), "at": "quarter", "distance": d})
		await _measure(views)
		get_tree().quit()
		return
	if "--roofs-bench" in args:
		# Prompt 080: frame rate over the houses nearest to the village
		# marker (the roof snow costs only on roof pixels).
		_find_points()
		var host: SimHost = main.sim_host
		var kinds := host.object_kinds()
		var nearest := INF
		var village: Vector3 = _points["village"]
		_points["house"] = village
		for group: Dictionary in host.object_groups(128.0):
			var id: String = kinds[group.kind].get_slice(":", 0)
			if not (id.begins_with("env_house") or id == "env_barn"):
				continue
			var buffer: PackedFloat32Array = group.buffer
			for k in buffer.size() / 12:
				var p := Vector3(buffer[k * 12 + 3], buffer[k * 12 + 7], buffer[k * 12 + 11])
				if p.distance_to(village) < nearest:
					nearest = p.distance_to(village)
					_points["house"] = p
		var views: Array[Dictionary] = []
		for d in [35.0, 110.0, 350.0]:
			views.append({"name": "roofs-%d" % int(d), "at": "house", "distance": d})
		await _measure(views)
		get_tree().quit()
		return
	if "--rocks" in args or "--rocks-bench" in args:
		_find_rock_points()
		if "--rocks-bench" in args:
			await _rocks_bench()
		else:
			await _rock_shots()
		get_tree().quit()
		return
	_find_points()
	if "--objects-shots" in args:
		await _shots()
	else:
		await _bench()
	get_tree().quit()


const ROOF_DISTANCES: Array[float] = [15.0, 35.0, 110.0, 350.0]
## Every roof covering (prompt 075): clay tile, slate, thatch, planks,
## corrugated steel, flat plaster, and the church (slate, tower).
const ROOF_COVERINGS: Array[String] = [
	"env_house_farm", "env_house_stone", "env_barn", "env_house_log", "env_steel_hall",
	"env_house_desert_two", "env_church",
]
## Damage line-up: [id, variant, charred].
const ROOF_DAMAGE: Array = [
	["env_house_farm", "Intact", 0.0], ["env_house_farm", "Damaged", 0.0], ["env_house_farm", "Ruin", 0.0],
	["env_house_stone", "Damaged", 0.0], ["env_house_farm", "Damaged", 0.6], ["env_house_farm", "Damaged", 1.0],
]
const ROOF_CONDITIONS: Array = [
	["forest", "day"], ["forest", "evening"], ["forest", "night"], ["desert", "day"], ["winter", "day"],
]
## Snow on roofs (prompt 080): every roof kind of the winter maps.
const SNOW_COVERINGS: Array[String] = [
	"env_house_farm", "env_house_farm_brick", "env_house_stone", "env_barn", "env_house_log", "env_house_town",
	"env_house_mud", "env_steel_hall", "env_depot", "env_church", "env_block_town", "env_block_desert",
]
## [biome, daytime, weather, snow amount (-1: the biome's)].
const SNOW_CONDITIONS: Array = [
	["winter", "day", "clear", -1.0], ["winter", "evening", "clear", -1.0], ["winter", "night", "clear", -1.0],
	["winter", "day", "snow", -1.0],
]


## Roof views (prompt 075), saved as `<out>/<biome>-<daytime>-<view>.png`.
func _roof_shots() -> void:
	var host: SimHost = main.sim_host
	var rig: Node3D = main.camera_rig
	var objects: Node3D = main.map_objects
	var out := "build/roof-shots"
	var biomes: Array[String] = []
	var daytimes: Array[String] = []
	var only: Array[String] = []
	var snow := "--snow" in OS.get_cmdline_user_args()
	var amounts: Array[float] = [0.3, 0.6, 0.85, 1.0]
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--biomes="):
			biomes.assign(arg.trim_prefix("--biomes=").split(","))
		elif arg.begins_with("--daytimes="):
			daytimes.assign(arg.trim_prefix("--daytimes=").split(","))
		elif arg.begins_with("--only="):
			only.assign(arg.trim_prefix("--only=").split(","))
		elif arg.begins_with("--amounts="):
			amounts.assign(Array(arg.trim_prefix("--amounts=").split(",")).map(func(a: String) -> float: return a.to_float()))
	var conditions: Array = []
	if snow:
		conditions.append_array(SNOW_CONDITIONS)
		for a in amounts:
			conditions.append(["winter", "day", "clear", a])
	else:
		for c: Array in ROOF_CONDITIONS:
			conditions.append([c[0], c[1], "clear", -1.0])
	DirAccess.make_dir_recursive_absolute(out)
	main.fog_of_war.set_fog_enabled(false)
	main.hud.visible = false
	var marker: Variant = host.map_marker("village")
	if marker == null:
		marker = host.map_marker("town_square")
	var center: Vector3 = marker if marker != null else Vector3.ZERO
	# Village views: the 128 m chunk with the most houses, centered on the
	# mean of its houses, the 15 / 35 m views on the house nearest to that.
	var kinds := host.object_kinds()
	var houses: Dictionary[Vector2i, Array] = {}
	var best := Vector2i.ZERO
	for group: Dictionary in host.object_groups(128.0):
		var id: String = kinds[group.kind].get_slice(":", 0)
		if not (id.begins_with("env_house") or id == "env_barn" or id == "env_church"):
			continue
		var buffer: PackedFloat32Array = group.buffer
		var c: Vector2i = group.chunk
		if not houses.has(c):
			houses[c] = []
		for k in buffer.size() / 12:
			houses[c].append(Vector3(buffer[k * 12 + 3], buffer[k * 12 + 7], buffer[k * 12 + 11]))
		if houses[c].size() > houses.get(best, []).size():
			best = c
	var village := center
	var house := center
	if houses.has(best):
		village = Vector3.ZERO
		for p: Vector3 in houses[best]:
			village += p
		village /= houses[best].size()
		var nearest := INF
		for p: Vector3 in houses[best]:
			if p.distance_to(village) < nearest:
				nearest = p.distance_to(village)
				house = p
	village.y = host.height_at(village.x, village.z)
	print("objects-shots: roofs, village chunk %s with %d buildings around %s, line-ups at %s" % [
		best, houses.get(best, []).size(), village, center])
	center.y = host.height_at(center.x, center.z)
	# Line-ups, built once at the village, shown with the map objects hidden.
	var lineup := Node3D.new()
	lineup.name = "RoofLineup"
	main.add_child(lineup)
	var views: Array[Dictionary] = []
	for d in ROOF_DISTANCES:
		views.append({"name": "village-%d" % int(d), "at": house if d < 100.0 else village, "distance": d, "yaw": 0.9})
	views.append({"name": "village-compare", "at": village, "distance": 90.0, "yaw": 0.5})
	if snow:
		# Like the reference picture: two houses from about 30 m, ridges
		# running diagonally through the picture.
		views.append({"name": "village-compare-near", "at": house, "distance": 30.0, "yaw": 0.4})
	var x := center.x - 60.0
	var row_at := Vector3.ZERO
	for id in (SNOW_COVERINGS if snow else ROOF_COVERINGS):
		var node := _lineup_house(lineup, id, "Intact", 0.0, Vector3(x, 0.0, center.z))
		var size := _extent(node)
		node.position.x += size.x * 0.5
		views.append({"name": "covering-%s-15" % id.trim_prefix("env_"), "at": node.position, "distance": 15.0, "yaw": 0.6})
		x += size.x + 5.0
	row_at = Vector3((center.x - 60.0 + x) * 0.5, center.y, center.z)
	views.append({"name": "coverings-35", "at": row_at + Vector3(-25.0, 0.0, 0.0), "distance": 35.0, "yaw": 0.6})
	views.append({"name": "coverings-110", "at": row_at, "distance": 110.0, "yaw": 0.6})
	for i in 20:
		_lineup_house(lineup, "env_house_farm", "Intact", 0.0,
			Vector3(center.x - 40.0 + (i % 5) * 18.0, 0.0, center.z + 45.0 + (i / 5) * 16.0))
	var twenty := Vector3(center.x - 4.0, center.y, center.z + 69.0)
	views.append({"name": "twenty-60", "at": twenty, "distance": 60.0, "yaw": 0.5})
	views.append({"name": "twenty-110", "at": twenty, "distance": 110.0, "yaw": 0.5})
	for i in ROOF_DAMAGE.size():
		var entry: Array = ROOF_DAMAGE[i]
		_lineup_house(lineup, entry[0], entry[1], entry[2], Vector3(center.x - 45.0 + i * 16.0, 0.0, center.z - 45.0))
	views.append({"name": "damage-70", "at": Vector3(center.x - 5.0, center.y, center.z - 45.0), "distance": 70.0, "yaw": 0.6})
	var i := 0
	for condition: Array in conditions:
		if not biomes.is_empty() and not condition[0] in biomes:
			continue
		if not daytimes.is_empty() and not condition[1] in daytimes:
			continue
		main.atmosphere.set_weather(condition[2], condition[1])
		main.set_biome(condition[0])
		for mesh in lineup.find_children("*", "GeometryInstance3D", true, false):
			var char_amount: float = mesh.get_instance_shader_parameter(&"env_char")
			objects._apply_look(mesh as GeometryInstance3D)
			mesh.set_instance_shader_parameter(&"env_char", char_amount)
		var amount: float = condition[3]
		if amount >= 0.0:
			# Every object's snow scaled to the amount (straw keeps its share via env_foliage_mod.a).
			RenderingServer.global_shader_parameter_set(&"env_snow", amount)
		for view in views:
			if not only.is_empty() and not view.name in only:
				continue
			var at_village: bool = view.name.begins_with("village")
			objects.set_objects_visible(at_village)
			lineup.visible = not at_village
			rig.snap_to(view.at, view.distance, view.yaw)
			var tag: String = "" if condition[2] == "clear" else "-" + str(condition[2])
			if amount >= 0.0:
				tag += "-a%03d" % roundi(amount * 100.0)
			var name := "%s-%s%s-%s" % [condition[0], condition[1], tag, view.name]
			print("objects-shots: roof view %d '%s'" % [i, name])
			await _frames(SHOT_FRAMES * 2)
			get_viewport().get_texture().get_image().save_png(out.path_join(name + ".png"))
			i += 1
	objects.set_objects_visible(true)


## One building of a roof line-up standing on the ground at `at`.
func _lineup_house(parent: Node3D, id: String, variant: String, charred: float, at: Vector3) -> Node3D:
	var host: SimHost = main.sim_host
	at.y = host.height_at(at.x, at.z)
	var node: Node3D = main.map_objects._variant_node(id, variant, Transform3D(Basis.IDENTITY, at))
	node.name = "%s_%s" % [id, variant]
	parent.add_child(node)
	for mesh in node.find_children("*", "GeometryInstance3D", true, false):
		mesh.set_instance_shader_parameter(&"env_char", charred)
	return node


## Size of a line-up building (merged mesh bounds, meters).
func _extent(node: Node3D) -> Vector3:
	var box := AABB()
	var first := true
	for mesh: MeshInstance3D in node.find_children("*", "MeshInstance3D", true, false):
		var b := mesh.transform * mesh.get_aabb()
		box = b if first else box.merge(b)
		first = false
	return box.size


## Rock points (prompt 074): per kind the instance nearest to the map
## center; "massif-slope": the ridge piece with the largest height
## difference over 20 m; "ridge-field": the 128 m chunk with the most ridge
## pieces; "slab-field": the slab with the most slabs within 40 m.
func _find_rock_points() -> void:
	var host: SimHost = main.sim_host
	var kinds := host.object_kinds()
	var best_d: Dictionary[String, float] = {}
	var steepest := -1.0
	var per_chunk: Dictionary[Vector2i, int] = {}
	var slabs: Array[Vector3] = []
	for group: Dictionary in host.object_groups(128.0):
		var id: String = kinds[group.kind].get_slice(":", 0)
		if not id in ROCK_KINDS:
			continue
		var buffer: PackedFloat32Array = group.buffer
		for k in buffer.size() / 12:
			var p := Vector3(buffer[k * 12 + 3], buffer[k * 12 + 7], buffer[k * 12 + 11])
			var d := Vector2(p.x, p.z).length()
			if d < best_d.get(id, INF):
				best_d[id] = d
				_points[id] = p
			if id == "env_rock_massif":
				var c: Vector2i = group.chunk
				per_chunk[c] = per_chunk.get(c, 0) + 1
				var drop := absf(host.height_at(p.x + 10.0, p.z) - host.height_at(p.x - 10.0, p.z)) \
					+ absf(host.height_at(p.x, p.z + 10.0) - host.height_at(p.x, p.z - 10.0))
				if drop > steepest:
					steepest = drop
					_points["massif-slope"] = p
			elif id == "env_rock_slab":
				slabs.append(p)
	var best := Vector2i.ZERO
	for c: Vector2i in per_chunk:
		if per_chunk[c] > per_chunk.get(best, 0):
			best = c
	var half := host.map_size() * 0.5
	var field := Vector3(-half.x + (best.x + 0.5) * 128.0, 0.0, -half.y + (best.y + 0.5) * 128.0)
	_points["ridge-field"] = Vector3(field.x, host.height_at(field.x, field.z), field.z)
	var most := -1
	for a in slabs:
		var n := 0
		for b in slabs:
			if a.distance_to(b) < 40.0:
				n += 1
		if n > most:
			most = n
			_points["slab-field"] = a
	print("objects-shots: rocks %s, steepest ridge piece drop %.1f m, ridge field chunk %s (%d pieces), slab field %d slabs" % [
		_points, steepest, best, per_chunk.get(best, 0), most
	])


func _rock_shot_list() -> Array[Dictionary]:
	var shots: Array[Dictionary] = []
	for id in ROCK_KINDS:
		if not _points.has(id):
			continue
		for d in ROCK_DISTANCES:
			shots.append({"name": "%s-%d" % [id.trim_prefix("env_rock_"), int(d)], "at": id, "distance": d, "yaw": 0.6})
	if _points.has("massif-slope"):
		shots.append({"name": "massif-slope-110", "at": "massif-slope", "distance": 110.0, "yaw": 0.6})
		shots.append({"name": "massif-slope-35", "at": "massif-slope", "distance": 35.0, "yaw": 2.2})
	# Like the reference pictures: slabs with an outcrop from about 120 m,
	# an outcrop from about 60 m, a ridge field from high above.
	if _points.has("slab-field"):
		shots.append({"name": "compare-slabs", "at": "slab-field", "distance": 120.0, "yaw": 0.3})
	if _points.has("env_rock_outcrop"):
		shots.append({"name": "compare-outcrop", "at": "env_rock_outcrop", "distance": 60.0, "yaw": 1.0})
	shots.append({"name": "compare-ridge", "at": "ridge-field", "distance": 260.0, "yaw": 0.0})
	return shots


## With `--out=<dir>` every view is saved as `<dir>/<biome>-<view>.png`
## (`just rock-shots`); otherwise `--write-movie` records the frames.
func _rock_shots() -> void:
	var rig: Node3D = main.camera_rig
	var shots := _rock_shot_list()
	var out := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
	if not out.is_empty():
		DirAccess.make_dir_recursive_absolute(out)
	main.fog_of_war.set_fog_enabled(false)
	main.hud.visible = false
	var biomes: Array[String] = ["forest", "desert", "winter"]
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--biomes="):
			biomes.assign(arg.trim_prefix("--biomes=").split(","))
	var i := 0
	for biome in biomes:
		main.set_biome(biome)
		for shot in shots:
			rig.snap_to(_points[shot.at], shot.distance, shot.yaw)
			var name := "%s-%s" % [biome, shot.name]
			print("objects-shots: view %d '%s' in frames %d..%d" % [
				i, name, i * SHOT_FRAMES, i * SHOT_FRAMES + SHOT_FRAMES - 1
			])
			# Real time without a movie: give the decoration time to build.
			await _frames(SHOT_FRAMES if out.is_empty() else SHOT_FRAMES * 2)
			if not out.is_empty():
				var image := get_viewport().get_texture().get_image()
				image.save_png(out.path_join(name + ".png"))
			i += 1


## Frame rate over the densest ridge field (prompt 074) at 35, 110 and
## 350 m, like `_bench`.
func _rocks_bench() -> void:
	var views: Array[Dictionary] = []
	for d in [35.0, 110.0, 350.0]:
		views.append({"name": "ridges-%d" % int(d), "at": "ridge-field", "distance": d})
	await _measure(views)


## Wood: center of the chunk with the most trees; edge: 50 m south of it
## (so near views look into the wood); village: the map marker; hedge: a
## hedge segment near the meadow.
func _find_points() -> void:
	var host: SimHost = main.sim_host
	var kinds := host.object_kinds()
	var trees: Dictionary[Vector2i, int] = {}
	var best := Vector2i.ZERO
	for group: Dictionary in host.object_groups(128.0):
		if not kinds[group.kind].begins_with("env_tree_"):
			continue
		var chunk: Vector2i = group.chunk
		trees[chunk] = trees.get(chunk, 0) + (group.buffer as PackedFloat32Array).size() / 12
		if trees[chunk] > trees.get(best, 0):
			best = chunk
	var half := host.map_size() * 0.5
	var wood := Vector3(-half.x + (best.x + 0.5) * 128.0, 0.0, -half.y + (best.y + 0.5) * 128.0)
	_points["wood"] = wood
	_points["edge"] = wood + Vector3(0.0, 0.0, 50.0)
	var village: Variant = host.map_marker("village")
	_points["village"] = village if village != null else Vector3.ZERO
	_points["village-edge"] = _points["village"] + Vector3(0.0, 0.0, 48.0)
	_points["meadow"] = Vector3(-6.0, 0.0, 44.0)
	_points["hedge"] = Vector3.ZERO
	var nearest := INF
	for group: Dictionary in host.object_groups(128.0):
		if kinds[group.kind] != "env_hedge:Segment":
			continue
		var buffer: PackedFloat32Array = group.buffer
		for k in buffer.size() / 12:
			var p := Vector3(buffer[k * 12 + 3], 0.0, buffer[k * 12 + 11])
			if p.length() < nearest:
				nearest = p.length()
				_points["hedge"] = p
	print("objects-bench: wood chunk %s with %d trees, village %s, hedge %s" % [
		best, trees.get(best, 0), _points["village"], _points["hedge"]
	])


func _bench() -> void:
	# Prompt 124: `--markers=a,b` adds views 150 m over these map markers
	# (trench lines, crater fields).
	var views: Array[Dictionary] = VIEWS.duplicate()
	for arg in OS.get_cmdline_user_args():
		if not arg.begins_with("--markers="):
			continue
		for name in arg.trim_prefix("--markers=").split(","):
			var at: Variant = main.sim_host.map_marker(name)
			if at == null:
				push_warning("objects-bench: no marker %s" % name)
				continue
			_points[name] = at as Vector3
			views.append({"name": name, "at": name, "distance": 150.0})
	await _measure(views)


func _measure(views: Array) -> void:
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	var viewport_rid := get_viewport().get_viewport_rid()
	RenderingServer.viewport_set_measure_render_time(viewport_rid, true)
	var rig: Node3D = main.camera_rig
	var objects: Node3D = main.map_objects
	print("objects-bench: map '%s', %d objects in %d nodes (built in %.0f ms), render size %s, %s" % [
		main.sim_host.map_name(), objects.object_instances, objects.object_nodes, objects.build_msec,
		get_viewport().get_visible_rect().size, RenderingServer.get_video_adapter_name(),
	])
	for view: Dictionary in views:
		var hide: bool = view.get("hide", false)
		objects.set_objects_visible(not hide and view.get("objects", true))
		objects.set_decoration_visible(not hide and view.get("deco", true))
		objects.set_shadows(view.get("shadows", true))
		var distance: float = view.distance
		var start: Vector3 = _points[view.at]
		rig.snap_to(start, distance, 0.0)
		await _frames(WARMUP_FRAMES)
		var began := Time.get_ticks_usec()
		var cpu := 0.0
		var draw_calls := 0
		var triangles := 0
		var worst := 0.0
		var last := began
		var deco := 0
		for i in MEASURE_FRAMES:
			var t := float(i) / MEASURE_FRAMES
			rig.snap_to(start + Vector3(0.3, 0.0, -0.25) * distance * t, distance, 0.3)
			await _frame()
			var now := Time.get_ticks_usec()
			worst = maxf(worst, (now - last) / 1000.0)
			last = now
			cpu += RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid) + RenderingServer.get_frame_setup_time_cpu()
			draw_calls += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
			triangles += RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
			deco += objects.deco_instances
		var seconds := (Time.get_ticks_usec() - began) / 1e6
		print("objects-bench: %-8s %4.0f m  %6.1f FPS  frame %5.2f ms (worst %5.2f)  render cpu %5.2f ms  %5d draw calls  %5.2f M triangles  decoration %d" % [
			view.name, distance, MEASURE_FRAMES / seconds, seconds * 1000.0 / MEASURE_FRAMES, worst,
			cpu / MEASURE_FRAMES, draw_calls / MEASURE_FRAMES, triangles / float(MEASURE_FRAMES) / 1e6,
			deco / MEASURE_FRAMES,
		])
	objects.set_objects_visible(true)
	objects.set_decoration_visible(true)
	print("objects-bench: frames rendered by force_draw (window not drawable): %d" % forced_draws)


func _shots() -> void:
	var rig: Node3D = main.camera_rig
	for i in SHOTS.size():
		var shot := SHOTS[i]
		rig.snap_to(_points[shot.at], shot.distance, shot.yaw)
		print("objects-shots: view %d '%s' in frames %d..%d" % [i, shot.name, i * SHOT_FRAMES, i * SHOT_FRAMES + SHOT_FRAMES - 1])
		await _frames(SHOT_FRAMES)


func _frames(count: int) -> void:
	for i in count:
		await _frame()


## Waits one frame; renders it by hand if the window cannot draw.
func _frame() -> void:
	if not DisplayServer.window_can_draw():
		RenderingServer.force_draw(false)
		forced_draws += 1
	await get_tree().process_frame


const BLOCK_IDS: Array[String] = ["env_block_desert", "env_block_desert_long", "env_block_town"]
const BLOCK_DISTANCES: Array[float] = [15.0, 35.0, 110.0, 350.0]
const BLOCK_CONDITIONS: Array = [
	["desert", "day"], ["desert", "evening"], ["desert", "night"], ["forest", "day"], ["winter", "day"],
]


## Quarter: the marker `blocks` (else the mean of the blocks); "block": the
## long block nearest to it.
func _find_block_points() -> void:
	var host: SimHost = main.sim_host
	var kinds := host.object_kinds()
	var sum := Vector3.ZERO
	var n := 0
	var nearest := INF
	var marker: Variant = host.map_marker("blocks")
	var blocks: Array[Vector3] = []
	for group: Dictionary in host.object_groups(128.0):
		var id: String = kinds[group.kind].get_slice(":", 0)
		if not id in BLOCK_IDS:
			continue
		var buffer: PackedFloat32Array = group.buffer
		for k in buffer.size() / 12:
			var p := Vector3(buffer[k * 12 + 3], buffer[k * 12 + 7], buffer[k * 12 + 11])
			sum += p
			n += 1
			if id == "env_block_desert_long":
				blocks.append(p)
	var quarter: Vector3 = marker if marker != null else (sum / maxf(n, 1))
	quarter.y = host.height_at(quarter.x, quarter.z)
	_points["quarter"] = quarter
	_points["block"] = quarter
	for p in blocks:
		if p.distance_to(quarter) < nearest:
			nearest = p.distance_to(quarter)
			_points["block"] = p
	print("objects-shots: %d blocks, quarter %s, long block %s" % [n, quarter, _points["block"]])


func _block_shots() -> void:
	_find_block_points()
	var rig: Node3D = main.camera_rig
	var objects: Node3D = main.map_objects
	var out := "build/block-shots"
	var biomes: Array[String] = []
	var daytimes: Array[String] = []
	var only: Array[String] = []
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--biomes="):
			biomes.assign(arg.trim_prefix("--biomes=").split(","))
		elif arg.begins_with("--daytimes="):
			daytimes.assign(arg.trim_prefix("--daytimes=").split(","))
		elif arg.begins_with("--only="):
			only.assign(arg.trim_prefix("--only=").split(","))
	DirAccess.make_dir_recursive_absolute(out)
	main.fog_of_war.set_fog_enabled(false)
	main.hud.visible = false
	var quarter: Vector3 = _points["quarter"]
	var block: Vector3 = _points["block"]
	var views: Array[Dictionary] = []
	# The street in front of the long block (its entrance side).
	var street := Vector3(block.x, 0.0, block.z + signf(block.z - quarter.z) * 10.0)
	street.y = main.sim_host.height_at(street.x, street.z)
	for d in BLOCK_DISTANCES:
		views.append({"name": "quarter-%d" % int(d), "at": street if d < 100.0 else quarter, "distance": d,
			"yaw": 0.5 if block.z > quarter.z else PI + 0.5})
	views.append({"name": "compare-overview", "at": quarter, "distance": 160.0, "yaw": 0.6})
	views.append({"name": "compare-near", "at": block, "distance": 45.0, "yaw": 2.4})
	views.append({"name": "street-60", "at": street, "distance": 60.0, "yaw": 1.2})
	# Line-ups south of the quarter, shown with the map objects hidden.
	var lineup := Node3D.new()
	lineup.name = "BlockLineup"
	main.add_child(lineup)
	var row := quarter + Vector3(-140.0, 0.0, 122.0)
	var x := row.x
	for id in BLOCK_IDS:
		var x0 := x
		for variant: String in ["Intact", "Damaged", "Ruin"]:
			var node := _lineup_house(lineup, id, variant, 0.0, Vector3(x, 0.0, row.z))
			x += _extent(node).x + 10.0
		var at := Vector3((x0 + x - 10.0) * 0.5, row.y, row.z)
		views.append({"name": "damage-%s" % id.trim_prefix("env_"), "at": at, "distance": 80.0, "yaw": 0.6})
	var burning := _lineup_house(lineup, "env_block_desert_long", "Damaged", 0.6, Vector3(x + 5.0, 0.0, row.z))
	for mesh in burning.find_children("*", "GeometryInstance3D", true, false):
		mesh.set_meta(&"char", 0.6)
	views.append({"name": "burnt", "at": burning.position, "distance": 60.0, "yaw": 0.6})
	var i := 0
	for condition: Array in BLOCK_CONDITIONS:
		if not biomes.is_empty() and not condition[0] in biomes:
			continue
		if not daytimes.is_empty() and not condition[1] in daytimes:
			continue
		main.atmosphere.set_weather("clear", condition[1])
		main.set_biome(condition[0])
		for mesh in lineup.find_children("*", "GeometryInstance3D", true, false):
			objects._apply_look(mesh as GeometryInstance3D)
			mesh.set_instance_shader_parameter(&"env_char", mesh.get_meta(&"char", 0.0))
		for view in views:
			if not only.is_empty() and not view.name in only:
				continue
			var on_map: bool = not (view.name.begins_with("damage") or view.name == "burnt")
			objects.set_objects_visible(on_map)
			lineup.visible = not on_map
			rig.snap_to(view.at, view.distance, view.yaw)
			var name := "%s-%s-%s" % [condition[0], condition[1], view.name]
			print("objects-shots: block view %d '%s'" % [i, name])
			await _frames(SHOT_FRAMES * 2)
			get_viewport().get_texture().get_image().save_png(out.path_join(name + ".png"))
			i += 1
	objects.set_objects_visible(true)


func _block_fight() -> void:
	var host: SimHost = main.sim_host
	var rig: Node3D = main.camera_rig
	var out := "build/block-fight"
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
	DirAccess.make_dir_recursive_absolute(out)
	main.fog_of_war.set_fog_enabled(false)
	_find_block_points()
	var block: Vector3 = _points["block"]
	var building := host.building_at(block.x, block.z)
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var types := host.unit_type_ids()
	host.set_hold_fire(PackedInt64Array(ids), true)
	var squads: Array[int] = []
	var tanks: Array[int] = []
	var others: Array[int] = []
	for i in ids.size():
		if owners[i] != host.local_player():
			continue
		if types[i].contains("_inf_") and squads.is_empty():
			squads.append(ids[i])
		elif not types[i].contains("_inf_") and tanks.size() < 2:
			tanks.append(ids[i])
		else:
			others.append(ids[i])
	print("block-fight: building %d at %s, squad %s, tanks %s, others %d" % [building, block, squads, tanks, others.size()])
	var away := signf(block.z - _points["quarter"].z)
	var yaw := 0.4 if away > 0 else PI + 0.4
	host.garrison_units(PackedInt64Array(squads), building)
	host.move_units(PackedInt64Array(tanks), block.x + 30.0, block.z + away * 85.0)
	# The others cross the quarter to the far side.
	host.move_units(PackedInt64Array(others), 60.0, -away * 44.0)
	Engine.time_scale = 3.0
	var shot := 0
	var condition := ""
	var fired := false
	for step in 1200:
		await _frames(10)
		var info: Dictionary = host.building_info(building)
		var occupants: int = info.get("occupants", 0)
		if not fired and occupants > 0 and step > 40:
			host.set_hold_fire(PackedInt64Array(tanks), false)
			host.attack_ground(PackedInt64Array(tanks), block.x, block.z)
			fired = true
		var now: String = info.get("condition", "")
		var changed := now != condition
		condition = now
		if changed or step % 15 == 0:
			var moving := step < 240 and step % 30 == 15
			if moving and not others.is_empty():
				var p := Vector3.ZERO
				var n := 0
				var positions := host.current_positions()
				for i in ids.size():
					if ids[i] in others and i < positions.size():
						p += positions[i]
						n += 1
				rig.snap_to(p / maxf(n, 1), 70.0, 0.8)
			else:
				rig.snap_to(block, 75.0, yaw)
			await _frames(4)
			var name := "%03d-%s-occupants-%d" % [shot, "move" if moving else condition, occupants]
			get_viewport().get_texture().get_image().save_png(out.path_join(name + ".png"))
			print("block-fight: step %d %s, occupants %d" % [step, condition, occupants])
			shot += 1
		if condition == "ruin":
			await _frames(60)
			rig.snap_to(block, 45.0, yaw)
			await _frames(20)
			get_viewport().get_texture().get_image().save_png(out.path_join("%03d-ruin-near.png" % shot))
			break
	Engine.time_scale = 1.0
