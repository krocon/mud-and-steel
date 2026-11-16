extends Node3D
## Soldier motion of prompt 048, used by `infantry_view.gd`: falling and
## getting up, ragdolls, flying helmets, the clips for cover, crouching,
## diving and rolling, and turning head first. Presentation only: the
## simulation decides who dies or is knocked down, where he lies, when he
## gets up and where he takes cover; random choices here (impulses, helmets)
## come from a presentation RandomNumberGenerator.

const Ragdolls := preload("res://scenes/world/ragdolls.gd")
const Helmets := preload("res://scenes/world/helmets.gd")
const TurnLead := preload("res://scenes/world/turn_lead.gd")

## The body turns this much slower than the head when standing (share of
## the view's turn rate), so the head and chest visibly lead.
const STANDING_TURN := 0.4
## Standing soldiers whose body lags their aim by more than this (radians)
## step on the spot (`turn_in_place`).
const TURN_STEP := 0.35
## Turns smaller than this (radians) get no head-first lead.
const LEAD_FROM := 0.15
## Prone soldiers moving within this angle (radians) of sideways roll.
const ROLL_WINDOW := PI / 6.0
## Objects up to this height (m) are low cover, higher ones tall cover.
const LOW_COVER_M := 1.5
## Throw of a knocked-down soldier: horizontal and upward speed (m/s); the
## simulation throws him up to 2.5 m [provisional].
const THROW_SPEED := 2.4
const THROW_UP := 3.2
## Spin of a thrown body (rad/s) and of one hit by a bullet.
const THROW_SPIN := 4.0
const HIT_SPEED := 1.2
const HIT_SPIN := 2.5
## Chance that a blast takes the helmet off, and a hit near the head.
const HELMET_BLAST_CHANCE := 0.5
const HELMET_HEAD_CHANCE := 0.6
## Hits within this distance (m) of the head bone count as head hits.
const HEAD_HIT_M := 0.3
## Blend from a ragdoll into the get-up clip (s).
const RELEASE_SECONDS := 0.5

var view: Node3D
var ragdolls: Ragdolls
var helmets: Helmets
## Presentation randomness only (never the simulation's).
var rng := RandomNumberGenerator.new()
## Cover object kinds by object index: "low", "tall", "trench".
var _cover_kinds: Dictionary[int, Dictionary] = {}
## Statistics for the self-test: knockdowns, get-ups, dives, rolls.
var counts := {"knocked": 0, "got_up": 0, "dive": 0, "roll": 0, "cover": 0}


func setup(infantry_view: Node3D) -> void:
	view = infantry_view
	rng.seed = 48
	ragdolls = Ragdolls.new()
	ragdolls.name = "Ragdolls"
	add_child(ragdolls)
	helmets = Helmets.new()
	helmets.name = "Helmets"
	add_child(helmets)


## Remembers where a soldier was hit from (direction of his fall) and takes
## the helmet off on hits at the head.
func on_hit(event: Dictionary) -> void:
	if event.get("surface", "") != "unit":
		return
	var keys: Array = view._by_unit.get(int(event["target"]), [])
	if keys.is_empty():
		return
	var at: Vector3 = event["position"]
	var best: Dictionary = {}
	var best_d := INF
	for key: int in keys:
		var s: Dictionary = view._soldiers[key]
		var p: Vector3 = s["position"]
		var d := Vector2(p.x - at.x, p.z - at.z).length_squared()
		if d < best_d:
			best_d = d
			best = s
	if best.is_empty():
		return
	var shooter: Node3D = view.main._unit_nodes.get(int(event["shooter"]))
	if shooter != null:
		var dir: Vector3 = (best["position"] as Vector3) - shooter.global_position
		dir.y = 0.0
		if dir.length() > 0.01:
			best["hit_dir"] = dir.normalized()
	var member: Node3D = best["node"]
	var camera_position: Vector3 = view.main.camera.global_position
	if not member.visible or member.global_position.distance_to(camera_position) > Ragdolls.MAX_DISTANCE_M:
		return
	if not best.has("skeleton"):
		best["skeleton"] = SoldierLook.find_skeleton(member)
	var skeleton: Skeleton3D = best["skeleton"]
	var head := skeleton.find_bone("Head") if skeleton != null else -1
	if head < 0:
		return
	var head_at := skeleton.global_transform * skeleton.get_bone_global_pose(head).origin
	if head_at.distance_to(at) <= HEAD_HIT_M and rng.randf() < HELMET_HEAD_CHANCE:
		var dir: Vector3 = best.get("hit_dir", Vector3.FORWARD)
		_helmet_off(best, dir * 2.0 + Vector3.UP * 2.5)


## The body's wanted yaw: prone soldiers moving sideways roll and keep
## facing their aim.
func body_yaw(s: Dictionary, yaw: float, aim: float, moving: bool, stance: int, down: bool) -> float:
	s["roll_dir"] = 0
	if down:
		return yaw
	if moving and stance == 2:
		var diff := wrapf(yaw - aim, -PI, PI)
		if absf(absf(diff) - PI * 0.5) < ROLL_WINDOW:
			s["roll_dir"] = 1 if diff < 0.0 else -1
			return aim
	return yaw


