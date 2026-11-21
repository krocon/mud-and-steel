extends Node
## Pictures of the depth of field by view angle (prompt 183) as PNG files,
## started by `main.gd` with `-- --dof-shots=<dir>` (`just dof-shots`): two
## own vehicles, selected, driven into the street with the most houses
## around (as in the user's picture), in the lower part of the picture,
## looking the way they drove, for the modes "subtle" and "miniature" at the zoom's own
## angle, at 25 degrees (flattest in a match) and at 8 degrees (photo mode's
## range), all at the same camera distance. The HUD panels are hidden, the selection rings show. Extra
## arguments (`--map=`, `--daytime=`) pick the scene; `--prefix=` names the
## files (`<prefix>_<mode>_<angle>_<before|after>.png`). Each view is taken
## twice in a row, without the view angle rule (`atmosphere.gd`
## `dof_view_angle` off: as before prompt 183) and with it, and the pairs go
## on a sheet per mode (`<prefix>_<mode>_compare.png`: before left, after
## right; the zoom's angle, 25, 8 degrees from the top). Needs a window.
## Quits when done.

const Settings := preload("res://scenes/menu/settings.gd")

## Camera distance (m): both modes are fully on here.
const DISTANCE := 28.0
## The vehicles stand this far up the screen (0 = lower edge, 1 = upper).
const UNIT_ROW := 0.3
## Angles: 0 is the zoom's own, else degrees of elevation.
const ANGLES: Array[float] = [0.0, 25.0, 8.0]
const MODES: Array[String] = ["subtle", "miniature"]
## Width of one picture on the comparison sheet (px).
const SHEET_WIDTH := 800

var main: Node3D
var dir := ""
var prefix := "dof"


func _ready() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--dof-shots="):
			dir = arg.trim_prefix("--dof-shots=")
		if arg.begins_with("--prefix="):
			prefix = arg.trim_prefix("--prefix=")
	DirAccess.make_dir_recursive_absolute(dir)
	_run()


func _run() -> void:
	await _frames(30)
	var host: SimHost = main.sim_host
	var ids := host.unit_ids()
	var owners := host.unit_owners()
	var stances := host.unit_stances()
	var positions := host.current_positions()
	# The two own vehicles closest to each other.
	var vehicles: Array[int] = []
	for i in ids.size():
		if owners[i] == host.local_player() and stances[i] < 0:
			vehicles.append(i)
	if vehicles.size() < 2:
		push_error("dof shots: fewer than two own vehicles")
		get_tree().quit(1)
		return
	var pair := Vector2i(vehicles[0], vehicles[1])
	var best := INF
	for a in vehicles:
		for b in vehicles:
			if a < b and positions[a].distance_to(positions[b]) < best:
				best = positions[a].distance_to(positions[b])
				pair = Vector2i(a, b)
	var at: Vector3 = (positions[pair.x] + positions[pair.y]) * 0.5
	var pair_ids := PackedInt64Array([ids[pair.x], ids[pair.y]])
	main.selection.set_selection([ids[pair.x], ids[pair.y]])
	# As in the user's picture: drive them into a street between houses.
	var street := _street_near(host, at)
	var ahead := Vector2(street.x - at.x, street.y - at.z)
	if street != Vector2(at.x, at.z):
		host.speed = 8.0
		host.move_units(pair_ids, street.x, street.y)
		for frame in 3600:
			await get_tree().process_frame
			var now := _positions(host, pair_ids)
			if Vector2(now.x, now.z).distance_to(street) < 12.0 and frame > 60:
				break
		host.speed = 1.0
		# Let the dust of the drive settle (it veils the vehicles).
		await _frames(480)
		at = _positions(host, pair_ids)
		ahead = _along_street(host, at)
	var rig: Node3D = main.camera_rig
	if ahead.length() < 1.0:
		var center: Vector2 = rig.bounds.get_center()
		ahead = Vector2(center.x - at.x, center.y - at.z)
	if ahead.length() < 1.0:
		ahead = Vector2(0.0, -1.0)
	ahead = ahead.normalized()
	# Yaw 0 looks north (-Z); positive yaw turns left.
	var yaw := atan2(-ahead.x, -ahead.y)
	# Only the HUD panels: the selection rings stay, as in the photo mode.
	(main.get_node("Hud") as CanvasLayer).visible = false
	var saved_min: float = rig.tilt_min_deg
	rig.tilt_min_deg = 4.0
	for mode in MODES:
		Settings.set_value("depth_of_field", mode)
		var sheet: Image = null
		for row in ANGLES.size():
			var angle := ANGLES[row]
			rig.snap_to(at, DISTANCE, yaw)
			var elevation: float = rig.base_pitch() if angle == 0.0 else deg_to_rad(angle)
			rig.snap_tilt(elevation - rig.base_pitch())
			# Move the focus ahead so the vehicles stand at UNIT_ROW.
			var depth := _row_depth(elevation, deg_to_rad(rig.camera.fov), UNIT_ROW)
			var up := atan((UNIT_ROW * 2.0 - 1.0) * tan(deg_to_rad(rig.camera.fov) * 0.5))
			var ground := depth / cos(up) * cos(elevation - up)
			var shift := cos(elevation) - ground
			rig.snap_to(at + Vector3(ahead.x, 0.0, ahead.y) * shift * DISTANCE, DISTANCE, yaw)
			var tag := "zoom" if angle == 0.0 else "%02d" % int(angle)
			# Before (no view angle rule) and after, in the same frame state.
			for column in 2:
				main.atmosphere.dof_view_angle = column == 1
				await _frames(40)
				var image := await _save("%s_%s_%s_%s.png" % [prefix, mode, tag, ["before", "after"][column]])
				image.resize(SHEET_WIDTH, int(image.get_height() * float(SHEET_WIDTH) / image.get_width()))
				if sheet == null:
					sheet = Image.create(SHEET_WIDTH * 2, image.get_height() * ANGLES.size(), false, image.get_format())
				sheet.blit_rect(image, Rect2i(Vector2i.ZERO, image.get_size()), Vector2i(column * SHEET_WIDTH, row * image.get_height()))
		var sheet_path := dir.path_join("%s_%s_compare.png" % [prefix, mode])
		sheet.save_png(sheet_path)
		print("dof shots: %s" % sheet_path)
	main.atmosphere.dof_view_angle = true
	rig.tilt_min_deg = saved_min
	get_tree().quit()


