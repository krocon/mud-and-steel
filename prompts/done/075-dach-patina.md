# Prompt 075 – Hausdächer mit Patina

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ASSETS.md` den Abschnitt „Environment objects (prompt 007)“ mit „Structure:
> variants“ und „Materials and shaders“ und den Abschnitt „Thumbnails“, in
> `docs/ARCHITECTURE.md` „Map objects and decoration“ (darin „Variety (prompt 056)“)
> und bei „Terrain rendering“ die Ergebnisse zum Flimmern (Prompt 068 und 071). Dann
> den Code: `godot/assets/shaders/env.gdshader` (Zweig „15 Roof“ und die Slots Metal,
> Wood, Plaster), `env_common.gdshaderinc`, `godot/assets/shaders/env_look.gd` und
> die Dachfunktionen in `tools/blender/kit/buildings.py` (`gable_roof`, `flat_roof`).

**Vorlage:** `.temp/haus-daecher.png` (nicht im Repo, `.temp/` ist ignoriert). Ein
Bildschirmfoto aus einem anderen Strategiespiel, 454 × 674 Pixel: ein Dorf aus kleinen
Bauernhäusern von schräg oben. Liegt die Datei nicht vor, reicht die Beschreibung in
Abschnitt 2.

## 1. Ziel

Die Dächer der Häuser sehen alt und benutzt aus: verwittert, fleckig, ausgebessert,
bemoost, mit Schmutzfahnen und nachgedunkelten Ziegeln. Heute sind sie zu sauber und
zu gleichmäßig. Aus der Spielkamera sind die Dächer der größte Teil jedes Hauses; ihre
Patina bestimmt, ob ein Dorf gebaut oder gewachsen wirkt.

Fertig ist der Prompt, wenn

- ein Dorf im Spiel neben der Vorlage liegt und die Dächer in Struktur, Fleckigkeit
  und Helligkeitsverlauf dazu passen,
- jede Deckung ihre eigene Patina hat (Tonziegel, Schiefer, Stroh, Bretter,
  Wellblech) und kein Dach wie das Nachbardach aussieht,
- die Patina bei 15, 35, 110 und 350 m Kameraabstand wirkt und beim Schwenken über
  ein Dorf nichts flimmert,
- Schnee, Wüstenstaub, Brandspuren und beschädigte Dächer weiter stimmen,
- die Bildrate über einem Dorf vorher und nachher gemessen ist und nicht merklich
  fällt,
- `just check`, `just validate-assets` und `just selftest` grün sind.

## 2. Die Vorlage

Das Bild ist **Vorbild für Wirkung und Aufbau**, kein Material: keine Bildausschnitte
und keine Texturen daraus, nichts aus anderen Spielen (`docs/SOURCES.md`). Alle Muster
kommen wie bisher aus dem Shader. Das Bild ist klein; es taugt für Verhältnisse und
Farben, nicht für Einzelheiten.

Was die Dächer in der Vorlage ausmacht:

- **Ziegelreihen bleiben lesbar.** Waagerechte dunkle Fugen gliedern jede Dachfläche,
  auch aus der Entfernung, und sie laufen nicht ganz gerade.
- **Jeder Ziegel hat seinen eigenen Ton.** Helle, orangerote neben dunklen,
  braunroten Ziegeln; stellenweise Gruppen neuerer Ziegel, wo ausgebessert wurde.
  Die Unterschiede sind deutlich stärker als heute im Spiel.
- **Schmutzfahnen laufen die Dachneigung hinab.** Dunkle Streifen vom First und vom
  Schornstein abwärts, zur Traufe hin wird das Dach dunkler und schmutziger.
- **Flecken in großem Maßstab.** Ganze Bereiche einer Dachfläche sind dunkler,
  ausgeblichener oder grünlicher als der Rest.
- **Jedes Dach hat seine eigene Grundfarbe.** Von leuchtendem Ziegelrot über
  Rotbraun bis Dunkelbraun.
- **Graue Holz- und Strohdächer** auf Scheunen und Schuppen: ausgeblichen
  silbergrau bis graubraun, mit dunklen Fugen und ungleichmäßigen Bahnen.
- **Licht- und Schattenseite** unterscheiden sich stark; die Dächer wirken
  plastisch. Das ist Sache des Lichts, aber die Patina darf den Unterschied nicht
  einebnen.

## 3. Ausgangslage

Der Zweig „15 Roof“ in `env.gdshader` zeichnet heute:

- waagerechte Reihen (sechs je Meter) als leichte Abdunkelung und je Ziegel eine
  zufällige Helligkeit von ± 10 %,
- je Haus eine von drei Ziegelfarben, etwa jedes fünfte Haus dunkle Zementziegel
  (Prompt 056, aus `v_seed`, der Position des Hauses),
- auf Tonziegeln schwache Moosflecken aus Rauschen, in der Wüste keine,
- Schiefer, Stroh und Bretter je mit einer Grundfarbe und kaum Struktur.

Es fehlen: Schmutzfahnen entlang der Neigung, Verlauf vom First zur Traufe,
ausgebesserte Stellen, starke Unterschiede von Ziegel zu Ziegel, Flechten, Alter je
Haus, und jede Patina auf Schiefer, Stroh, Brettern und Wellblech.

Halte den Stand vor der ersten Änderung als Bildreihe fest (Abschnitt 7).

## 4. Was zu bauen ist

Die Patina entsteht im Shader, aus Schichten, die sich je Deckung unterscheiden. Was
genau, entscheidet der Vergleich mit der Vorlage; die Tabelle ist der erwartete Umfang.

| Schicht | Wirkung |
|---|---|
| Alter je Haus | Aus `v_seed`: wenige neue Dächer, viele gealterte, einige sehr alte. Das Alter steuert die Stärke aller anderen Schichten. |
| Ton je Ziegel | Deutliche Unterschiede in Helligkeit und Farbton von Ziegel zu Ziegel, nicht nur ± 10 %. |
| Ausbesserungen | Gruppen von einigen bis einigen Dutzend Ziegeln in frischerem oder fremdem Ton, je Haus an anderer Stelle, auf neuen Dächern keine. |
| Schmutzfahnen | Dunkle, unregelmäßige Streifen in Richtung der Dachneigung, dichter unter First und Schornstein. |
| Verlauf First zu Traufe | Zur Traufe hin dunkler und schmutziger, der First heller und ausgewaschen. |
| Flecken | Große, weiche Bereiche: nachgedunkelt, ausgeblichen, feucht. |
| Bewuchs | Moos in Fugen und an der Traufe, Flechten als helle graugelbe Tupfen; mehr auf der sonnenabgewandten Seite und auf alten Dächern; in der Wüste keiner. |
| Reihen | Fugen kräftiger und leicht wellig, einzelne verrutschte Ziegel als kurze Versätze. |

**Je Deckung:**

- **Tonziegel:** alle Schichten. Das ist das Dach der Vorlage.
- **Schiefer:** Platten in leicht verschiedenen Grautönen, helle Kalk- und
  Flechtenflecken, dunkle Wasserläufe, einzelne ersetzte Platten.
- **Stroh:** Bahnen statt Reihen, zum First hin heller, an der Traufe dunkel und
  feucht, Moospolster, ausgefranste dunkle Stellen.
- **Bretter und Schindeln:** silbergrau ausgeblichen, dunkle Fugen, einzelne
  dunklere oder neuere Bretter, grüne Ansätze an der Wetterseite.
- **Wellblech** (Slot Metal, Halle und Depot aus Prompt 056): Rostfahnen an Nähten
  und von den Befestigungen abwärts, matte und blanke Bahnen.
- **Flachdächer der Wüstenhäuser** (Slot Plaster): Schmutzränder an der Brüstung,
  Wasserflecken, feine Risse, Staub in den Ecken. Nur wenn es ohne Umbau der
  Wandfarben geht; sonst offener Punkt.

Die Farben sind Urteilswerte wie die Wandfarben aus Prompt 056 und stehen als
benannte Werte mit Kommentar im Shader. Sie sollen zu ländlichen Häusern in Mittel-
und Osteuropa um 1940 passen: gebrannter Ton, Reet und Stroh, graue Schindeln.

## 5. Regeln für den Shader

- **Fest am Haus.** Jede Schicht hängt nur von der Position am Modell und von
  `v_seed` ab. Dasselbe Haus sieht immer gleich aus, in jedem Spiel und auf jedem
  Rechner, und nichts wandert bei Kamerabewegung.
- **Nichts flimmert.** Ziegelreihen von 17 cm sind ab etwa 100 m Abstand kleiner als
  ein Pixel. Jede feine Schicht wird nach der Pixelgröße geglättet und ausgeblendet,
  bevor sie flimmert; die groben Schichten (Alter, Flecken, Fahnen, Verlauf) tragen
  die Ferne. Prüfe das auch für die Reihen, die es heute schon gibt.
- **Reihenfolge bleibt:** Patina vor Kantenabnutzung, Staub, Schnee und Brand. Unter
  Schnee und Ruß bleibt sie nur, wo diese sie durchscheinen lassen.
- **Beschädigte Dächer** (`Damaged`, mit Löchern) und Trümmer mit Slot Roof bekommen
  dieselbe Patina wie das heile Dach desselben Hauses.
- **Kosten.** `env.gdshader` zeichnet alle Umwelt-Objekte. Die neuen Schichten
  kosten nur auf Dachpixeln; Schattendurchgänge bleiben unberührt.
- Nur Darstellung: kein Rust-Code in `crates/sim`, der goldene Hash bleibt gleich.

**Was der Shader vom Dach weiß.** Er kennt die Position am Modell, die Normale und
die gebackenen Vertexfarben (R Umgebungsverdunkelung, G Kantenabnutzung, B Höhe über
dem Boden geteilt durch 4 m). Die Richtung der Dachneigung folgt aus der Normale.
Wo First und Traufe liegen, weiß er nicht.

- **Stufe 1, ohne Modelle anzufassen:** alle Schichten aus dem, was da ist. Fahnen
  laufen entlang der Neigung, der Verlauf zur Traufe wird aus der gebackenen
  Verdunkelung und der Höhe angenähert.
- **Stufe 2, nur wenn Stufe 1 im Vergleich sichtbar zu kurz greift:** der Baukasten
  schreibt je Dachfläche den Abstand zu First und Traufe und die Lage des
  Schornsteins ins Modell (zweite UV oder ein freier Kanal). Das heißt: alle Häuser
  neu bauen (`just env`), prüfen und neue Vorschaubilder. Zeig vorher die Bilder
  von Stufe 1 und begründe, warum sie nicht reichen.

## 6. Biome und Tageszeit

- **Wald:** die volle Patina mit Moos und Flechten.
- **Wüste:** kein Bewuchs, dafür ausgeblichen, staubig, sandfarbene Ablagerungen in
  den Fugen (`env_dry`, `env_dust`).
- **Winter:** Schnee liegt wie bisher auf den Dachflächen; wo er dünn ist, scheint
  die Patina durch.
- Bei flacher Sonne und bei Nacht ansehen: die Patina darf die Dächer nicht zu
  dunklen Flächen machen.

## 7. Prüfen

1. **Vergleichsbild.** Ein Dorf im Spiel bei ähnlichem Blickwinkel und Maßstab neben
   der Vorlage, vorher und nachher. Nach jeder Änderungsrunde neu.
2. **Bildreihe.** Eine neue Auswahl für `just objects-shots` (oder ein eigenes
   Rezept), die Dächer zeigt: ein Dorf bei 15, 35, 110 und 350 m, eine Reihe mit
   allen Deckungen nebeneinander, beschädigte Häuser, ein brennendes und ein
   ausgebranntes Haus, alle drei Biome, Tag, flache Sonne und Nacht. Die
   Modell-Galerie (`just gallery`) für die Nahsicht.
3. **Vielfalt.** Ein Bild mit zwanzig gleichen Häusern nebeneinander: keine zwei
   Dächer sehen gleich aus, und keines sieht wie ein Muster aus.
4. **Bewegung.** Kamerafahrt über ein Dorf (`town_small`) aufnehmen und ansehen, ob
   die Dächer flimmern; mit dem Flimmermaß aus Prompt 071 (`tools/ground_flicker.py`)
   vorher und nachher messen, mit einem Messfenster auf einer Dachfläche.
5. **Bildrate.** `just objects-bench` vorher und nachher, die Zeile „Village“.
6. `just check`, `just validate-assets`, `just selftest`.

Die Bilder dem Nutzer zeigen. Was nicht erreicht wurde, steht im Abschlussbericht mit
Grund, nicht als erledigt.

## 8. Abschluss

- **Vorschaubilder.** Die Bilder aller Gebäude zeigen ihr Dach und sind nach der
  Änderung veraltet, auch wenn kein Modell neu gebaut wurde. Alle Katalogeinträge
  mit Dach neu rendern: `just thumbnails --only=<id>,<id>` (braucht ein Fenster),
  danach `just units-web-sync`.
- `docs/ASSETS.md` („Materials and shaders“: die Schichten, was je Deckung gilt; bei
  Stufe 2 der neue Kanal und die Prüfung), `docs/ARCHITECTURE.md` („Map objects and
  decoration“: ein Absatz neben „Variety (prompt 056)“, Messwerte, offene Punkte).
  Auf Englisch, ohne das Bild der Vorlage.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, in sinnvollen Schritten (Tonziegel, übrige Deckungen,
  Vorschaubilder). Kein `git push`.

## 9. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Patina an Wänden:** Wasserränder am Sockel, abgeplatzter Putz, Ruß über
  Fenstern.
- **Dachform:** durchhängende Firste, schiefe Traufen, Firstziegel, Dachrinnen,
  Gauben, neue Schornsteine.
- **Rauch aus Schornsteinen.**
- **Eigene Dächer je Landschaft** (Holzschindeln im Osten, Reet im Norden).

## Entscheidungen (autonom, Nachtlauf 17.11.2026)

| Frage | Entscheidung (vorläufig) |
|---|---|
| Stufe 1 oder 2? | Stufe 1 reicht im Vergleich: Abstand zum First aus der Lage über dem Modell-Ursprung (Häuser haben den First darüber), Schornstein-Fahnen bleiben offen. Keine neuen Kanäle in den Modellen. |
| Stroh fehlt auf Gebäuden | `env_barn` bekommt Strohdach (`roof_kind = 2`), neu gebaut. |
| Wellblech am Depot | Das heile Depot hatte glattes Stahlblech; jetzt verzinktes Wellblech (`kit/base_buildings.py`), neu gebaut. |
| Farben | Dachfarben in sRGB gesetzt und am Ende linearisiert; die alten linearen Werte wirkten blass lachsfarben. Urteilswerte. |
| Ziegelreihen | Die alten Reihen liefen die Neigung hinab statt längs zum First; neu längs, gefiltert statt ausgeblendet. |
| Brennendes Haus in der Bildreihe | Ohne Flammen (die kommen aus der Simulation): beschädigtes Haus mit Ruß 0,6 und 1,0. |
