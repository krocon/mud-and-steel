// Data sheet blocks of prompt 065: the five ratings, the game values and the technical
// data. Display only: the game values come from the simulation (`cli unit-stats` ->
// stats.json), the technical data from data/vehicles/*.json. Nothing here computes a
// game value; numbers are only divided by their `_x10` and given a unit.

/** Rating keys in display order with their German names. */
export const RATINGS = [
  ['speed', 'Tempo'],
  ['protection', 'Schutz'],
  ['anti_vehicle', 'Gegen Fahrzeuge'],
  ['anti_infantry', 'Gegen Infanterie'],
  ['anti_structure', 'Gegen Bauten'],
];

const WEAPON_CLASS = {
  rifle: 'Gewehr', machine_gun: 'MG', gun_37mm: '3,7 cm', gun_57mm: '5,7 cm',
  gun_75mm: '7,5 cm', gun_88mm: '8,8 cm', rocket_launcher: 'Panzerschreck',
};

/** The five ratings as bars with their number. */
export function ratingBars({ h }, card) {
  const ratings = card?.ratings;
  if (!ratings) return null;
  return h('div', { class: 'ratings' }, RATINGS.filter(([key]) => ratings[key] != null).map(([key, label]) => h('div', {
    class: 'rating', role: 'meter', 'aria-label': label, 'aria-valuemin': 0, 'aria-valuemax': 100, 'aria-valuenow': ratings[key],
  }, h('span', null, label), h('i', null, h('b', { style: `--w:${ratings[key]}%` })), h('em', null, String(ratings[key])))));
}

// --- game values --------------------------------------------------------------------

function list(h, rows) {
  const shown = rows.filter(Boolean);
  return shown.length ? h('dl', null, shown.flatMap(([label, value]) => [h('dt', null, label), h('dd', null, value)])) : null;
}

function table(h, head, rows, cls = '') {
  if (!rows.length) return null;
  return h('div', { class: 'scroll' }, h('table', { class: `grid ${cls}` },
    h('thead', null, h('tr', null, head.map((cell) => h('th', null, cell)))),
    h('tbody', null, rows.map((row) => h('tr', null, row.map((cell, i) => h(i ? 'td' : 'th', cell?.cls ? { class: cell.cls } : null, cell?.text ?? cell)))))));
}