## A free spot with the most buildings around (35 m) within 1500 m of
## `from`, on a 20 m grid (the nearest of equal ones); `from` itself if there are no buildings.
func _street_near(host: SimHost, from: Vector3) -> Vector2:
	var best := Vector2(from.x, from.z)
	var most := 2
	for gx in range(-75, 76):
		for gz in range(-75, 76):
			var x := from.x + gx * 20.0
			var z := from.z + gz * 20.0
			if host.buildings_near(x, z, 8.0) > 0:
				continue
			var count := host.buildings_near(x, z, 35.0)
			if count > most:
				most = count
				best = Vector2(x, z)
	print("dof shots: street at %s with %d buildings around" % [best, most])
	return best


## The direction (x, z) from `at` with the longest free way ahead (up to
## 150 m) and 40 m free behind for the camera, of 24; more houses beside
## the way win a tie.
func _along_street(host: SimHost, at: Vector3) -> Vector2:
	var best := Vector2.ZERO
	var best_score := -1.0
	for n in 24:
		var dir := Vector2.from_angle(TAU * n / 24.0)
		var free := 0
		while free < 15 and host.buildings_near(at.x + dir.x * (free + 1) * 10.0, at.z + dir.y * (free + 1) * 10.0, 6.0) == 0:
			free += 1
		var back := true
		for k in range(1, 5):
			if host.buildings_near(at.x - dir.x * k * 10.0, at.z - dir.y * k * 10.0, 7.0) > 0:
				back = false
		if not back:
			continue
		var beside := host.buildings_near(at.x + dir.x * 40.0, at.z + dir.y * 40.0, 30.0)
		var score := free * 100.0 + beside
		if score > best_score:
			best_score = score
			best = dir
	return best


## Mean position of `units`.
func _positions(host: SimHost, units: PackedInt64Array) -> Vector3:
	var ids := host.unit_ids()
	var positions := host.current_positions()
	var sum := Vector3.ZERO
	for id in units:
		sum += positions[Array(ids).find(id)]
	return sum / units.size()


## As `atmosphere.gd` `row_depth` (own copy: the tool also takes the
## pictures before prompt 183, with the old atmosphere).
static func _row_depth(elevation: float, fov: float, row: float) -> float:
	var up := atan((row * 2.0 - 1.0) * tan(fov * 0.5))
	return sin(elevation) * cos(up) / sin(maxf(elevation - up, deg_to_rad(1.0)))


func _save(file: String) -> Image:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	image.save_png(dir.path_join(file))
	print("dof shots: %s" % dir.path_join(file))
	return image


func _frames(count: int) -> void:
	for i in count:
		await get_tree().process_frame
