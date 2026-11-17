// Sound page (prompt 083): plays every audio file of godot/assets/audio/ with Web Audio.
// Data: index.json, written by tools/sounds_web.py; audio files straight from the repository.

const ROOT = "../../"; // the page lives at /web/sounds/, paths in index.json start at the repository root
const ORIGIN = { synth: "synthetisiert", recording: "Aufnahme", private: "privat" };
const STORE = { volume: "sounds.volume", game: "sounds.game" };

const $ = (id) => document.getElementById(id);
let audio = null; // AudioContext, created on the first play (browsers need a gesture)
let master = null;
const buffers = new Map(); // path -> Promise<AudioBuffer>
const playing = new Set(); // {source, gain, button, row, sequence}

function load(key, fallback) {
  try {
    const value = localStorage.getItem(key);
    return value === null ? fallback : JSON.parse(value);
  } catch {
    return fallback;
  }
}

function save(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage blocked: the page works without it.
  }
}

function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (key === "class") node.className = value;
    else if (key === "text") node.textContent = value;
    else node.setAttribute(key, value);
  }
  node.append(...children.filter((c) => c !== null && c !== undefined));
  return node;
}

function seconds(value) {
  return value == null ? "…" : `${value.toFixed(value < 10 ? 2 : 1)}\u00a0s`;
}

function dbText(db) {
  return `${db > 0 ? "+" : db < 0 ? "−" : ""}${Math.abs(db).toFixed(1).replace(".0", "")}\u00a0dB`;
}

// --- Audio -------------------------------------------------------------------------

function context() {
  if (!audio) {
    audio = new AudioContext();
    master = audio.createGain();
    master.gain.value = Number($("volume").value);
    master.connect(audio.destination);
  }
  if (audio.state === "suspended") audio.resume();
  return audio;
}

function buffer(path) {
  if (!buffers.has(path)) {
    const promise = fetch(ROOT + path)
      .then((r) => {
        if (!r.ok) throw new Error(`${r.status} ${path}`);
        return r.arrayBuffer();
      })
      .then((data) => context().decodeAudioData(data));
    promise.catch(() => buffers.delete(path));
    buffers.set(path, promise);
  }
  return buffers.get(path);
}

// Level and pitch of the row's class, as battle_sound.gd / ui_sound.gd apply them.
function shaping(row) {
  if (!$("game").checked || !row.spec) return { gain: 1, rate: 1 };
  const spread = Number(row.spec.pitch_spread || 0);
  return {
    gain: Math.pow(10, Number(row.spec.volume_db || 0) / 20),
    rate: 1 + (Math.random() * 2 - 1) * spread,
  };
}

function stopEntry(entry) {
  playing.delete(entry);
  entry.ended = true;
  try {
    entry.source?.stop();
  } catch {
    // Already stopped.
  }
  entry.button?.classList.remove("playing");
  entry.sequence?.classList.remove("playing");
  if (![...playing].some((e) => e.row === entry.row)) entry.row.classList.remove("playing");
}

function stopRow(rowNode) {
  for (const entry of [...playing]) if (entry.row === rowNode) stopEntry(entry);
}

function stopAll() {
  for (const entry of [...playing]) stopEntry(entry);
}

// Plays one file of a row; resolves when it ends or is stopped.
async function play(row, file, button, rowNode, entry) {
  const ctx = context();
  entry.button = button;
  entry.row = rowNode;
  playing.add(entry);
  button.classList.add("playing");
  rowNode.classList.add("playing");
  let data;
  try {
    data = await buffer(file.path);
  } catch (error) {
    console.warn(`sounds: ${error.message}`);
    stopEntry(entry);
    button.title = `Nicht abspielbar: ${error.message}`;
    return;
  }
  if (entry.ended) return;
  const { gain, rate } = shaping(row);
  const source = ctx.createBufferSource();
  source.buffer = data;
  source.loop = row.loop;
  source.playbackRate.value = rate;
  const level = ctx.createGain();
  level.gain.value = gain;
  source.connect(level).connect(master);
  entry.source = source;
  await new Promise((resolve) => {
    source.onended = resolve;
    source.start();
  });
  if (entry.button === button) button.classList.remove("playing");
  if (!entry.sequence) stopEntry(entry);
}

function toggleFile(row, file, button, rowNode) {
  const was = button.classList.contains("playing");
  stopRow(rowNode);
  if (!was) play(row, file, button, rowNode, {});
}

async function sequence(row, rowNode, seqButton, fileButtons) {
  const was = seqButton.classList.contains("playing");
  stopRow(rowNode);
  if (was) return;
  const entry = { sequence: seqButton };
  seqButton.classList.add("playing");
  for (let i = 0; i < row.files.length; i++) {
    if (entry.ended) return;
    await play(row, row.files[i], fileButtons[i], rowNode, entry);
  }
  stopEntry(entry);
}

// --- Page ----------------------------------------------------------------------------