/** The block "Spielwerte": every group the card has. */
export function gameValues(ctx, entry, card, reference) {
  const { h, num, name } = ctx;
  if (!card) return [];
  const x10 = (v, digits = 1) => num(v / 10, digits);
  const group = (title, ...content) => (content.some(Boolean) ? h('div', { class: 'gv' }, h('h4', null, title), ...content) : null);
  const both = (real, game) => `${real} real · ${game} im Spiel`;
  const reach = (m, range) => (m < 0 ? { text: '–', cls: 'no' } : { text: m >= range ? 'immer' : `bis ${num(m)} m`, cls: 'yes' });
  const out = [];

  const mv = card.movement;
  if (mv) {
    const speed = (real, game) => (real == null ? `${x10(game)} m/s im Spiel` : both(`${x10(real, real % 10 ? 1 : 0)} km/h`, `${x10(game)} m/s`));
    out.push(group('Bewegung', list(h, [
      ['Straße', speed(mv.road_real_kmh_x10 ?? mv.max_real_kmh_x10, mv.road_game_mps_x10)],
      mv.offroad_real_kmh_x10 != null && ['Gelände', speed(mv.offroad_real_kmh_x10, mv.offroad_game_mps_x10)],
      mv.cruise_real_kmh_x10 != null && ['Reiseflug', `${x10(mv.cruise_real_kmh_x10, 0)} km/h real`],
      mv.water_real_kmh_x10 != null && ['Im Wasser', `${x10(mv.water_real_kmh_x10)} km/h real`],
      mv.hull_turn_game_deg_per_s != null && ['Wanne dreht', `${mv.hull_turn_game_deg_per_s}°/s`],
      mv.turret_turn_game_deg_per_s != null && ['Turm dreht', `${mv.turret_turn_game_deg_per_s}°/s`],
      mv.gun_traverse_deg != null && ['Seitenrichtbereich', `${mv.gun_traverse_deg}° (Kasematte)`],
      mv.mud_speed_pct != null && ['Im Schlamm', `${mv.mud_speed_pct} % des Tempos`],
      mv.snow_speed_pct != null && ['Im Tiefschnee', `${mv.snow_speed_pct} % des Tempos`],
      mv.max_grade_pct != null && ['Steigung', `bis ${mv.max_grade_pct} %, dort ${mv.steepest_grade_speed_pct} % des Tempos`],
      mv.crushes && ['Überrollt', mv.crushes.length ? mv.crushes.map(name).join(', ') : 'nichts'],
    ])));
  }

  const pr = card.protection;
  if (pr) {
    const faces = pr.armor_turret_mm
      ? [['hull_front', 'Wanne Front'], ['hull_side', 'Seite'], ['hull_rear', 'Heck'], ['turret_front', 'Turm Front'], ['turret_side', 'Seite'], ['turret_rear', 'Heck']]
      : [['hull_front', 'Front'], ['hull_side', 'Seite'], ['hull_rear', 'Heck']];
    const armored = pr.armor_hull_mm != null;
    const rows = pr.versus.map((v) => [
      `${WEAPON_CLASS[v.weapon] || v.weapon}`,
      ...(armored ? faces.map(([key]) => reach(v[`${key}_real_m`], v.range_real_m)) : []),
      v.hits_per_soldier != null ? `${v.hits_per_soldier} je Soldat` : String(v.hits_to_destroy),
    ]);
    out.push(group('Empfindlichkeit',
      list(h, [
        ['Trefferpunkte', pr.soldiers != null ? `${num(pr.hp)} (${pr.soldiers} × ${pr.soldier_hp})` : num(pr.hp)],
        armored && ['Panzerung Wanne', `${pr.armor_hull_mm.join(' / ')} mm (Front / Seite / Heck)`],
        pr.armor_turret_mm && ['Panzerung Turm', `${pr.armor_turret_mm.join(' / ')} mm`],
        armored && ['Bug wirksam', `${pr.hull_front_effective_mm} mm bei ${pr.hull_front_slope_deg}° Neigung`],
        pr.unarmored && ['Panzerung', 'keine: jeder Treffer wirkt'],
        pr.hurt_by_fragments != null && ['Granatsplitter', pr.hurt_by_fragments ? 'verwundbar' : 'nur direkter Treffer'],
      ]),
      table(h, ['Vergleichswaffe', ...(armored ? faces.map(([, label]) => label) : []), 'Treffer'], rows, 'matrix'),
      armored ? h('p', { class: 'fine' }, 'Bis zu welcher realen Entfernung die Waffe die Seite durchschlägt (ohne Streuung, senkrechter Aufprall); „–“ = nie, „immer“ = auf ihre ganze Reichweite. Treffer = durchschlagende Treffer bis zum Ausfall.') : null));
  }

  const weapons = card.weapons || [];
  const av = card.against_vehicles;
  if (weapons.length) {
    const distances = reference?.distances_real_m || [];
    out.push(group('Waffen im Spiel', table(h,
      ['Waffe', 'Reichweite', 'Schuss/min', ...distances.map((d) => `${num(d)} m`)],
      weapons.map((w) => [
        `${w.count > 1 ? `${w.count}× ` : ''}${w.name}`,
        `${num(w.range_game_m)} m (${num(w.range_real_m)} m real)`,
        x10(w.shots_per_game_min_x10, w.shots_per_game_min_x10 % 10 ? 1 : 0),
        ...w.penetration_mm.map((mm, i) => ({ text: `${mm} mm`, cls: distances[i] > w.range_real_m ? 'far' : '' })),
      ])),
    h('p', { class: 'fine' }, 'Durchschlag gegen eine senkrechte Platte nach realer Entfernung; grau = jenseits der Reichweite der Waffe. Hohlladungen verlieren nichts.')));
  }
  if (av) {
    out.push(group('Gegen Fahrzeuge',
      list(h, [
        ['Stärkste Waffe', av.weapon],
        ['Durchschlag', `${av.penetration_mm} mm auf ${num(av.distance_real_m)} m`],
        ['Treffer bis zum Ausfall', String(av.hits_to_destroy)],
      ]),
      table(h, ['Ziel', 'Front', 'Seite', 'Heck'], av.targets.map((t) => [
        name(t.id), ...['hull_front', 'hull_side', 'hull_rear'].map((key) => ({
          text: t[key] ? 'ja' : (t[`${key}_real_m`] >= 0 ? `erst ab ${num(t[`${key}_real_m`])} m` : 'nein'),
          cls: t[key] ? 'yes' : 'no',
        })),
      ]), 'matrix'),
      h('p', { class: 'fine' }, `Durchschlägt die Wanne des Ziels auf ${num(av.distance_real_m)} m real.`)));
  }

  const ai = card.against_infantry;
  if (ai) {
    out.push(group('Gegen Infanterie', list(h, [
      ai.damage_per_hit != null && ['Schaden je Treffer', `${num(ai.damage_per_hit)} (Soldat: ${ai.soldier_hp} TP)`],
      ai.hits_per_soldier != null && ['Treffer je Soldat', String(ai.hits_per_soldier)],
      ai.automatic_weapons != null && ['Automatische Waffen', `${ai.automatic_weapons}, zusammen ${x10(ai.automatic_shots_per_game_min_x10, 0)} Schuss/min`],
      ai.blast_radius_game_m_x10 != null && ['Splitterradius', `${x10(ai.blast_radius_game_m_x10)} m`],
      ['Soldaten je Minute', `${x10(ai.soldiers_per_game_min_x10, 0)} (wenn jeder Schuss trifft)`],
    ])));
  }

  const as = card.against_structures;
  if (as) {
    out.push(group('Gegen Mauern und Gebäude',
      as.effective ? list(h, [['Schaden je Treffer', num(as.damage_per_hit)]])
        : h('p', { class: 'fine' }, `Wirkungslos: unter dem Mindestkaliber von ${as.min_caliber_mm} mm.`),
      as.effective ? table(h, ['Objekt', 'Trefferpunkte', 'Treffer bis zum Einsturz'],
        as.objects.map((o) => [name(o.id), num(o.hp), String(o.hits)])) : null));
  }

  const ind = card.indirect;
  if (ind) {
    out.push(group('Indirektes Feuer', list(h, [
      ['Waffe', ind.name],
      ['Reichweite', `${num(ind.min_range_game_m)}–${num(ind.range_game_m)} m im Spiel (${num(ind.range_real_m)} m real)`],
      ['Aufbauzeit', `${x10(ind.setup_game_s_x10)} s`],
      ['Schuss/min', x10(ind.shots_per_game_min_x10, ind.shots_per_game_min_x10 % 10 ? 1 : 0)],
      ['Splitterradius', `${x10(ind.blast_radius_game_m_x10)} m`],
      ['Volltreffer', `${num(ind.direct_hit_damage)} Schaden, durchschlägt Dächer bis ${ind.direct_hit_roof_mm} mm`],
      ['Nebelgranaten', ind.smoke ? 'ja' : 'nein'],
    ])));
  }

  const other = card.other;
  if (other) {
    out.push(group('Sonstiges', list(h, [
      other.sight_game_m != null && ['Sichtweite', `${num(other.sight_game_m)} m im Spiel (${num(other.sight_real_m)} m real)`],
      other.ammo_salvos != null && ['Munition', `${num(other.ammo_salvos)} Salven${other.ammo_game_min_x10 != null ? `, ${x10(other.ammo_game_min_x10)} min Dauerfeuer` : ''}`],
      other.fuel_game_m != null && ['Kraftstoff', `${num(other.fuel_game_m)} m im Spiel (${num(other.fuel_real_km)} km real)`],
      other.passengers != null && ['Sitzplätze', String(other.passengers)],
    ])));
  }

  const object = card.object;
  if (object) {
    out.push(group('Zerstörbarkeit', list(h, [
      ['Trefferpunkte', object.hp ? num(object.hp) : 'nicht zu zerschießen'],
      object.hp > 0 && ['Mindestkaliber', `${object.min_caliber_mm} mm`],
      object.crush_min_mass_kg != null && ['Überrollbar ab', `${num(object.crush_min_mass_kg / 1000, 1)} t Fahrzeugmasse`],
    ])));
  }
  const shown = out.filter(Boolean);
  if (shown.length) {
    shown.push(h('p', { class: 'fine' }, 'Grundwerte bei trockenem Boden, ohne Erfahrung und Wetter. Regeln und damit Spielwerte sind vorläufig und ändern sich mit dem Balancing.'));
  }
  return shown;
}

