// 3D view of the unit browser: loads the game's .glb models and squad scenes and
// dresses them like the game does (port of godot/assets/shaders/unit_camo.gdshader
// with the palettes of godot/assets/materials/camo/). Soldiers and environment
// objects get flat slot colors only; markings (decals) are not shown.

import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import * as SkeletonUtils from 'three/addons/utils/SkeletonUtils.js';

export const BIOMES = ['forest', 'desert', 'winter'];
const FACTIONS = ['de', 'su', 'us', 'uk'];

// Same slot numbers as the `slot` uniform of unit_camo.gdshader.
const UNIT_SLOTS = { Paint: 0, Metal: 1, Rubber: 2, Wood: 3, Canvas: 4, Glass: 5, Skin: 6, Cloth: 7 };

// Approximate colors of the environment slots (the game uses procedural shaders).
const ENV_COLORS = {
  Stone: '#8d8a80', Wood: '#6e4a2c', Roof: '#96503a', Glass: '#1d2a2a', Metal: '#3b3c3d',
  Ground: '#5d4c36', Plaster: '#d6cdb6', Bark: '#4b3b2b', Paint: '#545951', Canvas: '#7e7657',
  Rubber: '#232323',
};
// EnvLook foliage colors (godot/assets/shaders/env_look.gd).
const FOLIAGE = { forest: [0.25, 0.36, 0.13], desert: [0.33, 0.36, 0.17], winter: [0.17, 0.25, 0.13] };
const NEUTRAL_PALETTE = {
  base: [0.33, 0.35, 0.32], color_1: [0.33, 0.35, 0.32], color_2: [0.33, 0.35, 0.32],
  pattern: 0, scale: 1.5, coverage: 0, winter_wash: 0, snow: 0, mud: 0.3,
  mud_color: [0.25, 0.2, 0.14], dust: 0, wear: 0.4,
};

/** Parses the `key = value` lines of a Godot .tres resource (numbers, strings, colors). */
function parseTres(text) {
  const out = {};
  for (const [, key, raw] of text.matchAll(/^(\w+) = (.+)$/gm)) {
    const color = raw.match(/^Color\(([^)]*)\)/);
    if (color) out[key] = color[1].split(',').map(Number);
    else if (raw.startsWith('"')) out[key] = raw.slice(1, -1);
    else if (!Number.isNaN(Number(raw))) out[key] = Number(raw);
  }
  return out;
}

// Palettes of German vehicles whose year_from is before EARLY_UNTIL
// (godot/assets/shaders/unit_camo.gd: Dunkelgrau RAL 7021 until 1943).
const EARLY_UNTIL = 1943;
const EARLY_PALETTES = ['de_forest_early', 'de_winter_early'];

/** The palette of a catalog entry in a biome, like UnitCamo.palette_for. */
function paletteFor(entry, biome, camo) {
  const early = entry.faction === 'de' && entry.year_from > 0 && entry.year_from < EARLY_UNTIL
    && !entry.category.startsWith('aircraft') && !entry.category.startsWith('fort');
  const key = `${entry.faction}_${biome}`;
  if (early && camo[`${key}_early`]) return camo[`${key}_early`];
  return camo[key] || NEUTRAL_PALETTE;
}

/** Loads camouflage palettes and infantry slot colors. `text(path)` fetches a repo file. */
export async function loadLook(text) {
  const camo = {};
  await Promise.all(FACTIONS.flatMap((f) => BIOMES.map(async (b) => {
    camo[`${f}_${b}`] = parseTres(await text(`godot/assets/materials/camo/${f}_${b}.tres`));
  })));
  // Early German grey (RAL 7021) before 1943, as UnitCamo.palette_for in the game.
  await Promise.all(EARLY_PALETTES.map(async (key) => {
    camo[key] = parseTres(await text(`godot/assets/materials/camo/${key}.tres`));
  }));
  const uniforms = JSON.parse(await text('godot/assets/models/infantry/uniform_palettes.json'));
  return { camo, uniforms };
}

const srgb = (c) => new THREE.Color().setRGB(c[0], c[1], c[2], THREE.SRGBColorSpace);

