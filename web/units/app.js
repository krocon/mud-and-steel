// Unit page: hero with a live 3D model, the three armies, a duel of two units and the
// full catalog with a data sheet per entry (3D model, dimensions, armor, weapons, sources).
// Reads the repository files directly (no build step): data/catalog/*.toml,
// data/economy.toml, the camouflage palettes and the models under godot/assets/.
// Start it with `just units-web` (serves the repository root). The website serves
// web/units/ as its root, where `../../` stays at the root: `just units-web-sync` puts
// copies of those files under web/units/ for it.

import { parse } from 'smol-toml';
import { BIOMES, Viewer, loadLook } from './viewer.js';
import { Weather, calm, countUp, reveal } from './fx.js';

const ROOT = new URL('../../', import.meta.url);
const url = (path) => new URL(path, ROOT).href;
async function text(path) {
  const response = await fetch(url(path));
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return response.text();
}

const FACTIONS = [['de', 'Deutschland'], ['su', 'Sowjetunion'], ['us', 'USA'], ['neutral', 'Neutral']];
const FACTION_NAMES = Object.fromEntries(FACTIONS);
const ARMIES = ['de', 'su', 'us'];
const BIOME_NAMES = { forest: 'Wald', desert: 'Wüste', winter: 'Winter' };
// Same colors as --accent in style.css; the dimension lines of the 3D view use them.
const BIOME_ACCENT = { forest: 0xdcb64c, desert: 0xf0a552, winter: 0xa8d4ea };
const GROUPS = [
  ['inf', 'Infanterie'], ['veh', 'Fahrzeuge'], ['gun', 'Geschütze'], ['air', 'Flugzeuge'],
  ['fort', 'Befestigungen'], ['base', 'Basisgebäude'], ['env', 'Umgebung'],
];
const CATEGORY_NAMES = {
  inf_rifle: 'Schützengruppe', inf_lmg: 'lMG-Trupp', inf_hmg: 'sMG-Trupp',
  inf_mortar_light: 'leichter Granatwerfer', inf_mortar_medium: 'mittlerer Granatwerfer',
  inf_recon: 'Spähtrupp', inf_sniper: 'Scharfschützen', inf_medic: 'Sanitäter',
  inf_engineer: 'Pioniere', inf_at: 'Panzerabwehrtrupp', inf_flamethrower: 'Flammenwerfer',
  inf_command: 'Führungstrupp', inf_crew: 'Besatzung',
  tank_light: 'leichter Panzer', tank_medium: 'mittlerer Panzer', tank_heavy: 'schwerer Panzer',
  tank_destroyer: 'Jagdpanzer', assault_gun: 'Sturmgeschütz', halftrack: 'Halbkette',
  truck: 'Lkw', recon_vehicle: 'Spähwagen', self_propelled: 'Selbstfahrlafette',
  bridgelayer: 'Brückenleger',
  gun_at: 'Pak', gun_aa_light: 'leichte Flak', gun_aa_heavy: 'schwere Flak',
  gun_howitzer: 'Haubitze', mortar_heavy: 'schwerer Granatwerfer',
  fort_bunker: 'Bunker', fort_trench: 'Graben', fort_sandbag: 'Sandsackstellung',
  fort_obstacle: 'Sperre', fort_wire: 'Stacheldraht', fort_minefield_at: 'Panzerminenfeld',
  fort_minefield_ap: 'Schützenminenfeld',
  aircraft_fighter: 'Jäger', aircraft_attack: 'Schlachtflugzeug', aircraft_bomber: 'Bomber',
  aircraft_recon: 'Aufklärer', aircraft_transport: 'Transporter',
  env_tree: 'Baum', env_bush: 'Busch', env_grass: 'Gras', env_rock: 'Fels',
  env_building: 'Gebäude', env_bridge: 'Brücke', env_fence: 'Zaun', env_pole: 'Mast',
  env_prop: 'Objekt', base: 'Basisgebäude',
};
// Shown in the hero, one per army (falls back to that army's first tank).
const FEATURED = ['de_tiger1_e', 'su_t34_76_1943', 'us_m4a3_75'];
const DUEL_DEFAULT = ['de_tiger1_e', 'su_is2'];
const HERO_HOLD = 11; // seconds per featured unit

// Static markup only (never catalog text), so innerHTML is safe here.
const STAR = '12,1.5 14.59,8.94 22.46,9.1 16.18,13.86 18.47,21.4 12,16.9 5.53,21.4 7.82,13.86 1.54,9.1 9.41,8.94';
const MARKS = {
  de: '<svg viewBox="0 0 24 24"><path d="M9 2h6v7h7v6h-7v7H9v-7H2V9h7z" fill="#15130f" stroke="#ece4d0" stroke-width="1.7"/></svg>',
  su: `<svg viewBox="0 0 24 24"><polygon points="${STAR}" fill="#d2493a"/></svg>`,
  us: `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="11.5" fill="#4d5c3b"/><polygon points="${STAR}" fill="#ece4d0" transform="translate(2.4 2.6) scale(.8)"/></svg>`,
  neutral: '<svg viewBox="0 0 24 24"><path d="M12 3l9 9-9 9-9-9z" fill="none" stroke="#8a96a0" stroke-width="2"/></svg>',
};
const stroke = (d) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">${d}</svg>`;
const ICONS = {
  fuel: stroke('<path d="M12 3c3.5 4.5 6 7.6 6 11a6 6 0 0 1-12 0c0-3.4 2.5-6.5 6-11z"/>'),
  ammo: stroke('<path d="M9 21V10c0-3 1.3-5.5 3-7 1.7 1.5 3 4 3 7v11z"/><path d="M9 17h6"/>'),
  manpower: stroke('<path d="M4 15a8 8 0 0 1 16 0v1H4z"/><path d="M2.5 19h19"/>'),
  time: stroke('<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>'),
};

function groupOf(category) {
  if (category === 'base') return 'base';
  if (category.startsWith('inf_')) return 'inf';
  if (category.startsWith('gun_') || category === 'mortar_heavy') return 'gun';
  if (category.startsWith('aircraft_')) return 'air';
  if (category.startsWith('fort_')) return 'fort';
  if (category.startsWith('env_')) return 'env';
  return 'veh';
}

