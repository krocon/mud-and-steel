# Prompt 004 – Blender-Baukasten für Fahrzeuge, Geschütze und Flugzeuge

> In einer frischen Session im Repo-Ordner ausführen. Voraussetzung: Prompt 003
> (Katalog, `docs/ASSETS.md`, `docs/SCALE.md`). Lies diese Dokumente und `CLAUDE.md`.
> Kann parallel zu Prompt 005 und zum Spiel-Strang laufen (eigener Worktree/Branch).

## 1. Ziel

Eine skriptgesteuerte Pipeline, die aus einem kurzen Bauplan pro Einheit ein fertiges,
geprüftes Modell erzeugt: `.blend`, `.glb` und ein Vorschaubild. Damit sich viele
Einheiten später parallel und einheitlich bauen lassen, ohne jedes Modell von Hand.

Erwartung: gut erkennbare Low- bis Mid-Poly-Modelle mit stimmigen Proportionen. Details
kommen über Materialien, Normal-Maps und Abnutzung, nicht über Polygone.

## 2. Umfang

### 2.1 Baukasten (`tools/blender/kit/`)
Python-Paket, das nur über `blender -b` läuft (kein laufendes Blender nötig).
Bauteile mit Parametern, alle Maße in Metern:
- **Laufwerke:** Ketten mit Laufrollen in mehreren Bauarten (Blattfeder-Drehgestelle wie
  beim Panzer IV, große Christie-Laufrollen wie beim T-34, VVSS-Drehgestelle wie beim
  Sherman, Halbkette), Antriebs- und Leitrad, Stützrollen, Kettenglieder als Textur
  oder einfache Geometrie.
- **Räder und Achsen** für Lkw, Aufklärer, Geschützlafetten.
- **Wannen und Aufbauten** aus Profilen mit Schrägen (gerade Platten), optional
  gegossene, gerundete Formen.
- **Türme** aus Grundriss und Seitenprofil, Kommandantenkuppel, Luken.
- **Waffen:** Rohre mit Segmenten, Mündungsbremsen (Typen), Blenden, MGs, Schutzschilde.
- **Anbauteile:** Schürzen, Ersatzketten, Werkzeug, Kanister, Scheinwerfer, Antennen.
- **Geschütze:** Spreizlafette, Schild, Räder, Lafettenholme.
- **Flugzeuge:** Rumpf als Loft aus Querschnitten, Tragflächen mit Zuspitzung, Pfeilung
  und V-Stellung, Leitwerk, Propeller, Kabinenhaube, Fahrwerk (eingefahren und
  ausgefahren).
Die Bauteile setzen die Namen, Hierarchie, Anbaupunkte (`Socket_*`) und Material-Slots
aus `docs/ASSETS.md` automatisch.

### 2.2 Bauplan und Generator
- Bauplan pro Einheit: `assets-src/blueprints/<faction>/<id>.toml`, enthält nur die Wahl
  der Bauteile und ihre Parameter. Hauptmaße kommen aus dem Katalog (003) und werden
  nicht doppelt gepflegt.
- `tools/blender/build_unit.py <faction> <id>` erzeugt
  `assets-src/blender/<faction>/<id>.blend` und
  `godot/assets/models/<faction>/<id>.glb`.
- Fraktionsspezifische Bauteile in eigenen Modulen (`kit/parts_de.py` …), damit parallele
  Sessions nicht dieselben Dateien ändern.

### 2.3 Tarn- und Material-System
- In Blender bekommen Flächen nur Slot-Materialien (`Paint`, `Metal`, `Rubber` …) und
  eine gebackene Vertex-Farbe für Umgebungsverdeckung.
- In Godot ein gemeinsamer Shader `godot/assets/shaders/unit_camo.gdshader`:
  Fraktions-Grundfarbe, Tarnmuster (Wald, Wüste, Winter mit weißem Anstrich und
  Schneeauflage auf Oberseiten), Abnutzung an Kanten, Schlamm und Staub von unten,
  Hoheitszeichen und Nummern als Decal. Paletten als Ressourcen pro Fraktion × Biom.
- Ein Import-Skript (Godot `EditorScenePostImport`) ersetzt die Slot-Materialien beim
  Import automatisch durch den Shader.

### 2.4 Qualitätssicherung
- `tools/blender/validate.py`: Maße gegen den Katalog (Toleranz ± 2 %), Transformationen
  angewendet, Namen und Anbaupunkte vollständig, Dreiecksbudget eingehalten,
  Ursprung korrekt. Bricht mit Fehler ab, wenn etwas nicht stimmt.
- Vorschau: Drehteller-Rendering (4 Ansichten, Eevee headless) pro Modell und eine
  **Aufstellung** aller vorhandenen Modelle nebeneinander mit der 1,80-m-Referenzfigur
  aus 003. Ausgabe in den Scratchpad bzw. einen ignorierten Ordner, nicht ins Repo.
- LOD: Zuerst prüfen, ob Godots automatische Mesh-LODs beim Import reichen. Nur wenn
  nicht, eigene LOD-Stufen per Decimate erzeugen.
- `just`-Rezepte: `just unit <faction> <id>`, `just units` (alle), `just validate-assets`,
  `just lineup`.

### 2.5 Modell-Galerie in Godot
- Szene `godot/scenes/tools/model_gallery.tscn`: lädt alle Modelle aus
  `godot/assets/models/`, stellt sie nach Fraktion und Kategorie auf, Umschalter für
  Biom/Tarnung, frei drehbare Kamera, Anzeige der Katalogwerte zum ausgewählten Modell.
- `just gallery` startet sie.

### 2.6 Nachweis
Drei Modelle, je eines pro Fraktion, plus ein Geschütz:
Panzer IV Ausf. H, T-34/76, M4A3 Sherman, 7,5 cm PaK 40. Alle bestehen die Prüfung und
stehen in der Galerie in allen drei Tarnungen.

## 3. Nicht-Ziele

Infanterie (005), alle weiteren Einheiten (006, 008), Umwelt-Objekte (007),
Einbindung ins Spiel über die Galerie hinaus (009 ff.).

## 4. Akzeptanzkriterien

1. `just check`, `just validate-assets` und `just selftest` sind grün.
2. Die vier Modelle sind erzeugt, geprüft und in der Galerie sichtbar.
3. Aufstellungs-Rendering im Scratchpad; im Bericht beschrieben, inklusive Abgleich der
   Größenverhältnisse.
4. `docs/ASSETS.md` beschreibt Baukasten, Bauplan-Format, Material-System und den
   Ablauf „neue Einheit in 5 Schritten“.

## 5. Arbeitsweise

- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe.
- Blender-API für die installierte Version (5.3) nachschlagen, nicht raten (z. B. über
  `bpy`-Introspektion im Headless-Modus oder die gebündelte Doku des Blender-MCP).
  Mein laufendes Blender und meine offenen Dateien nicht anfassen.
- `.blend` und `.glb` über Git LFS. Kein Test wird abgeschwächt. Pro Schritt ein Commit.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` aktualisieren, committen.