const GLSL_NOISE = /* glsl */ `
uniform int mc_slot;
uniform vec3 mc_base; uniform vec3 mc_color_1; uniform vec3 mc_color_2; uniform vec3 mc_mud_color;
uniform float mc_pattern; uniform float mc_scale; uniform float mc_coverage;
uniform float mc_winter_wash; uniform float mc_snow; uniform float mc_mud; uniform float mc_dust;
uniform float mc_wear;
varying vec3 mcLocal; varying vec3 mcWN; varying vec2 mcUv;
float mc_rough; float mc_metal;
float mc_hash(vec3 p) {
  p = fract(p * 0.3183099 + vec3(0.11, 0.17, 0.13));
  p *= 17.0;
  return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}
float mc_noise(vec3 x) {
  vec3 i = floor(x); vec3 f = fract(x);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(mix(mc_hash(i), mc_hash(i + vec3(1, 0, 0)), f.x),
                 mix(mc_hash(i + vec3(0, 1, 0)), mc_hash(i + vec3(1, 1, 0)), f.x), f.y),
             mix(mix(mc_hash(i + vec3(0, 0, 1)), mc_hash(i + vec3(1, 0, 1)), f.x),
                 mix(mc_hash(i + vec3(0, 1, 1)), mc_hash(i + vec3(1, 1, 1)), f.x), f.y), f.z);
}
float mc_fbm(vec3 p) {
  float v = 0.0; float a = 0.5;
  for (int i = 0; i < 4; i++) { v += a * mc_noise(p); p = p * 2.03 + vec3(1.7, 9.2, 3.1); a *= 0.5; }
  return v;
}
float mc_blotch(float n, float coverage, float edge) {
  float t = 1.0 - coverage;
  return smoothstep(t * 0.5 + 0.25 - edge, t * 0.5 + 0.25 + edge, n);
}
vec3 mc_camo(vec3 p) {
  vec3 q = p / max(mc_scale, 0.05);
  vec3 col = mc_base;
  if (mc_pattern < 0.5) return col;
  float edge = mc_pattern < 1.5 ? 0.06 : 0.01;
  float m1 = mc_blotch(mc_fbm(q * 0.6), mc_coverage, edge);
  float m2 = mc_blotch(mc_fbm(q * 0.6 + vec3(23.1, 7.7, 11.3)), mc_coverage * 0.6, edge) * (1.0 - m1);
  col = mix(col, mc_color_1, m1);
  col = mix(col, mc_color_2, m2);
  if (mc_pattern > 2.5) col = mix(col, mc_base, step(0.78, mc_noise(q * 9.0)) * max(m1, m2));
  return col;
}
`;

