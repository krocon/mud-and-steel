extends RefCounted
## Particle systems of the effects (prompt 019). `build(kind, ...)` returns a
## fresh `GPUParticles3D` for one effect kind; `effects.gd` keeps pools of
## them. Sizes are meters for a 7.5 cm gun; the emitter's node scale scales
## an effect (particles inherit the emission transform, size and speed).
##
## All particles are camera-facing quads drawn with one small spatial shader
## (`_shader`): texture × particle color × per-instance `tint` (ground color),
## lit with a rounded fake normal so puffs read as volumes in the sun, or
## unshaded (fire, flashes, sparks; HDR values feed the glow). Soft
## particles fade where they cut into the ground (quality "fx_soft").
##
## Prompt 200: at medium and high quality (`fx_clouds`, `use_clouds`) smoke,
## dust and haze are drawn as clouds rather than particles: fewer, bigger,
## heavily overlapping puffs of a soft texture without a rim (`cloud`), that
## do not spin, always soft against the ground, and lit as one body: the
## normal comes from where a puff sits in its cloud (`cloud_center`, set by
## whoever places the emitter), so the sun side and the shadow side run over
## the whole column instead of over every quad. Presentation only.

const EffectTextures := preload("res://scenes/world/effect_textures.gd")

## Kinds that are fired as one-shot bursts (pooled).
const BURSTS: Array[String] = [
	"flash", "powder", "blast_dust", "fountain", "spray", "impact_dust", "puff",
	"sparks", "fireball", "explosion_smoke", "debris",
	"core", "rays", "streaks", "earth_cloud", "embers", "dust_cloud", "rocket_blast",
	"muzzle_core", "muzzle_tongue", "muzzle_gas", "muzzle_dust", "muzzle_puffs",
	"smoke_collar", "ground_haze",
	"water_column", "water_jets", "water_drops", "water_ring",
]
## Kinds that run continuously (wreck fires, vehicles, a firing rocket
## position).
const CONTINUOUS: Array[String] = [
	"fire", "smoke_column", "wreck_collar", "dust_trail", "exhaust", "salvo_smoke",
	"mud_clods", "snow_dust", "bow_spray", "wake_foam", "waterfall_mist",
]

## Prompt 200: kinds drawn as clouds at medium and high quality, with
## (share of the particles, factor on the puff size): fewer, bigger puffs
## that overlap into one mass; about the same covered area.
const CLOUDS := {
	"powder": Vector2(0.7, 1.25),
	"earth_cloud": Vector2(0.6, 1.3),
	"explosion_smoke": Vector2(0.65, 1.32),
	"smoke_collar": Vector2(0.65, 1.25),
	"ground_haze": Vector2(0.7, 1.25),
	"dust_cloud": Vector2(0.65, 1.25),
	"smoke_column": Vector2(0.6, 1.35),
	"wreck_collar": Vector2(0.6, 1.25),
	"dust_trail": Vector2(0.6, 1.3),
	"salvo_smoke": Vector2(0.65, 1.25),
	"blast_dust": Vector2(0.7, 1.2),
	"impact_dust": Vector2(0.7, 1.2),
	"muzzle_dust": Vector2(0.7, 1.2),
	"exhaust": Vector2(0.7, 1.2),
}

const SHADER_PATHS: Dictionary[String, String] = {
	"lit": "res://assets/shaders/particle_lit.gdshader",
	"unshaded": "res://assets/shaders/particle_unshaded.gdshader",
	"add": "res://assets/shaders/particle_add.gdshader",
	"billow": "res://assets/shaders/particle_billow.gdshader",
	"fireball": "res://assets/shaders/particle_fireball.gdshader",
	"glow_billow": "res://assets/shaders/particle_glow_billow.gdshader",
	"streak": "res://assets/shaders/particle_streak.gdshader",
	"streak_add": "res://assets/shaders/particle_streak_add.gdshader",
	"cloud": "res://assets/shaders/particle_cloud.gdshader",
}
## Kinds whose `ParticleProcessMaterial` is mutated per burst (`effects.gd:_burst`
## sets `initial_velocity_min/max` for `reach > 0.0`). Their `ShaderMaterial`,
## `QuadMesh`, `CurveTexture` and `GradientTexture1D` are still shared.
const MUTABLE_PROCESS_KINDS: Array[String] = ["muzzle_tongue"]

static var _shaders: Dictionary[String, Shader] = {}
static var _materials: Dictionary = {}
static var _templates: Dictionary = {}
## Prompt 200: smoke, dust and haze as clouds (quality `fx_clouds`).
static var _clouds := true
## Soft-particle fade of clouds (m): wide, so a puff never shows a line
## where it meets the ground.
const CLOUD_SOFT_M := 2.0
## Trailer recordings (prompt 082): every particle system gets a fixed seed
## (its number), so a shot is recorded the same again. -1: not asked yet,
## 0: off (the game: random as ever).
static var _fixed_seeds := -1
static var _next_seed := 1


## Clouds or plain particles for the systems built from now on (prompt
## 200); the callers rebuild their emitters when the quality changes.
static func use_clouds(on: bool) -> void:
	_clouds = on


## A new particle system of `kind`. `factor` scales particle counts
## (quality), `soft` turns on soft particles, `wind` (x, z) drifts smoke.
## Prompt 208: shares immutable `ParticleProcessMaterial`, `QuadMesh`,
## `ShaderMaterial`, `CurveTexture` and `GradientTexture1D` across all
## emitters of the same `(kind, soft, clouds, wind)`.
static func build(kind: String, factor: float, soft: bool, wind: Vector2) -> GPUParticles3D:
	var p := GPUParticles3D.new()
	p.emitting = false
	var key := [kind, soft, _clouds, roundi(wind.x * 100.0), roundi(wind.y * 100.0)]
	var tpl: Dictionary
	if _templates.has(key):
		tpl = _templates[key]
	else:
		tpl = _build_template(kind, soft, wind)
		_templates[key] = tpl
	# Assign process_material and draw_pass_1 before configuring the other
	# GPUParticles3D properties; otherwise Godot 4.7 leaks ParticlesShaderRD
	# RIDs at exit when process_material is shared from a cached template.
	var pm: ParticleProcessMaterial = tpl.process_material
	p.process_material = pm.duplicate(false) if kind in MUTABLE_PROCESS_KINDS else pm
	if tpl.draw_pass_1 != null:
		p.draw_pass_1 = tpl.draw_pass_1
	p.name = kind
	p.local_coords = false
	p.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	p.one_shot = kind in BURSTS
	p.fixed_fps = 0
	p.interpolate = false
	if _fixed_seeds < 0:
		_fixed_seeds = 0
		for arg in OS.get_cmdline_user_args():
			if arg.begins_with("--trailer="):
				_fixed_seeds = 1
	if _fixed_seeds == 1:
		p.use_fixed_seed = true
		p.seed = _next_seed
		_next_seed += 1
	p.lifetime = tpl.lifetime
	p.explosiveness = tpl.explosiveness
	p.visibility_aabb = tpl.visibility_aabb
	if tpl.sorting_offset != 0.0:
		p.sorting_offset = tpl.sorting_offset
	p.amount = maxi(1, int(round(int(tpl.base_amount) * factor)))
	return p