## Falling, lying and getting up. Returns true when the soldier's clip is
## decided here (the view's normal choice is skipped).
func animate(s: Dictionary, dead: bool, knocked: int, from: Vector3, at: Vector3,
		camera_position: Vector3, tick_rate: float) -> bool:
	var player: AnimationPlayer = s["player"]
	if player == null:
		return false
	if not s.has("knock_from"):
		s["knock_from"] = from
		s["knocked"] = knocked
	var blast := from.distance_squared_to(s["knock_from"]) > 0.0001
	var renewed := knocked > int(s["knocked"])
	s["knock_from"] = from
	s["knocked"] = knocked
	var member: Node3D = s["node"]
	if s["dead"]:
		return true
	if dead:
		s["hold_yaw"] = true
		var dir := _away(at, from, s) if blast else (s.get("hit_dir", _back_of(s)) as Vector3)
		var velocity := dir * HIT_SPEED + Vector3.UP * 0.3
		var spin := Vector3.UP.cross(dir) * HIT_SPIN
		if blast:
			velocity = dir * THROW_SPEED * rng.randf_range(0.8, 1.2) + Vector3.UP * THROW_UP
			spin = Vector3.UP.cross(dir) * THROW_SPIN * rng.randf_range(0.7, 1.3)
			if rng.randf() < HELMET_BLAST_CHANCE:
				_helmet_off(s, velocity * 1.4 + Vector3.UP)
		if ragdolls.can_start(at, camera_position):
			ragdolls.start(member, velocity, spin, at.y, "dead")
		# The view plays a death clip (under a ragdoll it drops the weapon).
		return false
	if knocked > 0:
		if not s.get("down", false) or (renewed and not s.get("getting_up", false) and not ragdolls.is_active(member)):
			_knock_down(s, from, at, camera_position)
		var left := float(knocked) / maxf(tick_rate, 1.0)
		var get_up: float = float(SoldierLook.clips().get("get_up_back", {}).get("length", 2.6)) / view.TIME_FACTOR
		if not s.get("getting_up", false) and left <= get_up:
			_get_up(s)
		return true
	if s.get("down", false):
		s["down"] = false
		s["getting_up"] = false
		s["hold_yaw"] = false
		s["clip"] = ""
	return false


func _knock_down(s: Dictionary, from: Vector3, at: Vector3, camera_position: Vector3) -> void:
	var member: Node3D = s["node"]
	var player: AnimationPlayer = s["player"]
	s["down"] = true
	s["getting_up"] = false
	s["hold_yaw"] = true
	counts["knocked"] += 1
	var dir := _away(at, from, s)
	# The knockdown clip falls backwards: his back turns to the blast's far side.
	s["yaw"] = atan2(dir.x, dir.z)
	member.rotation.y = s["yaw"]
	if player.has_animation("knockdown"):
		player.speed_scale = view.TIME_FACTOR
		player.play("knockdown", 0.1)
		s["clip"] = "knockdown"
	var velocity := dir * THROW_SPEED * rng.randf_range(0.8, 1.2) + Vector3.UP * THROW_UP * rng.randf_range(0.8, 1.1)
	if rng.randf() < HELMET_BLAST_CHANCE:
		_helmet_off(s, velocity * 1.4 + Vector3.UP)
	if ragdolls.can_start(at, camera_position):
		ragdolls.start(member, velocity, Vector3.UP.cross(dir) * THROW_SPIN * rng.randf_range(0.7, 1.3),
				at.y, "down")


func _get_up(s: Dictionary) -> void:
	var member: Node3D = s["node"]
	var player: AnimationPlayer = s["player"]
	s["getting_up"] = true
	counts["got_up"] += 1
	var clip := "get_up_back"
	var blend := 0.2
	var lying := ragdolls.lying(member)
	if not lying.is_empty():
		var head: Vector3 = lying["head"]
		if lying["back"]:
			# The clip starts on the back, the head behind the root.
			s["yaw"] = atan2(head.x, head.z)
		else:
			clip = "get_up_front"
			s["yaw"] = atan2(-head.x, -head.z)
		member.rotation.y = s["yaw"]
		ragdolls.release(member, RELEASE_SECONDS)
		blend = 0.0
	if player.has_animation(clip):
		player.speed_scale = view.TIME_FACTOR
		player.play(clip, blend)
		s["clip"] = clip


## Plays a one-shot clip (the dive when the squad goes prone); true while it
## runs.
func one_shot(s: Dictionary, stance: int, moving: bool, _now: int) -> bool:
	var last := int(s.get("last_stance", stance))
	s["last_stance"] = stance
	var player: AnimationPlayer = s["player"]
	if stance == 2 and last != 2 and not moving and player.has_animation("dive"):
		player.speed_scale = view.TIME_FACTOR
		player.play("dive", view.BLEND_SECONDS)
		s["clip"] = "dive"
		counts["dive"] += 1
	# Game time, not wall time: the player advances with the game.
	return s["clip"] == "dive" and player.is_playing() and player.current_animation == "dive"


