# Prompt 185 – Gebäude aus einem Referenzbild: Vorschau-Blatt in Godot und Vergleich

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> dann `docs/ASSETS.md` (Abschnitte „Environment objects“, „Materials and
> shaders“, „Modular variants“, Thumbnails) und `docs/ARCHITECTURE.md` zu den
> Bild-Werkzeugen (`*_shots.gd`). Im Code `godot/scenes/tools/thumbnail_render.gd`,
> `godot/scenes/tools/model_gallery.gd`, `tools/blender/build_env.py`,
> `tools/blender/env_lineup.py` und die `justfile`-Rezepte `env`,
> `env-lineup`, `thumbnails`. Godot-APIs (SubViewport, Image, Kamera
> orthogonal) für die festgelegte Version nachsehen (`docs/VERSIONS.md`),
> nicht raten.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/arkadenhaus/referenz.webp`: Referenzbild des Nutzers vom
> 2026-11-21 mit Schrägansicht, vier Seitenansichten mit Maßen, Draufsicht
> und drei Schadensstufen. Es zeigt die **Art von Blatt**, die das Werkzeug
> erzeugen soll.

## 1. Ziel

Der Nutzer will künftig ein Bild eines Gebäudes geben und das gebaute Ergebnis
sehen, mit **Wänden, Mustern und Dachziegeln so, wie sie im Spiel aussehen**.
Ein Programm, das Bilder vollautomatisch in 3D umwandelt, ist **nicht** das
Ziel. Den Teil vom Bild zum Blueprint macht die Claude-Session: Sie liest das
Bild, schätzt die Maße und schreibt den Blueprint. Dieser Prompt baut das
Werkzeug, mit dem die Session (und der Nutzer) das Ergebnis schnell neben dem
Referenzbild sieht und in Schleifen verbessern kann.

Wichtig: Muster und Oberflächen entstehen erst in Godot (`env.gdshader`,
prozedural, ohne Texturen). Eine Blender-Vorschau zeigt nur Slot-Farben.
Darum wird **in Godot gerendert**, mit dem echten Shader und `EnvLook`.

## 2. Werkzeug `just building-preview`

- Aufruf etwa `just building-preview <id> [--ref <bild>] [--biome forest|desert|winter] [--out build/preview/<id>]`.
  Baut das Modell aus dem Blueprint neu (wie `just env <id>`), importiert es
  und rendert in Godot ein **Vorschau-Blatt** als PNG:
  - große Schrägansicht (`Intact`) unter Spiellicht, wie im Referenzbild;
  - vier **orthogonale Ansichten** (+X, +Y, −X, −Y) und eine **Draufsicht**,
    alle im gleichen Maßstab, mit Maßlinien für Länge, Breite, Traufe und
    First aus Katalog bzw. Modell (Bounding Box), Beschriftung in Metern;
  - die drei Schadensstufen `Intact`, `Damaged`, `Ruin` nebeneinander;
  - eine **Nahansicht** einer Wand und einer Dachfläche (Ausschnitt mit
    höherer Auflösung), damit Putz, Ziegel und Dachziegel beurteilbar sind;
  - eine Ansicht **aus Spielhöhe** (normaler Zoom der RTS-Kamera), weil dort
    entschieden wird, was im Spiel wirkt;
  - Kopfzeile mit id, Maßen, Dreieckszahl pro Variante (gegen das Budget
    aus `docs/ASSETS.md`) und Zahl der Oberflächen.
- Mit `--ref` kommt das Referenzbild als eigenes Feld aufs Blatt bzw. als
  zweites Blatt im gleichen Raster daneben, damit Form, Proportion und
  Oberfläche direkt vergleichbar sind.
- Läuft mit Fenster wie `just thumbnails`; wenn kopflos möglich, umso besser
  (prüfen, nicht versprechen). Dauer für ein Haus nennen; Ziel unter einer
  Minute, damit Schleifen schnell sind.
- Wiederverwenden statt neu bauen: Kamera und Licht aus
  `thumbnail_render.gd` bzw. der Modellgalerie, Gebäude-Aufbau aus den
  vorhandenen Szenen. Keine neue Szene, wenn eine bestehende reicht.

## 3. Ablauf vom Bild zum Blueprint dokumentieren

In `docs/ASSETS.md` einen kurzen Abschnitt „Building from a reference
picture“: welche Angaben aus einem Bild gelesen werden (Grundriss,
Geschosse, Öffnungen je Seite, Dach, Wand- und Dachmaterial, Schäden), wie
Maße aus einem Anhaltspunkt geschätzt werden (Türhöhe etwa 2,1 m,
Geschosshöhe 2,8 bis 3,5 m), was als `uncertain` in den Katalog geht, dass
Bilder nur Referenz sind (`docs/SOURCES.md`) und dass Text in einem
Referenzbild nicht als Wahrheit gilt. Dazu die Schleife: Blueprint schreiben,
`just building-preview <id> --ref …`, Blatt ansehen, korrigieren, bis es
passt, und dann Thumbnail, `just units-web-sync`, `just catalog-doc`. Wenn
ein Generator eine Form nicht kann, das als Lücke melden und nicht mit
falschen Parametern nachbauen.

Prüfen, ob ein Projekt-Skill unter `.claude/skills/` (falls im Repo üblich)
für diesen Ablauf sinnvoll ist; nur anlegen, wenn er mehr als die Doku
bringt, und in `CLAUDE.md` den Befehl unter „Commands“ nachtragen.

## 4. Prüfen und zeigen

- Blätter für drei bestehende Häuser erzeugen (`env_house_town`,
  `env_town_hall`, ein Wüstenhaus) und in die Session zeigen; eines davon mit
  `--ref` gegen `docs/.vorlagen/arkadenhaus/referenz.webp`, um das Layout zu
  zeigen (die Formen werden natürlich nicht passen, das kommt mit Prompt 184).
- Einen Selbsttest oder Lauf in `just check` nur, wenn er schnell und ohne
  Fenster geht; sonst ein Rezept `just building-preview-selftest`, das ein
  Blatt erzeugt und prüft, dass alle Felder gerendert sind (Bildgröße, nicht
  leer).
- Keine Änderung an Simulation, Katalogwerten oder Modellen; Golden Hash
  bleibt.
- `just check`.

## 5. Nicht Teil dieses Prompts

- Automatische Bilderkennung oder Umwandlung von Bild zu 3D.
- Neue Formen in den Generatoren (Prompt 184) und feinere Oberflächen
  (Prompt 186).

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Wo rendern | Godot | Muster und Oberflächen entstehen nur im Godot-Shader |
| Bild zu Blueprint | macht die Session, das Werkzeug zeigt das Ergebnis | vollautomatische Erkennung wäre unsicher und teuer |

## Ergebnis

- `just building-preview <id> [--ref=…] [--biome=…] [--out=…]` baut das
  Objekt (Blender), importiert es und rendert das Blatt in Godot
  (`godot/scenes/tools/building_preview.gd`, 2400 × 1800 px nach
  `build/preview/<id>/<id>_<biome>.png`): Kopfzeile (Maße aus Modell und
  Katalog, Dreiecke je Variante gegen das Budget, Oberflächen),
  Schrägansicht neben dem Referenzbild, vier Ansichten und Draufsicht in
  einem Maßstab mit Maßlinien, Schadensstufen, Nahansicht Wand und Dach,
  Spielansicht (Startzoom der RTS-Kamera).
- `just building-sheet <id>` dasselbe ohne Neubau (für Shader-Arbeit wie
  Prompt 186). Grund: ein Neubau ohne Änderung schreibt `.blend` und `.glb`
  neu (nicht bitgleich).
- Dauer: Blatt 0,7 s, mit Godot-Start etwa 7 s, mit Blender-Neubau etwa
  12 s. Kopflos geht es nicht (Godots Headless-Renderer zeichnet nichts).
- `just building-preview-selftest` erzeugt das Blatt von `env_house_town`
  und prüft Größe und alle 12 Felder (nicht einfarbig). Nicht in
  `just check`, weil es ein Fenster braucht.
- Ablauf vom Bild zum Blueprint: `docs/ASSETS.md`, Abschnitt „Building from
  a reference picture“; Befehl in `CLAUDE.md`. Kein Projekt-Skill: die
  Doku und das Rezept reichen, ein Skill brächte nichts dazu.
- Blätter: `env_house_town` (mit dem Arkadenhaus als Referenz),
  `env_town_hall`, `env_house_desert_large` (Wüste).
