#!/usr/bin/env node
// Downloads vehicle reference material from onwar.com (WWII tanks).
//
// For every vehicle linked on the nation index pages it writes
//   <TARGET_DIR>/<NATION>/<family>/<variant>/data.json   technical data
//   <TARGET_DIR>/<NATION>/<family>/<variant>/<image>     drawings, if the page has any
//
// Usage:
//   node tools/onwar_download.mjs [--nation GER|US|RU] [--limit N] [--force]
//                                 [--delay MS] [--out DIR]
//
//   --nation  only one nation (default: all three)
//   --limit   only the first N vehicles per nation (for a quick test)
//   --force   download images again even if they already exist
//   --delay   pause between requests in milliseconds (default 400)
//   --out     write somewhere else than TARGET_DIR (for testing)
//
// Needs Node 18+ (built-in fetch), no dependencies. The target folder is in
// .gitignore: the drawings are copyrighted reference material, not project assets.

import { mkdir, writeFile, access } from 'node:fs/promises';
import path from 'node:path';

const TARGET_DIR = '/Users/marckronberg/RustroverProjects/mud-and-steel/docs/.vorlagen/fahrzeuge';

const NATIONS = [
  { code: 'GER', url: 'https://www.onwar.com/wwii/tanks/germany/index.html' },
  { code: 'US', url: 'https://www.onwar.com/wwii/tanks/usa/index.html' },
  { code: 'RU', url: 'https://www.onwar.com/wwii/tanks/ussr/index.html' },
];

// Folder names for index groups whose heading does not make a good folder name.
const FAMILY_OVERRIDES = {
  'PzKpfw 35(t), PzKpfw 38(t)': 'PzKpfw 35(t) and 38(t)',
};

// The server (mod_security) answers 406 without browser-like headers.
const HEADERS = {
  'User-Agent':
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36',
  Accept: 'text/html,application/xhtml+xml,image/*,*/*;q=0.8',
  'Accept-Language': 'en-US,en;q=0.9',
};

// Icons and layout pictures that are not vehicle images.
const IGNORED_IMAGES = /(pktanksm\.gif|logo\.png)$/i;

const opts = parseArgs(process.argv.slice(2));
const warnings = [];

// ---------------------------------------------------------------- network

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function fetchBytes(url) {
  let lastError;
  for (let attempt = 1; attempt <= 3; attempt++) {
    await sleep(opts.delay * attempt);
    try {
      const res = await fetch(url, { headers: HEADERS, redirect: 'follow' });
      if (res.ok) return Buffer.from(await res.arrayBuffer());
      lastError = new Error(`HTTP ${res.status}`);
      if (res.status === 404) break;
    } catch (err) {
      lastError = err;
    }
  }
  throw new Error(`${url}: ${lastError.message}`);
}

async function fetchHtml(url) {
  const bytes = await fetchBytes(url);
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return new TextDecoder('windows-1252').decode(bytes);
  }
}

// ---------------------------------------------------------------- html helpers

const ENTITIES = {
  amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ', shy: '',
  deg: '°', verbar: '|', copy: '©', frac12: '½', frac14: '¼', frac34: '¾',
  times: '×', plusmn: '±', ndash: '–', mdash: '—', sup2: '²', sup3: '³',
  auml: 'ä', ouml: 'ö', uuml: 'ü', Auml: 'Ä', Ouml: 'Ö', Uuml: 'Ü', szlig: 'ß',
  eacute: 'é', egrave: 'è', agrave: 'à', ccedil: 'ç',
};