// --- technical data -----------------------------------------------------------------

const SECTION_NAMES = { general: 'Allgemein', firepower: 'Feuerkraft', mobility: 'Beweglichkeit' };
const FIELD_NAMES = {
  designation: 'Bezeichnung', other_designations: 'Weitere Bezeichnungen', manufacturer: 'Hersteller',
  production_quantity: 'Stückzahl', production_period: 'Bauzeitraum', crew: 'Besatzung',
  length_mm: 'Länge über alles', length_hull_mm: 'Wannenlänge', width_mm: 'Breite',
  width_skirts_mm: 'Breite mit Schürzen', height_mm: 'Höhe', barrel_overhang_mm: 'Rohrüberstand',
  mass_kg: 'Gefechtsgewicht', radio: 'Funkgerät',
  main_gun: 'Hauptbewaffnung', main_gun_caliber_mm_x10: 'Kaliber', main_gun_length_cal_x10: 'Rohrlänge',
  main_ammo: 'Munition Hauptwaffe', traverse_deg: 'Seitenrichtbereich', traverse_drive: 'Richtantrieb',
  elevation_min_deg_x10: 'Höhenrichtbereich von', elevation_max_deg_x10: 'Höhenrichtbereich bis',
  traverse_time_360_s: 'Turmdrehung 360°', sight: 'Zieloptik', secondary: 'Nebenbewaffnung',
  secondary_ammo: 'Munition Nebenwaffen',
  engine: 'Motor', engine_type: 'Bauart', displacement_cc: 'Hubraum', power_hp: 'Leistung',
  power_rpm: 'bei Drehzahl', power_to_weight_hp_per_t_x10: 'Leistungsgewicht', gearbox: 'Getriebe',
  gears_forward: 'Vorwärtsgänge', gears_reverse: 'Rückwärtsgänge', fuel: 'Kraftstoff',
  fuel_capacity_l: 'Tankinhalt', consumption_road_l_per_100km: 'Verbrauch Straße',
  consumption_offroad_l_per_100km: 'Verbrauch Gelände', range_road_km: 'Reichweite Straße',
  range_offroad_km: 'Reichweite Gelände', speed_road_kmh_x10: 'Geschwindigkeit Straße',
  speed_offroad_kmh_x10: 'Geschwindigkeit Gelände', speed_water_kmh_x10: 'Geschwindigkeit im Wasser',
  track_width_mm: 'Kettenbreite', track_ground_contact_mm: 'Kettenauflage', track_links: 'Kettenglieder je Kette',
  ground_pressure_psi_x10: 'Bodendruck (psi)', ground_pressure_kpa_x10: 'Bodendruck',
  ground_clearance_mm: 'Bodenfreiheit', turning_radius_mm: 'Wendekreis', gradient_deg: 'Steigfähigkeit',
  vertical_obstacle_mm: 'Kletterfähigkeit', fording_mm: 'Watfähigkeit', trench_crossing_mm: 'Grabenüberschreitung',
};
/** Database field -> catalog field holding the same value (the catalog wins). */
const IN_CATALOG = {
  crew: 'crew', length_mm: 'length_mm', length_hull_mm: 'length_hull_mm', width_mm: 'width_mm',
  height_mm: 'height_mm', mass_kg: 'mass_kg', power_hp: 'power_hp', range_road_km: 'range_road_km',
  speed_road_kmh_x10: 'speed_road_kmh_x10', speed_offroad_kmh_x10: 'speed_offroad_kmh_x10',
};
const PART_NAMES = {
  hull: 'Wanne', hull_upper: 'Wanne oben', hull_lower: 'Wanne unten', superstructure: 'Aufbau',
  turret: 'Turm', main_turret: 'Hauptturm', upper_turret: 'Oberer Turm', lower_turret: 'Unterer Turm',
  gun_turrets: 'Geschütztürme', mg_turrets: 'MG-Türme', mantlet: 'Blende', gun_shield: 'Schild',
};
const FACE_NAMES = { front: 'Front', side: 'Seite', rear: 'Heck', top_bottom: 'Dach / Boden' };

