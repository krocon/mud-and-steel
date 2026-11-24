# Prompt 126 – Karte nach Bildvorlage: Tafelberge in der Wüste

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md`, `prompts/done/089-wueste.md` und
> das Ergebnis von `prompts/done/060-berge-taeler.md` (Reliefprofile, Felsbänder, Plateau), in
> `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz, „Relief profiles
> and mountains (prompt 060)“, die Hangregeln und die Wüstenkarten
> (`desert_lagoon`, `desert_village`), in `docs/ARCHITECTURE.md` Wegfindung und
> Hänge, Sicht („Seeing downhill“) und indirektes Feuer. Dann
> `crates/cli/src/gen_sketch.rs`, `crates/cli/src/gen_map/relief.rs`,
> `data/map_sketches/desert_lagoon.toml`, `data/terrain.toml` (`[slope]`),
> `data/objects.toml` (Felsen, Wüstenobjekte), `data/scale.toml` (`[range]`),
> `docs/AI.md`.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/tafelberge/tafelberge.webp`.
> Eine Draufsicht auf eine Wüstenlandschaft (Höhenschattierung), nur Vorbild für
> Anordnung und Formen.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Wüstenkarte `mesa_desert` (im Spiel „Tafelberge“ / „Mesas“) nach der
Vorlage: flache, sandige **Becken** (heller Sand, dunklere Senken), aus denen
**Tafelberge** mit steilen Abbruchkanten aufragen. Viele Tafelberge haben
**Stufen** (eine zweite Kante weiter innen) oder eine **Mulde** auf dem Plateau,
an den Rändern liegen große, zerklüftete Hochflächen, in der Mitte ein Ring kleiner
Zeugenberge und Felsnadeln.

Spielgedanke: **Höhe gegen Weite.** Die Plateaus sehen weit über die Becken und sind
gute Stellungen für Geschütze und Beobachter, aber nur über wenige Rampen
erreichbar. In den Becken ist Platz für Panzer, aber kaum Deckung. Zwischen den
Tafelbergen entstehen Gassen und Engpässe.

## 2. Neue Bausteine der Skizze (Stufe 1)

Der Skizzen-Generator bekommt eine Tabelle **`[[mesa]]`** (in `docs/MAPS.md`
beschreiben):

- **Umriss** als Polygon (der Fuß des Tafelbergs), **Höhe** über dem Becken
  (Vorschlag 15–40 m, die großen Randplateaus höher), **Kante:** eine steile
  Abbruchkante (steiler als 60 %, unpassierbar) mit schmalem Schutthang am Fuß
  (35–60 %, Felsbrocken als Objekte wie in Prompt 060).
- **Stufen:** optional ein oder zwei innere Umrisse mit eigener Höhe (Stufe auf dem
  Plateau) oder eine **Mulde** (innerer Umriss tiefer als das Plateau, mit Kante).
- **Rampen:** Liste von Punkten auf dem Umriss, an denen ein befahrbarer Aufgang
  (Steigung unter dem Grenzwert für Fahrzeuge, Breite für Panzer) vom Becken aufs
  Plateau führt; ebenso zwischen Stufen. Ohne Rampe ist ein Plateau nur Kulisse
  (die kleinen Felsnadeln).
- **Rauheit:** die Kanten leicht ausgefranst (Rauschen am Umriss) wie in der
  Vorlage, deterministisch, eigener Zufallsstrom.
- **Becken:** `[[paint]]` für dunklere Senken (z. B. fester Boden bzw. Kies statt
  Sand, falls es eine passende Geländeart gibt; sonst `sand` mit leichter
  Vertiefung), flache Mulden über `[[hill]]` mit negativer Höhe, falls der
  Generator das kann, sonst ergänzen.

Bestehende Skizzenkarten müssen Bit für Bit gleich bleiben (Test).

## 3. Karte nach der Vorlage (Stufe 1)

`data/map_sketches/mesa_desert.toml`, erzeugt mit `just gen-sketch mesa_desert`:

- **Größe** etwa 2,4 km × 2,4 km (quadratisch wie die Vorlage), Biom `desert`,
  Wetter klar.
- **Tafelberge nach der Vorlage nachgezeichnet:** vier große, zerklüftete
  Randplateaus (Nordwest, Nordost, Südwest, Südost) mit mehreren Stufen und Mulden,
  mittlere Tafelberge dazwischen, in der Mitte ein Ring kleiner Zeugenberge und
  Felsnadeln um ein offenes Zentrum, die großen offenen Becken im Nordosten,
  Südwesten und in der Mitte.
- **Symmetrie:** Die Vorlage ist ungefähr punktsymmetrisch. Die Karte wird es genau
  (Punktsymmetrie um die Mitte wie `city_large`): eine Hälfte nach der Vorlage
  zeichnen und spiegeln. 1 gegen 1 und 2 gegen 2.
- **Rampen:** jedes große Plateau zwei bis drei Rampen, mittlere eine bis zwei,
  Zeugenberge in der Mitte teils eine, teils keine. Die Rampen so legen, dass der
  Weg aufs gegnerische Plateau weit und einsehbar ist.
- **Starts** in den Becken hinter den Randplateaus (Nordwest/Südost bzw. alle vier
  Ecken bei 2 gegen 2), 60 m eben und frei.
- **Versorgungspunkte:** das offene Zentrum (umkämpft), zwei mittlere Plateaus mit
  Rampe (Höhe), zwei Becken, eine Mulde auf einem Randplateau (geschützt, nur über
  eine Rampe).
- **Wenig Objekte:** vereinzelte Felsbrocken am Fuß der Kanten, trockenes Gestrüpp
  in den Senken, ein bis zwei verlassene Gehöfte oder ein Brunnen aus dem
  Wüstendorf (Prompt 059) an Wegspuren; schmale Pisten (`track`) zwischen den
  Becken und zu den Rampen.

## 4. Prüfen und Abschluss (Stufe 2)

- `just map-check mesa_desert`, `just map-reach mesa_desert`: jede Rampe für Panzer
  befahrbar, alle Starts und Versorgungspunkte im Hauptteil, Plateaus ohne Rampe
  wirklich unerreichbar; Weglängen beider Seiten gleich (Symmetrietest).
- **Sicht:** Sichtfeld-Bilder von zwei Plateaus und aus einem Becken; die Plateaus
  sehen weit, aber nicht über die ganze Karte (Sicht 1/4 der echten Weite).
- **Reichweite:** Tabelle wie in Prompt 123/124 (indirekt 1/8): welches Geschütz von
  welchem Plateau welche Becken und Versorgungspunkte erreicht.
- **KI-Turnier**, Ergebnis in `docs/AI.md`: Findet die KI die Rampen, besetzt sie
  Höhen, bleiben Kolonnen am Fuß der Kanten hängen? Probleme als offene Punkte; die
  KI nicht umbauen.
- Bildrate und Wegsuche messen (`docs/PERFORMANCE.md`).
- Menüname und Übersetzung, Tabelle der Skizzenkarten in `docs/MAPS.md`, Test wie bei
  den anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
  `just selftest`. Der Golden Hash ändert sich nicht (neue Karte); wenn doch,
  Ursache finden.

## 5. Nicht Teil dieses Prompts

- Wanderdünen, Sandstürme, Hitzeflimmern.
- Höhlen oder Durchgänge durch Tafelberge.
- Ein eigenes Felswüsten-Biom (rote Felsen wie im Südwesten der USA); hier das
  vorhandene `desert`.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 24.11.2026, alle vorläufig)

| # | Frage | Entscheidung | Warum |
|---|---|---|---|
| 1 | Profil der Kante | Vom Fuß nach innen: Schutthang (ein Fünftel der Höhe, höchstens 5 m, 35 → 60 %), darüber Wand mit 180 %; der Tafelberg addiert sich zum Grund (Plateau behält das sanfte Rauschen). **[vorläufig]** | Prompt (Kante > 60 %, schmaler Schutthang 35–60 %); 180 % hält die Kante bei 35 m auf rund 30 m Breite. |
| 2 | Umriss | Polygon zweimal mit Ecken-Abschneiden (Chaikin) gerundet, dann mit drei Oktaven Rauschen (60/20/7 m, `rough_m` Standard 8 m, Stufen 0,6 davon) ausgefranst; eigener Rauschstrom (Salz 126–128). **[vorläufig]** | Grob gezeichnete Polygone sahen eckig aus (erster Entwurf: Sechsecke als Mulden). |
| 3 | Rampen | 20 % Steigung, 18 m Sohle mit Feldweg, halb in die obere Ebene eingeschnitten (Felswände 150 %), halb aufgeschüttet (Flanken 50 %); eine 35-m-Rampe ist 175 m lang; Marker `ramp_<name>_<k>` hinter jeder Rampe (Bilder `_image`). **[vorläufig]** | Unter dem Grenzwert 60 %, bergauf 60 % Tempo; 8 m Panzer + 2 m Rand auch schräg zum Wegraster. 70 % Einschnitt führte auf dem kleinen Nordwest-Plateau in die Mulde (ausprobiert, zurück auf 50 %). |
| 4 | Stufen und Mulden | Eine Tabelle `[[mesa.tier]]`: Höhe ab Becken, höher = Stufe, tiefer = Mulde (Wand nach unten, Schutt am Grund); Eltern-Ebene = Tafelberg oder frühere Stufe mit dem ersten Punkt. **[vorläufig]** | Ein Baustein für beide; Rampen derselbe Code. |
| 5 | Becken | Senken als `[[hill]]` mit negativer Höhe (konnte der Generator schon), dunkle Flecken als `tall_grass` (Wüstenlook: trockenes Gras auf dunklem Grund, volles Tempo) statt `sand` (70 %), dazu `[[scatter]] kind = "scrub"` (neu: Büsche und Steine); eine breite Anhebung um 5 m hält alle Senken über 0 m. **[vorläufig]** | Keine Kies-Geländeart vorhanden; Sand hätte Panzer in den Becken gebremst (Prompt: Platz für Panzer). Werkzeuge zählen Grund ≤ 0 m als Wasser. |
| 6 | Look | Neuer Karten-Look `mesa` (`desert_mesa.tres`): Gebirgslook aus 060 voll (Schichten an den Wänden, grauer Schutt), Sonne 38° statt 58°; kein neues Biom. **[vorläufig]** | Mit dem normalen Wüstenlook lasen sich die Tafelberge kaum (erste Bilder Stufe 1). |
| 7 | Anordnung | Nordhälfte nach der Vorlage, punktsymmetrisch um (1, 1): Randplateaus Nordwest (geteilt durch eine Gasse von der Ecke), Nord (mit Grat nach Osten), Ost; mittlere Tafelberge, Ring aus großem Zeugenberg, länglichem Zeugenberg, Sichel, zwei kleinen Zeugenbergen, Felsnadeln. **[vorläufig]** | Vorlage etwa punktsymmetrisch; Formen vereinfacht, Becken größer als in der Vorlage. |
| 8 | Starts | In den Ecken hinter den Randplateaus (1v1 Nordwest gegen Südost, 2v2 alle vier Ecken); erster Entwurf in West- und Ostbecken verworfen. **[vorläufig]** | Dort sah das gegnerische Ostplateau aus 808 m auf einen Start (Raketenwerfer, schwere Mörser in Reichweite); Prompt nennt „hinter den Randplateaus“. |
| 9 | Versorgungspunkte | 7: Zentrum, großer Zeugenberg je Hälfte (Rampe zur eigenen Seite), Nordost- und Südwestbecken, Mulde auf dem Nordwest-/Südostplateau (eine Rampe vom Plateau). **[vorläufig]** | Prompt-Liste; wegen der Symmetrie je eins pro Seite. |
| 10 | Geschützstellungen | `battery_*` an den Plateaurändern, 8 m hinter der Wandkante. **[vorläufig]** | 40 m zurück verdeckt das Plateau selbst das Becken (erste Messung: 0 ha unterhalb sichtbar). |
| 11 | Höhen | Randplateaus 35/30 m, Stufen 45/40 m, Mulden 15–20 m, mittlere 20–25 m, Zeugenberge 12–25 m, Nadeln 10 m; Relief 52 m (1-mm-Schritte). **[vorläufig]** | Vorschlag 15–40 m, große Randplateaus höher. |
| 12 | Objekte | Felsbrocken auf dem Schutt (eigener Zufallsstrom), je Hälfte zwei verlassene Gehöfte (drei Wüstenhäuser zerstört, Lehmmauern, Brunnen) an Pisten, Gestrüpp in den Senken. **[vorläufig]** | Prompt („wenig Objekte“). |
| 13 | Werkzeuge | `map-sight --only=<Präfixe>` (Bild nur dieser Punkte), `basin_*` als Reichweitenziel; Symmetrietest übergeht `needle_*` bei den Wegen. **[vorläufig]** | Sichtbilder einzelner Plateaus und eines Beckens (Prompt); Nadeln sind absichtlich unerreichbar. |

### Ergebnis

- **Karte** `data/maps/mesa_desert/` aus `data/map_sketches/mesa_desert.toml`
  (2,4 × 2,4 km, Biom `desert`, Look `mesa`, punktsymmetrisch), im Menü
  „Tafelberge“ / „Mesas“: je Hälfte 14 Tafelberge (Randplateaus Nordwest
  geteilt, Nord mit Grat, Ost; mittlere Tafelberge; Ring aus Zeugenbergen und
  Felsnadeln um das offene Zentrum), 18 Rampen (12 aus den Becken, 6 zu Stufen
  und in Mulden), 380 Felsbrocken, Senken mit trockenem Gras und Gestrüpp,
  Pisten, zwei verlassene Gehöfte mit Brunnen; Starts in den vier Ecken,
  sieben Versorgungspunkte, drei Geschützstellungen je Seite an den Rändern.
- **Generator**: `[[mesa]]` mit `[[mesa.tier]]` (Stufen, Mulden), Rampen,
  Rauheit, Felsbrocken, Rampen-Marker; `[[scatter]] kind = "scrub"`; alle
  anderen Skizzenkarten bitgleich (geprüft). Neuer Look `mesa`
  (`desert_mesa.tres`, `sim::map::Look::Mesa`, nicht im Hash).
  `map-sight --only`, `basin_*` als Reichweitenziel.
- **Prüfungen**: `map-check` ok; `map-reach`: alle Starts, Punkte, Stellungen,
  Rampenköpfe im Hauptteil (89 % für Fahrzeuge), Zeugenberge ohne Rampe
  getrennt, Weglängen beider Seiten gleich; Tests
  `mesa_desert_is_point_symmetric`, `mesa_desert_ranges_and_sight`, Karte im
  Test „zweimal erzeugt, gleiche Dateien“; `stuck-audit` 176/176; Golden Hash
  unverändert.
- **Sicht**: Stellungen an den Rändern sehen 7,7–12,7 ha (höchstens 2,2 % der
  Fläche), davon 3–6 ha 20 m tiefer; Becken sehen nichts von oben.
- **KI-Turnier** (gleiche KI und Fraktion beidseitig): normal 54 % Nord, schwer
  44 %, zusammen 48 %; die KI nimmt Punkte auf Plateaus über die Rampen, besetzt
  Höhen aber nicht gezielt; niemand bleibt am Fuß der Wände hängen.
- **Messungen**: path-bench Planung 4,4 ms, alle Ticks 0,34 ms; Bildrate hoch
  ≥ 97,9 FPS.
- **Bilder**: `build/mesa_desert/stage1/` (Stufe 1),
  `build/mesa_desert/final/compare_{top,oblique,ring,plateau}.png`,
  `sight_three.png` (Nordplateau, Ostplateau, Becken), `reach.png`.
- **Offen**: KI nutzt Höhen nicht für Geschütze und Beobachter (eigener
  Prompt); Darstellung: Plateauflächen und Becken haben dieselbe Farbe (die
  Vorlage zeigt orange Kuppen), Wände wirken von oben als Erdband; die
  Rampen-Aufschüttungen sind spitze Keile; eigenes Wüstengestrüpp-Modell fehlt
  (Haselbusch als Platzhalter); Sichel-Felsnadel zu schmal für eine Kuppe.