const num = (value, digits = 0) => value.toLocaleString('de-DE', { minimumFractionDigits: digits, maximumFractionDigits: digits });
const meters = (mm) => num(mm / 1000, 2);
const mass = (kg) => (kg >= 1000 ? `${num(kg / 1000, kg % 1000 ? 1 : 0)} t` : `${num(kg)} kg`);
const years = (e) => (e.year_from == null ? '' : `${e.year_from}–${String(e.year_to).slice(2)}`);
const categoryName = (e) => CATEGORY_NAMES[e.category] || e.category;
/** The weapon with the largest caliber. */
const mainWeapon = (e) => (e.weapon || []).reduce(
  (best, w) => (!best || (w.caliber_mm_x10 || 0) > (best.caliber_mm_x10 || 0) ? w : best), null);
const caliber = (c) => `${num(c / 10, c % 10 ? 1 : 0)} mm`;
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Tiny DOM builder: h('td', { class: 'x' }, 'text', node). */
function h(tag, attrs, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs || {})) {
    if (value === false || value == null) continue;
    if (key.startsWith('on')) node.addEventListener(key.slice(2), value);
    else node.setAttribute(key, value === true ? '' : value);
  }
  node.append(...children.flat().filter((c) => c != null && c !== false));
  return node;
}
/** A span holding one of the static SVGs above. */
function icon(markup, cls) {
  const span = h('span', { class: cls, 'aria-hidden': 'true' });
  span.innerHTML = markup;
  return span;
}

const state = {
  entries: [],
  byId: new Map(),
  costs: new Map(),
  faction: 'de',
  group: 'all',
  query: '',
  biome: 'forest',
  selected: null,
  shown: [],
  maxArmor: 1,
  maxRange: 1,
};
try {
  const saved = JSON.parse(localStorage.getItem('units-web') || '{}');
  if (FACTIONS.some(([id]) => id === saved.faction)) state.faction = saved.faction;
  if (BIOMES.includes(saved.biome)) state.biome = saved.biome;
} catch { /* storage unavailable: defaults */ }
function remember() {
  try {
    localStorage.setItem('units-web', JSON.stringify({ faction: state.faction, biome: state.biome }));
  } catch { /* ignore */ }
}

const $ = (id) => document.getElementById(id);
const dialog = $('dossier');
let viewer = null;
let snapper = null;
let weather = null;

// --- data ---------------------------------------------------------------------------

async function loadData() {
  const files = await Promise.all(['de', 'su', 'us', 'neutral'].map((f) => text(`data/catalog/${f}.toml`)));
  const entries = files.flatMap((file) => parse(file).unit || []);
  const economy = parse(await text('data/economy.toml'));
  for (const b of economy.building || []) {
    const { id, name, cost, ...rest } = b;
    entries.push({ id, faction: 'neutral', category: 'base', name_de: name, name_en: name, ...rest });
  }
  state.entries = entries;
  state.byId = new Map(entries.map((e) => [e.id, e]));
  for (const e of entries) {
    for (const [key, value] of Object.entries(e)) {
      if (/^armor_.*_mm$/.test(key)) state.maxArmor = Math.max(state.maxArmor, value);
    }
    for (const w of e.weapon || []) state.maxRange = Math.max(state.maxRange, w.range_effective_m || 0, w.range_aa_m || 0);
  }
  // Prices come from the simulation (`cli economy-table`), written by tools/units_web.py
  // next to this page.
  try {
    const response = await fetch(new URL('economy.md', import.meta.url));
    if (!response.ok) throw new Error(`economy.md: HTTP ${response.status}`);
    const table = await response.text();
    for (const line of table.split('\n')) {
      const cells = line.split('|').map((c) => c.trim());
      const id = (cells[2] || '').match(/^`(.+)`$/);
      if (!id) continue;
      state.costs.set(id[1], {
        builtIn: cells[1], fuel: Number(cells[3]), ammo: Number(cells[4]),
        manpower: Number(cells[5]), time: Number(cells[6]),
      });
    }
  } catch {
    $('status').textContent = 'Keine Preise: economy.md fehlt (`just units-web` oder `just units-web-sync`).';
  }
}

// --- pictures -----------------------------------------------------------------------

const snapshots = new Map();
let snapQueue = Promise.resolve();
function snapshot(entry) {
  const key = `${entry.id}|${state.biome}`;
  if (!snapshots.has(key)) {
    const biome = state.biome;
    const job = snapQueue.then(() => snapper.snapshot(entry, biome));
    snapQueue = job.catch(() => {});
    snapshots.set(key, job);
  }
  return snapshots.get(key);
}

const observer = new IntersectionObserver((items) => {
  for (const item of items) {
    if (!item.isIntersecting) continue;
    observer.unobserve(item.target);
    fillPicture(item.target);
  }
}, { rootMargin: '200px' });

/** An empty picture frame that fills itself once it scrolls into view. */
function picture(entry, cls = '') {
  const pic = h('span', { class: `pic ${cls}`, 'data-id': entry.id });
  observer.observe(pic);
  return pic;
}

/** Game thumbnail (forest look) if there is one, else a rendered snapshot of the model. */
function fillPicture(pic) {
  const entry = state.byId.get(pic.dataset.id);
  const biome = state.biome;
  const img = h('img', { alt: '' });
  const render = () => {
    if (!entry.model) { pic.replaceChildren(h('span', null, 'kein Modell')); return; }
    snapshot(entry).then((src) => {
      if (biome !== state.biome) return;
      img.onerror = null;
      img.src = src;
      pic.replaceChildren(img);
    }, () => { pic.replaceChildren(h('span', null, 'Modell fehlt')); });
  };
  if (biome === 'forest' || !entry.model) {
    img.onerror = render;
    img.onload = () => pic.replaceChildren(img);
    img.src = url(`godot/assets/thumbnails/${entry.id}.png`);
  } else {
    render();
  }
}

// --- biome: camouflage, colors and weather of the whole page ------------------------

function renderBiomes() {
  for (const box of document.querySelectorAll('[data-biomes]')) {
    box.replaceChildren(...BIOMES.map((id) => h('button', {
      'aria-pressed': String(state.biome === id), onclick: () => setBiome(id),
    }, BIOME_NAMES[id])));
  }
}

