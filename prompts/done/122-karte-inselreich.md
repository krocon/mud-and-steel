# Prompt 122 – Karte nach Bildvorlage: Inselreich, farbenfroh

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` (Ziel, Entscheidungen, Ergebnis),
> in `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz (alle Tabellen
> der Skizze, besonders `sea`, `[[land]]`, `[[bay]]`, `[[causeway]]`, `[[shoal]]`,
> `[[crossing]]`, `[[hill]]`, `[[wood]]` mit `rock`, `[[paint]]`), „Bridges“ und
> „Biome“, `data/map_sketches/island_chain.toml` ganz als nächstes Muster,
> `crates/cli/src/gen_sketch.rs`, in `docs/ARCHITECTURE.md` „Lighting, atmosphere
> and biomes (presentation only)“ und „Water (presentation only)“,
> `godot/assets/biomes/forest.tres` und `biome_preset.gd`,
> `godot/scenes/world/atmosphere.gd`, `godot/scenes/terrain/terrain.gd`
> (`apply_biome`), das Wasser-Shader-Setup und `data/objects.toml` (Felsen,
> Brücken). Ist Prompt 115 (lange Brücken) erledigt, auch dessen Ergebnis.
>
> **Vorlagen** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/inseln/inseln_draufsicht.webp` (Draufsicht, Grundlage für die
> Skizze) und `docs/.vorlagen/inseln/inseln_schraeg.webp` (Schrägansicht derselben
> Landschaft, Vorbild für Höhen, Licht und Farben). Beide genau ansehen.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `archipelago` (im Spiel „Inselreich“ / „Archipelago“) nach der
Vorlage: acht größere Inseln und Halbinseln um eine **ringförmige Mittelinsel mit
einer Lagune** und einem Eiland darin, verbunden durch **Steinbogenbrücken mit
mehreren Bögen**; zwei bis drei **Felsberge mit Schneekuppen**, Wasserfälle und
Stromschnellen in den Meerengen, Dörfer mit roten Ziegeldächern und Kirchtürmen,
goldene und grüne Felder, dichte Nadelwälder, Felsküsten mit hellen Sandbuchten,
Holzstege an den Außenküsten und viele kleine Felseilande ringsum.

Der Nutzer will die Karte **genauso farbenfroh wie die Vorlage**: sattes Grün,
türkis-blaues Wasser mit hellem Saum an den Küsten, goldgelbe Felder, rote Dächer,
helle Felsen, klarer Himmel. Das ist eine bewusste Ausnahme von der
zurückhaltenden Optik der anderen Karten und gilt nur für diese Karte
(Abschnitt 4).

Die Vorlagen sind nur Vorbild für Anordnung und Wirkung. Nichts daraus übernehmen
(keine Texturen, keine Bildausschnitte), alles mit den Bausteinen des Spiels
nachbauen.

## 2. Skizze (Stufe 1)

`data/map_sketches/archipelago.toml`, erzeugt mit `just gen-sketch archipelago`:

- **Größe** 3,6 km × 2,4 km (Seitenverhältnis der Draufsicht), `sea = true`,
  Biom `forest`, Wetter klar.
- **Inseln** als `[[land]]`-Polygone nach der Draufsicht nachgezeichnet, die
  Meerengen dazwischen als `[[bay]]`, wo sie innen liegen. Die Mittelinsel als Ring
  um die Lagune, das Eiland darin als `[[island]]`. Die Felseilande am Rand als
  kleine `[[land]]`-Polygone ohne Zugang.
- **Berge:** `[[hill]]` für die drei Felsberge (Nord-Mitte, Nordost, Südwest) mit
  deutlich mehr Höhe als bisher in Skizzen (Vorschlag 80–150 m; der Generator muss
  das tragen, ggf. erweitern), steile Flanken und Gipfel über 60 % Steigung
  (unpassierbar), Felsen mit `[[wood]]` `rock` und `env_rock_massif`. **Schnee auf
  den Gipfeln** als reine Darstellung (Abschnitt 4), nicht als Geländeart `snow`,
  damit sich die Wege nicht ändern; oder `[[paint]]` mit `snow` nur dort, wo ohnehin
  niemand hinkommt. Die Session wählt und begründet.
- **Brücken:** In der Vorlage sind es sechs lange Steinbrücken mit drei bis fünf
  Bögen über breite Meerengen. Der Skizzen-Generator verengt das Wasser heute auf
  21 m (längste Steinbrücke). Wenn Prompt 115 erledigt ist: eine Variante
  **mehrbogige Steinbrücke** (Teile aneinandergereiht, 60–120 m) ergänzen und die
  Meerengen breit lassen. Sonst wie bei `island_chain` mit Dämmen (`[[causeway]]`)
  und kurzen Brücken, und die langen Brücken als offener Punkt.
- **Zweiter Weg für Panzer:** wie bei `island_chain` ein bis zwei Untiefen
  (`[[shoal]]`) als Furt, damit eine gesprengte oder gesperrte Brücke nicht das
  ganze Spiel entscheidet.
- **Wasserfälle und Stromschnellen:** Stromschnellen gibt es (`valley_fields`). Ein
  Wasserfall vom Berg in die Meerenge ist neu: als reine Darstellung (Objekt oder
  Effekt mit Gischt, Geräusch) an einer Felskante, ohne Regelwirkung. Zwei bis drei
  Stück wie in der Vorlage.
- **Dörfer:** vier bis fünf Dörfer mit Kirche (NW, NO mit größerer Kirche, W, Mitte
  unten bei der Südwest-Insel, SO), `[[village]]`/`[[town]]` mit Häusern mit roten
  Ziegeldächern aus dem Katalog; Feldwege zwischen den Dörfern und zu den Brücken
  wie in der Draufsicht.
- **Felder:** `[[patchwork]]` und `[[field]]` mit `grain` (gold) und `green`.
- **Wälder:** viel Nadelwald (`conifer`), an den Rändern gemischt; Sandbuchten an
  den Außenküsten.
- **Holzstege:** Die Vorlage hat Stege an den Außenküsten. Es gibt kein
  Steg-Objekt. Ein einfaches Objekt `env_pier_wood` (Bohlen auf Pfählen, blockiert
  nichts, Darstellung im Wasser) neu bauen, mit Vorschaubild, oder als offener
  Punkt notieren, wenn Stufe 3 sonst zu groß wird. Kleine Boote am Steg nur als
  Dekoration.
- **Starts und Versorgungspunkte:** zwei Teams, West (Südwest- und Westinsel)
  gegen Ost (Nordost- und Südostinsel), je zwei Starts, 60 m eben und frei.
  Versorgungspunkte auf der Mittelinsel (umkämpft), auf den Berginseln an den
  Pässen, in den Dörfern und an den Brückenköpfen. Mit `just map-reach`
  ausgleichen.

## 3. Prüfen der Karte (Stufe 2)

- `just map-check archipelago`, `just map-reach archipelago` (alle Starts und Punkte
  für Fahrzeuge erreichbar, über Brücken und Furten), Test wie bei den anderen
  Skizzenkarten (zweimal erzeugt, gleiche Dateien).
- KI-Turnier auf der Karte, Ergebnis in `docs/AI.md` (bleiben Kolonnen an Brücken
  hängen, wird um die Mittelinsel gekämpft?).
- Messung der Bildrate in jeder Ansicht (`just bench-view`), `docs/PERFORMANCE.md`.
- Menüname und Übersetzung, Eintrag in der Tabelle in `docs/MAPS.md`.

## 4. Farbenfroher Look (Stufe 3)

Reine Darstellung, nur für diese Karte, andere Karten bleiben Bit für Bit gleich im
Aussehen:

- Ein **Look je Karte**: neues optionales Feld in `map.toml` (Vorschlag
  `look = "vivid"`, wie `biome` nicht im Karten- oder Spielstand-Hash, von der
  Simulation nur geprüft), das auf das Biom-Preset eine Abwandlung legt. Oder ein
  eigenes Preset `forest_vivid`, das `camo_biome` und `env_biome` von `forest`
  übernimmt. Die Session wählt das Einfachere und begründet es.
- **Wirkung** wie in den Vorlagen: kräftigere Sättigung und etwas mehr Kontrast,
  warmes, helles Sonnenlicht, klarer Himmel, wenig Dunst; Gras saftig grün, Wald
  dunkler grün, Felder goldgelb, Felsen hellgrau, Sand hell; Wasser in der Tiefe
  kräftig blau, in flachem Wasser türkis, deutlicher heller Saum und Schaum an den
  Küsten. Dächer der Dörfer kräftig rot (die vorhandenen Häuser tönen, nicht neu
  modellieren).
- **Schnee auf den Gipfeln** ab einer Höhe in diesem Look (Shader-Anteil wie die
  Schneedecke im Winter, nur oberhalb einer Grenze und auf flacheren Stellen).
- Wolkenfetzen am Bildrand wie in der Schrägansicht nur, wenn der Fotomodus oder
  die Übersicht es ohne Aufwand trägt; sonst nicht.
- **Vergleich:** dieselben drei Ansichten mit `forest` und mit dem neuen Look neben
  der Vorlage. Der Nutzer entscheidet am Ende, ob es farbig genug ist; die Werte
  liegen in einer Datei, damit sie leicht nachzustellen sind.
- Unit-Farben und Tarnungen bleiben unverändert (Einheiten sollen auf der bunten
  Karte erkennbar bleiben). Prüfen, dass Auswahlringe, Nebel des Krieges und HUD
  noch gut lesbar sind.

## 5. Abschluss

`just check`, `just selftest`, Vorschaubild der Karte im Menü (falls Karten Bilder
haben), Doku in `docs/MAPS.md` (Tabelle der Skizzenkarten, neues Feld bzw. Preset,
Wasserfall, Steg) und `docs/ARCHITECTURE.md` (Look je Karte). Der Golden Hash
ändert sich nicht (neue Karte, reine Darstellung); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Andere Karten auf den bunten Look umstellen (erst nach dem Urteil des Nutzers).
- Boote als Einheiten (Prompt 119); Stege sind hier nur Kulisse.
- Fließendes Wasser mit Gefälle zwischen Meer und Bergsee (Wasserfälle bleiben
  Darstellung).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 23.11.2026, alle vorläufig)

| # | Frage | Entscheidung | Warum |
|---|---|---|---|
| 1 | Lange Brücken | Prompt 115 ist erledigt: neue Variante `env_bridge_stone` `ArchHigh` (ein hoher Bogen, 20 m, 16 m Spannweite, Fahrbahn 8 m über dem Fuß), im Generator `[[crossing]] kind = "stone_long"`: Teile Ende an Ende über die ganze Meerenge (Kette nach 115, eine Fahrbahn), Fahrbahn 6 m über dem Ufer (`rise_m`), Rampen 12 %; sechs Brücken mit 4–6 Bögen (80–120 m), dazu eine kurze Steinbrücke über den Bach westlich der Lagune. **[vorläufig]** | Kein neues Brückenmodell nötig, die Widerlager zweier Teile bilden den Pfeiler; Meerengen bleiben breit. |
| 2 | Schnee auf den Gipfeln | Reine Darstellung über den Look: `snow_line_m = 95` (Schneegrenze wie die Bergschneedecke aus 060, nur fest und auf Karten jeder Höhe), keine Geländeart `snow`. **[vorläufig]** | Wege und Regeln bleiben gleich; in `forest` ohne Look kein Schnee. |
| 3 | Berge | `[[hill]] kind = "peak"`: spitze Felsberge (Nord 150 m, Nordost 115 m, Südwest 125 m) mit Graten aus Grat-Rauschen, Gipfel über 60 %; dazu vier Felshügel; Wald `crag` (Felsmassive, Ausbisse, wenige Nadelbäume). Höhen über 60 m Relief mit gröberem Höhenschritt (3 mm), alte Karten bleiben bei 1 mm. **[vorläufig]** | Der Generator trug nur 60 m; Grat-Amplitude nach den ersten Bildern verkleinert (wirkte als Wurmmuster). |
| 4 | Look je Karte | Neues Feld `look = "vivid"` in `map.toml` (Sim prüft und speichert, nicht im Hash), Godot lädt `<biom>_<look>.tres`, also `forest_vivid.tres` als vollständiges Preset mit `id`/`camo_biome`/`env_biome` = `forest`. **[vorläufig]** | Einfacher als eine Überlagerung einzelner Felder; alle Werte in einer Datei zum Nachstellen; andere Biome/Karten unverändert. |
| 5 | Rote Dächer | Globaler Shader-Parameter `env_roof_tint` (Preset-Feld `roof_tint`): Ziegel-, Zement- und Schieferdächer nehmen den Farbton an (Hell/Dunkel bleibt), Stroh und Bretter nicht; sonst 0. **[vorläufig]** | Häuser getönt, nicht neu modelliert (Prompt). |
| 6 | Wasserfälle | `[[waterfall]]` in der Skizze → Marker `waterfall_<n>_top`/`_foot`; `world/waterfalls.gd`: Band mit herabrinnenden Schlieren über dem Boden, Gischt (`waterfall_mist`), Flussgeräusch als 3D-Schleife. Drei Stück (Nordberg, Felsen am Ring, Südwestberg). **[vorläufig]** | Keine Regelwirkung, kein neues Modell; Marker sind im Format schon da. |
| 7 | Stromschnellen | `[[rapids]]`: Steine (`env_rock` klein/mittel, blockieren nichts) im Wasser unter den Wasserfällen; das Wasser schäumt an ihnen. Tiefes Wasser bleibt tief. **[vorläufig]** | Schaum an Steinen gibt es seit 014. |
| 8 | Holzstege | Neues Objekt `env_pier_wood` (`Span` 10 m, `Boat` mit zwei Ruderbooten), Vorschaubild, blockiert nichts, unzerstörbar; Godot stellt es auf den Wasserspiegel (`FLOATING_KINDS`). Sechs Stege an den Außenküsten. **[vorläufig]** | Klein genug für diesen Prompt; Boote nur Kulisse. |
| 9 | Inseln und Starts | West (Startinseln West, Südwest; Landbrücke) gegen Ost (Nordost, Südost; Ost- und Südostinsel mit Landbrücke, Furt zur Nordostinsel); Nordwestinsel über eine Furt von West, über Brücke 1 von der Mitte; Mitte = Nordberginsel + Ringinsel (eine Landmasse), Lagune mit Eiland (neu: `[[island]]` auch in Buchten). Jede Seite hat drei Brücken in die Mitte. **[vorläufig]** | Vorlage nachempfunden, aber spielbar und ausgeglichen. |
| 10 | Furten | Zwei Untiefen innerhalb einer Seite (Nordwest–West, Nordost–Ost); keine zusätzliche Furt in die Mitte. **[vorläufig]** | Drei Brücken je Seite: eine gesprengte Brücke entscheidet nicht. |
| 11 | Versorgungspunkte | Elf: Lagune Nord/Süd, Nordpass, Brückenkopf West/Ost, Dörfer NW/O/SW/SO, Pässe SW/NO; mit `map-reach` ausgeglichen (umkämpft West 5 081 m, Ost 5 158 m; eigene 3 199 / 3 172 m). **[vorläufig]** | Prompt. |
| 12 | Größere Kirche Nordost | `env_church` plus `env_town_hall` und das größte Dorf (22 Häuser). **[vorläufig]** | Keine größere Kirche im Katalog (`env_church_med` ist mediterran). |
| 13 | Wälder | Viele Nadelwälder (handgezeichnet + Raster von Waldflecken außerhalb von Dörfern, Feldern, Punkten), 9 m Abstand. **[vorläufig]** | 6,5 m gab 64 000 Bäume und 30 FPS im dichtesten Wald. |
| 14 | Wolken am Rand | Nicht umgesetzt. **[vorläufig]** | Fotomodus/Übersicht tragen es nicht ohne Aufwand. |

### Ergebnis

- **Skizze** `data/map_sketches/archipelago.toml`, Karte `data/maps/archipelago/`
  (3,6 × 2,4 km, 39 129 Objekte, 31 Brückenteile, 51 Stegteile, 105
  Stromschnellen-Steine), im Menü „Inselreich“ / „Archipelago“.
- **Generator** (`gen_sketch.rs`): `stone_long`, `rise_m`, `kind = "peak"`,
  Höhenschritt über 60 m, `[[island]]` in Buchten, Wald `crag`,
  `[[waterfall]]`, `[[pier]]`, `[[rapids]]`, `look`; alle anderen
  Skizzenkarten Bit für Bit gleich (geprüft).
- **Sim/Editor/Bridge**: `Look` (`sim::Map::look`, Test
  `look_is_read_but_not_hashed`), Editor behält `look`,
  `SimHost.map_look`, `MapEditor.water_level_at`. Golden Hash unverändert.
- **Godot**: `forest_vivid.tres`, Preset-Felder `snow_line_m`,
  `mountain_look`, `roof_tint`, `atmosphere.gd` (`look_from`,
  `load_preset(id, look)`), `env.gdshader` (Dächer), `terrain.gd`
  (Schneegrenze), `waterfalls.gd`, `map_objects.gd` (Stege auf dem
  Wasser), Effekt `waterfall_mist`; `just archipelago-shots`.
- **Prüfungen**: `map-check` ok; `map-reach`: alle Starts und Punkte für
  Fahrzeuge erreichbar, 78,5 % des Landes befahrbar; Test zweimal erzeugt
  gleich (ALL); KI-Turnier und `stuck-audit` in `docs/AI.md` (395 von 400
  angekommen, zwei LKW mit Geschütz geben am Ostende von Brücke 1 auf,
  offen); Bildrate in `docs/PERFORMANCE.md` (hoch: Wald 36 FPS, Übersicht
  57, Dorf 57; mittel 67–120).
- **Bilder**: `build/archipelago/final/compare_{top,oblique,village}.png`
  (Vorlage, forest, vivid) und Einzelbilder, `build/archipelago/view/`
  (HUD, Ringe lesbar), `build/archipelago/reach.png`.
- **Offen**: Wolkenfetzen; Bildrate im dichten Wald auf „hoch“ unter 60
  FPS (Schatten); zwei LKW mit Geschütz am Brückenende; der Angreifer
  verliert gegen die passive KI öfter als er gewinnt (viele Zeitabläufe);
  Wasserfälle nur 20–30 m hoch (Küstenabfall begrenzt die Höhe); Nutzer
  entscheidet, ob der Look farbig genug ist (Werte in `forest_vivid.tres`).