function decodeEntities(text) {
  return text
    // The pages write soft hyphens inside words without a semicolon ("Desig&shynation").
    .replace(/&shy;?/g, '')
    .replace(/&#x([0-9a-f]+);?/gi, (_, hex) => String.fromCodePoint(parseInt(hex, 16)))
    .replace(/&#(\d+);?/g, (_, dec) => String.fromCodePoint(Number(dec)))
    .replace(/&([a-zA-Z][a-zA-Z0-9]*);?/g, (match, name) => (name in ENTITIES ? ENTITIES[name] : match));
}

// Cell content to plain text. Line breaks become "\n"; footnote marks become
// "[1]", but "cm<sup>2</sup>" stays a unit ("cm²").
function htmlToText(html) {
  const text = html
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/\s+/g, ' ')
    .replace(/(cm|in|m)\s*<sup>\s*2\s*<\/sup>/gi, '$1²')
    .replace(/<sup>\s*([^<]*?)\s*<\/sup>/gi, ' [$1]')
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/<[^>]+>/g, '');
  return decodeEntities(text)
    .replace(/­/g, '')
    .split('\n')
    .map((line) => line.replace(/[ \t ]+/g, ' ').trim())
    .join('\n')
    .trim();
}

const oneLine = (html) => htmlToText(html).replace(/\s*\n\s*/g, ' ').trim();

function attr(tag, name) {
  const m = new RegExp(`\\b${name}\\s*=\\s*("([^"]*)"|'([^']*)'|([^\\s>]+))`, 'i').exec(tag);
  return m ? decodeEntities(m[2] ?? m[3] ?? m[4]) : null;
}

// The part of the page between the navigation bar and the footer.
function pageBody(html) {
  const start = html.search(/<div class="page">/i);
  const end = html.search(/<div class="footer">/i);
  return html.slice(start < 0 ? 0 : start, end < 0 ? html.length : end);
}

function tables(html) {
  return [...html.matchAll(/<table\b[^>]*>([\s\S]*?)<\/table>/gi)].map((table) =>
    [...table[1].matchAll(/<tr\b[^>]*>([\s\S]*?)<\/tr>/gi)].map((row) =>
      [...row[1].matchAll(/<(th|td)\b([^>]*)>([\s\S]*?)<\/\1>/gi)].map((cell) => ({
        header: cell[1].toLowerCase() === 'th',
        label: /class\s*=\s*["']?subheading/i.test(cell[2]),
        attrs: cell[2],
        html: cell[3],
      })),
    ),
  );
}

// ---------------------------------------------------------------- index page

// "T-34 Series and Variants" names a family; "MEDIUM TANKS" is only a category.
function familyFromGroup(group) {
  if (!group) return null;
  const m = /^(.*?)\s+(?:series\s+)?and\s+variants$/i.exec(group);
  if (!m) return null;
  const name = m[1].trim();
  if (/\b(tanks?|vehicles|destroyers|artillery|tankettes)\b/i.test(name)) return null;
  return FAMILY_OVERRIDES[name] ?? name;
}

// Fallback: "T-60 Model 1941" -> "T-60", "SU-76M" -> "SU-76".
function familyFromName(name) {
  const base = name
    .replace(/\s+Model\s+\d{4}.*$/i, '')
    .replace(/\s+Ausf\.?\s+\S+.*$/i, '')
    .replace(/^([A-Z]+-\d+)[A-Za-z]$/, '$1')
    .trim();
  return base || name;
}

function parseIndex(html, indexUrl) {
  const vehicles = [];
  for (const block of pageBody(html).matchAll(/<p\b[^>]*>([\s\S]*?)<\/p>/gi)) {
    const heading = /<strong>([\s\S]*?)<\/strong>/i.exec(block[1]);
    const group = heading ? oneLine(heading[1]) : null;
    for (const link of block[1].matchAll(/<a\b([^>]*)>([\s\S]*?)<\/a>/gi)) {
      const href = attr(link[1], 'href');
      const name = oneLine(link[2]);
      if (!href || !name || !/\.html?$/i.test(href) || href.includes('/')) continue;
      vehicles.push({
        name,
        group,
        family: familyFromGroup(group) ?? familyFromName(name),
        url: new URL(href, indexUrl).href,
      });
    }
  }
  return vehicles;
}

// ---------------------------------------------------------------- detail page

const sectionKey = (title) => title.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '');