// Replaces three's <color_fragment>; mirrors fragment() of unit_camo.gdshader.
const GLSL_FRAGMENT = /* glsl */ `
#if defined( USE_COLOR_ALPHA )
  vec4 vc = vColor;
#elif defined( USE_COLOR )
  vec4 vc = vec4(vColor, 1.0);
#else
  vec4 vc = vec4(1.0);
#endif
float ao = vc.r; float edge = vc.g; float height = vc.b * 4.0; float is_track = 1.0 - vc.a;
vec3 p = mcLocal; vec3 wn = normalize(mcWN);
float grain = mc_noise(p * 6.0);
vec3 albedo; mc_rough = 0.8; mc_metal = 0.0;
if (mc_slot == 0) {
  albedo = mc_camo(p) * (0.94 + 0.12 * grain); mc_rough = 0.82;
} else if (mc_slot == 1) {
  albedo = vec3(0.27, 0.26, 0.25) * (0.85 + 0.3 * grain); mc_rough = 0.55; mc_metal = 0.55;
  if (is_track > 0.5) {
    float v = fract(mcUv.y);
    float grouser = smoothstep(0.08, 0.14, v) * (1.0 - smoothstep(0.86, 0.92, v));
    float guide = 1.0 - smoothstep(0.42, 0.46, abs(mcUv.x - 0.5) + 0.38);
    albedo = mix(vec3(0.10, 0.09, 0.085), vec3(0.30, 0.28, 0.26), grouser);
    albedo = mix(albedo, vec3(0.34, 0.32, 0.30), guide * 0.4);
    mc_rough = mix(0.9, 0.45, grouser);
  }
} else if (mc_slot == 2) {
  albedo = vec3(0.055, 0.055, 0.05); mc_rough = 0.92;
} else if (mc_slot == 3) {
  albedo = vec3(0.40, 0.29, 0.17) * (0.8 + 0.4 * mc_noise(p * vec3(2.0, 2.0, 30.0)));
} else if (mc_slot == 4) {
  albedo = mix(mc_base, vec3(0.45, 0.42, 0.32), 0.6) * (0.9 + 0.2 * grain); mc_rough = 0.95;
} else if (mc_slot == 5) {
  albedo = vec3(0.05, 0.07, 0.08); mc_rough = 0.08; mc_metal = 0.4;
} else if (mc_slot == 6) {
  albedo = vec3(0.72, 0.55, 0.44); mc_rough = 0.6;
} else {
  albedo = mc_color_1; mc_rough = 0.95;
}
bool painted = mc_slot == 0 || mc_slot == 4 || mc_slot == 7;
if (painted && mc_winter_wash > 0.0) {
  float cov = mc_fbm(p * 1.3 + vec3(5.0)) + 0.25 - edge * 0.35 - (1.0 - ao) * 0.4;
  float wash = mc_winter_wash * smoothstep(0.5, 0.62, cov);
  albedo = mix(albedo, vec3(0.78, 0.79, 0.76) * (0.95 + 0.1 * grain), wash);
}
if (mc_slot == 0) {
  float chips = edge * smoothstep(0.45, 0.6, mc_noise(p * 9.0)) + step(0.9, mc_noise(p * 17.0)) * 0.6;
  float w = clamp(chips * mc_wear, 0.0, 1.0);
  albedo = mix(albedo, vec3(0.22, 0.21, 0.20), w);
  mc_rough = mix(mc_rough, 0.5, w); mc_metal = mix(mc_metal, 0.4, w);
}
if (mc_slot != 5) {
  float line = 0.55 + 0.45 * mc_fbm(p * vec3(0.8, 2.0, 0.8)) + is_track * 1.5;
  float mud_mask = mc_mud * (1.0 - smoothstep(line - 0.25, line + 0.1, height));
  mud_mask *= 0.55 + 0.45 * mc_noise(p * 4.0);
  albedo = mix(albedo, mc_mud_color * (0.85 + 0.3 * grain), clamp(mud_mask * 1.3, 0.0, 1.0));
  mc_rough = mix(mc_rough, 0.95, mud_mask); mc_metal *= 1.0 - mud_mask;
  float dust_mask = mc_dust * (0.25 + 0.5 * (1.0 - smoothstep(0.0, 2.5, height)) + 0.3 * max(wn.y, 0.0));
  albedo = mix(albedo, mc_mud_color * 1.15, clamp(dust_mask * (0.6 + 0.4 * grain), 0.0, 0.85));
  float s = mc_snow * smoothstep(0.5, 0.75, wn.y + (mc_fbm(p * 2.5) - 0.5) * 0.5);
  albedo = mix(albedo, vec3(0.92, 0.93, 0.95), s);
  mc_rough = mix(mc_rough, 0.7, s); mc_metal *= 1.0 - s;
}
diffuseColor.rgb = albedo * mix(0.5, 1.0, ao);
`;

function unitMaterial(slot, pal) {
  const mat = new THREE.MeshStandardMaterial({ vertexColors: true });
  const uniforms = {
    mc_slot: { value: slot },
    mc_base: { value: srgb(pal.base) },
    mc_color_1: { value: srgb(pal.color_1) },
    mc_color_2: { value: srgb(pal.color_2) },
    mc_mud_color: { value: srgb(pal.mud_color) },
    mc_pattern: { value: pal.pattern },
    mc_scale: { value: pal.scale },
    mc_coverage: { value: pal.coverage },
    mc_winter_wash: { value: pal.winter_wash },
    mc_snow: { value: pal.snow },
    mc_mud: { value: pal.mud },
    mc_dust: { value: pal.dust },
    mc_wear: { value: pal.wear },
  };
  mat.onBeforeCompile = (shader) => {
    Object.assign(shader.uniforms, uniforms);
    shader.vertexShader = shader.vertexShader
      .replace('#include <common>', '#include <common>\nvarying vec3 mcLocal; varying vec3 mcWN; varying vec2 mcUv;')
      .replace('#include <begin_vertex>', '#include <begin_vertex>\nmcLocal = position; mcWN = normalize(mat3(modelMatrix) * normal); mcUv = uv;');
    shader.fragmentShader = shader.fragmentShader
      .replace('#include <common>', `#include <common>\n${GLSL_NOISE}`)
      .replace('#include <color_fragment>', GLSL_FRAGMENT)
      .replace('#include <roughnessmap_fragment>', 'float roughnessFactor = mc_rough;')
      .replace('#include <metalnessmap_fragment>', 'float metalnessFactor = mc_metal;');
  };
  mat.customProgramCacheKey = () => 'mc_unit';
  return mat;
}