static func _build_template(kind: String, soft: bool, wind: Vector2) -> Dictionary:
	var p := {
		"lifetime": 1.0,
		"explosiveness": 0.9 if kind in BURSTS else 0.0,
		"visibility_aabb": AABB(Vector3(-25, -5, -25), Vector3(50, 40, 50)),
		"sorting_offset": 0.0,
	}
	var pm := ParticleProcessMaterial.new()
	pm.angle_min = 0.0
	pm.angle_max = 360.0
	var drift := Vector3(wind.x, 0.0, wind.y)
	var amount := 8
	var size := 1.0
	# Non-square quads (streaks); else size x size.
	var quad_size := Vector2.ZERO
	var material: ShaderMaterial
	var fire_glow := 0.0
	match kind:
		"flash":
			amount = 5
			p.lifetime = 0.08
			p.explosiveness = 1.0
			size = 1.5
			_cone(pm, Vector3(0, 0, -1), 6.0, 3.0, 9.0)
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 1.0), Vector2(1, 0.4)])
			pm.color_ramp = _ramp([Color(1, 0.95, 0.75, 1), Color(1, 0.6, 0.2, 0.9), Color(1, 0.4, 0.1, 0)], [0.0, 0.5, 1.0])
			material = _material("add", EffectTextures.flash(), 3.0, false)
		"powder":
			# Powder smoke of a gun (prompt 077): pale grey, thrown out
			# along the barrel (a long cloud of 1-7 m at scale 1), then
			# hanging and drifting with the wind; thin enough to see units
			# through (opacity at most 0.38). Gone by ~3 s.
			amount = 10
			p.lifetime = 3.0
			p.explosiveness = 0.95
			pm.lifetime_randomness = 0.3
			size = 2.2
			_cone(pm, Vector3(0, 0, -1), 10.0, 5.0, 24.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.3
			pm.damping_min = 30.0
			pm.damping_max = 45.0
			pm.gravity = Vector3(0, 0.3, 0) + drift * 0.6
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.35), Vector2(0.15, 1.1), Vector2(1, 2.4)])
			pm.angular_velocity_min = -20.0
			pm.angular_velocity_max = 20.0
			pm.color_ramp = _ramp([Color(0.95, 0.94, 0.92, 0), Color(0.95, 0.94, 0.92, 0.38), Color(0.9, 0.89, 0.87, 0.22),
				Color(0.88, 0.87, 0.86, 0)], [0.0, 0.08, 0.45, 1.0])
			# A little emission: pale against the light too (judged).
			material = _material("lit", EffectTextures.smoke(), 0.25, soft)
		"blast_dust":
			amount = 16
			p.lifetime = 2.6
			size = 1.8
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_RING
			pm.emission_ring_axis = Vector3.UP
			pm.emission_ring_radius = 0.8
			pm.emission_ring_inner_radius = 0.2
			pm.emission_ring_height = 0.1
			pm.direction = Vector3.UP
			pm.spread = 10.0
			pm.initial_velocity_min = 0.3
			pm.initial_velocity_max = 1.2
			pm.radial_velocity_min = 3.0
			pm.radial_velocity_max = 8.0
			pm.damping_min = 3.0
			pm.damping_max = 5.0
			pm.gravity = Vector3(0, 0.2, 0) + drift * 0.3
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 2.4)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0), Color(1, 1, 1, 0.5), Color(1, 1, 1, 0)], [0.0, 0.08, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		"fountain":
			amount = 44
			p.lifetime = 1.8
			size = 0.6
			_cone(pm, Vector3.UP, 16.0, 7.0, 19.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.4
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_min = 0.4
			pm.scale_max = 1.3
			pm.angular_velocity_min = -300.0
			pm.angular_velocity_max = 300.0
			pm.color_ramp = _ramp([Color(0.75, 0.75, 0.75, 1), Color(0.75, 0.75, 0.75, 1), Color(0.75, 0.75, 0.75, 0)], [0.0, 0.8, 1.0])
			material = _material("lit", EffectTextures.clod(), 0.0, false)
		"spray":
			amount = 22
			p.lifetime = 1.8
			size = 1.0
			_cone(pm, Vector3.UP, 9.0, 7.0, 15.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.3
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 1.8)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0.9), Color(1, 1, 1, 0.6), Color(1, 1, 1, 0)], [0.0, 0.6, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		"water_column":
			# A shell in water (from 37 mm): a white column shoots up
			# (~12-18 m at scale 1, the speeds spread it along its height),
			# hangs and falls back under gravity; gone by ~3.5 s. Judged
			# after film of shell splashes, no measured heights.
			amount = 46
			p.lifetime = 3.4
			p.explosiveness = 1.0
			pm.lifetime_randomness = 0.15
			size = 1.5
			_cone(pm, Vector3.UP, 6.0, 9.0, 22.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.9
			pm.gravity = Vector3(0, -9.8, 0) + drift * 0.15
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.6), Vector2(0.3, 1.2), Vector2(1, 2.2)])
			pm.angular_velocity_min = -30.0
			pm.angular_velocity_max = 30.0
			pm.color_ramp = _ramp([Color(1, 1, 1, 0.95), Color(0.96, 0.98, 1, 0.85), Color(0.94, 0.96, 1, 0.5), Color(0.94, 0.96, 1, 0)],
				[0.0, 0.4, 0.75, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.25, soft)
		"water_jets":
			# The spiky outline of an underwater blast (after the user's
			# reference video of a sea mine): long jets of water fanning up
			# and out, white at first, greyer as they hang and fall (the
			# mud they carry; a second layer is tinted dark).
			amount = 34
			p.lifetime = 2.8
			p.explosiveness = 1.0
			pm.lifetime_randomness = 0.2
			size = 1.0
			_cone(pm, Vector3.UP, 16.0, 11.0, 24.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.6
			pm.particle_flag_align_y = true
			pm.gravity = Vector3(0, -9.8, 0)
			pm.damping_min = 0.5
			pm.damping_max = 1.5
			pm.scale_min = 0.7
			pm.scale_max = 1.4
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(0.25, 1.2), Vector2(1, 1.8)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0.95), Color(0.86, 0.88, 0.9, 0.85), Color(0.66, 0.68, 0.7, 0.5), Color(0.6, 0.62, 0.64, 0)],
				[0.0, 0.35, 0.75, 1.0])
			quad_size = Vector2(1.8, 5.5)
			material = _material("streak", EffectTextures.streak(), 0.0, false)
		"water_drops":
			# Drops and strands thrown wider than the column, falling in
			# arcs around it.
			amount = 90
			p.lifetime = 2.8
			p.explosiveness = 1.0
			pm.lifetime_randomness = 0.25
			size = 0.18
			_cone(pm, Vector3.UP, 26.0, 6.0, 16.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.5
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_min = 0.5
			pm.scale_max = 1.5
			pm.color_ramp = _ramp([Color(1, 1, 1, 1), Color(0.95, 0.97, 1, 0.9), Color(0.95, 0.97, 1, 0)], [0.0, 0.8, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.3, false)
		"water_ring":
			# The base surge: a ring of white water and mist rolls out over
			# the surface where the column falls back.
			amount = 22
			p.lifetime = 3.6
			p.explosiveness = 1.0
			size = 2.4
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_RING
			pm.emission_ring_axis = Vector3.UP
			pm.emission_ring_radius = 1.2
			pm.emission_ring_inner_radius = 0.4
			pm.emission_ring_height = 0.1
			pm.direction = Vector3.UP
			pm.spread = 10.0
			pm.initial_velocity_min = 0.5
			pm.initial_velocity_max = 1.5
			pm.radial_velocity_min = 3.0
			pm.radial_velocity_max = 7.0
			pm.damping_min = 1.5
			pm.damping_max = 2.5
			pm.gravity = Vector3(0, 0.1, 0) + drift * 0.3
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 2.2)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0), Color(1, 1, 1, 0.75), Color(0.95, 0.97, 1, 0.35), Color(1, 1, 1, 0)],
				[0.0, 0.08, 0.5, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.2, soft)
		"impact_dust":
			amount = 18
			p.lifetime = 4.0
			size = 2.0
			_cone(pm, Vector3.UP, 18.0, 2.0, 11.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.6
			pm.damping_min = 2.5
			pm.damping_max = 3.5
			pm.gravity = Vector3(0, 0.25, 0) + drift * 0.4
			pm.scale_curve = _curve([Vector2(0, 0.6), Vector2(1, 2.8)])
			pm.angular_velocity_min = -15.0
			pm.angular_velocity_max = 15.0
			pm.color_ramp = _ramp([Color(0.8, 0.8, 0.8, 0), Color(0.8, 0.8, 0.8, 0.95), Color(1, 1, 1, 0.45), Color(1, 1, 1, 0)], [0.0, 0.04, 0.45, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		"puff":
			amount = 4
			p.lifetime = 1.0
			size = 0.7
			_cone(pm, Vector3.UP, 50.0, 0.5, 1.6)
			pm.damping_min = 1.0
			pm.damping_max = 2.0
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 1.6)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0.7), Color(1, 1, 1, 0)], [0.0, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, false)
		"sparks":
			amount = 24
			p.lifetime = 0.5
			p.explosiveness = 1.0
			size = 0.3
			_cone(pm, Vector3.UP, 75.0, 5.0, 18.0)
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_min = 0.5
			pm.scale_max = 1.2
			pm.color_ramp = _ramp([Color(1, 0.95, 0.8, 1), Color(1, 0.6, 0.15, 1), Color(0.8, 0.2, 0.05, 0)], [0.0, 0.4, 1.0])
			material = _material("add", EffectTextures.spark(), 6.0, false)
		# Sizes and times below (prompt 072) are measured in the reference
		# video (docs/ARCHITECTURE.md, "Explosions after a video reference"),
		# for scale 1: a 7.5 cm shell, or a vehicle of 7 m.
		"core":
			# The first frame: a white-yellow core, ~3.5 m, gone after 1-2 frames.
			amount = 3
			p.lifetime = 0.15
			p.explosiveness = 1.0
			size = 3.5
			_cone(pm, Vector3.UP, 30.0, 0.0, 1.0)
			pm.scale_curve = _curve([Vector2(0, 0.6), Vector2(0.3, 1.0), Vector2(1, 0.7)])
			pm.color_ramp = _ramp([Color(1, 0.97, 0.85, 1), Color(1, 0.75, 0.35, 0.7), Color(1, 0.5, 0.15, 0)], [0.0, 0.35, 1.0])
			material = _material("add", EffectTextures.spark(), 5.0, false)
		"rays":
			# Vehicle explosion, 0-0.3 s: a pale ray burst ~11 m across that
			# fades as the fireball grows behind it.
			amount = 3
			p.lifetime = 0.4
			p.explosiveness = 1.0
			size = 10.0
			_cone(pm, Vector3.UP, 20.0, 0.0, 0.5)
			pm.scale_min = 0.8
			pm.scale_max = 1.1
			pm.scale_curve = _curve([Vector2(0, 0.35), Vector2(0.35, 1.0), Vector2(1, 1.1)])
			pm.color_ramp = _ramp([Color(1, 0.95, 0.8, 0.95), Color(1, 0.88, 0.65, 0.6), Color(1, 0.8, 0.55, 0)], [0.0, 0.45, 1.0])
			material = _material("unshaded", EffectTextures.rays(), 1.5, false)
		"streaks":
			# Ground hits: dark radial streaks of earth, a fan ~5.5 m wide and
			# ~7 m high at 0.3-0.4 s (11 m and 12-16 m for the 15 cm class of
			# the video), lumpy columns by 0.6 s, down by 1.6 s.
			amount = 30
			p.lifetime = 1.3
			p.explosiveness = 1.0
			size = 1.0
			_cone(pm, Vector3.UP, 40.0, 7.0, 14.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.3
			pm.particle_flag_align_y = true
			pm.gravity = Vector3(0, -6.0, 0)
			pm.damping_min = 2.0
			pm.damping_max = 4.0
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.6), Vector2(0.2, 1.2), Vector2(1, 1.6)])
			pm.color_ramp = _ramp([Color(0.5, 0.45, 0.4, 1), Color(0.55, 0.5, 0.45, 0.9), Color(0.6, 0.56, 0.52, 0)], [0.0, 0.45, 1.0])
			quad_size = Vector2(1.5, 3.2)
			material = _material("streak", EffectTextures.streak(), 0.0, false)
		"earth_cloud":
			# Ground hits: the earth fan turns into a cloud of brown lumps
			# (~6 m at 1 s), rises, turns grey-brown by 3 s and thins out;
			# dense to ~3 s, gone by ~8 s.
			amount = 26
			p.lifetime = 8.0
			p.explosiveness = 0.95
			size = 3.2
			_cone(pm, Vector3.UP, 35.0, 3.0, 10.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.9
			pm.damping_min = 2.0
			pm.damping_max = 3.5
			pm.gravity = Vector3(0, 0.35, 0) + drift * 0.5
			pm.scale_min = 0.6
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.45), Vector2(0.12, 1.3), Vector2(1, 2.6)])
			pm.angular_velocity_min = -10.0
			pm.angular_velocity_max = 10.0
			pm.lifetime_randomness = 0.35
			_atlas(pm)
			_turbulence(pm, 0.6)
			pm.color_ramp = _ramp([
				Color(0.7, 0.66, 0.62, 0), Color(0.75, 0.7, 0.66, 0.95), Color(1.4, 1.38, 1.36, 0.8),
				Color(1.75, 1.75, 1.75, 0.35), Color(1.8, 1.8, 1.8, 0)], [0.0, 0.03, 0.35, 0.65, 1.0], true)
			material = _material("billow", EffectTextures.billow(), 0.0, soft)
		"fireball":
			# Vehicle explosion: a cream-white ball of lumps that grows to
			# ~10 m (1.4 vehicle lengths) and ~12 m high by 1.0 s, turns
			# mottled orange at 1.5 s, dark red-brown with glowing spots at
			# 1.8 s and black at 2.2 s, where the smoke takes over.
			amount = 10
			p.lifetime = 2.4
			p.explosiveness = 0.92
			size = 5.0
			_cone(pm, Vector3.UP, 75.0, 3.0, 9.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 1.0
			pm.damping_min = 3.5
			pm.damping_max = 5.5
			pm.gravity = Vector3(0, 1.6, 0)
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.3), Vector2(0.18, 1.1), Vector2(0.45, 1.35), Vector2(1, 1.55)])
			pm.angular_velocity_min = -25.0
			pm.angular_velocity_max = 25.0
			_atlas(pm)
			pm.color_ramp = _ramp([
				Color(1.0, 0.95, 0.76, 1), Color(1.0, 0.74, 0.30, 1), Color(0.96, 0.42, 0.10, 1),
				Color(0.45, 0.14, 0.05, 0.97), Color(0.05, 0.035, 0.025, 0.9), Color(0.03, 0.025, 0.02, 0)],
				[0.0, 0.38, 0.58, 0.74, 0.9, 1.0])
			material = _material("fireball", EffectTextures.billow(), 0.85, soft)
		"embers":
			# Glowing spots in the darkening fireball and young smoke
			# (1.3-3 s), carried up with it.
			amount = 16
			p.lifetime = 1.8
			p.explosiveness = 0.6
			pm.lifetime_randomness = 0.4
			size = 0.45
			_cone(pm, Vector3.UP, 60.0, 0.8, 3.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 2.6
			pm.damping_min = 0.8
			pm.damping_max = 1.6
			pm.gravity = Vector3(0, 1.2, 0)
			pm.scale_min = 0.5
			pm.scale_max = 1.5
			pm.color_ramp = _ramp([Color(1, 0.75, 0.35, 0), Color(1, 0.6, 0.2, 1), Color(0.85, 0.25, 0.05, 0.8), Color(0.5, 0.1, 0.02, 0)], [0.0, 0.1, 0.6, 1.0])
			material = _material("add", EffectTextures.spark(), 3.0, false)
		"explosion_smoke":
			# Vehicle & heavy impact upper plume (prompts 072, 200, 220):
			# cohesive, velvety deep-black oil-smoke mushroom rising cleanly
			# above the bright warm-lit lower collar. [provisional]
			amount = 13
			p.lifetime = 8.5
			p.explosiveness = 0.82
			pm.lifetime_randomness = 0.25
			size = 4.4
			_cone(pm, Vector3.UP, 14.0, 1.8, 4.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 1.1
			pm.damping_min = 1.2
			pm.damping_max = 1.8
			pm.gravity = Vector3(0, 0.24, 0) + drift * 0.45
			pm.scale_min = 0.8
			pm.scale_max = 1.25
			pm.scale_curve = _curve([Vector2(0, 0.7), Vector2(0.16, 1.28), Vector2(1, 2.15)])
			pm.angular_velocity_min = -8.0
			pm.angular_velocity_max = 8.0
			_atlas(pm)
			_turbulence(pm, 0.55)
			pm.color_ramp = _ramp([
				Color(0.008, 0.008, 0.007, 0), Color(0.010, 0.009, 0.009, 1.0), Color(0.018, 0.017, 0.016, 0.98),
				Color(0.048, 0.045, 0.043, 0.65), Color(0.10, 0.095, 0.092, 0)], [0.0, 0.04, 0.50, 0.78, 1.0])
			material = _material("billow", EffectTextures.billow(), 0.0, soft)
		"smoke_collar":
			# Lower bright-grey billowing steam/dust collar right at the
			# impact/fire core, warmly underlit by fire_glow (prompt 220). [provisional]
			amount = 8
			p.lifetime = 4.2
			p.explosiveness = 0.88
			pm.lifetime_randomness = 0.22
			size = 3.2
			_cone(pm, Vector3.UP, 36.0, 1.0, 2.5)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 1.1
			pm.damping_min = 1.5
			pm.damping_max = 2.2
			pm.gravity = Vector3(0, 0.14, 0) + drift * 0.35
			pm.scale_min = 0.75
			pm.scale_max = 1.2
			pm.scale_curve = _curve([Vector2(0, 0.55), Vector2(0.2, 1.2), Vector2(1, 1.65)])
			_atlas(pm)
			_turbulence(pm, 0.45)
			pm.color_ramp = _ramp([
				Color(0.86, 0.84, 0.80, 0), Color(0.84, 0.82, 0.78, 0.96),
				Color(0.64, 0.63, 0.61, 0.72), Color(0.35, 0.34, 0.33, 0)], [0.0, 0.06, 0.50, 1.0])
			fire_glow = 1.05
			material = _material("billow", EffectTextures.billow(), 0.0, soft)
		"ground_haze":
			# Broad, flat, low-lying (1-3 m) semi-transparent beige-grey dust
			# and smoke veil drifting across road and grass after heavy hits
			# (prompt 220). [provisional]
			amount = 6
			p.lifetime = 5.5
			p.explosiveness = 0.9
			pm.lifetime_randomness = 0.22
			size = 5.2
			p.visibility_aabb = AABB(Vector3(-35, -3, -35), Vector3(70, 16, 70))
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_RING
			pm.emission_ring_axis = Vector3.UP
			pm.emission_ring_radius = 2.6
			pm.emission_ring_inner_radius = 0.6
			pm.emission_ring_height = 0.25
			pm.direction = Vector3.UP
			pm.spread = 12.0
			pm.initial_velocity_min = 0.1
			pm.initial_velocity_max = 0.3
			pm.radial_velocity_min = 3.5
			pm.radial_velocity_max = 7.2
			pm.damping_min = 3.2
			pm.damping_max = 4.6
			pm.gravity = Vector3(0, 0.03, 0) + drift * 0.45
			pm.scale_min = 0.8
			pm.scale_max = 1.2
			pm.scale_curve = _curve([Vector2(0, 0.6), Vector2(0.22, 1.3), Vector2(1, 1.75)])
			pm.color_ramp = _ramp([
				Color(0.96, 0.94, 0.89, 0), Color(0.95, 0.93, 0.88, 0.48),
				Color(0.91, 0.89, 0.85, 0.30), Color(0.88, 0.86, 0.82, 0)], [0.0, 0.08, 0.54, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.06, soft)
		"dust_cloud":
			# Big blasts: a grey-beige dome of dust that is ~16 m across
			# within 0.5 s (15 cm) and keeps rolling; the tint is the ground
			# colour mixed with grey. Also collapsing buildings.
			amount = 24
			p.lifetime = 8.0
			p.explosiveness = 0.95
			pm.lifetime_randomness = 0.3
			size = 5.0
			_cone(pm, Vector3.UP, 80.0, 10.0, 20.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 1.5
			pm.damping_min = 14.0
			pm.damping_max = 20.0
			pm.gravity = Vector3(0, 0.4, 0) + drift * 0.5
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(0.08, 1.3), Vector2(1, 2.4)])
			pm.angular_velocity_min = -10.0
			pm.angular_velocity_max = 10.0
			_atlas(pm)
			_turbulence(pm, 0.6)
			pm.color_ramp = _ramp([
				Color(1.2, 1.2, 1.2, 0), Color(1.2, 1.2, 1.2, 0.95), Color(1.15, 1.15, 1.15, 0.65),
				Color(1.15, 1.15, 1.15, 0.25), Color(1.15, 1.15, 1.15, 0)], [0.0, 0.02, 0.3, 0.7, 1.0], true)
			material = _material("billow", EffectTextures.billow(), 0.0, soft)
		"leaves":
			# Leaves, needles or frond bits falling from a tree stripped by a
			# big blast (prompt 085): small lit flakes thrown out of the crown,
			# then fluttering down slowly; tinted with the foliage color.
			amount = 40
			p.lifetime = 4.0
			p.explosiveness = 0.9
			pm.lifetime_randomness = 0.3
			size = 0.22
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(3.0, 2.5, 3.0)
			_cone(pm, Vector3.UP, 80.0, 1.0, 5.0)
			pm.gravity = Vector3(0, -2.0, 0) + drift * 0.5
			pm.damping_min = 1.5
			pm.damping_max = 3.0
			pm.scale_min = 0.5
			pm.scale_max = 1.3
			pm.angular_velocity_min = -360.0
			pm.angular_velocity_max = 360.0
			_turbulence(pm, 0.8)
			pm.color_ramp = _ramp([Color(1, 1, 1, 1), Color(1, 1, 1, 1), Color(1, 1, 1, 0)], [0.0, 0.8, 1.0])
			material = _material("lit", EffectTextures.clod(), 0.0, false)
		"debris":
			# Dark tumbling fragments (sprites: a mesh with a StandardMaterial3D
			# leaked its shader at exit).
			amount = 14
			p.lifetime = 2.0
			p.explosiveness = 1.0
			size = 0.35
			_cone(pm, Vector3.UP, 55.0, 5.0, 14.0)
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_min = 0.5
			pm.scale_max = 1.4
			pm.angular_velocity_min = -500.0
			pm.angular_velocity_max = 500.0
			pm.color_ramp = _ramp([Color(0.12, 0.11, 0.1, 1), Color(0.12, 0.11, 0.1, 1), Color(0.12, 0.11, 0.1, 0)], [0.0, 0.85, 1.0])
			material = _material("lit", EffectTextures.clod(), 0.0, false)
		"fire":
			amount = 26
			p.lifetime = 0.9
			size = 1.3
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(0.6, 0.15, 0.6)
			_cone(pm, Vector3.UP, 12.0, 1.2, 2.6)
			pm.gravity = Vector3(0, 1.8, 0) + drift * 0.2
			pm.scale_min = 0.6
			pm.scale_max = 1.2
			pm.scale_curve = _curve([Vector2(0, 0.7), Vector2(0.3, 1.0), Vector2(1, 0.2)])
			pm.angular_velocity_min = -90.0
			pm.angular_velocity_max = 90.0
			pm.color_ramp = _ramp([Color(1, 0.85, 0.45, 0.9), Color(1, 0.5, 0.12, 0.85), Color(0.6, 0.15, 0.04, 0)], [0.0, 0.45, 1.0])
			material = _material("unshaded", EffectTextures.fire(), 1.2, soft)
		"smoke_column":
			# Upper velvety deep-black oil-smoke plume of burning wrecks and
			# fires (prompts 072, 200, 220). [provisional]
			amount = 48
			p.lifetime = 12.0
			size = 3.9
			p.visibility_aabb = AABB(Vector3(-60, -5, -60), Vector3(120, 90, 120))
			_cone(pm, Vector3.UP, 9.0, 2.8, 4.3)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.85
			pm.damping_min = 0.38
			pm.damping_max = 0.56
			pm.gravity = Vector3(0, 0.16, 0) + drift * 0.28
			pm.scale_curve = _curve([Vector2(0, 0.65), Vector2(0.32, 1.85), Vector2(1, 3.0)])
			pm.angular_velocity_min = -8.0
			pm.angular_velocity_max = 8.0
			_atlas(pm)
			pm.color_ramp = _ramp([
				Color(0.008, 0.007, 0.007, 0), Color(0.010, 0.009, 0.009, 1.0), Color(0.020, 0.019, 0.018, 0.98),
				Color(0.055, 0.052, 0.050, 0.58), Color(0.11, 0.105, 0.10, 0)], [0.0, 0.04, 0.52, 0.80, 1.0])
			material = _material("billow", EffectTextures.billow(), 0.0, soft)
		"wreck_collar":
			# Continuous bright-grey billowing lower collar right above a
			# burning wreck's flames, warmly underlit by fire_glow (prompt 220).
			# Transitions into the deep-black smoke_column above. [provisional]
			amount = 16
			p.lifetime = 3.4
			size = 2.4
			p.visibility_aabb = AABB(Vector3(-25, -3, -25), Vector3(50, 30, 50))
			_cone(pm, Vector3.UP, 24.0, 1.3, 2.3)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.8
			pm.damping_min = 0.7
			pm.damping_max = 1.05
			pm.gravity = Vector3(0, 0.12, 0) + drift * 0.25
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(0.35, 1.2), Vector2(1, 1.65)])
			_atlas(pm)
			pm.color_ramp = _ramp([
				Color(0.82, 0.80, 0.77, 0), Color(0.78, 0.77, 0.74, 0.94),
				Color(0.52, 0.51, 0.49, 0.60), Color(0.18, 0.17, 0.16, 0)], [0.0, 0.12, 0.62, 1.0])
			fire_glow = 0.9
			material = _material("billow", EffectTextures.billow(), 0.0, soft)
		"dust_trail":
			amount = 48
			p.lifetime = 3.2
			size = 2.2
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(1.4, 0.15, 0.5)
			_cone(pm, Vector3.UP, 45.0, 0.6, 2.2)
			pm.damping_min = 0.8
			pm.damping_max = 1.4
			pm.gravity = Vector3(0, 0.15, 0) + drift * 0.4
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 3.0)])
			pm.angular_velocity_min = -15.0
			pm.angular_velocity_max = 15.0
			pm.color_ramp = _ramp([Color(1, 1, 1, 0), Color(1, 1, 1, 0.55), Color(1, 1, 1, 0)], [0.0, 0.12, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		# Muzzle blast of a gun (prompt 077; effects.gd `muzzle_flash`,
		# sizes for scale 1 = 7.5 cm, docs/ARCHITECTURE.md "Muzzle blast").
		"muzzle_core":
			# The gleaming ball at the muzzle: overexposed white-yellow,
			# gone within 2-4 frames at 60 fps.
			amount = 2
			p.lifetime = 0.07
			p.explosiveness = 1.0
			size = 2.4
			_cone(pm, Vector3(0, 0, -1), 10.0, 0.0, 6.0)
			pm.scale_curve = _curve([Vector2(0, 1.0), Vector2(0.5, 0.85), Vector2(1, 0.3)])
			pm.color_ramp = _ramp([Color(1, 0.97, 0.86, 1), Color(1, 0.8, 0.45, 0.85), Color(1, 0.55, 0.2, 0)], [0.0, 0.5, 1.0])
			material = _material("add", EffectTextures.spark(), 7.0, false)
			p.sorting_offset = 20.0
		"muzzle_tongue":
			# The fire tongue: flame quads that fly out along the barrel and
			# stretch along their flight (`streak_add`), white-yellow at the
			# root to orange at the tip; ~3 m long at scale 1 (40 calibers,
			# beyond the intermediate flash at 20-25 calibers, docs/SOURCES.md).
			# effects.gd sets the speeds per shot (`TONGUE_SPEED`).
			amount = 5
			p.lifetime = 0.1
			p.explosiveness = 1.0
			size = 1.0
			_cone(pm, Vector3(0, 0, -1), 2.5, 10.0, 38.0)
			pm.particle_flag_align_y = true
			pm.scale_min = 0.8
			pm.scale_max = 1.2
			pm.scale_curve = _curve([Vector2(0, 0.7), Vector2(0.4, 1.0), Vector2(1, 0.5)])
			pm.color_ramp = _ramp([Color(1, 0.82, 0.45, 1), Color(1, 0.52, 0.14, 0.95), Color(1, 0.35, 0.08, 0)], [0.0, 0.5, 1.0])
			quad_size = Vector2(0.9, 2.2)
			# Drawn over the gas cloud (sorted as if closer).
			p.sorting_offset = 20.0
			material = _material("streak_add", EffectTextures.tongue(), 5.0, false)
		"muzzle_gas":
			# The glowing gas thrown forward: billows that are white-hot from
			# inside at first, swell, and turn into pale smoke within ~0.55 s,
			# when the powder smoke takes over. A cloud ~9 m long and ~3 m
			# thick at scale 1 (the reference: 1.5-2 vehicle lengths long,
			# one vehicle width thick at the end, for a heavy gun).
			amount = 9
			p.lifetime = 0.55
			p.explosiveness = 1.0
			pm.lifetime_randomness = 0.2
			size = 2.7
			_cone(pm, Vector3(0, 0, -1), 8.0, 16.0, 44.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.2
			pm.damping_min = 90.0
			pm.damping_max = 110.0
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.25), Vector2(0.3, 1.1), Vector2(1, 1.7)])
			pm.angular_velocity_min = -40.0
			pm.angular_velocity_max = 40.0
			_atlas(pm)
			pm.color_ramp = _ramp([
				Color(2.4, 2.25, 1.9, 0.9), Color(1.6, 1.58, 1.52, 0.8), Color(0.95, 0.94, 0.92, 0.4),
				Color(0.95, 0.94, 0.92, 0)], [0.0, 0.2, 0.55, 1.0], true)
			material = _material("glow_billow", EffectTextures.billow(), 0.2, soft)
		"muzzle_dust":
			# Dust blown off the ground under and around the muzzle, fanned
			# in the firing direction, tinted with the ground colour; flat
			# and brown on grass, much on sand and tracks, powder on snow,
			# mist on water. Settles within ~2.6 s.
			amount = 12
			p.lifetime = 2.6
			p.explosiveness = 0.95
			pm.lifetime_randomness = 0.3
			size = 2.6
			_cone(pm, Vector3(0, 0.12, -1), 70.0, 3.0, 11.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(1.2, 0.1, 1.2)
			pm.damping_min = 5.0
			pm.damping_max = 8.0
			pm.gravity = Vector3(0, 0.25, 0) + drift * 0.5
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.45), Vector2(0.15, 1.0), Vector2(1, 2.0)])
			pm.angular_velocity_min = -12.0
			pm.angular_velocity_max = 12.0
			pm.color_ramp = _ramp([
				Color(1.15, 1.15, 1.15, 0), Color(1.15, 1.15, 1.15, 0.65), Color(1.2, 1.2, 1.2, 0.3),
				Color(1.2, 1.2, 1.2, 0)], [0.0, 0.05, 0.4, 1.0], true)
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		"muzzle_puffs":
			# Compact chain of 4-6 soft, round light-grey smoke puffs streaming
			# ahead of a cannon muzzle and drifting off in the wind (prompt 220;
			# off on low via fx_muzzle_detail). Kept out of CLOUDS so the
			# individual puffs read as a distinct chain. [provisional]
			amount = 6
			p.lifetime = 2.6
			p.explosiveness = 0.86
			pm.lifetime_randomness = 0.14
			size = 1.35
			_cone(pm, Vector3(0, 0.08, -1), 10.0, 7.5, 17.5)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.18
			pm.damping_min = 8.5
			pm.damping_max = 13.5
			pm.gravity = Vector3(0, 0.32, 0) + drift * 0.75
			pm.scale_min = 0.85
			pm.scale_max = 1.2
			pm.scale_curve = _curve([Vector2(0, 0.55), Vector2(0.18, 1.18), Vector2(1, 1.7)])
			pm.angular_velocity_min = -18.0
			pm.angular_velocity_max = 18.0
			pm.color_ramp = _ramp([
				Color(0.88, 0.89, 0.87, 0), Color(0.88, 0.89, 0.87, 0.72),
				Color(0.83, 0.84, 0.82, 0.42), Color(0.78, 0.79, 0.78, 0)], [0.0, 0.05, 0.52, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.08, soft)
		# Rocket salvo (prompt 076, rocket_effects.gd): the motor's back
		# blast out of the rail's rear end, light smoke thrown back and down
		# hard and then hanging (judged from the reference pictures: about
		# as big as the launcher after a second).
		"rocket_blast":
			amount = 16
			p.lifetime = 3.2
			size = 2.4
			_cone(pm, Vector3(0, 0, -1), 28.0, 6.0, 18.0)
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
			pm.emission_sphere_radius = 0.5
			pm.damping_min = 5.0
			pm.damping_max = 8.0
			pm.gravity = Vector3(0, 0.35, 0) + drift * 0.5
			pm.scale_min = 0.7
			pm.scale_max = 1.3
			pm.scale_curve = _curve([Vector2(0, 0.4), Vector2(0.2, 1.4), Vector2(1, 2.8)])
			pm.angular_velocity_min = -20.0
			pm.angular_velocity_max = 20.0
			pm.color_ramp = _ramp([Color(0.95, 0.93, 0.9, 0), Color(0.95, 0.93, 0.9, 0.75), Color(0.9, 0.89, 0.87, 0.4), Color(0.9, 0.89, 0.87, 0)], [0.0, 0.05, 0.45, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.0, soft)
		# The haze that stands over a firing rocket position and drifts off
		# with the wind (prompt 076; the reference's last picture).
		"salvo_smoke":
			amount = 36
			p.lifetime = 11.0
			size = 4.5
			p.visibility_aabb = AABB(Vector3(-60, -5, -60), Vector3(120, 50, 120))
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(5.0, 0.8, 5.0)
			_cone(pm, Vector3.UP, 40.0, 0.3, 1.2)
			pm.damping_min = 0.2
			pm.damping_max = 0.4
			pm.gravity = Vector3(0, 0.12, 0) + drift * 0.6
			pm.scale_curve = _curve([Vector2(0, 0.6), Vector2(1, 2.6)])
			pm.angular_velocity_min = -6.0
			pm.angular_velocity_max = 6.0
			pm.color_ramp = _ramp([Color(0.9, 0.89, 0.87, 0), Color(0.9, 0.89, 0.87, 0.32), Color(0.88, 0.87, 0.86, 0)], [0.0, 0.15, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.0, soft)
		# Ground effects of driving (prompt 104; effects.gd `drive_effect`),
		# tinted by the ground like the dust trail.
		"mud_clods":
			# Short, heavy, dark clods thrown up behind tracks and wheels;
			# they fall back within a second, no hanging dust.
			amount = 30
			p.lifetime = 0.9
			size = 0.3
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(1.3, 0.1, 0.3)
			_cone(pm, Vector3(0.0, 0.8, 0.6).normalized(), 28.0, 2.0, 4.5)
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_min = 0.45
			pm.scale_max = 1.4
			pm.angular_velocity_min = -400.0
			pm.angular_velocity_max = 400.0
			pm.color_ramp = _ramp([Color(0.7, 0.7, 0.7, 1), Color(0.6, 0.6, 0.6, 1), Color(0.6, 0.6, 0.6, 0)], [0.0, 0.8, 1.0])
			material = _material("lit", EffectTextures.clod(), 0.0, false)
		"snow_dust":
			# Fine, bright powder snow: thrown up a little, settling soon.
			amount = 56
			p.lifetime = 1.8
			size = 1.3
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(1.4, 0.1, 0.4)
			_cone(pm, Vector3(0.0, 1.0, 0.5).normalized(), 40.0, 0.8, 2.6)
			pm.damping_min = 1.5
			pm.damping_max = 2.5
			pm.gravity = Vector3(0, -0.2, 0) + drift * 0.5
			pm.scale_curve = _curve([Vector2(0, 0.4), Vector2(1, 2.2)])
			pm.angular_velocity_min = -20.0
			pm.angular_velocity_max = 20.0
			pm.color_ramp = _ramp([Color(1, 1, 1, 0), Color(1, 1, 1, 0.7), Color(1, 1, 1, 0)], [0.0, 0.1, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.35, soft)
		"bow_spray":
			# Water thrown up and aside at the bow in a ford.
			amount = 36
			p.lifetime = 0.8
			size = 0.5
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(1.3, 0.05, 0.2)
			_cone(pm, Vector3(0.0, 1.0, -0.35).normalized(), 70.0, 2.5, 5.0)
			pm.gravity = Vector3(0, -9.8, 0)
			pm.scale_curve = _curve([Vector2(0, 0.6), Vector2(1, 1.7)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0.85), Color(1, 1, 1, 0.55), Color(1, 1, 1, 0)], [0.0, 0.6, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.2, soft)
		"waterfall_mist":
			# Spray at the foot of a waterfall (prompt 122, waterfalls.gd):
			# white mist rising out of the plunge pool, drifting with the
			# wind and fading.
			amount = 36
			p.lifetime = 3.2
			size = 3.2
			p.visibility_aabb = AABB(Vector3(-20, -2, -20), Vector3(40, 25, 40))
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(2.5, 0.3, 1.5)
			_cone(pm, Vector3.UP, 55.0, 0.8, 2.4)
			pm.gravity = Vector3(0, 0.25, 0) + drift * 0.25
			pm.damping_min = 0.3
			pm.damping_max = 0.6
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 2.6)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0), Color(1, 1, 1, 0.5), Color(1, 1, 1, 0)], [0.0, 0.15, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.35, soft)
		"wake_foam":
			# Foam on the water surface that spreads behind the vehicle
			# (the wake; particles stay where they were born).
			amount = 40
			p.lifetime = 4.0
			size = 1.6
			pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
			pm.emission_box_extents = Vector3(1.5, 0.02, 0.4)
			_cone(pm, Vector3(0, 0, 1), 85.0, 0.3, 0.9)
			pm.damping_min = 0.2
			pm.damping_max = 0.4
			pm.scale_curve = _curve([Vector2(0, 0.5), Vector2(1, 3.0)])
			pm.color_ramp = _ramp([Color(1, 1, 1, 0), Color(1, 1, 1, 0.5), Color(1, 1, 1, 0)], [0.0, 0.1, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.15, soft)
		"exhaust":
			amount = 16
			p.lifetime = 2.2
			size = 0.7
			_cone(pm, Vector3.UP, 20.0, 0.5, 1.2)
			pm.gravity = Vector3(0, 0.4, 0) + drift * 0.4
			pm.scale_curve = _curve([Vector2(0, 0.3), Vector2(1, 1.8)])
			pm.color_ramp = _ramp([Color(0.22, 0.21, 0.2, 0), Color(0.24, 0.23, 0.22, 0.5), Color(0.4, 0.39, 0.38, 0)], [0.0, 0.1, 1.0])
			material = _material("lit", EffectTextures.smoke(), 0.0, false)
		_:
			push_error("unknown effect kind '%s'" % kind)
	if _clouds and CLOUDS.has(kind):
		var look: Vector2 = CLOUDS[kind]
		amount = maxi(1, roundi(amount * look.x))
		size *= look.y
		pm.angular_velocity_min = 0.0
		pm.angular_velocity_max = 0.0
		pm.angle_min = 0.0
		pm.angle_max = 0.0
		_atlas(pm)
		var emission: float = material.get_shader_parameter("emission") if material != null else 0.0
		material = _material("cloud", EffectTextures.cloud(), emission, true, fire_glow)
	p.base_amount = amount
	p.process_material = pm
	p.draw_pass_1 = null
	if material != null:
		var quad := QuadMesh.new()
		quad.size = quad_size if quad_size != Vector2.ZERO else Vector2(size, size)
		quad.material = material
		p.draw_pass_1 = quad
	return p


## Emission along `direction` within `spread` degrees.
static func _cone(pm: ParticleProcessMaterial, direction: Vector3, spread: float, v_min: float, v_max: float) -> void:
	pm.direction = direction
	pm.spread = spread
	pm.initial_velocity_min = v_min
	pm.initial_velocity_max = v_max


## Picks one of the four atlas cells per particle (CUSTOM.z, see the
## "billow" shaders).
static func _atlas(pm: ParticleProcessMaterial) -> void:
	pm.anim_speed_min = 0.0
	pm.anim_speed_max = 0.0
	pm.anim_offset_min = 0.0
	pm.anim_offset_max = 1.0


## Slow rolling of smoke and dust (prompt 072).
static func _turbulence(pm: ParticleProcessMaterial, strength: float) -> void:
	pm.turbulence_enabled = true
	pm.turbulence_noise_strength = strength
	pm.turbulence_noise_scale = 6.0
	pm.turbulence_noise_speed_random = 0.3
	pm.turbulence_influence_min = 0.03
	pm.turbulence_influence_max = 0.08


static func _curve(points: Array[Vector2]) -> CurveTexture:
	var curve := Curve.new()
	var top := 1.0
	for point in points:
		top = maxf(top, point.y)
	curve.max_value = top
	for point in points:
		curve.add_point(point)
	var texture := CurveTexture.new()
	texture.curve = curve
	return texture


## A colour ramp; `hdr` keeps values above 1 (lighter than the tint).
static func _ramp(colors: Array[Color], offsets: Array[float], hdr := false) -> GradientTexture1D:
	var gradient := Gradient.new()
	gradient.offsets = PackedFloat32Array(offsets)
	gradient.colors = PackedColorArray(colors)
	var texture := GradientTexture1D.new()
	texture.use_hdr = hdr
	texture.gradient = gradient
	return texture


## mode: "lit" (alpha blend, sun light), "unshaded" (alpha blend, glowing),
## "add" (additive, glowing), "billow" (lit atlas billows, prompt 072),
## "fireball" (unshaded atlas billows that darken with age), "glow_billow"
## (unshaded atlas billows, glowing gas: prompt 077), "streak" (lit,
## the quad follows the velocity; needs `particle_flag_align_y`) or
## "streak_add" (additive and glowing like "add", following the velocity:
## the fire tongue, prompt 077) or "cloud" (lit soft puffs lit as one
## cloud, always soft: prompt 200).
static func _material(mode: String, texture: Texture2D, emission: float, soft: bool,
		fire_glow := 0.0) -> ShaderMaterial:
	var tex_id := texture.get_instance_id() if texture != null else 0
	var soft_dist := CLOUD_SOFT_M if mode == "cloud" else (0.8 if soft else 0.0)
	var key := [mode, tex_id, roundi(emission * 1000.0), roundi(soft_dist * 100.0), roundi(fire_glow * 1000.0)]
	if _materials.has(key):
		return _materials[key]
	if not _shaders.has(mode):
		_shaders[mode] = load(SHADER_PATHS[mode]) as Shader
	var material := ShaderMaterial.new()
	material.shader = _shaders[mode]
	material.set_shader_parameter("tex", texture)
	material.set_shader_parameter("emission", emission)
	if fire_glow > 0.0:
		material.set_shader_parameter("fire_glow", fire_glow)
	material.set_shader_parameter("soft_distance", soft_dist)
	_materials[key] = material
	return material


static func clear_cache() -> void:
	_shaders.clear()
	_materials.clear()
	_templates.clear()
