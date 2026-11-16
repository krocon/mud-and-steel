# Prompt 056 – Karten und Basis Stufe 2

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/MAPS.md`, `docs/GAME_DESIGN.md`, `docs/ASSETS.md`, Umwelt-Objekte aus 041.
>
> Konkretisiert und umgesetzt: siehe „Konkretisierung“ und „Ergebnis“ unten.

## 1. Ziel

Mehr und lebendigere Karten: eine Winterkarte mit Dorf, eine Landschaft mit Höfen, eine
Flusskarte; Versorgungspunkte an echten Orten der Karte statt abstrakten Markern; eine
Basis, die nach Fabrik und Feldlager aussieht.

## 2. Umfang

1. **Karten** (mit dem Editor 028 und Werkzeugen): „Winterdorf“, „Höfe“, „Flussland“
   (Namen frei wählbar, keine Orte mit politischer Bedeutung), je 2 und 4 Spieler.
2. **Versorgungspunkte an Orten:** Dorfplatz, Grabenstellung, Gehöft, Brückenkopf;
   Gebäude und Objekte dort gehören zum Punkt.
3. **Abwechslung:** jedes Dorfhaus mit eigener Farbe und Dachvariante (Material-Varianten,
   deterministisch nach Position), verschneite Bäume, die in 3D umfallen, Heuhaufen,
   Strohreste, Schnee auf Objekten.
4. **Basis-Optik:** Fahrzeugwerk als große Stahlhalle, Kaserne, Depot mit Fässern und
   Kisten; Bauvorschau gibt es schon (grün/rot).
5. **Ressourcen:** Personal als begrenzende Größe prüfen (Einkommen sinkt ohne Punkte);
   in `GAME_DESIGN.md` mit Begründung, Abgleich mit 045.
6. **Hinterhalt** [provisional]: Infanterie kann sich in Wald/Gebäuden verstecken und
   erst auf kurze Distanz feuern (Tarnung bis zum ersten Schuss).

## 3. Akzeptanzkriterien

1. `just map-check` für jede neue Karte, `just editor-selftest`, `just selftest`,
   `just gallery-check` grün; Benchmark je Karte.
2. Doku in `docs/MAPS.md` und `GAME_DESIGN.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code: Karten `skirmish`, `twin_meadow` (2v2), `town_small`
(041), `desert_village` (059, Gehöfte, Dorf-Pinsel), `mountain_pass` (060);
Generator `gen_map.rs` mit `--start-distance-m`, `--players`, `--relief`;
Versorgungspunkte als `supply*`-Marker (023), Grabenstellungen über
`strongpoint_<name>`-Marker (052), Feuer (041), Wetter je Karte (040),
Basisgebäude als geliehene Dorfhäuser (023), Produktionsbilder (058).
Tarnnetze (052) verstecken stehende Einheiten schon bis zum Schuss; Bäume
fallen schon in 3D (020), der Schnee kommt aber aus der Welt-Normale.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Ein neuer Generator `cli gen-land --map …`** (`crates/cli/src/gen_land.rs`, `just gen-land`) für alle drei Karten, statt `gen_map.rs` zu erweitern. | `gen_map.rs` ist geteilt (Testkarten, Golden-Hash); ein eigenes Modul lässt die Testkarten bit-identisch und erlaubt Orte nach Plan. |
| 2 | **Drei 1-km-Karten mit vier Starts in zwei Teams** (2 Spieler nutzen 0 und 1, diagonal): `winter_village` (Nord/Süd, Biom Winter, Wetter Schnee), `farms` (Nord/Süd), `river_land` (West/Ost, Fluss dazwischen, Wetter bedeckt). Keine echten Ortsnamen. | Wie `desert_village`; 1 km gibt Raum für Orte, ohne dass die erste Begegnung ewig dauert. |
| 3 | **Versorgungspunkte stehen auf Orten, die der Generator mit ihnen baut**: Dorfplatz (mit Kirche), Grabenstellung (`strongpoint_hill` + `supply_trench` auf derselben Hügelkuppe), Gehöfte (Hof in der Umzäunung), Brückenköpfe (Marker auf der Brückenmitte, beide Ufer im 25-m-Radius). Keine neue Sim-Regel „Gebäude gehören zum Punkt“. **[vorläufig]** | Gewünschte Wirkung (man kämpft um sichtbare Orte) ohne Regeländerung; eine Kopplung „Punkt zahlt weniger, wenn die Häuser zerstört sind“ wäre eine eigene Balancing-Frage. |
| 4 | **Haus-Abwechslung im Shader**, deterministisch aus der Position (Putzfarbe aus 7 Tönen, Dachdeckung rot/braun/verwittert/Zementziegel/Schiefer, Backsteinton, Brettverwitterung). Keine neuen Modellvarianten, keine neuen Daten. **[vorläufig]** | Billig (kein Speicher, kein Draw-Call mehr), gilt sofort für alle Karten; Dachformen (Walmdach) bräuchten neue Modelle. |
| 5 | **Verschneite Bäume:** Schnee aus der Modell-Normale (Objekte drehen sich nur um die Hochachse), Nadelzweige tragen mehr Schnee, liegende Objekte schwanken nicht. | Ein umfallender Baum behält so seinen Schnee; vorher wanderte er auf die neue Oberseite. |
| 6 | **Strohreste** als Darstellung: flache, trockene Grasbüschel im Ring um jeden Heuhaufen (eine MultiMesh). Winter dünnt Gras/Schilf/Steine aus. | Reine Optik, keine Sim-Objekte. |
| 7 | **Basis-Optik:** neue Env-Modelle `env_steel_hall` (Fahrzeugwerk), `env_barracks` (Kaserne), `env_depot` (Depot mit Kisten und Fässern) im Baukasten (`base_buildings.py`, auf dem `house`-Generator), Maße = `data/economy.toml`; Hauptquartier bleibt das Steinhaus. `just thumbnails` neu gerendert. | Maße unverändert → keine Regeländerung, Hash bleibt; drei Schadensstufen wie bisher. |
| 8 | **Personal:** Prüfung mit Preisen und Einkommen (Tabelle in `GAME_DESIGN.md`): Personal begrenzt schon die Infanterie, ohne Punkte sinkt es am wenigsten (65 %). Keine Zahlenänderung, Abgleich in 045. **[vorläufig]** | Balancing gehört in die Spieltests von 045; hier nur die Prüfung. |
| 9 | **Hinterhalt als Sim-Regel** (`data/vision.toml` `[ambush]`): stehender Trupp im Waldfeld oder im Gebäude, 20 s ohne Schuss → nur für Feinde ≤ 50 m (200 m real) sichtbar; der Schuss enthüllt 20 s. Zustand erst gehasht, wenn ein Trupp versteckt war. **[vorläufig]** | Muster der Tarnnetze (052); der Golden-Hash bleibt, weil die Demo-Trupps nie im Waldfeld stehen. |

