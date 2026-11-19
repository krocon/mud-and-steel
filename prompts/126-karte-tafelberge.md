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