function setBiome(id) {
  if (id === state.biome) return;
  state.biome = id;
  document.documentElement.dataset.biome = id;
  remember();
  renderBiomes();
  weather.set(id);
  for (const pic of document.querySelectorAll('.pic[data-id]')) {
    observer.unobserve(pic);
    observer.observe(pic);
  }
  const featured = hero.entries[hero.index];
  if (featured) hero.viewer.show(featured, id, { keepView: true }).catch(console.error);
  if (dialog.open) showModel(true);
}

// --- hero ---------------------------------------------------------------------------

const hero = {
  viewer: null, entries: [], index: 0,
  time: 0,      // seconds since the current unit appeared (drives the camera sweep)
  clock: 0,     // seconds towards the next unit
  drift: 0, pointer: 0, pointerTarget: 0, scroll: 0,
  hold: false, swapping: false, visible: true,
};
const lerp = (a, b, t) => a + (b - a) * t;
const easeOutExpo = (t) => (t >= 1 ? 1 : 1 - 2 ** (-10 * t));

function heroFrame(dt) {
  hero.time += dt;
  const still = calm.matches;
  const k = still ? 1 : easeOutExpo(Math.min(hero.time / 2.8, 1));
  if (!still) hero.drift += dt * 0.07;
  hero.pointer += (hero.pointerTarget - hero.pointer) * Math.min(1, dt * 3);
  const p = hero.scroll;
  hero.viewer.setOrbit(
    2.0 - (1 - k) * 1.5 + hero.drift + hero.pointer * 0.35,
    lerp(0.05, 0.27, k) + p * 0.6,
    lerp(0.55, 0.98, k) + p * 0.3);
  if (!still && !hero.hold && !hero.swapping && hero.entries.length > 1) {
    hero.clock += dt;
    $('plate').style.setProperty('--t', Math.min(hero.clock / HERO_HOLD, 1));
    if (hero.clock >= HERO_HOLD) heroShow((hero.index + 1) % hero.entries.length);
  }
}

async function heroShow(index) {
  if (hero.swapping) return;
  hero.swapping = true;
  hero.index = index;
  hero.clock = 0;
  const entry = hero.entries[index];
  const canvas = $('hero-view');
  canvas.classList.add('swap');
  renderPlate();
  await wait(calm.matches ? 0 : 340);
  try {
    await hero.viewer.show(entry, state.biome);
    hero.time = 0;
    hero.drift = 0;
  } catch (error) {
    console.error(error);
  }
  canvas.classList.remove('swap');
  hero.swapping = false;
}

function renderPlate() {
  const e = hero.entries[hero.index];
  const weapon = mainWeapon(e);
  const facts = [
    e.mass_kg != null && [mass(e.mass_kg), 'Masse'],
    e.year_from != null && [years(e), 'Im Dienst'],
    weapon && [weapon.name, 'Hauptwaffe'],
  ].filter(Boolean);
  const plate = $('plate');
  plate.style.setProperty('--t', 0);
  plate.replaceChildren(
    h('div', { class: 'pips' }, hero.entries.map((entry, i) => h('button', {
      'aria-pressed': String(i === hero.index), 'aria-label': entry.name_de, onclick: () => heroShow(i),
    }, icon(MARKS[entry.faction], 'mark'), h('i')))),
    h('p', { class: 'plate-kicker' }, `${categoryName(e)} · ${FACTION_NAMES[e.faction]}`),
    h('h2', null, e.name_de),
    h('ul', { class: 'facts' }, facts.map(([value, label]) => h('li', null, h('b', null, value), label))),
    h('button', { class: 'btn small', onclick: () => openDossier(e.id) }, 'Datenblatt'));
}

function renderKpis() {
  const units = state.entries.filter((e) => ARMIES.includes(e.faction));
  const weapons = new Set(state.entries.flatMap((e) => (e.weapon || []).map((w) => w.name)));
  const kpis = [
    [units.length, 'Einheiten und Stellungen'],
    [weapons.size, 'Waffen im Katalog'],
    [state.entries.reduce((sum, e) => sum + (e.source || []).length, 0), 'Quellenangaben'],
    [state.entries.filter((e) => e.model).length, '3D-Modelle'],
  ];
  $('kpis').replaceChildren(...kpis.map(([value, label]) => {
    const dd = h('dd', null, '0');
    countUp(dd, value, (v) => num(Math.round(v)), 1600);
    return h('div', null, dd, h('dt', null, label));
  }));
}

function fitTitle() {
  const title = $('title');
  title.style.fontSize = '100px';
  const size = 100 * (window.innerWidth * 0.93) / title.offsetWidth;
  title.style.fontSize = `${Math.min(size, window.innerHeight * 0.4)}px`;
}

function setupHero(ctx) {
  // One span per letter for the staggered entrance.
  let n = 0;
  for (const word of document.querySelectorAll('#title .word')) {
    word.replaceChildren(...[...word.textContent].map((ch) => h('span', { class: 'ch', style: `--n:${n++}`, 'aria-hidden': 'true' }, ch)));
  }
  fitTitle();
  document.fonts?.ready.then(fitTitle);
  window.addEventListener('resize', fitTitle);

  hero.viewer = new Viewer($('hero-view'), ctx, { controls: false, grid: false });
  hero.viewer.onFrame = heroFrame;
  weather = new Weather($('weather'));
  weather.set(state.biome);

  const section = $('top');
  let queued = false;
  const onScroll = () => {
    queued = false;
    hero.scroll = Math.min(Math.max(window.scrollY / section.offsetHeight, 0), 1);
    section.style.setProperty('--p', hero.scroll.toFixed(3));
    $('nav').classList.toggle('solid', window.scrollY > 40);
  };
  window.addEventListener('scroll', () => { if (!queued) { queued = true; requestAnimationFrame(onScroll); } }, { passive: true });
  onScroll();
  window.addEventListener('pointermove', (event) => { hero.pointerTarget = event.clientX / window.innerWidth - 0.5; }, { passive: true });
  $('plate').addEventListener('pointerenter', () => { hero.hold = true; });
  $('plate').addEventListener('pointerleave', () => { hero.hold = false; });
  new IntersectionObserver(([item]) => { hero.visible = item.isIntersecting; syncLoops(); }).observe(section);
  document.addEventListener('visibilitychange', syncLoops);
}