// Label/value pairs, two pairs per row. "Ammunition Carried" appears once per
// weapon, so it is qualified with the weapon label of its row.
function parsePairs(rows, url) {
  const data = {};
  for (const row of rows) {
    let rowLabel = null;
    for (let i = 0; i < row.length; i++) {
      if (!row[i].label) continue;
      let key = oneLine(row[i].html);
      const value = row[i + 1] && !row[i + 1].label ? htmlToText(row[i + 1].html) : null;
      if (value === null) {
        warnings.push(`${url}: label "${key}" without value`);
        continue;
      }
      i++;
      if (rowLabel === null) rowLabel = key;
      else if (/^ammunition carried$/i.test(key)) key = `${key} (${rowLabel})`;
      if (key in data) key = `${key} (${rowLabel})`;
      if (key in data) warnings.push(`${url}: duplicate field "${key}"`);
      data[key] = value;
    }
  }
  return data;
}

// First row holds the column names, the leading label cells of every other row
// the part. Two layouts need extra care: a part split over two rows
// ("Hull" spanning "Lower" and "Upper" -> "Hull Lower", "Hull Upper") and one
// value cell spanning all columns and several parts ("8mm to 35mm").
function parseArmor(rows) {
  const [head, ...body] = rows;
  const columns = head.slice(1).map((cell) => oneLine(cell.html).replace(/\s*\/\s*/g, '/'));
  const data = {};
  let prefix = { text: '', rows: 0 };
  let spanned = { text: '', rows: 0 };
  for (const row of body) {
    const split = row.findIndex((cell) => !cell.label);
    const labels = (split < 0 ? row : row.slice(0, split)).map((cell) => oneLine(cell.html));
    const values = split < 0 ? [] : row.slice(split);
    if (!labels.length) continue;

    const part = [prefix.rows > 0 ? prefix.text : '', ...labels].filter(Boolean).join(' ');
    if (prefix.rows > 0) prefix.rows--;
    const rowspan = Number(attr(row[0].attrs, 'rowspan')) || 1;
    if (rowspan > 1 && labels.length > 1) prefix = { text: labels[0], rows: rowspan - 1 };

    if (values.length === 1 && Number(attr(values[0].attrs, 'colspan')) > 1) {
      spanned = { text: htmlToText(values[0].html), rows: Number(attr(values[0].attrs, 'rowspan')) || 1 };
    }
    if (spanned.rows > 0) {
      data[part] = spanned.text;
      spanned.rows--;
    } else if (values.length) {
      data[part] = {};
      values.forEach((cell, i) => {
        data[part][columns[i] ?? `Column ${i + 2}`] = htmlToText(cell.html);
      });
    }
  }
  return data;
}

function parseNotes(html) {
  const notes = {};
  const parts = htmlToText(html).split('\n').filter(Boolean);
  parts.forEach((line, i) => {
    const m = /^\[([^\]]+)\]\s*(.*)$/.exec(line);
    if (m) notes[m[1]] = m[2];
    else notes[`note ${i + 1}`] = line;
  });
  return notes;
}

function imagesIn(html, baseUrl) {
  return [...html.matchAll(/<img\b[^>]*>/gi)]
    .map((tag) => attr(tag[0], 'src'))
    .filter((src) => src && !IGNORED_IMAGES.test(src))
    .map((src) => new URL(src, baseUrl).href);
}