## Ergebnis

- **Karten** (`cli gen-land`, `just gen-land [map]`, `crates/cli/src/gen_land.rs`):
  `winter_village` (63 Gebäude, 5 201 Bäume, 17 % Schneeverwehung),
  `farms` (33 Gebäude, ~2 000 Heckensegmente, 190 Heuhaufen),
  `river_land` (3 Brücken, 2 Furten, 29 Gebäude); je 1 km, 501 × 501
  Stützstellen, vier Starts. Ein Test erzeugt jede Karte zweimal (gleiche
  Dateien) und prüft Starts, Versorgungspunkte, Gebäude, Heu, Bäume,
  Grabenmarker, Hecken und Brücken. Doku: `docs/MAPS.md` „Landscape maps“.
- **Versorgungspunkte an Orten:** Dorfplatz, Grabenstellung auf dem Hügel
  (052), Gehöft (winter_village); fünf Gehöfte (farms); drei Brückenköpfe
  und zwei Gehöfte (river_land).
- **Abwechslung:** Putzfarbe, Dachdeckung, Backstein- und Brettton je Haus
  aus der Position (`env.gdshader`), Wellblech-Rippen; Schnee bleibt auf
  gefällten Bäumen, Nadelbäume verschneiter, liegende Objekte ohne Wind;
  Strohreste um Heuhaufen; Gras im Winter ausgedünnt (`map_objects.gd`).
- **Basis:** `env_steel_hall` (Fahrzeugwerk), `env_barracks` (Kaserne),
  `env_depot` (Depot mit Kisten und Fässern), je 1 576–1 600 Dreiecke,
  drei Schadensstufen, `base_view.gd` nutzt sie; Bilder neu (`just thumbnails`).