function flatMaterial(color, name) {
  return new THREE.MeshStandardMaterial({
    color,
    roughness: name === 'Glass' ? 0.1 : name === 'Metal' ? 0.5 : 0.9,
    metalness: name === 'Metal' ? 0.5 : 0,
    side: name === 'Foliage' ? THREE.DoubleSide : THREE.FrontSide,
  });
}

const loader = new GLTFLoader();
const gltfCache = new Map();
function loadGltf(url) {
  if (!gltfCache.has(url)) gltfCache.set(url, loader.loadAsync(url));
  return gltfCache.get(url);
}

/** Members of a squad scene (.tscn): model path, placement and animation clip. */
function parseSquadScene(text) {
  const paths = {};
  for (const [, path, id] of text.matchAll(/\[ext_resource [^\]]*path="res:\/\/([^"]+)" id="([^"]+)"\]/g)) {
    paths[id] = path;
  }
  const clips = JSON.parse((text.match(/"squad_clips": (\[[^\]]*\])/) || [, '[]'])[1]);
  const members = [];
  const re = /\[node name="[^"]+" parent="\." instance=ExtResource\("([^"]+)"\)\]\s*(?:transform = Transform3D\(([^)]*)\))?/g;
  for (const [, id, transform] of text.matchAll(re)) {
    const n = transform ? transform.split(',').map(Number) : [1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0];
    const matrix = new THREE.Matrix4().set(
      n[0], n[1], n[2], n[9], n[3], n[4], n[5], n[10], n[6], n[7], n[8], n[11], 0, 0, 0, 1);
    members.push({ path: paths[id], matrix, clip: clips[members.length] });
  }
  return members;
}

/**
 * Builds the model of a catalog entry in the given biome.
 * Returns { root, mixers, variants } where variants are switchable child nodes
 * (damage states of buildings, tree species).
 */
async function buildModel(entry, biome, ctx) {
  const isSquad = entry.model.endsWith('.tscn');
  const isEnv = entry.category.startsWith('env_');
  const camo = paletteFor(entry, biome, ctx.look.camo);
  const soldier = ctx.look.uniforms.factions[entry.faction]?.[biome] || {};
  const cache = new Map();
  const material = (rawName) => {
    const name = (rawName || '').trim();
    if (!cache.has(name)) {
      let mat;
      if (isSquad) {
        mat = flatMaterial(soldier[name] || ctx.look.uniforms.fixed[name] || '#777777', name);
      } else if (!isEnv && name in UNIT_SLOTS) {
        mat = unitMaterial(UNIT_SLOTS[name], camo);
      } else if (name === 'Foliage') {
        mat = flatMaterial(srgb(FOLIAGE[biome]), name);
      } else {
        mat = flatMaterial(ENV_COLORS[name] || '#777777', name);
      }
      cache.set(name, mat);
    }
    return cache.get(name);
  };
  const dress = (object) => object.traverse((o) => {
    if (!o.isMesh) return;
    o.material = Array.isArray(o.material) ? o.material.map((m) => material(m.name)) : material(o.material.name);
    o.castShadow = true;
    o.receiveShadow = true;
  });

  const root = new THREE.Group();
  const mixers = [];
  let variants = [];
  if (isSquad) {
    const members = parseSquadScene(await ctx.text(`godot/${entry.model}`));
    await Promise.all(members.map(async (member) => {
      const gltf = await loadGltf(ctx.url(`godot/${member.path}`));
      const body = SkeletonUtils.clone(gltf.scene);
      dress(body);
      const holder = new THREE.Group();
      holder.applyMatrix4(member.matrix);
      holder.add(body);
      root.add(holder);
      const clip = gltf.animations.find((a) => a.name === member.clip) || gltf.animations[0];
      if (clip) {
        const mixer = new THREE.AnimationMixer(body);
        mixer.clipAction(clip).play();
        mixer.update(0.4);
        mixers.push(mixer);
      }
    }));
  } else {
    const gltf = await loadGltf(ctx.url(`godot/${entry.model}`));
    const body = SkeletonUtils.clone(gltf.scene);
    dress(body);
    root.add(body);
    const top = body.children.length === 1 ? body.children[0] : body;
    for (const child of top.children) if (child.name.endsWith('_Far')) child.visible = false;
    if (isEnv) {
      variants = top.children.filter((c) => !c.name.endsWith('_Far'));
      if (variants.length < 2) variants = [];
    }
  }
  return { root, mixers, variants };
}