function parseDetail(html, url) {
  const body = pageBody(html);
  const h1 = /<h1\b[^>]*>([\s\S]*?)<\/h1>/i.exec(body);
  const detail = { title: h1 ? oneLine(h1[1]) : null, sections: {}, imagePages: [], images: imagesIn(body, url) };

  for (const rows of tables(body)) {
    const headerRow = rows[0];
    if (!headerRow?.length || !headerRow[0].header) {
      warnings.push(`${url}: table without heading skipped`);
      continue;
    }
    const title = oneLine(headerRow[0].html);
    const key = sectionKey(title);
    const rest = rows.slice(1);
    const restHtml = rest.flat().map((cell) => cell.html).join('\n');

    if (/drawings|images/i.test(title)) {
      const seen = new Set();
      for (const link of restHtml.matchAll(/<a\b([^>]*)>([\s\S]*?)<\/a>/gi)) {
        const href = attr(link[1], 'href');
        const label = oneLine(link[2]);
        if (!href) continue;
        const pageUrl = new URL(href, url).href;
        if (!seen.has(pageUrl)) {
          seen.add(pageUrl);
          detail.imagePages.push({ url: pageUrl, label: '' });
        }
        if (label) detail.imagePages.find((p) => p.url === pageUrl).label = label;
      }
    } else if (/^notes?$/i.test(title)) {
      detail.sections.notes = parseNotes(restHtml);
    } else if (/^references?$/i.test(title)) {
      detail.sections.references = oneLine(restHtml).split(/\s*,\s*/).filter(Boolean);
    } else if (/armou?r/i.test(title) && rest[0]?.every((cell) => cell.label)) {
      detail.sections[key] = parseArmor(rest);
    } else if (rest.some((row) => row.some((cell) => cell.label))) {
      detail.sections[key] = parsePairs(rest, url);
    } else {
      detail.sections[key] = htmlToText(restHtml);
    }
  }
  if (!detail.sections.general_data) warnings.push(`${url}: no GENERAL DATA table found`);
  return detail;
}

// A drawing page: one picture with a title, a caption and a credit line.
function parseImagePage(html, url) {
  const body = pageBody(html);
  const text = (tag) => {
    const m = new RegExp(`<${tag}\\b[^>]*class="cntr"[^>]*>([\\s\\S]*?)</${tag}>`, 'i').exec(body);
    return m ? oneLine(m[1]) : '';
  };
  return { images: imagesIn(body, url), title: text('h2'), caption: text('p'), credit: text('h3') };
}

// ---------------------------------------------------------------- output