/** Only the view the visitor looks at renders: the hero, or the data sheet's model. */
function syncLoops() {
  const heroOn = hero.visible && !dialog.open && !document.hidden;
  hero.viewer.setRunning(heroOn);
  weather.setRunning(heroOn);
  viewer.setRunning(dialog.open && !document.hidden);
}

// --- armies -------------------------------------------------------------------------

function pickFaction(id) {
  state.faction = id;
  state.group = 'all';
  state.query = '';
  $('search').value = '';
  remember();
  renderControls();
  renderList();
}

function renderArmies() {
  $('armies').replaceChildren(...ARMIES.map((faction, i) => {
    const all = state.entries.filter((e) => e.faction === faction);
    const counts = GROUPS.map(([group, name]) => [name, all.filter((e) => groupOf(e.category) === group).length])
      .filter(([, count]) => count);
    const heaviest = all.filter((e) => e.mass_kg).sort((a, b) => b.mass_kg - a.mass_kg)[0];
    const order = ['veh', 'gun', 'air', 'inf'];
    const parade = all.filter((e) => e.model && order.includes(groupOf(e.category)))
      .sort((a, b) => order.indexOf(groupOf(a.category)) - order.indexOf(groupOf(b.category)))
      .slice(0, 14);
    const strip = (hidden) => h('div', { class: 'strip', 'aria-hidden': hidden ? 'true' : null }, parade.map((e) => picture(e)));
    return h('a', {
      class: 'army', href: '#arsenal', 'data-faction': faction, 'data-reveal': true, style: `--i:${i}`,
      onclick: () => pickFaction(faction),
    },
    h('div', { class: 'army-head' }, icon(MARKS[faction], 'mark'), h('h3', null, FACTION_NAMES[faction]),
      h('span', { class: 'count' }, `${all.length} Einträge`)),
    h('ul', { class: 'army-stats' }, counts.map(([name, count]) => h('li', null, h('b', null, String(count)), name))),
    heaviest ? h('p', { class: 'army-fact' }, 'Schwerstes Gerät ', h('strong', null, `${heaviest.name_de} · ${mass(heaviest.mass_kg)}`)) : null,
    h('div', { class: 'marquee' }, h('div', { class: 'track', style: `--dur:${parade.length * 4.2}s` }, strip(false), strip(true))),
    h('span', { class: 'more' }, 'Im Arsenal ansehen'));
  }));
}

// --- duel ---------------------------------------------------------------------------

const duel = { ids: [...DUEL_DEFAULT] };
const duelPool = () => state.entries.filter((e) => ARMIES.includes(e.faction) && ['veh', 'gun'].includes(groupOf(e.category)));
const main = (field) => (e) => mainWeapon(e)?.[field];
const price = (field) => (e) => state.costs.get(e.id)?.[field];
// [label, value(entry), format(value), neutral]: neutral rows have no "more is stronger".
const DUEL_ROWS = [
  ['Masse', (e) => e.mass_kg, mass, true],
  ['Straße', (e) => e.speed_road_kmh_x10, (v) => `${num(v / 10)} km/h`],
  ['Gelände', (e) => e.speed_offroad_kmh_x10, (v) => `${num(v / 10)} km/h`],
  ['Motorleistung', (e) => e.power_hp, (v) => `${num(v)} PS`],
  ['Wanne Front', (e) => e.armor_hull_front_mm, (v) => `${v} mm`],
  ['Wanne Seite', (e) => e.armor_hull_side_mm, (v) => `${v} mm`],
  ['Turm Front', (e) => e.armor_turret_front_mm, (v) => `${v} mm`],
  ['Kaliber', main('caliber_mm_x10'), caliber],
  ['Mündungsgeschw.', main('muzzle_velocity_mps'), (v) => `${num(v)} m/s`],
  ['Reichweite', main('range_effective_m'), (v) => `${num(v)} m`],
  ['Kadenz', main('rate_rpm'), (v) => `${num(v)}/min`],
  ['Treibstoff', price('fuel'), num, true],
  ['Munition', price('ammo'), num, true],
  ['Personal', price('manpower'), num, true],
  ['Bauzeit', price('time'), (v) => `${num(v, 1)} s`, true],
];

function renderDuel() {
  const pool = duelPool();
  duel.ids = duel.ids.map((id, i) => (pool.some((e) => e.id === id) ? id : pool[i]?.id));
  const [a, b] = duel.ids.map((id) => state.byId.get(id));
  if (!a || !b) { $('duel').replaceChildren(); return; }
  const side = (entry, index) => {
    const select = h('select', {
      'aria-label': index ? 'Rechte Einheit' : 'Linke Einheit',
      onchange: (event) => { duel.ids[index] = event.target.value; renderDuel(); },
    }, ARMIES.map((faction) => h('optgroup', { label: FACTION_NAMES[faction] },
      pool.filter((e) => e.faction === faction).map((e) => h('option', { value: e.id, selected: e.id === entry.id }, e.name_de)))));
    const weapon = mainWeapon(entry);
    return h('div', { class: `side side-${index ? 'b' : 'a'}`, 'data-faction': entry.faction },
      picture(entry, 'big'),
      h('p', { class: 'side-kicker' }, icon(MARKS[entry.faction], 'mark'), `${categoryName(entry)} · ${years(entry)}`),
      h('h3', null, entry.name_de),
      h('p', { class: 'side-sub' }, weapon ? weapon.name : ''),
      select,
      h('button', { class: 'link', onclick: () => openDossier(entry.id) }, 'Datenblatt'));
  };
  const rows = DUEL_ROWS.map(([label, value, format, neutral]) => {
    const [va, vb] = [value(a), value(b)];
    if (va == null && vb == null) return null;
    const max = Math.max(va || 0, vb || 0) || 1;
    const cell = (v, other, cls) => h('div', { class: `val ${cls}${!neutral && v != null && other != null && v > other ? ' win' : ''}` },
      h('span', null, v == null ? '–' : format(v)),
      h('i', { style: `--w:${v == null ? 0 : Math.max((v / max) * 100, 1.5)}%` }));
    return h('div', { class: 'row' }, cell(va, vb, 'a'), h('span', { class: 'label' }, label), cell(vb, va, 'b'));
  }).filter(Boolean);
  const random = () => {
    const first = pool[Math.floor(Math.random() * pool.length)];
    // An opponent of the same kind from another army, if there is one.
    const rivals = pool.filter((e) => e.faction !== first.faction && e.category === first.category);
    const others = rivals.length ? rivals : pool.filter((e) => e.id !== first.id);
    duel.ids = [first.id, others[Math.floor(Math.random() * others.length)].id];
    renderDuel();
  };
  $('duel').replaceChildren(
    h('div', { class: 'duel-head' }, side(a, 0),
      h('div', { class: 'versus' }, h('span', null, 'VS'),
        h('button', { class: 'btn small', onclick: random }, 'Zufallsduell'),
        h('button', { class: 'link', onclick: () => { duel.ids.reverse(); renderDuel(); } }, 'Seiten tauschen')),
      side(b, 1)),
    h('div', { class: 'rows' }, rows),
    h('p', { class: 'duel-note' }, 'Reale Katalogwerte der jeweils stärksten Waffe; Preise aus der Simulation. Kein Urteil – das fällt auf dem Gefechtsfeld.'));
}

