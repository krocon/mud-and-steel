extends RefCounted
## Turns screen positions into points on the ground.
##
## Pure math, no Godot physics. Once `height_source` is set (main.gd sets it
## to the SimHost after the map is loaded), rays are intersected with the
## simulation's height field (`SimHost.ray_pick`: ray marching plus
## bisection). Without it, the ground is the flat plane y = 0.
##
## Use it via `const GroundPick := preload("res://scenes/camera/ground_pick.gd")`.

const GROUND_PLANE := Plane(Vector3.UP, 0.0)

## Object with `ray_pick(origin, direction) -> Variant` and
## `height_at(x, z) -> float` (the SimHost), or null for a flat ground.
static var height_source: Object = null


## Whether `(point.x, point.z)` lies inside the playable map rectangle
## (prompt 222: clicks outside the map never give orders or editor strokes).
static func in_map(point: Vector3) -> bool:
	if height_source == null or not height_source.has_method(&"map_size"):
		return true
	var half: Vector2 = height_source.map_size() * 0.5
	return absf(point.x) <= half.x + 0.001 and absf(point.z) <= half.y + 0.001


## Returns the ground point hit by the ray inside the playable map, or null if
## the ray misses or hits the non-playable surround.
static func intersect_ray(origin: Vector3, direction: Vector3) -> Variant:
	var hit: Variant = intersect_ray_any(origin, direction)
	if hit == null or not in_map(hit as Vector3):
		return null
	return hit


## Returns the ground point hit by the ray including outside the map bounds
## (used only for camera pan-drag and zoom-anchor math; never for orders).
static func intersect_ray_any(origin: Vector3, direction: Vector3) -> Variant:
	if height_source != null:
		return height_source.ray_pick(origin, direction)
	return GROUND_PLANE.intersects_ray(origin, direction)


## Ground height at (x, z) in meters.
static func height_at(x: float, z: float) -> float:
	if height_source != null:
		return height_source.height_at(x, z)
	return 0.0


## Highest top (y) of the standing buildings and solid rocks whose ground
## box, grown by `margin` meters, contains (x, z); a very low value if none
## (prompt 078: the camera keeps above apartment blocks).
static func object_top_at(x: float, z: float, margin: float) -> float:
	if height_source != null and height_source.has_method(&"object_top_at"):
		return height_source.object_top_at(x, z, margin)
	return -1e9


## Returns the playable ground point under `screen_position`, or null outside the map.
static func from_screen(camera: Camera3D, screen_position: Vector2) -> Variant:
	return intersect_ray(
		camera.project_ray_origin(screen_position), camera.project_ray_normal(screen_position)
	)


## Unbounded version of `from_screen` for camera pan drag and zoom anchoring.
static func from_screen_any(camera: Camera3D, screen_position: Vector2) -> Variant:
	return intersect_ray_any(
		camera.project_ray_origin(screen_position), camera.project_ray_normal(screen_position)
	)
