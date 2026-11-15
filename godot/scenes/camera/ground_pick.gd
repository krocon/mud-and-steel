extends RefCounted
## Turns screen positions into points on the ground.
##
## Pure math, no Godot physics. The ground is currently the flat plane y = 0;
## prompt 011 replaces `intersect_ray` with a lookup against the height field,
## and every caller (camera, mouse commands) follows automatically.
##
## Use it via `const GroundPick := preload("res://scenes/camera/ground_pick.gd")`.

const GROUND_PLANE := Plane(Vector3.UP, 0.0)


## Returns the ground point hit by the ray, or null if the ray misses
## (pointing at the sky or parallel to the ground).
static func intersect_ray(origin: Vector3, direction: Vector3) -> Variant:
	return GROUND_PLANE.intersects_ray(origin, direction)


## Returns the ground point under `screen_position` as seen by `camera`, or null.
static func from_screen(camera: Camera3D, screen_position: Vector2) -> Variant:
	return intersect_ray(
		camera.project_ray_origin(screen_position), camera.project_ray_normal(screen_position)
	)