// --- arsenal ------------------------------------------------------------------------

function searchText(entry) {
  return [entry.id, entry.name_de, entry.name_en, entry.designation, CATEGORY_NAMES[entry.category],
    ...(entry.weapon || []).map((w) => w.name)].join(' ').toLowerCase();
}

function renderControls() {
  $('factions').replaceChildren(...FACTIONS.map(([id, name]) => h('button', {
    'aria-pressed': String(state.faction === id), 'data-faction': id, onclick: () => pickFaction(id),
  }, icon(MARKS[id], 'mark'), name, h('small', null, String(state.entries.filter((e) => e.faction === id).length)))));
  const present = new Set(state.entries.filter((e) => e.faction === state.faction).map((e) => groupOf(e.category)));
  const chips = [['all', 'Alle'], ...GROUPS.filter(([id]) => present.has(id))];
  $('groups').replaceChildren(...chips.map(([id, name]) => h('button', {
    'aria-pressed': String(state.group === id),
    onclick: () => { state.group = id; renderControls(); renderList(); },
  }, name)));
}

/** Two short facts under the name of a card. */
function cardFacts(e) {
  const weapon = mainWeapon(e);
  const group = groupOf(e.category);
  const facts = [];
  if (group === 'inf' && e.squad_size != null) facts.push(`${e.squad_size} Mann`);
  if (group === 'air' && e.speed_max_kmh_x10 != null) facts.push(`${num(e.speed_max_kmh_x10 / 10)} km/h`);
  if (e.mass_kg != null && group !== 'inf') facts.push(mass(e.mass_kg));
  if (weapon?.caliber_mm_x10 != null && group !== 'inf') facts.push(caliber(weapon.caliber_mm_x10));
  if (!facts.length && e.length_mm != null && e.width_mm != null) facts.push(`${meters(e.length_mm)} × ${meters(e.width_mm)} m`);
  if (!facts.length && e.hp != null) facts.push(`${num(e.hp)} TP`);
  return facts.slice(0, 2);
}

function renderList() {
  for (const pic of document.querySelectorAll('#list .pic')) observer.unobserve(pic);
  const query = state.query.trim().toLowerCase();
  // A search looks through all armies, so the army and group buttons step back.
  $('toolbar').classList.toggle('searching', Boolean(query));
  const shown = state.entries.filter((e) => (query
    ? searchText(e).includes(query)
    : e.faction === state.faction && (state.group === 'all' || groupOf(e.category) === state.group)));
  const sections = [];
  state.shown = [];
  for (const [group, title] of GROUPS) {
    const members = shown.filter((e) => groupOf(e.category) === group);
    if (!members.length) continue;
    state.shown.push(...members.map((e) => e.id));
    sections.push(
      h('h3', { class: 'group' }, title, h('small', null, String(members.length))),
      h('div', { class: 'cards' }, members.map((e, i) => h('button', {
        class: 'card', 'data-id': e.id, 'data-faction': e.faction, style: `--i:${Math.min(i, 16)}`,
        'aria-current': String(e.id === state.selected), onclick: () => openDossier(e.id),
      },
      picture(e),
      h('span', { class: 'cat' }, categoryName(e), e.year_from != null ? h('em', null, years(e)) : null),
      h('span', { class: 'name' }, e.name_de),
      h('span', { class: 'facts' }, cardFacts(e).map((fact) => h('i', null, fact)))))));
  }
  $('list').replaceChildren(...(sections.length ? sections : [h('p', { class: 'empty' }, 'Keine Treffer.')]));
}

// --- data sheet ---------------------------------------------------------------------

function sequence() {
  return state.shown.includes(state.selected) ? state.shown : state.entries.map((e) => e.id);
}

function step(delta) {
  const ids = sequence();
  const index = ids.indexOf(state.selected);
  openDossier(ids[(index + delta + ids.length) % ids.length]);
}

function openDossier(id) {
  if (!state.byId.has(id)) return;
  state.selected = id;
  history.replaceState(null, '', `#${id}`);
  for (const card of document.querySelectorAll('.card')) {
    card.setAttribute('aria-current', String(card.dataset.id === id));
  }
  if (!dialog.open) dialog.showModal();
  syncLoops();
  renderSheet();
  showModel(false);
  const ids = sequence();
  $('position').textContent = `${ids.indexOf(id) + 1} / ${ids.length}`;
}

function closeDossier() {
  history.replaceState(null, '', location.pathname + location.search);
  viewer.clear();
  syncLoops();
}

function dimensions(entry) {
  const width = entry.width_mm ?? entry.wingspan_mm;
  if (!$('measure').checked || entry.length_mm == null || width == null || entry.height_mm == null) return null;
  return {
    length: entry.length_mm / 1000, width: width / 1000, height: entry.height_mm / 1000,
    color: BIOME_ACCENT[state.biome],
    text: {
      length: `L ${meters(entry.length_mm)} m`,
      width: `${entry.width_mm == null ? 'Spannweite' : 'B'} ${meters(width)} m`,
      height: `H ${meters(entry.height_mm)} m`,
    },
  };
}