/** A database number with its unit, by the suffix of the field name. */
function formatField(num, key, value) {
  if (typeof value !== 'number') return String(value);
  if (key === 'turning_radius_mm' && value === 0) return 'dreht auf der Stelle';
  if (key.endsWith('_kmh_x10')) return `${num(value / 10, value % 10 ? 1 : 0)} km/h`;
  if (key.endsWith('_deg_x10')) return `${num(value / 10, value % 10 ? 1 : 0)}°`;
  if (key === 'main_gun_caliber_mm_x10') return `${num(value / 10, value % 10 ? 1 : 0)} mm`;
  if (key === 'main_gun_length_cal_x10') return `L/${num(value / 10, value % 10 ? 1 : 0)}`;
  if (key === 'ground_pressure_kpa_x10') return `${num(value / 10, 1)} kPa`;
  if (key === 'ground_pressure_psi_x10') return `${num(value / 10, 1)} psi`;
  if (key.endsWith('_hp_per_t_x10')) return `${num(value / 10, 1)} PS/t`;
  if (key.endsWith('_x10')) return num(value / 10, 1);
  if (key.endsWith('_l_per_100km')) return `${num(value)} l/100 km`;
  if (key.endsWith('_mm')) return value >= 1000 ? `${num(value / 1000, 2)} m` : `${num(value)} mm`;
  if (key.endsWith('_kg')) return value >= 1000 ? `${num(value / 1000, 1)} t` : `${num(value)} kg`;
  if (key.endsWith('_km')) return `${num(value)} km`;
  if (key.endsWith('_deg')) return `${value}°`;
  if (key.endsWith('_cc')) return `${num(value / 1000, 1)} l`;
  if (key.endsWith('_hp')) return `${num(value)} PS`;
  if (key.endsWith('_rpm')) return `${num(value)}/min`;
  if (key.endsWith('_l')) return `${num(value)} l`;
  if (key.endsWith('_s')) return `${num(value)} s`;
  return num(value);
}

