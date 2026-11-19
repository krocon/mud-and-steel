# Prompt 128 – Skizzenkarte: Kesselsee (eigener Entwurf)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` (Ziel, Entscheidungen, Ergebnis),
> in `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz (alle Tabellen,
> besonders `[[lake]]`, `[[shoal]]`, `[[hill]]`, `[[wood]]` mit `rock`,
> `[[village]]`, `[[ring_road]]`, `[[patchwork]]`), „Relief profiles and mountains
> (prompt 060)“ und den Abschnitt zur Punktsymmetrie von `city_large`;
> `data/map_sketches/island_chain.toml` als Muster, `crates/cli/src/gen_sketch.rs`,
> `data/terrain.toml` (`[slope]`), `data/objects.toml` (Felsen), `data/scale.toml`
> (`[range]`), `docs/AI.md`. Ist Prompt 126 (Tafelberge, `[[mesa]]`) erledigt, auch
> dessen Ergebnis: die Felsen des Kessels können damit gebaut werden.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/kesselsee/kesselsee_entwurf.svg`,
> ein eigener Entwurf aus der Unterhaltung mit dem Nutzer, von ihm so angenommen.
> Maßstab: das Kartenquadrat (x 60–620, y 20–580 im SVG) ist 2 km, 1 px = 3,57 m,
> Mitte (340, 300) = Kartenmitte; x nach Osten, y nach Süden wie in den Skizzen.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach Stufe 1 Bilder aus dem Spiel neben
> dem Entwurf zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `kessel_lake` (im Spiel „Kesselsee“ / „Kessel lake“), 2 km ×
2 km, **punktsymmetrisch** um die Mitte, gebaut als Skizze für `just gen-sketch`.
Herkunft: Der Nutzer brachte eine Minimap aus einem kommerziellen Spiel als Idee;
die wird **nicht** nachgebaut. Diese Karte ist ein eigener Entwurf mit ähnlichen
Zutaten (Seeufer, Felsen in der Mitte, Straßendörfer, Bergrand), aber eigener
Anordnung. Nur der Entwurf oben ist Vorlage.

## 2. Aufbau nach dem Entwurf

Richtwerte in Metern (x Osten, y Süden, Mitte 0/0); alles im Südosten ist das
Spiegelbild (x, y) → (−x, −y) des Nordwestens, die Session übernimmt die Werte und
glättet sie nach den Stufenbildern.

- **Felskessel in der Mitte:** Felsring um (0, 0), Mittellinie Radius etwa 250 m,
  Ring etwa 70–80 m breit, 15–25 m hoch, Außen- und Innenseite steil (über 60 %
  an den meisten Stellen, Felsbrocken und `env_rock_massif`), nur stellenweise für
  Infanterie kletterbar. **Zwei Durchlässe** im Norden und Süden (bei (0, −250) und
  (0, 250)), je etwa 60–80 m breit, eben, durch beide läuft ein Feldweg von
  (0, −371) nach (0, 371). Innen eine Wiese mit Radius etwa 185 m und ein **Weiler**
  aus vier bis sechs Häusern um die Mitte.
- **Ringweg** um den Kessel, Radius etwa 370 m (`[[ring_road]]` oder Weg-Polygon).
- **See Nordwest:** Polygon etwa (−1000, −1000), (−1000, −136), (−857, −100),
  (−679, −171), (−550, −350), (−636, −536), (−600, −743), (−743, −886),
  (−786, −1000); Sandstrand und eine Sandzunge am Ostufer (um (−600, −640) und
  (−630, −180)), ein Inselchen bei (−757, −536), eine kleine Sandbank bei
  (−857, −321). **Furt:** eine Untiefe (`[[shoal]]`) von der Sandzunge zum Ufer
  bei etwa (−679, −464) → (−550, −436) als Umweg für Fahrzeuge am See entlang.
  Spiegelbild im Südosten.
