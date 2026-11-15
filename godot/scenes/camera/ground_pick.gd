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


## Returns the ground point hit by the ray, or null if the ray misses
## (pointing at the sky or parallel to the ground).
static func intersect_ray(origin: Vector3, direction: Vector3) -> Variant:
	if height_source != null:
		return height_source.ray_pick(origin, direction)
	return GROUND_PLANE.intersects_ray(origin, direction)


## Ground height at (x, z) in meters.
static func height_at(x: float, z: float) -> float:
	if height_source != null:
		return height_source.height_at(x, z)
	return 0.0


## Returns the ground point under `screen_position` as seen by `camera`, or null.
static func from_screen(camera: Camera3D, screen_position: Vector2) -> Variant:
	return intersect_ray(
		camera.project_ray_origin(screen_position), camera.project_ray_normal(screen_position)
	)
