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
## Prompt 194: throws by the simulation of at least this length (m) count
## as a big blast: own ragdoll limit (`Ragdolls.MAX_BURST`), and the weapon
## may fly off with this chance.
const BURST_FROM_M := 3.0
const WEAPON_BLAST_CHANCE := 0.6
## Gravity of a thrown body in game time: the simulation's flight time is
## real time divided by the time factor, so bodies fall factor² as fast.
const FLIGHT_GRAVITY := 9.81 * 4.0

## Cover clip tables (prompt 206: avoid per-frame array allocations).
const TRENCH_CLIPS: Array[String] = ["trench_idle", "trench_aim", "trench_fire"]
const LOW_COVER_CLIPS: Array[String] = ["cover_low", "cover_low_aim", "cover_low_fire"]
const TALL_COVER_R_CLIPS: Array[String] = ["cover_tall", "cover_peek_r_aim", "cover_peek_r_fire"]
const TALL_COVER_L_CLIPS: Array[String] = ["cover_tall", "cover_peek_l_aim", "cover_peek_l_fire"]

var view: Node3D
var ragdolls: Ragdolls
var helmets: Helmets
## Presentation randomness only (never the simulation's).
var rng := RandomNumberGenerator.new()
## Cover object kinds by object index: "low", "tall", "trench".
var _cover_kinds: Dictionary[int, Dictionary] = {}
## Cached `get_up_back` length in game seconds (prompt 206).
var _get_up_seconds := -1.0
## Statistics for the self-test: knockdowns, get-ups, dives, rolls.
var counts := {"knocked": 0, "got_up": 0, "dive": 0, "roll": 0, "cover": 0, "thrown": 0}


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
	var target_id := int(event["target"])
	if not view._by_unit.has(target_id):
		return
	var keys: Array = view._by_unit[target_id]
	if keys.is_empty():
		return
	var at: Vector3 = event["position"]
	var best: Object = null
	var best_d := INF
	for key: int in keys:
		var s: Object = view._soldiers.get(key)
		if s == null:
			continue
		var p: Vector3 = s.position
		var d := Vector2(p.x - at.x, p.z - at.z).length_squared()
		if d < best_d:
			best_d = d
			best = s
	if best == null:
		return
	var shooter: Node3D = view.main._unit_nodes.get(int(event["shooter"]))
	if shooter != null:
		var dir: Vector3 = best.position - shooter.global_position
		dir.y = 0.0
		if dir.length() > 0.01:
			best.hit_dir = dir.normalized()
			best.has_hit_dir = true
	var member: Node3D = best.node
	var camera_position: Vector3 = view.main.camera.global_position
	if not member.visible or member.global_position.distance_squared_to(camera_position) > Ragdolls.MAX_DISTANCE_M * Ragdolls.MAX_DISTANCE_M:
		return
	if best.skeleton == null:
		best.skeleton = SoldierLook.find_skeleton(member)
	var skeleton: Skeleton3D = best.skeleton
	var head := skeleton.find_bone("Head") if skeleton != null else -1
	if head < 0:
		return
	var head_at := skeleton.global_transform * skeleton.get_bone_global_pose(head).origin
	if head_at.distance_to(at) <= HEAD_HIT_M and rng.randf() < HELMET_HEAD_CHANCE:
		var dir: Vector3 = best.hit_dir if best.has_hit_dir else Vector3.FORWARD
		_helmet_off(best, dir * 2.0 + Vector3.UP * 2.5)


## The body's wanted yaw: prone soldiers moving sideways roll and keep
## facing their aim.
func body_yaw(s: Object, yaw: float, aim: float, moving: bool, stance: int, down: bool) -> float:
	s.roll_dir = 0
	if down:
		return yaw
	if moving and stance == 2:
		var diff := wrapf(yaw - aim, -PI, PI)
		if absf(absf(diff) - PI * 0.5) < ROLL_WINDOW:
			s.roll_dir = 1 if diff < 0.0 else -1
			return aim
	return yaw