export class Viewer {
  /**
   * `ctx` = { look, text(path), url(path) }. Non-interactive viewers only make snapshots.
   * `controls: false` leaves the camera to the caller (`onFrame` + `setOrbit`), `labels`
   * is the element that receives the texts of the dimension lines.
   */
  constructor(canvas, ctx, { interactive = true, controls = interactive, grid = interactive, labels = null } = {}) {
    this.ctx = ctx;
    this.canvas = canvas;
    this.interactive = interactive;
    this.showGrid = grid;
    this.gridColor = 0x000000;
    this.margin = 1.02; // distance of the fitted camera, 1 = model touches the frame
    this.labels = labels;
    this.onFrame = null;
    this.dimSpec = null;
    this.dimLines = null;
    this.dimAnchors = [];
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, preserveDrawingBuffer: !interactive });
    this.renderer.setPixelRatio(interactive ? Math.min(window.devicePixelRatio, 2) : 1);
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.toneMapping = THREE.NeutralToneMapping;
    this.scene = new THREE.Scene();
    const pmrem = new THREE.PMREMGenerator(this.renderer);
    this.scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    this.scene.environmentIntensity = 0.45;
    this.camera = new THREE.PerspectiveCamera(28, 4 / 3, 0.1, 1000);
    this.sun = new THREE.DirectionalLight(0xfff4e0, 2.6);
    this.sun.castShadow = true;
    this.sun.shadow.mapSize.set(2048, 2048);
    this.sun.shadow.bias = -0.0005;
    this.scene.add(this.sun, this.sun.target, new THREE.HemisphereLight(0xdfe8ff, 0x4a4436, 0.9));
    this.ground = new THREE.Mesh(new THREE.CircleGeometry(1, 64), new THREE.ShadowMaterial({ opacity: 0.35 }));
    this.ground.rotation.x = -Math.PI / 2;
    this.ground.receiveShadow = true;
    this.scene.add(this.ground);
    this.model = null;
    this.mixers = [];
    this.variants = [];
    this.token = 0;
    this.clock = new THREE.Clock();
    if (interactive) {
      if (controls) {
        this.controls = new OrbitControls(this.camera, canvas);
        this.controls.enableDamping = true;
        this.controls.autoRotate = true;
        this.controls.autoRotateSpeed = 1.2;
        this.controls.maxPolarAngle = Math.PI * 0.495;
      }
      new ResizeObserver(() => this.resize()).observe(canvas);
      this.resize();
      this.setRunning(true);
    } else {
      this.renderer.setSize(canvas.width, canvas.height, false);
      this.camera.aspect = canvas.width / canvas.height;
      this.camera.updateProjectionMatrix();
    }
  }

  resize() {
    const w = this.canvas.clientWidth;
    const h = this.canvas.clientHeight;
    if (!w || !h) return;
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
  }

  /** Starts or pauses the render loop (a hidden view should not cost frames). */
  setRunning(on) {
    if (this.running === on) return;
    this.running = on;
    this.clock.getDelta();
    this.renderer.setAnimationLoop(on ? () => this.frame() : null);
  }

  frame() {
    const dt = Math.min(this.clock.getDelta(), 0.1);
    for (const mixer of this.mixers) mixer.update(dt);
    if (this.controls) this.controls.update();
    if (this.onFrame) this.onFrame(dt);
    this.renderer.render(this.scene, this.camera);
    if (this.dimAnchors.length) this.placeLabels();
  }

  /** Puts the camera on a sphere around the model: angles in radians, distance as a factor of the fitted one. */
  setOrbit(azimuth, elevation, factor = 1) {
    if (!this.center) return;
    const d = this.fitDistance * factor;
    const flat = Math.cos(elevation) * d;
    this.camera.position.set(
      this.center.x + flat * Math.sin(azimuth),
      this.center.y + Math.sin(elevation) * d,
      this.center.z + flat * Math.cos(azimuth));
    this.camera.near = d / 100;
    this.camera.far = d * 10;
    this.camera.updateProjectionMatrix();
    this.camera.lookAt(this.center);
  }

  setGrid(on) {
    this.showGrid = on;
    if (this.model) this.fit(true);
  }

  /**
   * Dimension lines along the bounding box, labelled with the real catalog values:
   * `{ length, width, height }` in meters plus `text: { length, width, height }` and `color`,
   * or null for none.
   */
  setDimensions(spec) {
    this.dimSpec = spec;
    if (this.model) this.fit(true);
  }

  buildDimensions(box) {
    if (this.dimLines) {
      this.scene.remove(this.dimLines);
      this.dimLines.geometry.dispose();
      this.dimLines.material.dispose();
    }
    this.dimLines = null;
    this.dimAnchors = [];
    if (this.labels) this.labels.replaceChildren();
    const spec = this.dimSpec;
    if (!spec || !this.labels || box.isEmpty()) return;
    const { min, max } = box;
    const size = box.getSize(new THREE.Vector3());
    // The catalog length belongs to the horizontal axis it fits better.
    const lengthOnX = Math.abs(size.x - spec.length) + Math.abs(size.z - spec.width)
      <= Math.abs(size.z - spec.length) + Math.abs(size.x - spec.width);
    const gap = Math.max(size.x, size.z) * 0.07 + 0.15;
    const tick = gap * 0.35;
    const y = 0.02;
    const points = [];
    const v = (x, yy, z) => new THREE.Vector3(x, yy, z);
    const seg = (a, b) => points.push(a.x, a.y, a.z, b.x, b.y, b.z);
    const dim = (a, b, out, text) => {
      seg(a, b);
      for (const end of [a, b]) seg(end.clone().addScaledVector(out, -gap), end.clone().addScaledVector(out, tick));
      const label = document.createElement('span');
      label.className = 'dim';
      label.textContent = text;
      this.labels.append(label);
      this.dimAnchors.push({ at: a.clone().lerp(b, 0.5).addScaledVector(out, tick), label });
    };
    // On the two sides that face the fitted camera, the height at the corner to their left.
    dim(v(min.x, y, min.z - gap), v(max.x, y, min.z - gap), v(0, 0, -1), lengthOnX ? spec.text.length : spec.text.width);
    dim(v(max.x + gap, y, min.z), v(max.x + gap, y, max.z), v(1, 0, 0), lengthOnX ? spec.text.width : spec.text.length);
    dim(v(max.x, 0, max.z + gap), v(max.x, max.y, max.z + gap), v(0, 0, 1), spec.text.height);
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(points, 3));
    const material = new THREE.LineBasicMaterial({ color: spec.color || 0xffffff, transparent: true, opacity: 0.9, depthTest: false });
    this.dimLines = new THREE.LineSegments(geometry, material);
    this.dimLines.renderOrder = 10;
    this.scene.add(this.dimLines);
  }

  placeLabels() {
    const w = this.canvas.clientWidth;
    const h = this.canvas.clientHeight;
    const p = new THREE.Vector3();
    for (const { at, label } of this.dimAnchors) {
      p.copy(at).project(this.camera);
      // Kept inside the view, so a label at the edge stays readable.
      const x = Math.min(Math.max(((p.x + 1) / 2) * w, 48), w - 48);
      const y = Math.min(Math.max(((1 - p.y) / 2) * h, 16), h - 16);
      label.style.transform = `translate(${x}px, ${y}px) translate(-50%, -50%)`;
      label.style.visibility = p.z < 1 ? 'visible' : 'hidden';
    }
  }

  clear() {
    this.token++;
    if (this.model) this.scene.remove(this.model);
    if (this.grid) this.scene.remove(this.grid);
    this.model = null;
    this.grid = null;
    this.mixers = [];
    this.variants = [];
    this.buildDimensions(new THREE.Box3());
  }

  /** Shows an entry; resolves to the names of its variants. Throws if the model fails to load. */
  async show(entry, biome, { keepView = false } = {}) {
    const token = ++this.token;
    const built = await buildModel(entry, biome, this.ctx);
    if (token !== this.token) return null;
    if (this.model) this.scene.remove(this.model);
    this.model = built.root;
    this.mixers = built.mixers;
    this.variants = built.variants;
    this.scene.add(this.model);
    if (this.variants.length) {
      const intact = this.variants.findIndex((v) => v.name === 'Intact');
      this.setVariant(Math.max(intact, 0), keepView);
    } else {
      this.fit(keepView);
    }
    return this.variants.map((v) => v.name);
  }

  setVariant(index, keepView = false) {
    this.variants.forEach((v, i) => { v.visible = i === index; });
    this.fit(keepView);
  }

  /** Frames the visible model and sizes ground, grid, light and shadow to it. */
  fit(keepView) {
    this.model.updateMatrixWorld(true);
    const box = new THREE.Box3();
    this.model.traverseVisible((o) => { if (o.isMesh) box.expandByObject(o); });
    if (box.isEmpty()) box.set(new THREE.Vector3(-1, 0, -1), new THREE.Vector3(1, 1, 1));
    const sphere = box.getBoundingSphere(new THREE.Sphere());
    const r = Math.max(sphere.radius, 0.5);
    const center = sphere.center;
    const flat = new THREE.Vector3(center.x, 0, center.z);
    this.ground.position.copy(flat);
    this.ground.scale.setScalar(r * 3);
    if (this.grid) this.scene.remove(this.grid);
    this.grid = null;
    if (this.showGrid) {
      const cells = Math.max(4, Math.ceil(r * 1.5) * 2);
      this.grid = new THREE.GridHelper(cells, cells, this.gridColor, this.gridColor);
      this.grid.material.transparent = true;
      this.grid.material.opacity = 0.14;
      this.grid.position.set(Math.round(flat.x), 0.005, Math.round(flat.z));
      this.scene.add(this.grid);
    }
    this.sun.position.copy(center).add(new THREE.Vector3(-0.9, 1.7, -1.1).multiplyScalar(r * 2));
    this.sun.target.position.copy(center);
    const cam = this.sun.shadow.camera;
    cam.left = cam.bottom = -r * 1.6;
    cam.right = cam.top = r * 1.6;
    cam.near = 0.1;
    cam.far = r * 8;
    cam.updateProjectionMatrix();
    this.buildDimensions(box);
    const half = THREE.MathUtils.degToRad(this.camera.fov / 2);
    const fitV = r / Math.sin(half);
    const fitH = r / Math.sin(Math.atan(Math.tan(half) * this.camera.aspect));
    const distance = Math.max(fitV, fitH) * this.margin;
    this.center = center.clone();
    this.fitDistance = distance;
    if (keepView) {
      if (this.controls) this.controls.target.copy(center);
      return;
    }
    // Models look along -Z: the camera stands in front of them, like in the game's thumbnails.
    this.camera.position.copy(center).add(new THREE.Vector3(0.62, 0.48, -0.62).normalize().multiplyScalar(distance));
    this.camera.near = distance / 100;
    this.camera.far = distance * 10;
    this.camera.updateProjectionMatrix();
    this.camera.lookAt(center);
    if (this.controls) {
      this.controls.target.copy(center);
      this.controls.minDistance = r * 0.4;
      this.controls.maxDistance = distance * 3;
    }
  }

  /** Renders one entry and returns an object URL of the picture. */
  async snapshot(entry, biome) {
    await this.show(entry, biome);
    this.renderer.render(this.scene, this.camera);
    const blob = await new Promise((resolve) => this.canvas.toBlob(resolve, 'image/png'));
    return URL.createObjectURL(blob);
  }
}