- **Berge Nordost:** Polygon etwa (464, −1000), (1000, −1000), (1000, −339),
  (879, −407), (743, −564), (643, −671), (543, −821), Höhe zum Rand hin bis 80–120 m,
  Gipfelbereich in der Ecke (um (850, −850)) am höchsten, Fuß sanft (Bergfuß für
  Starts nutzbar), Flanken mit Felsbändern, Nadelwald am Fuß. Spiegelbild im
  Südwesten.
- **Straßendorf Nord** an der Kreuzung bei (321, −571): Hauptstraße nach Norden zum
  Start (321, −893), Abzweig nach Südosten zum Ringweg bei (0, −371) über
  (179, −500), Nebenweg nach Osten (643, −393) → (786, −250). Felder (gold und grün)
  östlich und südöstlich des Dorfes, ein Feld bei (96, −643). Spiegelbild: Dorf Süd
  bei (−321, 571).
- **Seeweg:** ein Feldweg vom Westeingang des Ringwegs (−364, 57) nach Norden am
  See vorbei: (−446, −179), (−436, −536), (−250, −714), (−71, −893), (−36, −1000).
  Spiegelbild im Südosten.
- **Hauptstraße West:** vom Start West (−857, 107) über (−536, 86) zum Ringweg bei
  (−364, 57). Spiegelbild Ost.
- **Wald:** Wäldchen nördlich des Seewegs (um (−230, −680)), kleine Gruppen am
  Bergfuß und bei den Starts; Spiegelbilder.
- **Gehöfte:** je ein Gehöft bei (643, −257) und (−643, 257).

## 3. Spiel

- **Starts** (60 m eben und frei): Team West (−857, 107) und (−321, 893), Team Ost
  (857, −107) und (321, −893). 1 gegen 1 nutzt die beiden ersten.
- **Versorgungspunkte:** Kessel (0, 0) umkämpft; Dorf Nord (321, −443) und Dorf Süd
  (−321, 443); Furt Nordwest (−514, −443) und Furt Südost (514, 443).
- **Prüfen:** `just map-check kessel_lake`, `just map-reach kessel_lake`: alle Starts
  und Punkte für Fahrzeuge erreichbar, der Kessel nur durch die beiden Durchlässe
  (für Fahrzeuge), die Furten wirklich befahrbar; Weglängen beider Seiten gleich
  (Symmetrietest: gespiegelte Höhen, Gelände, Objekte gleich).
- **Sicht und Reichweite:** Sichtfeld-Bilder vom Kesselrand, aus dem Weiler und vom
  Bergfuß; Reichweitentabelle wie in Prompt 123 (indirekt 1/8): welche Punkte von
  welchem Start bzw. Bergfuß aus erreichbar sind.
- **KI-Turnier**, Ergebnis in `docs/AI.md`: Kämpft die KI um den Kessel, nutzt sie
  die Furten, staut sie sich in den Durchlässen? Probleme als offene Punkte; die KI
  nicht umbauen.

## 4. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/kessel_lake.toml` nach Abschnitt 2 und 3,
erzeugt mit `just gen-sketch kessel_lake`, Biom `forest`, Wetter klar. Reicht der
Generator für den Felsring nicht (Ring mit Lücken, steile Flanken), ergänzen: ein
`[[hill]]`-Ring bzw. `[[mesa]]` aus Prompt 126, oder eine neue Tabelle
`[[rock_ring]]` (Mittelpunkt, Radius, Breite, Höhe, Lücken als Winkel und Breite).
Bestehende Skizzenkarten müssen Bit für Bit gleich bleiben.

**Stufe 2 – Prüfen und Abschluss:** Abschnitt 3, Messung von Bildrate und Wegsuche
(`docs/PERFORMANCE.md`), Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md`, Test wie bei den anderen Skizzenkarten (zweimal
erzeugt, gleiche Dateien). `just check`, `just selftest`. Der Golden Hash ändert
sich nicht (neue Karte); wenn doch, Ursache finden.

## 5. Nicht Teil dieses Prompts

- Winter- oder Wüstenvariante (über `base` in der Skizze leicht nachzuholen).
- Boote auf dem See (Prompt 119).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