## Falling, lying and getting up. Returns true when the soldier's clip is
## decided here (the view's normal choice is skipped).
func animate(s: Object, dead: bool, knocked: int, from: Vector3, at: Vector3,
		camera_position: Vector3, tick_rate: float) -> bool:
	var player: AnimationPlayer = s.player
	if player == null:
		return false
	if not s.has_knock_from:
		s.knock_from = from
		s.has_knock_from = true
		s.knocked = knocked
	var blast := from.distance_squared_to(s.knock_from) > 0.0001
	var renewed: bool = knocked > int(s.knocked)
	s.knock_from = from
	s.knocked = knocked
	var member: Node3D = s.node
	if s.dead:
		return true
	if dead:
		s.hold_yaw = true
		var dir: Vector3 = _away(at, from, s) if blast else ((s.hit_dir as Vector3) if s.has_hit_dir else _back_of(s))
		var velocity: Vector3 = dir * HIT_SPEED + Vector3.UP * 0.3
		var spin := Vector3.UP.cross(dir) * HIT_SPIN
		var has_fl: bool = blast and s.has_flight
		if blast:
			velocity = dir * THROW_SPEED * rng.randf_range(0.8, 1.2) + Vector3.UP * THROW_UP
			spin = Vector3.UP.cross(dir) * THROW_SPIN * rng.randf_range(0.7, 1.3)
			if has_fl:
				velocity = _flight_velocity(s.flight_to, s.flight_left, at)
			_tear_off(s, velocity, has_fl)
		var burst: bool = has_fl and float(s.flight_distance) >= BURST_FROM_M
		if ragdolls.can_start(at, camera_position, burst):
			if not has_fl:
				ragdolls.start(member, velocity, spin, at.y, "dead")
			else:
				ragdolls.start(member, velocity, spin, s.flight_to.y, "dead",
						s.flight_to, FLIGHT_GRAVITY / 9.8)
		# The view plays a death clip (under a ragdoll it drops the weapon).
		return false
	if knocked > 0:
		if not s.down or (renewed and not s.getting_up and not ragdolls.is_active(member)):
			_knock_down(s, from, at, camera_position)
		var left := float(knocked) / maxf(tick_rate, 1.0)
		if _get_up_seconds < 0.0:
			var clips: Dictionary = SoldierLook.clips()
			var entry: Variant = clips.get("get_up_back")
			var len_s := float(entry.get("length", 2.6)) if entry is Dictionary else 2.6
			_get_up_seconds = len_s / view.TIME_FACTOR
		if not s.getting_up and left <= _get_up_seconds:
			_get_up(s)
		return true
	if s.down:
		s.down = false
		s.getting_up = false
		s.hold_yaw = false
		s.clip = ""
	return false


func _knock_down(s: Object, from: Vector3, at: Vector3, camera_position: Vector3) -> void:
	var member: Node3D = s.node
	var player: AnimationPlayer = s.player
	s.down = true
	s.getting_up = false
	s.hold_yaw = true
	counts["knocked"] += 1
	var dir := _away(at, from, s)
	# The knockdown clip falls backwards: his back turns to the blast's far side.
	s.yaw = atan2(dir.x, dir.z)
	member.rotation.y = s.yaw
	s.last_yaw = s.yaw
	if player.has_animation("knockdown"):
		player.speed_scale = view.TIME_FACTOR
		s.last_speed_scale = view.TIME_FACTOR
		player.play("knockdown", 0.1)
		s.clip = "knockdown"
	var velocity := dir * THROW_SPEED * rng.randf_range(0.8, 1.2) + Vector3.UP * THROW_UP * rng.randf_range(0.8, 1.1)
	var has_fl: bool = s.has_flight
	if has_fl:
		velocity = _flight_velocity(s.flight_to, s.flight_left, at)
	_tear_off(s, velocity, has_fl)
	var spin := Vector3.UP.cross(dir) * THROW_SPIN * rng.randf_range(0.7, 1.3)
	var burst: bool = has_fl and float(s.flight_distance) >= BURST_FROM_M
	if ragdolls.can_start(at, camera_position, burst):
		if not has_fl:
			ragdolls.start(member, velocity, spin, at.y, "down")
		else:
			ragdolls.start(member, velocity, spin, s.flight_to.y, "down",
					s.flight_to, FLIGHT_GRAVITY / 9.8)


