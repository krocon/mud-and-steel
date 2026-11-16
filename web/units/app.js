// Unit browser: lists every catalog entry with picture, 3D model and values.
// Reads the repository files directly (no build step): data/catalog/*.toml,
// data/economy.toml, the camouflage palettes and the models under godot/assets/.
// Start it with `just units-web` (serves the repository root).

import { parse } from 'smol-toml';
import { BIOMES, Viewer, loadLook } from './viewer.js';

const ROOT = new URL('../../', import.meta.url);
const url = (path) => new URL(path, ROOT).href;
async function text(path) {
  const response = await fetch(url(path));
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return response.text();
}

const FACTIONS = [['de', 'Deutschland'], ['su', 'Sowjetunion'], ['us', 'USA'], ['neutral', 'Neutral']];
const BIOME_NAMES = { forest: 'Wald', desert: 'Wüste', winter: 'Winter' };
const GROUPS = [
  ['inf', 'Infanterie'], ['veh', 'Fahrzeuge'], ['gun', 'Geschütze'], ['air', 'Flugzeuge'],
  ['fort', 'Befestigungen'], ['base', 'Basisgebäude'], ['env', 'Umgebung'],
];
const CATEGORY_NAMES = {
  inf_rifle: 'Schützengruppe', inf_lmg: 'lMG-Trupp', inf_hmg: 'sMG-Trupp',
  inf_mortar_light: 'leichter Granatwerfer', inf_mortar_medium: 'mittlerer Granatwerfer',
  inf_recon: 'Spähtrupp', inf_sniper: 'Scharfschützen', inf_medic: 'Sanitäter',
  inf_engineer: 'Pioniere', inf_at: 'Panzerabwehrtrupp', inf_flamethrower: 'Flammenwerfer',
  inf_command: 'Führungstrupp',
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

const state = {
  entries: [],
  byId: new Map(),
  costs: new Map(),
  faction: 'de',
  group: 'all',
  query: '',
  biome: 'forest',
  selected: null,
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
let viewer = null;
let snapper = null;
let ctx = null;

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
  // Prices come from the simulation (`cli economy-table`), written by tools/units_web.py.
  try {
    const table = await text('build/units-web/economy.md');
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
    $('status').textContent = 'Keine Preise: build/units-web/economy.md fehlt (Seite mit `just units-web` starten).';
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

/** Game thumbnail (forest look) if there is one, else a rendered snapshot of the model. */
function fillPicture(pic) {
  const entry = state.byId.get(pic.dataset.id);
  const biome = state.biome;
  const img = h('img', { alt: '' });
  const render = () => {
    if (!entry.model) { pic.textContent = 'kein Modell'; return; }
    snapshot(entry).then((src) => {
      if (biome !== state.biome) return;
      img.onerror = null;
      img.src = src;
      pic.replaceChildren(img);
    }, () => { pic.textContent = 'Modell fehlt'; });
  };
  if (biome === 'forest' || !entry.model) {
    img.onerror = render;
    img.onload = () => pic.replaceChildren(img);
    img.src = url(`godot/assets/thumbnails/${entry.id}.png`);
  } else {
    render();
  }
}

// --- list ---------------------------------------------------------------------------

function searchText(entry) {
  return [entry.id, entry.name_de, entry.name_en, entry.designation, CATEGORY_NAMES[entry.category],
    ...(entry.weapon || []).map((w) => w.name)].join(' ').toLowerCase();
}

function renderControls() {
  $('factions').replaceChildren(...FACTIONS.map(([id, name]) => h('button', {
    'aria-pressed': String(state.faction === id),
    onclick: () => { state.faction = id; state.group = 'all'; remember(); renderControls(); renderList(); },
  }, name)));
  $('biomes').replaceChildren(...BIOMES.map((id) => h('button', {
    'aria-pressed': String(state.biome === id),
    onclick: () => { state.biome = id; remember(); renderControls(); renderList(); showModel(true); },
  }, BIOME_NAMES[id])));
  const present = new Set(state.entries.filter((e) => e.faction === state.faction).map((e) => groupOf(e.category)));
  const chips = [['all', 'Alle'], ...GROUPS.filter(([id]) => present.has(id))];
  $('groups').replaceChildren(...chips.map(([id, name]) => h('button', {
    'aria-pressed': String(state.group === id),
    onclick: () => { state.group = id; renderControls(); renderList(); },
  }, name)));
}

function renderList() {
  observer.disconnect();
  const query = state.query.trim().toLowerCase();
  const shown = state.entries.filter((e) => (query
    ? searchText(e).includes(query)
    : e.faction === state.faction && (state.group === 'all' || groupOf(e.category) === state.group)));
  const sections = [];
  for (const [group, title] of GROUPS) {
    const members = shown.filter((e) => groupOf(e.category) === group);
    if (!members.length) continue;
    sections.push(h('h2', null, `${title} (${members.length})`), h('div', { class: 'cards' }, members.map((e) => {
      const pic = h('div', { class: 'pic', 'data-id': e.id });
      observer.observe(pic);
      return h('button', {
        class: 'card', 'data-id': e.id, 'aria-current': String(e.id === state.selected),
        onclick: () => select(e.id),
      }, pic, h('div', { class: 'name' }, e.name_de), h('div', { class: 'sub' }, CATEGORY_NAMES[e.category] || e.category));
    })));
  }
  $('list').replaceChildren(...(sections.length ? sections : [h('p', { class: 'empty' }, 'Keine Treffer.')]));
}

// --- detail -------------------------------------------------------------------------

function select(id) {
  if (!state.byId.has(id)) return;
  state.selected = id;
  history.replaceState(null, '', `#${id}`);
  for (const card of document.querySelectorAll('.card')) {
    card.setAttribute('aria-current', String(card.dataset.id === id));
  }
  renderSheet();
  showModel(false);
}

async function showModel(keepView) {
  const entry = state.byId.get(state.selected);
  const [canvas, still, msg, variant] = [$('view'), $('still'), $('stage-msg'), $('variant')];
  variant.hidden = true;
  msg.hidden = true;
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
  try {
    const variants = await viewer.show(entry, state.biome, { keepView });
    if (!variants || !variants.length) return;
    const current = keepView ? variant.value : '';
    variant.replaceChildren(...variants.map((name, i) => h('option', { value: String(i) }, name)));
    variant.value = current && variants[current] ? current : String(Math.max(variants.indexOf('Intact'), 0));
    viewer.setVariant(Number(variant.value), keepView);
    variant.hidden = false;
  } catch (error) {
    console.error(error);
    viewer.clear();
    msg.hidden = false;
    msg.textContent = `Modell konnte nicht geladen werden (${entry.model})`;
  }
}

function renderSheet() {
  const e = state.byId.get(state.selected);
  const unsure = new Set(e.uncertain || []);
  const used = new Set(['id', 'faction', 'category', 'name_de', 'name_en', 'designation', 'model',
    'weapon', 'source', 'uncertain', 'note', 'tier']);
  const rows = [];
  /** Adds a row when all its fields exist; marks uncertain values. */
  const row = (label, fields, format) => {
    fields.forEach((f) => used.add(f));
    if (fields.some((f) => e[f] == null)) return;
    const mark = fields.some((f) => unsure.has(f))
      ? h('span', { class: 'unsure', title: 'Wert laut Katalog unsicher' }, ' ?') : null;
    rows.push(h('dt', null, label), h('dd', null, format(...fields.map((f) => e[f])), mark));
  };
  row('Im Dienst', ['year_from', 'year_to'], (a, b) => `${a}–${b}`);
  row('Länge × Breite × Höhe', ['length_mm', 'width_mm', 'height_mm'], (l, w, t) => `${meters(l)} × ${meters(w)} × ${meters(t)} m`);
  row('Länge × Spannweite × Höhe', ['length_mm', 'wingspan_mm', 'height_mm'], (l, w, t) => `${meters(l)} × ${meters(w)} × ${meters(t)} m`);
  row('Wannenlänge', ['length_hull_mm'], (l) => `${meters(l)} m`);
  row('Masse', ['mass_kg'], (kg) => (kg >= 1000 ? `${num(kg / 1000, 1)} t` : `${num(kg)} kg`));
  row('Besatzung', ['crew'], (c) => `${c}`);
  row('Passagiere', ['passengers'], (c) => `${c}`);
  row('Truppstärke', ['squad_size'], (c) => `${c} Mann`);
  row('Straße / Gelände', ['speed_road_kmh_x10', 'speed_offroad_kmh_x10'], (a, b) => `${num(a / 10, 1)} / ${num(b / 10, 1)} km/h`);
  row('Im Wasser', ['speed_water_kmh_x10'], (a) => `${num(a / 10, 1)} km/h`);
  row('Höchstgeschwindigkeit', ['speed_max_kmh_x10'], (a) => `${num(a / 10)} km/h`);
  row('Reisegeschwindigkeit', ['speed_cruise_kmh_x10'], (a) => `${num(a / 10)} km/h`);
  row('Dienstgipfelhöhe', ['ceiling_m'], (a) => `${num(a)} m`);
  row('Reichweite (Straße)', ['range_road_km'], (a) => `${num(a)} km`);
  row('Motorleistung', ['power_hp'], (a) => `${num(a)} PS${e.mass_kg ? ` (${num(a / (e.mass_kg / 1000), 1)} PS/t)` : ''}`);
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

  const armorFields = ['front', 'side', 'rear'].flatMap((s) => [`armor_hull_${s}_mm`, `armor_turret_${s}_mm`]);
  armorFields.concat('armor_hull_front_deg').forEach((f) => used.add(f));
  const armorCell = (field) => h('td', null, e[field] != null ? `${e[field]} mm` : '–',
    unsure.has(field) ? h('span', { class: 'unsure', title: 'Wert laut Katalog unsicher' }, ' ?') : null);
  const armor = armorFields.some((f) => e[f] != null) && h('div', { class: 'tablewrap' }, h('table', null,
    h('tr', null, h('th'), h('th', null, 'Front'), h('th', null, 'Seite'), h('th', null, 'Heck')),
    h('tr', null, h('td', null, 'Wanne', e.armor_hull_front_deg != null ? h('small', null, `Front ${e.armor_hull_front_deg}° geneigt`) : null),
      armorCell('armor_hull_front_mm'), armorCell('armor_hull_side_mm'), armorCell('armor_hull_rear_mm')),
    h('tr', null, h('td', null, 'Turm / Aufbau'),
      armorCell('armor_turret_front_mm'), armorCell('armor_turret_side_mm'), armorCell('armor_turret_rear_mm'))));

  const weapons = (e.weapon || []).length > 0 && h('div', { class: 'tablewrap' }, h('table', null,
    h('tr', null, ...['Waffe', 'Kaliber', 'Kadenz', 'v₀', 'Reichweite', 'Munition'].map((t) => h('th', null, t))),
    e.weapon.map((w) => h('tr', null,
      h('td', null, `${w.count > 1 ? `${w.count}× ` : ''}${w.name}${w.aa ? ' (Fla)' : ''}`, w.ammo ? h('small', null, w.ammo) : null),
      h('td', null, w.caliber_mm_x10 != null ? `${num(w.caliber_mm_x10 / 10, 1)} mm` : w.bomb_kg != null ? `${w.bomb_kg} kg` : '–'),
      h('td', null, w.rate_rpm != null ? `${num(w.rate_rpm)}/min` : '–'),
      h('td', null, w.muzzle_velocity_mps != null ? `${num(w.muzzle_velocity_mps)} m/s` : '–'),
      h('td', null, w.range_effective_m != null ? `${num(w.range_effective_m)} m` : '–',
        w.range_aa_m != null ? h('small', null, `Fla ${num(w.range_aa_m)} m`) : null),
      h('td', null, w.ammo_carried != null ? num(w.ammo_carried) : '–')))));

  // Whatever the page does not know yet still shows up, so new catalog fields are not lost.
  for (const [key, value] of Object.entries(e)) {
    if (used.has(key)) continue;
    rows.push(h('dt', null, key), h('dd', null, Array.isArray(value) ? value.join(', ') || '–' : String(value)));
  }

  const cost = state.costs.get(e.id);
  const costBox = (label, value) => h('div', null, h('b', null, value), h('span', null, label));
  const parts = [
    h('h2', null, e.name_de),
    e.name_en !== e.name_de ? h('p', { class: 'en' }, e.name_en) : null,
    h('div', { class: 'tags' },
      h('span', { class: 'tag' }, CATEGORY_NAMES[e.category] || e.category),
      e.designation ? h('span', { class: 'tag' }, e.designation) : null,
      e.tier && e.faction !== 'neutral' ? h('span', { class: 'tag' }, e.tier === 1 ? 'Kerneinheit' : 'Stufe 2') : null,
      h('span', { class: 'tag' }, e.id)),
    cost ? h('h3', null, cost.builtIn && cost.builtIn !== '–' ? `Kosten im Spiel (${cost.builtIn})` : 'Kosten im Spiel') : null,
    cost ? h('div', { class: 'cost' }, costBox('Treibstoff', num(cost.fuel)), costBox('Munition', num(cost.ammo)),
      costBox('Personal', num(cost.manpower)), costBox('Bauzeit', `${num(cost.time, 1)} s`)) : null,
    rows.length ? h('h3', null, e.category === 'base' ? 'Spielwerte' : 'Reale Werte') : null,
    rows.length ? h('dl', null, rows) : null,
    armor ? h('h3', null, 'Panzerung') : null, armor || null,
    weapons ? h('h3', null, unsure.has('weapon') ? 'Bewaffnung (unsicher)' : 'Bewaffnung') : null, weapons || null,
    e.note ? h('h3', null, 'Anmerkung') : null,
    e.note ? h('p', { class: 'note' }, e.note) : null,
    (e.source || []).length ? h('h3', null, 'Quellen') : null,
    (e.source || []).length ? h('ul', { class: 'sources' }, e.source.map((s) => h('li', null,
      /^https?:\/\//.test(s.ref) ? h('a', { href: s.ref, target: '_blank', rel: 'noopener' }, s.ref) : s.ref,
      s.covers ? ` – ${s.covers}` : null))) : null,
  ];
  $('sheet').replaceChildren(...parts.filter(Boolean));
  $('detail').scrollTop = 0;
}

// --- start --------------------------------------------------------------------------

async function main() {
  // The detail panel sticks below the fixed header, whose height depends on wrapping.
  const header = document.querySelector('header');
  const measure = () => document.documentElement.style.setProperty('--header-h', `${header.offsetHeight}px`);
  new ResizeObserver(measure).observe(header);
  $('status').textContent = 'Lade Katalog …';
  const look = await loadLook(text);
  await loadData();
  if ($('status').textContent === 'Lade Katalog …') $('status').textContent = '';
  ctx = { look, text, url };
  viewer = new Viewer($('view'), ctx);
  const offscreen = document.createElement('canvas');
  offscreen.width = 400;
  offscreen.height = 300;
  snapper = new Viewer(offscreen, ctx, { interactive: false });

  $('search').addEventListener('input', (event) => { state.query = event.target.value; renderList(); });
  $('spin').addEventListener('change', (event) => { viewer.controls.autoRotate = event.target.checked; });
  $('variant').addEventListener('change', (event) => viewer.setVariant(Number(event.target.value), true));

  const wanted = decodeURIComponent(location.hash.slice(1));
  if (state.byId.has(wanted)) state.faction = state.byId.get(wanted).faction;
  renderControls();
  measure();
  renderList();
  const first = state.byId.has(wanted) ? wanted
    : (state.entries.find((e) => e.faction === state.faction && e.tier === 1 && groupOf(e.category) === 'veh')
      || state.entries.find((e) => e.faction === state.faction)).id;
  select(first);
}

main().catch((error) => {
  console.error(error);
  $('status').textContent = `Fehler: ${error.message}`;
});