async function showModel(keepView) {
  const entry = state.byId.get(state.selected);
  const [canvas, still, msg, variant] = [$('view'), $('still'), $('stage-msg'), $('variant')];
  variant.hidden = true;
  msg.hidden = true;
  $('watermark').textContent = entry.designation || entry.id;
  if (!entry.model) {
    viewer.clear();
    canvas.hidden = true;
    still.hidden = false;
    still.onerror = () => { still.hidden = true; msg.hidden = false; msg.textContent = 'Kein Modell vorhanden'; };
    still.src = url(`godot/assets/thumbnails/${entry.id}.png`);
    return;
  }
  canvas.hidden = false;
  still.hidden = true;
  $('stage').classList.toggle('loading', !keepView);
  try {
    viewer.dimSpec = dimensions(entry);
    const variants = await viewer.show(entry, state.biome, { keepView });
    if (variants === null) return; // a newer request took over
    $('stage').classList.remove('loading');
    if (!variants.length) return;
    const current = keepView ? variant.value : '';
    variant.replaceChildren(...variants.map((name, i) => h('option', { value: String(i) }, name)));
    variant.value = current && variants[current] ? current : String(Math.max(variants.indexOf('Intact'), 0));
    viewer.setVariant(Number(variant.value), keepView);
    variant.hidden = false;
  } catch (error) {
    console.error(error);
    viewer.clear();
    $('stage').classList.remove('loading');
    msg.hidden = false;
    msg.textContent = `Modell konnte nicht geladen werden (${entry.model})`;
  }
}

const unsureMark = () => h('span', { class: 'unsure', title: 'Wert laut Katalog unsicher' }, '?');

/** Service years on a time line that always shows the war years 1939–1945. */
function timeline(e) {
  const from = Math.min(e.year_from, 1939);
  const to = Math.max(e.year_to, 1945) + 1;
  const pos = (year) => `${((year - from) / (to - from)) * 100}%`;
  const ticks = [];
  for (let year = from; year < to; year++) {
    const edge = year === e.year_from || year === e.year_to;
    ticks.push(h('i', { class: edge ? 'edge' : null, style: `left:${pos(year + 0.5)}` }, edge || to - from <= 9 ? String(year).slice(2) : null));
  }
  return h('div', { class: 'years', role: 'img', 'aria-label': `Im Dienst ${e.year_from} bis ${e.year_to}` },
    h('span', { class: 'war', style: `left:${pos(1939)};right:calc(100% - ${pos(1946)})` }),
    h('span', { class: 'span', style: `left:${pos(e.year_from)};right:calc(100% - ${pos(e.year_to + 1)})` }),
    ticks);
}

/** Top views of hull and turret; the stroke of each face grows with its armor. */
function armorDiagram(e, unsure) {
  if (e.armor_hull_front_mm == null) return null;
  const width = (mm) => (mm == null ? 0.6 : 2 + 15 * Math.sqrt(mm / state.maxArmor));
  const value = (field) => (e[field] == null ? '–' : `${e[field]}${unsure.has(field) ? ' ?' : ''}`);
  const labels = (part) => `
    <text x="208" y="62" class="k">FRONT</text><text x="208" y="80">${value(`armor_${part}_front_mm`)}</text>
    <text x="32" y="62" class="k" text-anchor="end">HECK</text><text x="32" y="80" text-anchor="end">${value(`armor_${part}_rear_mm`)}</text>
    <text x="120" y="14" class="k" text-anchor="middle">SEITE ${value(`armor_${part}_side_mm`)}</text>`;
  const w = (part, face) => width(e[`armor_${part}_${face}_mm`]);
  const hull = `<svg viewBox="0 0 250 130" role="img" aria-label="Wanne von oben">
    <rect x="45" y="30" width="150" height="76" class="body"/>
    <path d="M45 30H195M45 106H195" stroke-width="${w('hull', 'side')}"/>
    <path d="M45 30V106" stroke-width="${w('hull', 'rear')}"/>
    <path d="M195 30V106" stroke-width="${w('hull', 'front')}" class="front"/>
    ${labels('hull')}</svg>`;
  const turret = ['front', 'side', 'rear'].some((face) => e[`armor_turret_${face}_mm`] != null) && `<svg viewBox="0 0 250 130" role="img" aria-label="Turm von oben">
    <circle cx="120" cy="68" r="36" class="body"/>
    <path d="M94.5 42.5A36 36 0 0 1 145.5 42.5M94.5 93.5A36 36 0 0 0 145.5 93.5" stroke-width="${w('turret', 'side')}"/>
    <path d="M94.5 42.5A36 36 0 0 0 94.5 93.5" stroke-width="${w('turret', 'rear')}"/>
    <path d="M145.5 42.5A36 36 0 0 1 145.5 93.5" stroke-width="${w('turret', 'front')}" class="front"/>
    <path d="M162 68H200" class="barrel"/>
    ${labels('turret')}</svg>`;
  const figure = (svg, caption) => {
    const box = h('figure', null);
    box.innerHTML = svg;
    box.append(h('figcaption', null, caption));
    return box;
  };
  return h('div', { class: 'armor' },
    figure(hull, 'Wanne'),
    turret ? figure(turret, e.gun_traverse_deg != null ? 'Aufbau' : 'Turm') : null,
    h('p', null, 'Millimeter, Ansicht von oben, Fahrtrichtung rechts.',
      e.armor_hull_front_deg != null ? ` Bugplatte ${e.armor_hull_front_deg}° gegen die Senkrechte geneigt.` : ''));
}