func _get_up(s: Object) -> void:
	var member: Node3D = s.node
	var player: AnimationPlayer = s.player
	s.getting_up = true
	counts["got_up"] += 1
	helmets.pick_up(member)
	var clip := "get_up_back"
	var blend := 0.2
	var lying := ragdolls.lying(member)
	if not lying.is_empty():
		var head: Vector3 = lying["head"]
		if lying["back"]:
			# The clip starts on the back, the head behind the root.
			s.yaw = atan2(head.x, head.z)
		else:
			clip = "get_up_front"
			s.yaw = atan2(-head.x, -head.z)
		member.rotation.y = s.yaw
		s.last_yaw = s.yaw
		ragdolls.release(member, RELEASE_SECONDS)
		blend = 0.0
	if player.has_animation(clip):
		player.speed_scale = view.TIME_FACTOR
		s.last_speed_scale = view.TIME_FACTOR
		player.play(clip, blend)
		s.clip = clip


## Plays a one-shot clip (the dive when the squad goes prone); true while it
## runs.
func one_shot(s: Object, stance: int, moving: bool, _now: int) -> bool:
	var last: int = s.last_stance if s.last_stance >= 0 else stance
	s.last_stance = stance
	var player: AnimationPlayer = s.player
	if stance == 2 and last != 2 and not moving and player.has_animation("dive"):
		player.speed_scale = view.TIME_FACTOR
		s.last_speed_scale = view.TIME_FACTOR
		player.play("dive", view.BLEND_SECONDS)
		s.clip = "dive"
		counts["dive"] += 1
	# Game time, not wall time: the player advances with the game.
	return s.clip == "dive" and player.is_playing() and player.current_animation == "dive"


## Locomotion of prompt 048: rolling sideways, crouched walking. Empty: the
## view's own choice.
func moving_clip(s: Object, stance: int, speed: float) -> String:
	var roll: int = s.roll_dir
	if stance == 2 and roll != 0:
		if s.clip != "roll_right" and s.clip != "roll_left":
			counts["roll"] += 1
		return "roll_right" if roll > 0 else "roll_left"
	if stance == 1 and speed < view.RUN_FROM:
		return "crouch_walk"
	return ""


## Standing still: cover poses by the cover object, stepping while turning.
## `phase` 0 idle, 1 aiming, 2 firing; `default` is the view's clip.
func standing_clip(s: Object, stance: int, phase: int, cover: int, default: String) -> String:
	var clip := default
	if cover >= 0 and stance != 2:
		var kind := _cover_kind(cover)
		if not kind.is_empty():
			match kind:
				"trench":
					clip = TRENCH_CLIPS[phase]
				"low":
					clip = LOW_COVER_CLIPS[phase]
				"tall":
					if stance == 0:
						clip = TALL_COVER_R_CLIPS[phase] if _peek_right(s, cover) else TALL_COVER_L_CLIPS[phase]
					elif phase == 0:
						clip = "crouch"
			if clip != default and s.clip != clip and phase == 0:
				counts["cover"] += 1
	if clip == "idle" and absf(s.turning) > TURN_STEP:
		clip = "turn_in_place"
	if clip == s.clip or clip == default:
		return clip
	var player: AnimationPlayer = s.player
	return clip if player.has_animation(clip) else default


## Head-first turning: near soldiers get a TurnLead modifier whose target
## is the angle between where they want to face and where the body faces.
func lead(s: Object, want_yaw: float, near: bool) -> void:
	var lagging := wrapf(want_yaw - s.yaw, -PI, PI)
	var still: bool = s.dead or s.hold_yaw
	s.turning = 0.0 if still else lagging
	var modifier: SkeletonModifier3D = s.lead
	if modifier == null:
		if not near or still or absf(lagging) < LEAD_FROM:
			return
		var skeleton: Skeleton3D = s.skeleton
		if skeleton == null:
			skeleton = SoldierLook.find_skeleton(s.node)
			s.skeleton = skeleton
		if skeleton == null:
			return
		modifier = TurnLead.new()
		modifier.name = "TurnLead"
		skeleton.add_child(modifier)
		s.lead = modifier
	# Only real turns: small aim corrections need no lead (and no script
	# work per soldier and frame).
	var target := lagging if near and not still and absf(lagging) >= LEAD_FROM else 0.0
	if modifier.target != target:
		modifier.target = target
	var active := absf(target) > 0.0 or absf(modifier.lead) > 0.01
	if modifier.active != active:
		modifier.active = active


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