// Keeps names readable but safe as a folder name on macOS, Linux and Windows.
function folderName(name) {
  return name
    .replace(/[\/\\:|]/g, '-')
    .replace(/[<>"?*\u0000-\u001f]/g, '')
    .replace(/\s+/g, ' ')
    .replace(/[. ]+$/, '')
    .trim();
}

const exists = (file) => access(file).then(() => true, () => false);

async function downloadImage(url, dir) {
  const file = path.join(dir, folderName(decodeURIComponent(path.basename(new URL(url).pathname))));
  if (!opts.force && (await exists(file))) return { file: path.basename(file), downloaded: false };
  const bytes = await fetchBytes(url);
  await writeFile(file, bytes);
  return { file: path.basename(file), downloaded: true };
}

async function processVehicle(nation, vehicle, usedDirs) {
  const html = await fetchHtml(vehicle.url);
  const detail = parseDetail(html, vehicle.url);

  let dir = path.join(opts.out, nation.code, folderName(vehicle.family), folderName(vehicle.name));
  if (usedDirs.has(dir)) dir += ` (${path.basename(vehicle.url, '.html')})`;
  usedDirs.add(dir);
  await mkdir(dir, { recursive: true });

  // Pictures directly on the detail page plus one per linked drawing page.
  const sources = detail.images.map((src) => ({ src, page: vehicle.url }));
  for (const page of detail.imagePages) {
    try {
      const info = parseImagePage(await fetchHtml(page.url), page.url);
      if (!info.images.length) warnings.push(`${page.url}: no image found`);
      for (const src of info.images) sources.push({ src, page: page.url, ...info, label: page.label });
    } catch (err) {
      warnings.push(`image page failed: ${err.message}`);
    }
  }

  const images = [];
  let downloaded = 0;
  for (const { src, page, label, title, caption, credit } of sources) {
    if (images.some((image) => image.source_url === src)) continue;
    try {
      const result = await downloadImage(src, dir);
      if (result.downloaded) downloaded++;
      images.push({
        file: result.file,
        ...(label && { label }),
        ...(title && { title }),
        ...(caption && { caption }),
        ...(credit && { credit }),
        source_url: src,
        page_url: page,
      });
    } catch (err) {
      warnings.push(`image failed: ${err.message}`);
    }
  }

  const { notes, references, ...sections } = detail.sections;
  const json = {
    name: vehicle.name,
    title: detail.title,
    nation: nation.code,
    family: vehicle.family,
    index_group: vehicle.group,
    source: 'onwar.com (Ralph Zuljan)',
    source_url: vehicle.url,
    fetched: new Date().toISOString().slice(0, 10),
    ...sections,
    images,
    ...(notes && { notes }),
    ...(references && { references }),
  };
  await writeFile(path.join(dir, 'data.json'), JSON.stringify(json, null, 2) + '\n');
  return { dir, images: images.length, downloaded };
}

// ---------------------------------------------------------------- main

function parseArgs(args) {
  const result = { nation: null, limit: Infinity, force: false, delay: 400, out: TARGET_DIR };
  for (let i = 0; i < args.length; i++) {
    const value = () => {
      if (i + 1 >= args.length) fail(`missing value for ${args[i]}`);
      return args[++i];
    };
    switch (args[i]) {
      case '--nation': result.nation = value().toUpperCase(); break;
      case '--limit': result.limit = Number(value()); break;
      case '--delay': result.delay = Number(value()); break;
      case '--out': result.out = path.resolve(value()); break;
      case '--force': result.force = true; break;
      case '-h':
      case '--help':
        console.log('node tools/onwar_download.mjs [--nation GER|US|RU] [--limit N] [--force] [--delay MS] [--out DIR]');
        process.exit(0);
      default: fail(`unknown option ${args[i]}`);
    }
  }
  if (result.nation && !NATIONS.some((n) => n.code === result.nation)) fail(`unknown nation ${result.nation}`);
  if (!(result.limit > 0) || !(result.delay >= 0)) fail('--limit and --delay need a number');
  return result;
}

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(2);
}

async function main() {
  const totals = { vehicles: 0, images: 0, downloaded: 0, failed: 0 };
  console.log(`target: ${opts.out}`);

  for (const nation of NATIONS.filter((n) => !opts.nation || n.code === opts.nation)) {
    const vehicles = parseIndex(await fetchHtml(nation.url), nation.url);
    if (!vehicles.length) warnings.push(`${nation.url}: no vehicles found`);
    const selected = vehicles.slice(0, opts.limit);
    console.log(`\n${nation.code}: ${selected.length} of ${vehicles.length} vehicles`);

    const usedDirs = new Set();
    for (const [i, vehicle] of selected.entries()) {
      const prefix = `  [${String(i + 1).padStart(2)}/${selected.length}] ${vehicle.family} / ${vehicle.name}`;
      try {
        const result = await processVehicle(nation, vehicle, usedDirs);
        totals.vehicles++;
        totals.images += result.images;
        totals.downloaded += result.downloaded;
        console.log(`${prefix}: ${result.images} image(s)`);
      } catch (err) {
        totals.failed++;
        warnings.push(`vehicle failed: ${err.message}`);
        console.log(`${prefix}: FAILED (${err.message})`);
      }
    }
  }

  console.log(
    `\n${totals.vehicles} vehicles written, ${totals.images} images (${totals.downloaded} newly downloaded), ${totals.failed} failed`,
  );
  if (warnings.length) {
    console.log(`\n${warnings.length} warning(s):`);
    for (const warning of warnings) console.log(`  - ${warning}`);
  }
  process.exitCode = totals.failed ? 1 : 0;
}

await main();
