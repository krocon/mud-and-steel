# Prompt 123 – Karte nach Bildvorlage: Flussfront, Artillerie auf beiden Seiten

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` (Ziel, Entscheidungen, Ergebnis),
> in `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz, „Bridges“,
> „Objects“ (`fort_*`) und den Abschnitt zur Symmetrie der großen Stadtkarte
> (`city_large`, Punktsymmetrie), `data/map_sketches/river_fields.toml` und
> `valley_fields.toml` als Muster, `crates/cli/src/gen_sketch.rs`. In
> `docs/ARCHITECTURE.md` die Abschnitte zu indirektem Feuer, Sicht („Seeing
> downhill“) und Wegfindung; `data/scale.toml` (`[range]`: indirektes Feuer ein
> Achtel der echten Weite), `data/vision.toml`, die Geschütze und Mörser in
> `data/catalog/*.toml` (`range_effective_m`), `docs/AI.md`. Sind die Prompts 115
> (lange Brücken) oder 122 (Inselreich, farbenfroher Look) erledigt, auch deren
> Ergebnis.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/flussfront/flussfront.webp`. Genau ansehen.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel neben
> der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `river_front` (im Spiel „Flussfront“ / „River front“) nach der
Vorlage, gebaut für ein **Artillerieduell über einen breiten Fluss**: Beide Seiten
haben auf ihrem Ufer Höhen und vorbereitete Feuerstellungen, von denen aus sie den
Fluss, die Inseln und die Brücken beschießen können. Wer übersetzen will, muss unter
Feuer über Brücken und Inseln.

Die Vorlage zeigt:

- einen **breiten Fluss** von West nach Ost quer durch die Mitte, mit Stromschnellen,
  Felsen im Wasser und **vier bis fünf bewaldeten Inseln** in der Mitte;
- **etwa zehn Steinbrücken**: je eine lange Brücke an beiden Enden (West und Ost),
  eine Kette von Nord nach Süd über die Mittelinseln, dazu schräge Brücken zwischen
  Ufern und Inseln; an vielen Brückenenden kleine **Brückentürme bzw. Bunker**;
- je eine **Stadt mit Platz und großem Gebäude** in der Mitte des Nord- und des
  Südufers, verbunden durch die gerade Hauptstraße über die Mittelinseln;
- je eine **Ringstraße** auf beiden Ufern, die die Brückenköpfe verbindet, mit
  Gehöften, Feldern (gold und grün) und Wäldern;
- **Felsberge** in den vier Ecken und Hügel hinter den Städten.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung. Nichts daraus übernehmen,
alles mit den Bausteinen des Spiels nachbauen.

## 2. Spielgedanke: Artillerie auf beiden Seiten

- **Streng spiegelsymmetrisch** an der Flussachse (Nord gegen Süd), damit beide Seiten
  gleiche Chancen haben: Gelände, Höhen, Straßen, Brücken, Stellungen, Starts und
  Versorgungspunkte gespiegelt (wie die Punktsymmetrie von `city_large`, hier als
  Spiegelung; die Session wählt Spiegelung oder Punktsymmetrie danach, was die
  Vorlage besser trägt, und begründet es). Das Licht ist nicht symmetrisch; das
  ist in Ordnung.
- **Reichweiten zuerst rechnen:** Im Spiel reicht indirektes Feuer ein Achtel der
  echten Weite (105-mm-Haubitze etwa 1,4 km, 15-cm-Kaliber und schwere Mörser
  weniger). Die Abstände so wählen, dass
  - leichte Haubitzen aus den Feuerstellungen den ganzen Fluss, die Inseln und die
    gegnerischen Brückenköpfe erreichen,
  - die gegnerischen Feuerstellungen nur aus vorgeschobenen Stellungen oder mit den
    weitesten Geschützen erreichbar sind (Gegenbatteriefeuer kostet Bewegung und
    Risiko),
  - Mörser nur vom Ufer aus über den Fluss wirken.
  Die Rechnung mit den echten Katalogwerten und dem Faktor als Tabelle in
  `docs/MAPS.md` festhalten (Geschütz, Reichweite im Spiel, was es von welcher
  Stellung erreicht).
- **Feuerstellungen:** je Seite drei bis vier vorbereitete Stellungen am
  Hinterhang der Hügel bzw. an den Bergen in den Ecken (`fort_sandbag_position`,
  `fort_trench_section`), auf ebenen Flächen, für gezogene Geschütze erreichbar,
  vom Gegenufer aus nicht direkt einsehbar (Sichtfeld aus der Simulation prüfen).
- **Beobachtung:** Höhen am Ufer und die Brückentürme sind Beobachtungspunkte mit
  Sicht über den Fluss; Versorgungspunkte dort, damit um Sicht gekämpft wird.
- **Brückenköpfe:** Bunker (`fort_bunker_mg`) und Gräben an den Brückenenden
  beider Seiten, gespiegelt. Brücken sind sprengbar (Pioniere, Prompt 020/055).
- **Zweiter Weg:** eine bis zwei Furten bzw. Untiefen (`[[shoal]]`) an den
  Stromschnellen, damit gesprengte Brücken das Spiel nicht beenden.
- **Starts:** 1 gegen 1 und 2 gegen 2: Team Nord und Team Süd, je zwei Starts hinter
  der Ringstraße zwischen Stadt und Eckbergen, 60 m eben und frei, außerhalb der
  Reichweite leichter gegnerischer Artillerie aus deren Feuerstellungen.

## 3. Skizze (Stufe 1)

`data/map_sketches/river_front.toml`, erzeugt mit `just gen-sketch river_front`:

- **Größe** 3,6 km × 2,0 km (Seitenverhältnis der Vorlage; Höhe nach der
  Reichweitenrechnung anpassen, höchstens 4 km), Biom `forest`, Wetter klar.
- **Fluss** als `[[river]]` bzw. `[[lake]]`-Polygon in der Breite der Vorlage
  (mehrere hundert Meter mit Inseln), Inseln als `[[island]]` oder Land-Polygone,
  Stromschnellen und Felsen im Wasser.
- **Brücken:** Ist Prompt 115 erledigt, mehrbogige Steinbrücken in voller Länge (60–
  150 m, Teile aneinandergereiht); sonst Dämme (`[[causeway]]`) mit kurzen
  Steinbrücken wie `island_chain`, und die langen Brücken als offener Punkt.
  Achsen nur Nord-Süd oder West-Ost (Brücken sind achsparallel); die schrägen
  Brücken der Vorlage werden zu achsparallelen mit kurzem Knick der Straße.
- **Brückentürme:** vorhandene Steinhäuser bzw. Bunker an den Brückenenden als
  Stellvertreter; ein eigenes Torhaus-Modell als offener Punkt.
- **Städte:** je ein `[[town]]` mit `[[square]]` und großem öffentlichem Gebäude
  (Rathaus, Kirche), gespiegelt.
- **Ringstraßen, Feldwege, Gehöfte** (`[[farm]]`), **Felder** (`[[patchwork]]`,
  gold und grün), **Wälder** (Nadel- und Mischwald), **Felsberge** in den Ecken
  (`[[hill]]` mit `[[wood]]` `rock`, `env_rock_massif`), Hügel hinter den Städten
  für die Feuerstellungen. Die Berge brauchen mehr Höhe als bisherige Skizzen (50–
  100 m); den Generator erweitern, falls nötig.

## 4. Prüfen (Stufe 2)

- `just map-check river_front`, `just map-reach river_front`: alle Starts,
  Feuerstellungen und Versorgungspunkte für Fahrzeuge mit gezogenem Geschütz
  erreichbar; Weglängen beider Seiten gleich (Symmetrie im Test prüfen: gespiegelte
  Höhen, Gelände und Objekte gleich).
- **Sichtprüfung:** Bild des Sichtfelds von jeder Feuerstellung und jedem
  Beobachtungspunkt; die Feuerstellungen sehen das Gegenufer nicht direkt.
- **Reichweitenprüfung:** Test oder Werkzeug, das für jede Stellung zeigt, welche
  Versorgungspunkte, Brücken und gegnerischen Stellungen in Reichweite welcher
  Geschütze liegen.
- **KI-Turnier** auf der Karte, Ergebnis in `docs/AI.md`: Nutzt die KI Artillerie
  aus den Stellungen, greift sie über die Brücken an, endet es im Stillstand?
  Probleme beschreiben und als offenen Punkt notieren; die KI nicht umbauen.
- Messung der Bildrate in jeder Ansicht, `docs/PERFORMANCE.md`.

## 5. Darstellung und Abschluss (Stufe 3)

- Ist der farbenfrohe Look aus Prompt 122 erledigt und vom Nutzer angenommen,
  diese Karte ebenfalls damit; sonst `forest` wie die anderen.
- Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md` mit der Reichweitentabelle, Test wie bei den anderen Skizzenkarten
  (zweimal erzeugt, gleiche Dateien).
- `just check`, `just selftest`. Der Golden Hash ändert sich nicht (neue Karte);
  wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Eigene Modelle für Brückentürme und Torhäuser.
- Vorgegebene Startarmeen mit Artillerie für diese Karte (das wäre eine Mission
  oder ein eigenes Szenario).
- KI-Verbesserungen für Gegenbatteriefeuer und Flussübergänge.
- Boote (Prompt 119) über den Fluss.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