## Whether the soldier leans out on the right side of his cover.
func _peek_right(s: Object, cover: int) -> bool:
	if not _cover_kinds.has(cover):
		return true
	var info: Dictionary = _cover_kinds[cover]
	if not info.has("position"):
		return true
	var yaw: float = s.yaw
	var right := Vector3(cos(yaw), 0.0, -sin(yaw))
	var rel: Vector3 = s.position - (info["position"] as Vector3)
	return rel.dot(right) >= 0.0


## Prompt 194: where a thrown soldier flies, on the arc of the
## simulation: from `from` to `to` (his landing point), `f` of the flight
## done, `left` seconds to go. Remembers the flight for the ragdoll and
## returns the point on the arc (45 degrees: the apex at a quarter of the
## distance).
func flight_position(s: Object, from: Vector3, to: Vector3, f: float, left: float) -> Vector3:
	var distance := Vector2(to.x - from.x, to.z - from.z).length()
	var at := from.lerp(to, f) + Vector3.UP * (distance * f * (1.0 - f))
	s.has_flight = true
	s.flight_to = to
	s.flight_left = left
	s.flight_distance = distance
	if distance >= BURST_FROM_M and not s.flight_counted:
		s.flight_counted = true
		counts["thrown"] = int(counts.get("thrown", 0)) + 1
	return at


## The flight is over (or there is none).
func flight_ended(s: Object) -> void:
	s.has_flight = false
	s.flight_counted = false


## Start velocity of a ragdoll at `at` that lands on the simulation's point
## `to` after `left` seconds under FLIGHT_GRAVITY.
func _flight_velocity(to: Vector3, left: float, at: Vector3) -> Vector3:
	var t := maxf(left, 0.05)
	var v := (to - at) / t
	v.y = FLIGHT_GRAVITY * t * 0.5 - (at.y - to.y) / t
	return v


## Helmet and, for far throws, the weapon fly off.
## Helmets and weapons fall at real gravity: half the speed of a body on
## the game-time arc draws the same arc.
func _tear_off(s: Object, velocity: Vector3, has_flight: bool) -> void:
	var k := 0.5 if has_flight else 1.0
	if rng.randf() < HELMET_BLAST_CHANCE:
		_helmet_off(s, velocity * 1.4 * k + Vector3.UP)
	if has_flight and s.flight_distance >= BURST_FROM_M and rng.randf() < WEAPON_BLAST_CHANCE:
		var spin := Vector3(rng.randf_range(-1, 1), rng.randf_range(-1, 1), rng.randf_range(-1, 1)) * 7.0
		var v := velocity * k * rng.randf_range(0.6, 1.2) + Vector3.UP * rng.randf_range(0.5, 2.0)
		helmets.throw_weapon(s.node, v, spin, s.flight_to.y)


func _helmet_off(s: Object, velocity: Vector3) -> void:
	var member: Node3D = s.node
	var spin := Vector3(rng.randf_range(-1, 1), rng.randf_range(-1, 1), rng.randf_range(-1, 1)) * 9.0
	helmets.knock_off(member, s.faction, view._biome, velocity, spin, s.position.y)


## Horizontal direction from the blast to the soldier (backwards if he
## stood on it).
func _away(at: Vector3, from: Vector3, s: Object) -> Vector3:
	var dir := at - from
	dir.y = 0.0
	return dir.normalized() if dir.length() > 0.05 else _back_of(s)


func _back_of(s: Object) -> Vector3:
	var yaw: float = s.yaw
	return Vector3(sin(yaw), 0.0, cos(yaw))