## Locomotion of prompt 048: rolling sideways, crouched walking. Empty: the
## view's own choice.
func moving_clip(s: Dictionary, stance: int, speed: float) -> String:
	var roll := int(s.get("roll_dir", 0))
	if stance == 2 and roll != 0:
		if s["clip"] != "roll_right" and s["clip"] != "roll_left":
			counts["roll"] += 1
		return "roll_right" if roll > 0 else "roll_left"
	if stance == 1 and speed < view.RUN_FROM:
		return "crouch_walk"
	return ""


## Standing still: cover poses by the cover object, stepping while turning.
## `phase` 0 idle, 1 aiming, 2 firing; `default` is the view's clip.
func standing_clip(s: Dictionary, stance: int, phase: int, cover: int, default: String) -> String:
	var clip := default
	var kind := _cover_kind(cover)
	if stance != 2 and not kind.is_empty():
		match kind:
			"trench":
				clip = ["trench_idle", "trench_aim", "trench_fire"][phase]
			"low":
				clip = ["cover_low", "cover_low_aim", "cover_low_fire"][phase]
			"tall":
				if stance == 0:
					var side := _peek_side(s, cover)
					clip = ["cover_tall", "cover_peek_%s_aim" % side, "cover_peek_%s_fire" % side][phase]
				elif phase == 0:
					clip = "crouch"
		if clip != default and s["clip"] != clip and phase == 0:
			counts["cover"] += 1
	if clip == "idle" and absf(float(s.get("turning", 0.0))) > TURN_STEP:
		clip = "turn_in_place"
	var player: AnimationPlayer = s["player"]
	return clip if player.has_animation(clip) else default


## Head-first turning: near soldiers get a TurnLead modifier whose target
## is the angle between where they want to face and where the body faces.
func lead(s: Dictionary, want_yaw: float, near: bool) -> void:
	var lagging := wrapf(want_yaw - float(s["yaw"]), -PI, PI)
	var still: bool = s["dead"] or s.get("hold_yaw", false)
	s["turning"] = 0.0 if still else lagging
	var modifier: SkeletonModifier3D = s.get("lead")
	if modifier == null:
		if not near or still or absf(lagging) < LEAD_FROM:
			return
		var skeleton := SoldierLook.find_skeleton(s["node"])
		if skeleton == null:
			return
		modifier = TurnLead.new()
		modifier.name = "TurnLead"
		skeleton.add_child(modifier)
		s["lead"] = modifier
	# Only real turns: small aim corrections need no lead (and no script
	# work per soldier and frame).
	modifier.target = lagging if near and not still and absf(lagging) >= LEAD_FROM else 0.0
	modifier.active = absf(modifier.target) > 0.0 or absf(modifier.lead) > 0.01


func _cover_kind(cover: int) -> String:
	if cover < 0:
		return ""
	if not _cover_kinds.has(cover):
		var info: Dictionary = view.sim_host.cover_object_info(cover)
		var kind := ""
		if not info.is_empty():
			if str(info["id"]).contains("trench"):
				kind = "trench"
			elif float(info["height"]) <= LOW_COVER_M:
				kind = "low"
			else:
				kind = "tall"
		info["kind"] = kind
		_cover_kinds[cover] = info
	return _cover_kinds[cover]["kind"]


## Which side of his cover a soldier leans out: the side of the object's
## center he stands on, seen along his facing.
func _peek_side(s: Dictionary, cover: int) -> String:
	var info: Dictionary = _cover_kinds.get(cover, {})
	if not info.has("position"):
		return "r"
	var yaw := float(s["yaw"])
	var right := Vector3(cos(yaw), 0.0, -sin(yaw))
	var rel: Vector3 = (s["position"] as Vector3) - (info["position"] as Vector3)
	return "r" if rel.dot(right) >= 0.0 else "l"


func _helmet_off(s: Dictionary, velocity: Vector3) -> void:
	var member: Node3D = s["node"]
	var spin := Vector3(rng.randf_range(-1, 1), rng.randf_range(-1, 1), rng.randf_range(-1, 1)) * 9.0
	helmets.knock_off(member, s["faction"], view._biome, velocity, spin, (s["position"] as Vector3).y)


## Horizontal direction from the blast to the soldier (backwards if he
## stood on it).
func _away(at: Vector3, from: Vector3, s: Dictionary) -> Vector3:
	var dir := at - from
	dir.y = 0.0
	return dir.normalized() if dir.length() > 0.05 else _back_of(s)


func _back_of(s: Dictionary) -> Vector3:
	var yaw := float(s["yaw"])
	return Vector3(sin(yaw), 0.0, cos(yaw))