function specText(row) {
  const parts = [];
  const s = row.spec;
  if (s) {
    parts.push(`Gruppe ${s.group}`, `Bus ${s.bus}`, dbText(Number(s.volume_db || 0)));
    if (s.pitch_spread) parts.push(`Tonhöhe ±${Math.round(s.pitch_spread * 100)}\u00a0%`);
    if (s.range_m) parts.push(`Reichweite ${s.range_m}\u00a0m (voll bis ${s.ref_m}\u00a0m)`);
    parts.push(`Priorität ${s.priority}`);
  }
  if (row.loop) parts.push("Schleife");
  if (row.note) parts.push(row.note);
  return parts.join(" · ");
}

function renderRow(row) {
  const node = el("div", { class: "row", id: row.name });
  node.dataset.search = `${row.name} ${row.remark}`.toLowerCase();
  const fileButtons = row.files.map((file) => {
    const button = el(
      "button",
      { type: "button", class: "play", title: file.path },
      el("span", { class: "len", text: seconds(file.seconds) }),
      el("span", { class: `origin ${file.origin}`, text: ORIGIN[file.origin] }),
      file.displaced ? el("span", { class: "tag", text: "im Spiel verdrängt" }) : null,
    );
    if (file.displaced) button.classList.add("is-displaced");
    button.setAttribute("aria-label", `${file.file} abspielen`);
    button.addEventListener("click", () => toggleFile(row, file, button, node));
    if (file.seconds == null) {
      // MP3 or unreadable WAV: the browser reads the length from the metadata.
      const probe = new Audio();
      probe.preload = "metadata";
      probe.addEventListener("loadedmetadata", () => {
        button.querySelector(".len").textContent = seconds(probe.duration);
      });
      probe.src = ROOT + file.path;
    }
    return button;
  });
  const files = el("div", { class: "files" }, ...fileButtons);
  if (!row.loop && row.files.length > 1) {
    const seq = el("button", { type: "button", class: "seq", text: "alle nacheinander" });
    seq.addEventListener("click", () => sequence(row, node, seq, fileButtons));
    files.append(seq);
  }
  node.append(
    el("a", { class: "name", href: `#${row.name}`, text: row.name }),
    el("p", { class: row.remark ? "remark" : "remark missing", text: row.remark || "Keine Bemerkung in usage.toml." }),
    el("p", { class: "spec", text: specText(row) }),
    files,
  );
  return node;
}

function render(index) {
  const list = $("list");
  list.replaceChildren();
  const nav = $("nav");
  let rows = 0;
  let files = 0;
  for (const section of index.sections) {
    const count = section.rows.reduce((n, r) => n + r.files.length, 0);
    rows += section.rows.length;
    files += count;
    const box = el(
      "section",
      { id: `s-${section.key}` },
      el("h2", {}, section.title, el("span", { text: `${section.rows.length} Klänge, ${count} Dateien` })),
      ...section.rows.map(renderRow),
    );
    list.append(box);
    nav.append(el("a", { href: `#s-${section.key}` }, `${section.title} `, el("b", { text: String(count) })));
  }
  $("count").textContent = `${files} Dateien in ${rows} Klängen`;
}

function filter() {
  const words = $("search").value.toLowerCase().split(/\s+/).filter(Boolean);
  let shown = 0;
  for (const section of document.querySelectorAll("#list section")) {
    let any = false;
    for (const row of section.querySelectorAll(".row")) {
      const hit = words.every((w) => row.dataset.search.includes(w));
      row.hidden = !hit;
      any ||= hit;
      shown += hit ? 1 : 0;
    }
    section.hidden = !any;
  }
  $("nothing").hidden = shown > 0;
}

function jump() {
  document.querySelector(".row.target")?.classList.remove("target");
  const id = decodeURIComponent(location.hash.slice(1));
  const target = id && document.getElementById(id);
  if (!target) return;
  if (target.classList.contains("row")) {
    if (target.hidden) {
      $("search").value = "";
      filter();
    }
    target.classList.add("target");
  }
  target.scrollIntoView({ block: "start" });
}

function barHeight() {
  const bar = document.querySelector(".bar");
  const sticky = getComputedStyle(bar).position === "sticky";
  document.documentElement.style.setProperty("--bar-h", `${sticky ? bar.offsetHeight + 8 : 8}px`);
}

async function main() {
  $("volume").value = load(STORE.volume, 0.7);
  $("game").checked = load(STORE.game, false);
  $("volume").addEventListener("input", () => {
    const value = Number($("volume").value);
    if (master) master.gain.value = value;
    save(STORE.volume, value);
  });
  $("game").addEventListener("change", () => save(STORE.game, $("game").checked));
  $("stop").addEventListener("click", stopAll);
  $("search").addEventListener("input", filter);
  addEventListener("hashchange", jump);
  addEventListener("resize", barHeight);
  addEventListener("keydown", (event) => {
    if (event.key === "Escape") stopAll();
  });
  let index;
  try {
    const response = await fetch("index.json", { cache: "no-store" });
    if (!response.ok) throw new Error(String(response.status));
    index = await response.json();
  } catch (error) {
    $("list").replaceChildren(
      el("p", { class: "empty" }, "index.json fehlt (", el("code", { text: String(error.message) }), "). Start mit ", el("code", { text: "just sounds-web" }), "."),
    );
    return;
  }
  render(index);
  filter();
  // Fonts change the row heights: jump once they are in.
  await document.fonts.ready;
  barHeight();
  jump();
}

main();