function armorCell(cell) {
  if (!cell) return '–';
  const mm = cell.mm != null ? `${cell.mm} mm` : cell.mm_min != null ? `${cell.mm_min}–${cell.mm_max} mm` : null;
  const parts = [mm, cell.deg != null ? `${cell.deg}°` : null].filter(Boolean).join(' · ');
  // The original text says more than the numbers (curved plates, several angles).
  return cell.text ? { text: parts ? `${parts} *` : cell.text, title: cell.text } : parts;
}

/** The armor of every assembly and face. */
export function armorTable({ h }, record) {
  const parts = Object.entries(record.armor || {});
  if (!parts.length) return null;
  const faces = Object.keys(FACE_NAMES);
  const footnotes = [];
  const rows = parts.map(([part, cells]) => {
    const label = PART_NAMES[part] || part;
    if (cells.text && !faces.some((f) => cells[f])) return h('tr', null, h('th', null, label), h('td', { colspan: faces.length }, cells.text));
    return h('tr', null, h('th', null, label), faces.map((face) => {
      const cell = armorCell(cells[face]);
      if (cell.title) footnotes.push(`${label} ${FACE_NAMES[face]}: ${cell.title}`);
      return h('td', cell.title ? { title: cell.title } : null, cell.text ?? cell);
    }));
  });
  return h('div', null,
    h('div', { class: 'scroll' }, h('table', { class: 'grid' },
      h('thead', null, h('tr', null, h('th', null, 'Baugruppe'), faces.map((f) => h('th', null, FACE_NAMES[f])))),
      h('tbody', null, rows))),
    h('p', { class: 'fine' }, 'Dicke und Winkel gegen die Waagerechte (90° = senkrecht), wie in der Quelle.',
      footnotes.length ? ` * Wortlaut der Quelle: ${footnotes.join('; ')}.` : ''));
}

/** The block "Technische Daten" of a vehicle with a database entry. */
export function techData(ctx, entry, record) {
  const { h, num } = ctx;
  const out = [];
  let differs = false;
  for (const [section, title] of Object.entries(SECTION_NAMES)) {
    const data = record[section] || {};
    const texts = data.text || {};
    const rows = [];
    const keys = [...Object.keys(data).filter((k) => k !== 'text'), ...Object.keys(texts).filter((k) => !(k in data))];
    for (const key of keys) {
      const label = FIELD_NAMES[key] || key;
      const value = data[key];
      const ours = entry[IN_CATALOG[key]];
      let shown = value != null ? formatField(num, key, value) : `„${texts[key]}“`;
      if (value != null && texts[key]) shown += ` („${texts[key]}“)`;
      if (ours != null && typeof value === 'number') {
        // The catalog shows this value already; only a real difference gets a row.
        if (Math.abs(value - ours) <= Math.abs(ours) * 0.05) continue;
        differs = true;
        rows.push(h('dt', null, label), h('dd', null, shown, h('small', { class: 'differs' }, ` Katalog: ${formatField(num, key, ours)}`)));
        continue;
      }
      rows.push(h('dt', null, label), h('dd', null, shown));
    }
    if (rows.length) out.push(h('div', { class: 'gv' }, h('h4', null, title), h('dl', null, rows)));
  }
  const armor = armorTable(ctx, record);
  if (armor) out.push(h('div', { class: 'gv' }, h('h4', null, 'Panzerung'), armor));
  if (record.notes?.length) out.push(h('p', { class: 'fine' }, `Anmerkungen der Quelle: ${record.notes.join(' ')}`));
  if (differs) {
    out.push(h('p', { class: 'fine' }, 'Wo die Datenbank vom Katalog abweicht, gilt im Spiel der Katalog; der Grund steht in der Anmerkung unten.'));
  }
  const source = record.source || {};
  out.push(h('p', { class: 'fine' }, 'Quelle: ',
    h('a', { href: source.url, target: '_blank', rel: 'noopener' }, source.name || source.url),
    source.fetched ? `, abgerufen am ${source.fetched.split('-').reverse().join('.')}` : '',
    source.references?.length ? `; Literatur dort: ${source.references.join(', ')}` : '', '.'));
  return out;
}