function weaponList(e) {
  const spec = (label, value) => (value == null ? null : h('span', null, h('small', null, label), value));
  return h('ul', { class: 'weapons' }, e.weapon.map((w) => {
    const mm = (w.caliber_mm_x10 || 0) / 10;
    const range = w.range_effective_m;
    return h('li', null,
      h('span', { class: 'bore', style: `--d:${Math.min(Math.max(mm * 0.24 + 2, 4), 38)}px` }),
      h('div', { class: 'w-name' },
        h('strong', null, `${w.count > 1 ? `${w.count}× ` : ''}${w.name}`),
        w.aa ? h('em', null, 'Fla') : null,
        w.ammo ? h('small', null, w.ammo) : null),
      range != null ? h('div', { class: 'w-range' },
        h('i', { style: `--w:${Math.max(Math.sqrt(range / state.maxRange) * 100, 2)}%` }),
        h('span', null, `${num(range)} m`, w.range_aa_m != null ? h('small', null, ` · Fla ${num(w.range_aa_m)} m`) : null)) : null,
      h('div', { class: 'w-specs' },
        spec('Kaliber', w.caliber_mm_x10 != null ? caliber(w.caliber_mm_x10) : w.bomb_kg != null ? `${w.bomb_kg} kg` : null),
        spec('Kadenz', w.rate_rpm != null ? `${num(w.rate_rpm)}/min` : null),
        spec('v₀', w.muzzle_velocity_mps != null ? `${num(w.muzzle_velocity_mps)} m/s` : null),
        spec('Munition', w.ammo_carried != null ? num(w.ammo_carried) : null)));
  }));
}

function renderSheet() {
  const e = state.byId.get(state.selected);
  const unsure = new Set(e.uncertain || []);
  const group = groupOf(e.category);
  const used = new Set(['id', 'faction', 'category', 'name_de', 'name_en', 'designation', 'model',
    'weapon', 'source', 'uncertain', 'note', 'tier', 'year_from', 'year_to']);

  // The headline numbers: counted up as big tiles.
  const tiles = [];
  const tile = (label, fields, value, format, sub) => {
    if (fields.some((f) => e[f] == null)) return;
    fields.forEach((f) => used.add(f));
    const figure = h('b', null);
    countUp(figure, value, format, 900);
    tiles.push(h('div', null, figure, fields.some((f) => unsure.has(f)) ? unsureMark() : null,
      h('span', null, label), sub ? h('small', null, sub) : null));
  };
  tile('Masse', ['mass_kg'], e.mass_kg, (v) => mass(Math.round(v)));
  tile('Besatzung', ['crew'], e.crew, (v) => num(Math.round(v)));
  tile('Truppstärke', ['squad_size'], e.squad_size, (v) => `${num(Math.round(v))} Mann`);
  if (group !== 'inf') {
    tile('Straße', ['speed_road_kmh_x10', 'speed_offroad_kmh_x10'], e.speed_road_kmh_x10 / 10,
      (v) => `${num(Math.round(v))} km/h`, `Gelände ${num((e.speed_offroad_kmh_x10 || 0) / 10, e.speed_offroad_kmh_x10 % 10 ? 1 : 0)} km/h`);
  }
  tile('Höchstgeschwindigkeit', ['speed_max_kmh_x10'], e.speed_max_kmh_x10 / 10, (v) => `${num(Math.round(v))} km/h`);
  tile('Motorleistung', ['power_hp'], e.power_hp, (v) => `${num(Math.round(v))} PS`,
    e.mass_kg ? `${num(e.power_hp / (e.mass_kg / 1000), 1)} PS/t` : null);

  const rows = [];
  /** Adds a row when all its fields exist; marks uncertain values. */
  const row = (label, fields, format) => {
    if (fields.some((f) => used.has(f) || e[f] == null)) return;
    fields.forEach((f) => used.add(f));
    rows.push(h('dt', null, label), h('dd', null, format(...fields.map((f) => e[f])),
      fields.some((f) => unsure.has(f)) ? unsureMark() : null));
  };
  const lwh = (l, w, t) => `${meters(l)} × ${meters(w)} × ${meters(t)} m`;
  if (e.width_mm == null && e.wingspan_mm != null) row('Länge × Spannweite × Höhe', ['length_mm', 'wingspan_mm', 'height_mm'], lwh);
  else row('Länge × Breite × Höhe', ['length_mm', 'width_mm', 'height_mm'], lwh);
  row('Spannweite', ['wingspan_mm'], (w) => `${meters(w)} m`);
  row('Wannenlänge', ['length_hull_mm'], (l) => `${meters(l)} m`);
  row('Straße / Gelände', ['speed_road_kmh_x10', 'speed_offroad_kmh_x10'], (a, b) => `${num(a / 10, 1)} / ${num(b / 10, 1)} km/h`);
  row('Im Wasser', ['speed_water_kmh_x10'], (a) => `${num(a / 10, 1)} km/h`);
  row('Reisegeschwindigkeit', ['speed_cruise_kmh_x10'], (a) => `${num(a / 10)} km/h`);
  row('Dienstgipfelhöhe', ['ceiling_m'], (a) => `${num(a)} m`);
  row('Reichweite (Straße)', ['range_road_km'], (a) => `${num(a)} km`);
  row('Passagiere', ['passengers'], (c) => `${c}`);
  row('Seitenrichtbereich', ['gun_traverse_deg'], (a) => `${a}° (Kasematte)`);
  row('Brücke', ['bridge_length_mm', 'bridge_load_kg'], (l, kg) => `${meters(l)} m, bis ${num(kg / 1000, 1)} t`);
  row('Trefferpunkte', ['hp'], (a) => num(a));
  row('Panzerung', ['armor_mm'], (a) => `${a} mm rundum`);
  row('Bauzeit', ['build_s'], (a) => `${a} s`);
  row('Baubereich', ['build_radius_m'], (a) => `${a} m Umkreis`);
  row('Einkommen pro Minute', ['income_per_min'], (a) => `${a[0]} Treibstoff, ${a[1]} Munition, ${a[2]} Personal`);
  row('Höchstzahl', ['max_count'], (a) => (a ? `${a}` : 'unbegrenzt'));
  row('Voraussetzung', ['requires'], (a) => a.join(', ') || '–');
  row('Produziert', ['produces'], (a) => a || '–');

  const armor = armorDiagram(e, unsure);
  const armorFields = ['front', 'side', 'rear'].flatMap((s) => [`armor_hull_${s}_mm`, `armor_turret_${s}_mm`]);
  if (armor) {
    armorFields.concat('armor_hull_front_deg').forEach((f) => used.add(f));
  } else {
    // No hull (towed guns): the gun shield and whatever else exists as plain rows.
    const names = { hull: 'Wanne', turret: 'Schild / Aufbau', front: 'Front', side: 'Seite', rear: 'Heck' };
    for (const field of armorFields) {
      const [, part, face] = field.split('_');
      row(`Panzerung ${names[part]} ${names[face]}`, [field], (a) => `${a} mm`);
    }
  }

  // Whatever the page does not know yet still shows up, so new catalog fields are not lost.
  for (const [key, value] of Object.entries(e)) {
    if (used.has(key)) continue;
    rows.push(h('dt', null, key), h('dd', null, Array.isArray(value) ? value.join(', ') || '–' : String(value)));
  }

  const cost = state.costs.get(e.id);
  const costTile = (kind, label, value, format = (v) => num(Math.round(v))) => {
    const figure = h('b', null);
    countUp(figure, value, format, 900);
    return h('div', { class: kind }, icon(ICONS[kind], 'ico'), figure, h('span', null, label));
  };
  const section = (title, ...content) => (content.some(Boolean) ? h('section', null, h('h3', null, title), ...content) : null);
  const sources = e.source || [];
  const canDuel = duelPool().some((entry) => entry.id === e.id);
  const parts = [
    h('header', { 'data-faction': e.faction },
      h('p', { class: 'eyebrow' }, icon(MARKS[e.faction], 'mark'), `${categoryName(e)} · ${FACTION_NAMES[e.faction]}`),
      h('h2', null, e.name_de),
      e.name_en !== e.name_de ? h('p', { class: 'en' }, e.name_en) : null,
      h('div', { class: 'tags' },
        e.designation ? h('span', null, e.designation) : null,
        e.tier && e.faction !== 'neutral' ? h('span', null, e.tier === 1 ? 'Kerneinheit' : 'Stufe 2') : null,
        h('span', { class: 'id' }, e.id)),
      e.year_from != null && e.year_to != null ? timeline(e) : null),
    cost ? section(cost.builtIn && cost.builtIn !== '–' ? `Kosten im Spiel · ${cost.builtIn}` : 'Kosten im Spiel',
      h('div', { class: 'cost' },
        costTile('fuel', 'Treibstoff', cost.fuel), costTile('ammo', 'Munition', cost.ammo),
        costTile('manpower', 'Personal', cost.manpower), costTile('time', 'Bauzeit', cost.time, (v) => `${num(v, 1)} s`))) : null,
    section(e.category === 'base' ? 'Spielwerte' : 'Reale Werte',
      tiles.length ? h('div', { class: 'tiles' }, tiles) : null,
      rows.length ? h('dl', null, rows) : null),
    armor ? section('Panzerung', armor) : null,
    (e.weapon || []).length ? section(unsure.has('weapon') ? 'Bewaffnung · unsicher' : 'Bewaffnung', weaponList(e)) : null,
    e.note ? section('Anmerkung', h('p', { class: 'note' }, e.note)) : null,
    sources.length ? h('details', { class: 'sources' },
      h('summary', null, `Quellen (${sources.length})`),
      h('ul', null, sources.map((s) => h('li', null,
        /^https?:\/\//.test(s.ref) ? h('a', { href: s.ref, target: '_blank', rel: 'noopener' }, s.ref) : s.ref,
        s.covers ? ` – ${s.covers}` : null)))) : null,
    canDuel ? h('button', {
      class: 'btn',
      onclick: () => {
        if (duel.ids[1] === e.id) duel.ids.reverse();
        duel.ids[0] = e.id;
        renderDuel();
        dialog.close();
        $('duell').scrollIntoView();
      },
    }, 'Im Duell vergleichen') : null,
  ].filter(Boolean);
  parts.forEach((part, i) => part.style.setProperty('--i', i));
  $('sheet').replaceChildren(...parts);
  document.querySelector('.sheet-wrap').scrollTop = 0;
}