- **Personal:** Prüfung in `docs/GAME_DESIGN.md` („Manpower as a limit“),
  keine Zahlenänderung.
- **Hinterhalt:** `data/vision.toml` `[ambush]`,
  `crates/sim/src/sim/vision/ambush.rs`, drei Tests (`tests/ambush.rs`);
  der Deckungstest in `tests/infantry.rs` läuft ohne die Regel (er misst
  Treffer auf einen Trupp, der sonst unsichtbar bliebe).
- **Golden-Hash unverändert** (`0x9ca9_6477_eff5_75ef` nach dem letzten Merge mit
  `main`, Basisbau-Hash ebenso): der Hinterhalt wird erst gehasht, wenn ein
  Trupp versteckt war, und die Basismodelle ändern keine Regel.
- **Prüfungen:** `just check`, `map-check` (alle drei ok, 0 Warnungen),
  `editor-selftest`, `selftest`, `gallery-check`, `validate-assets`,
  `kit-test` grün; Testkarten erzeugen sich unverändert (Ausnahme seit 035:
  `twin_meadow` enthält zwei von Hand gesetzte Minenfelder, die der
  Generator nicht schreibt).

**Messungen** (Apple M1 Max, Release, Maschine durch andere Sitzungen
belastet):

| | winter_village | farms | river_land |
|---|---|---|---|
| Laden + Prüfen (`map-check`) | 82 ms | 103 ms | 97 ms |
| `path-bench` 200 Einheiten: Planung Mittel / p99 | 7,6 / 9,2 ms | 7,0 / 8,8 ms | 5,2 / 7,0 ms |
| `vision-bench` 500 Einheiten: Mittel / p99 | 6,1 / 9,6 ms | 7,0 / 9,9 ms | 4,9 / 8,4 ms |
| `terrain-bench` 1080p nah / 120 m / 600 m | 55 / 39 / 94 FPS | 99 / 79 / 88 FPS | 108 / 110 / 92 FPS |

Die Winterwerte nah/mittel liegen niedriger, weil der feste Messweg der
Bank (ab (−300, 200)) dort über einem dichten Wald liegt (640 Bäume im
Umkreis von 120 m, `farms` 106); Wetter, Biom und die neuen Shader ändern
daran nichts (gemessen: gleiche Werte mit klarem Wetter, Waldbiom und den
alten Shadern).

KI-Turnier (`ai-tournament --a hard --b normal`, je 4 Spiele): Schlacht
(Demo-Armeen, 6 000 Ticks): winter_village 4 Zeitablauf (Schnee, 840 m
Startabstand), farms 2 Siege Normal + 2 Zeitablauf, river_land 1 : 3
entschieden. Basisbau (24 000 Ticks = 20 Spielminuten): auf allen drei
Karten 4 Zeitablauf; die KI braucht für 1-km-Karten länger als 20 Minuten.

Bilder: `build/maps2/<karte>/` (Übersicht, flach, gefällte Bäume, jeder
Versorgungspunkt mittel und nah), erzeugt mit `SHOT_SET=land just
terrain-shots <dir> <karte>`.

**Vorläufig (bitte bestätigen):** Punkte an Orten ohne eigene Regel
(Gebäude gehören nur optisch dazu); Farb- und Dachpalette der Häuser;
Hinterhalt (50 m Entdeckung, 20 s Ruhe, nur stehende Trupps im Waldfeld
oder im Gebäude); Personal-Zahlen unverändert; Maße der drei Basisbauten
(= Wirtschaftsmaße); Schneeverwehungen als Gelände (60 % Tempo) auf 17 %
der Winterkarte; Standardwetter Schnee bzw. bedeckt.

**Offen:**
- Basisbau-KI: Spiele auf 1-km-Karten werden in 20 Minuten nicht entschieden
  (Abgleich mit 045/051).
- Dachformen (Walmdach, Krüppelwalm) als Modellvarianten; Hauptquartier
  ohne eigenes Modell; Baugerüst statt skalierter Baustelle.
- Anzeige „im Hinterhalt“ für eigene Trupps im HUD; KI nutzt den Hinterhalt
  nicht gezielt (047/051).
- Versorgungspunkte im Karteneditor setzen (bisher nur Generator/Text);
  Gebäude-Zustand als Einfluss auf das Einkommen eines Punkts.
- Strohreste brennen nicht mit; Schnee rieselt beim Fallen nicht ab.
