# Prompt 125 – Karte nach Bildvorlage: Front an der Flussgabel (unsymmetrisch)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 124
> (Grabenlinie) ist erledigt (`prompts/done/124-*`): Er bringt die Skizzen-Bausteine
> `[[trench]]`, `[[crater_field]]`, `[[wire]]`, Pfützen und den Schlamm-Look. Lies
> zuerst `CLAUDE.md`, dann `prompts/done/124-karte-grabenlinie.md` und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, das Ergebnis von Prompt 123
> (Reichweitentabelle), falls erledigt, in `docs/MAPS.md` „Maps from picture
> sketches (prompt 088)“ ganz mit der Karte `trench_line`, „Bridges“, in
> `docs/ARCHITECTURE.md` Sicht („Seeing downhill“), indirektes Feuer und
> Wegfindung, `data/map_sketches/trench_line.toml` und `river_fork.toml` als
> Muster, `crates/cli/src/gen_sketch.rs`, `data/scale.toml` (`[range]`),
> `docs/AI.md`.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/flussgabel/flussgabel.webp`.
> Eine mit KI gezeichnete, frei erfundene Lagekarte im Stil der Westfront; die
> Beschriftungen sind verstümmelt und werden **nicht** übernommen (keine Orts- oder
> Flussnamen in Karte, Markern, Menüs). Verwendet wird nur die Landschaft.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach jeder Stufe Bilder aus dem Spiel neben
> der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `river_fork_front` (im Spiel „Front an der Flussgabel“ /
„River fork front“) nach der Vorlage, mit den Bausteinen aus Prompt 124. Sie ist
**bewusst unsymmetrisch**: Eine Seite hält Hügel mit Wald, die andere liegt in der
Ebene mit Dörfern. Das Gleichgewicht kommt aus Ausgleich, nicht aus Spiegelung.

Die Vorlage zeigt:

- **drei Flüsse, die sich in der Mitte treffen:** der Hauptfluss schräg von
  Nordwest nach Südost, ein Nebenfluss von Nordosten, einer von Süden; mehrere
  Brücken, einige mitten im Grabengebiet;
- **zwei Fronten** quer über die Karte von Südwest nach Nordost, die die Flüsse
  mehrfach überqueren: tiefe Grabensysteme mit mehreren Linien, dazwischen
  Niemandsland mit Trichtern;
- **Hügelseite (Nordost):** wellige Höhen mit Höhenlinien, Wald am Rand, eine
  Batterie mit mehreren Geschützen am Waldrand, Dörfer und Gehöfte hinter der Front;
- **Ebene (Südwest):** flach, Dörfer mit Feldern, eine Batterie in einem Dorf,
  Straßen in alle Richtungen;
- verstreute Trichter auch hinter den Linien, Wald im Nordwesten.

## 2. Unsymmetrisch und trotzdem ausgeglichen

- **Hügelseite (Verteidiger):** Höhe gibt Sicht nach unten (Sichtregeln mit
  Geländehöhe) und gute Feuerstellungen am Hinterhang. Dafür: weniger Straßen, die
  Grabenlinien liegen am Vorderhang (von unten einsehbar), nur zwei Brücken auf
  ihrer Seite der Flüsse, Versorgungspunkte weiter auseinander.
- **Ebene (Angreifer):** mehr Brücken und Übergänge (Furten), Dörfer als Deckung
  bis nah an die Front, mehr Versorgungspunkte in Reichweite, kürzere Wege für
  Nachschub; aber jede Bewegung im Blick der Höhen.
- **Fronten über die Flüsse:** Jede Seite hat Abschnitte auf beiden Ufern; die
  Brücken in den Gräben sind Schlüsselstellen. Brücken achsparallel (Nord-Süd oder
  West-Ost), der Fluss wird an jeder Brücke kurz gerade geführt wie bisher im
  Generator.
- **Abstände nach Reichweite** (indirekt 1/8, Sicht 1/4; Tabelle wie in 123/124 in
  `docs/MAPS.md`): Mörser über das Niemandsland, leichte Haubitzen aus den Batterien
  bis zur gegnerischen Front, nicht bis zur gegnerischen Batterie. Die
  Batteriestellungen der Vorlage (Waldrand Nordost, Dorf Südwest) als vorbereitete
  Feuerstellungen.
- **Starts:** 1 gegen 1 und 2 gegen 2; Team Nordost auf den Hügeln hinter dem Wald,
  Team Südwest hinter den Dörfern; außerhalb der Reichweite leichter gegnerischer
  Artillerie.
- **Versorgungspunkte:** Flussgabel in der Mitte (umkämpft), Brücken in den Gräben,
  ein Dorf auf jeder Seite der Front, die Höhe über der Gabel, das Wäldchen im
  Nordwesten.
- **Ausgleich messen:** KI-Turnier über viele Seeds mit vertauschten Seiten (jede
  KI-Stufe spielt beide Seiten gleich oft). Ziel: keine Seite gewinnt deutlich über
  60 %. Liegt es daneben, an den Stellschrauben drehen (Brücken, Furten,
  Versorgungspunkte, Lage der Gräben, Höhe der Hügel), nicht an Regeln oder
  Katalogwerten. Ergebnis und Stellschrauben in `docs/AI.md` und `docs/MAPS.md`.

## 3. Stufen

**Stufe 1 – Skizze und Karte:** `data/map_sketches/river_fork_front.toml` nach der
Vorlage, etwa 3,6 km × 2,0 km (Seitenverhältnis der Vorlage, höchstens 4 km), Biom
`forest`, Schlamm-Look aus 124. Flüsse, Hügel (Höhe so, dass die Sicht nach unten
wirkt, Vorschlag 30–60 m; der Generator muss das tragen), Grabensysteme mit
mehreren Linien und Laufgräben, Draht mit Gassen, Trichterfelder dicht im
Niemandsland und lose dahinter, Dörfer, Felder in der Ebene, Wald, Wege,
Batteriestellungen. `just gen-sketch river_fork_front`, `just map-check`,
`just map-reach` (alle Starts, Stellungen und Punkte für Fahrzeuge mit gezogenem
Geschütz erreichbar), Sichtfeld-Bilder von Hügeln, Batterien und vordersten
Gräben, Reichweitentabelle.

**Stufe 2 – Ausgleich und Abschluss:** KI-Turnier mit vertauschten Seiten, Nachjustieren,
Messungen (Bildrate, Wegsuche im Trichterfeld) in `docs/PERFORMANCE.md`,
Menüname und Übersetzung, Tabelle der Skizzenkarten in `docs/MAPS.md` (mit „bewusst
unsymmetrisch“ und den Stellschrauben), Test wie bei den anderen Skizzenkarten
(zweimal erzeugt, gleiche Dateien). `just check`, `just selftest`. Der Golden Hash
ändert sich nicht (neue Karte); wenn doch, Ursache finden.

## 4. Nicht Teil dieses Prompts

- Eine Mission „Angriff auf die Höhen“ auf dieser Karte (sinnvoller Folgeauftrag:
  Spieler greift aus der Ebene an).
- Neue Skizzen-Bausteine; was fehlt, als offener Punkt für 124 bzw. einen Folgeauftrag.
- KI-Verbesserungen für Angriffe über Flüsse und gegen Höhen.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 24.11.2026, alle vorläufig)

| # | Frage | Entscheidung | Warum |
|---|---|---|---|
| 1 | Fronten und Seiten | Fronten von West-Südwest nach Ost-Nordost (Grundlinien y = −0,06 x ∓ 140, Stützlinien ∓ 260), Team 0 Nord (Höhen im Nordosten, dazu Land westlich des Hauptflusses und zwischen den Flüssen), Team 1 Süd (Ebene). **[vorläufig]** | So liegen die Gräben der Vorlage; „Südwest nach Nordost“ als Verlauf der Linien gelesen, Hügel- und Ebenenseite dann Nord bzw. Süd. |
| 2 | Flüsse | Hauptfluss 56 m als eine Linie NW→SO durch die Gabel, Nebenflüsse 36 m enden auf ihm; Gabel mitten im Niemandsland; deutlich schmaler als in der Vorlage. **[vorläufig]** | Breite Flüsse wären nur mit langen Brücken zu queren; eine durchgehende Linie verhindert einen Flachwasserring an der Gabel (endende Linien ergaben ihn im ersten Entwurf). |
| 3 | Fluss durch Hügel | Neues, rückwirkungsfreies Skizzenfeld `[[river]] through_hills` / `valley_m` (Wasser auf Höhe der Ebene, Ufer fallen über die Talbreite). **[vorläufig]** | Ohne es stieg das Wasser des Nebenflusses mit den Hügeln; nötig, damit der Generator die Höhen trägt (Prompt). Für 126/127/131/132 wiederverwendbar; alle anderen Skizzenkarten bitgleich. |
| 4 | Hügelform | Glacis aus breiten Buckeln (19 m, r 460) plus Kammbuckel (18 m, r 230): Front 4–6 m hoch auf ≤ 8 % Hang, Stützlinie 13–16 m, Kamm 30–40 m statt 30–60 m. **[vorläufig]** | Auf 20 % Hang sperrten die gegrabenen Gräben (75 %-Schnitt + Brustwehr) die Wegzellen für alle; Kammhöhe 26 → 18 m als Stellschraube im Turnier. |
| 5 | Brücken und Furten | Nord: 2 hinter den Linien + 2 im Grabensystem, keine Furt; Süd: 2 im Grabensystem + 4 Brücken und 2 Furten dahinter. **[vorläufig]** | Prompt (Hügelseite nur zwei Brücken, Ebene mehr Übergänge). |
| 6 | Versorgungspunkte | Gabel (neutral, Landzunge zwischen Hauptfluss und Südfluss), Höhe über der Gabel, je eine Grabenbrücke, je ein Dorf, Wäldchen NW (Nord), Mitteldorf und Weiler östlich des Hauptflusses (Süd): 4 + 4 + 1. **[vorläufig]** | Prompt-Liste; Nordpunkte über drei Sektoren verstreut, Südpunkte näher an den Starts. |
| 7 | Batterien | Je zwei: Nord am Waldrand NO und hinter dem Kamm (Hinterhang), Süd im Batteriedorf und südlich des Dorfs am Südfluss; Gegenstellungen ≥ 1 865 m (kein Geschütz reicht), Starts ≥ 2 190 m, gegnerische Front 617–950 m. **[vorläufig]** | Reichweitenregel (indirekt 1/8). |
| 8 | Starts | Nord (1 000, −910), (1 590, −930) in der Senke hinter dem Kamm; Süd (−990, 830), (−1 560, 800) hinter den Dörfern. **[vorläufig]** | Starts auf steilem Rückhang erzeugten steile Ränder; Süd-Starts 70 m nach vorn als Stellschraube (y 900 → 830). |
| 9 | Messung des Ausgleichs | Gleiche KI und gleiche Fraktion auf beiden Seiten (de, su, us, uk), Seiten abwechselnd; `ai-tournament --verbose` nennt dafür die Siegerseite. **[vorläufig]** | Die Standardarmeen (de gegen su) verzerren (wie auf `river_front` dokumentiert). |
| 10 | Trichter | 65 je ha im Niemandsland, 16 um die Gräben, 3 dahinter (3 669 Trichter). **[vorläufig]** | Bildrate und `Sim::new` wie bei 124; Niemandsland bleibt dicht. |
| 11 | Look, Wetter | Look `mud`, Wetter `clear`. **[vorläufig]** | Prompt; Bewölkung würde die Sicht- und Reichweitentabelle verändern. |
| 12 | Werkzeuge | `just map-sight` zeigt auch die Sicht aus `front_*`-Markern; Beobachtungspunkte `height_n_crest_*`. **[vorläufig]** | Sichtfeld-Bilder aus den vordersten Gräben (Prompt). |

### Ergebnis

- **Karte** `data/maps/river_fork_front/` aus `data/map_sketches/river_fork_front.toml`
  (3,6 × 2,0 km, Biom `forest`, Look `mud`), im Menü „Front an der Flussgabel“ /
  „River fork front“: drei Flüsse, die sich mitten im Niemandsland treffen, zwei
  Grabensysteme mit Front- und Stützlinien, 15 Laufgräben, 50 MG-Nestern, 148
  Unterständen, zwei Reihen Draht mit Lücken, zwei Gassen, 3 669 Trichter; Höhen
  mit Wald im Nordosten, Ebene mit Dörfern, Feldern und Straßen im Südwesten; zehn
  Brücken, zwei Furten, vier Feuerstellungen, neun Versorgungspunkte.
- **Generator**: `[[river]] through_hills`, `valley_m` (in `docs/MAPS.md`);
  alle anderen Skizzenkarten bitgleich. `map-sight` auch für `front_*`;
  `ai-tournament --verbose` nennt die Siegerseite.
- **Prüfungen**: `map-check` ok; `map-reach` alle Starts, Punkte, Stellungen,
  Brücken, Furten, Gassen im Hauptteil (91,5 % Fahrzeuge); `stuck-audit` 464/464;
  Test `river_fork_front_ranges_and_sight`, Karte im Test „zweimal erzeugt,
  gleiche Dateien“; Golden Hash unverändert.
- **Ausgleich** (gleiche KI, gleiche Fraktion beidseitig): normal 146 : 108 (57 %
  Nord), schwer 18 : 14 (56 %); je Fraktion schwankend (us Nord, su Süd).
- **Messungen**: path-bench Planung 6,9 ms, alle Ticks 0,55 ms; Bildrate hoch
  35 (dichter Wald) bis 116 FPS, mittel ≥ 69,8 FPS.
- **Bilder**: `build/river_fork_front/stage1/` (Stufe 1), `build/river_fork_front/final/compare_{top,oblique,fork,hill}.png`,
  `reach.png`, `sight.png`.
- **Offen**: KI für Angriffe über Flüsse, gegen Höhen, in Gräben (eigener Prompt);
  Fraktionsabhängige Seitenneigung; dichter Wald < 60 FPS auf „hoch“; heller
  Wasserstreifen an der Gabel (Darstellung der Wasseroberfläche, Gelände ist tief);
  Mission „Angriff auf die Höhen“; Vorlage zeigt breitere Flüsse und mehr Gräben.

### Zweiter Aufbau (24.11.2026, auf Wunsch des Nutzers)

Der erste Aufbau (zwei gerade Grabengürtel quer über die Karte, schmale Flüsse) traf
die Vorlage nicht. Geändert in `data/map_sketches/river_fork_front.toml`:

- **Flüsse breiter** (Hauptfluss 90 m, Nebenflüsse 52 m); Nebenflüsse behalten in der
  Mündung ihr eigenes Bett (`joins = true`, neu im Generator).
- **Westlich der Gabel folgt die Front dem Hauptfluss**: Nordfront am Nordufer,
  Südfront 250 m davon südwestlich des Flusses, je Stützlinie und dritte Linie; die
  zwei Steinbrücken liegen zwischen den Fronten, je mit Gasse. Das Land südwestlich des
  Flusses im Nordwesten gehört jetzt zum Süden.
- Die alte Südwest-Front bleibt als **zweite Stellung** des Südens.
- **Versorgungspunkte**: `supply_fork` ins Niemandsland östlich der Gabel, die beiden
  Brückenpunkte neben die Brücken, `supply_n_wood` → `supply_n_west` nördlich des
  Flusses. `battery_s1` nach (−1080, 360).
- **Look `front`** (neu): Schlamm-Look bei Tageslicht, ockerfarbenes Gras, blaugraues
  Wasser.
- Reichweitentest neu gefasst (Mitte des Niemandslands westlich der Gabel entlang des
  Flusses); `map-check`, `map-reach`, `stuck-audit` (447 von 448) bestanden.
- **Offen**: KI-Turnier und Bildrate nicht neu gemessen; der Südzufluss mündet weiter
  an der Gabel statt östlich davon; die Höhe im Nordosten hat keine Ringgräben; die
  Dörfer sind dünner als in der Vorlage.