// --- start --------------------------------------------------------------------------

async function start() {
  document.documentElement.dataset.biome = state.biome;
  renderBiomes();
  reveal();
  $('status').textContent = 'Lade Katalog …';
  const look = await loadLook(text);
  await loadData();
  if ($('status').textContent === 'Lade Katalog …') $('status').textContent = '';
  const ctx = { look, text, url };

  viewer = new Viewer($('view'), ctx, { labels: $('dims') });
  viewer.gridColor = 0xffffff;
  viewer.margin = 1.3; // room for the dimension lines
  const offscreen = document.createElement('canvas');
  offscreen.width = 400;
  offscreen.height = 300;
  snapper = new Viewer(offscreen, ctx, { interactive: false });
  setupHero(ctx);
  syncLoops();

  hero.entries = ARMIES.map((faction, i) => {
    const wanted = state.byId.get(FEATURED[i]);
    return wanted?.model ? wanted
      : state.entries.find((e) => e.faction === faction && e.model && e.category.startsWith('tank_'));
  }).filter(Boolean);
  if (hero.entries.length) heroShow(Math.max(hero.entries.findIndex((e) => e.faction === state.faction), 0));
  renderKpis();
  renderArmies();
  renderDuel();
  $('duel').addEventListener('reveal', renderDuel);

  $('search').addEventListener('input', (event) => { state.query = event.target.value; renderList(); });
  $('spin').addEventListener('change', (event) => { viewer.controls.autoRotate = event.target.checked; });
  $('measure').addEventListener('change', () => viewer.setDimensions(dimensions(state.byId.get(state.selected))));
  $('variant').addEventListener('change', (event) => viewer.setVariant(Number(event.target.value), true));
  $('prev').addEventListener('click', () => step(-1));
  $('next').addEventListener('click', () => step(1));
  $('close').addEventListener('click', () => dialog.close());
  dialog.addEventListener('close', closeDossier);
  dialog.addEventListener('keydown', (event) => {
    if (event.target.closest('input, select, summary')) return;
    if (event.key === 'ArrowLeft') step(-1);
    if (event.key === 'ArrowRight') step(1);
  });

  const fromHash = () => {
    const wanted = decodeURIComponent(location.hash.slice(1));
    if (!state.byId.has(wanted)) return false;
    if (!state.shown.includes(wanted)) pickFaction(state.byId.get(wanted).faction);
    openDossier(wanted);
    return true;
  };
  renderControls();
  renderList();
  reveal();
  fromHash();
  window.addEventListener('hashchange', fromHash);
}

start().catch((error) => {
  console.error(error);
  $('status').textContent = `Fehler: ${error.message}`;
});
