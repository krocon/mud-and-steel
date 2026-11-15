# Prompt 004 – Kartenformat und Höhenfeld in der Simulation

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` und `docs/DETERMINISM.md`. Deren Regeln gelten unverändert.

## 1. Ziel

Die Karte wird zu Daten, die der Simulation gehören. Höhen und Geländearten sind
spielrelevant (Steigung, Sichtlinie in 013, Krater in 016) und müssen deshalb
deterministisch in `sim` liegen. Die Darstellung (005) liest sie nur.

Zielgröße: bis 4 km × 4 km.

## 2. Vorschlag (in Phase 0 prüfen und bestätigen lassen)

- **Höhenfeld:** Raster mit 2 m Zellgröße (bei 4 km also 2001 × 2001 Stützpunkte),
  Höhen als Ganzzahl in Millimetern (`i32`). Höhe zwischen den Stützpunkten per
  bilinearer Interpolation in `Fixed`.
- **Geländearten:** eigenes Raster gleicher Auflösung mit einem Byte pro Zelle
  (z. B. Wiese, Feldweg, Straße, Sand, Schlamm, Schnee, flaches Wasser, tiefes Wasser).
  Eigenschaften pro Art als TOML in `data/terrain.toml` (Geschwindigkeitsfaktor,
  befahrbar ja/nein, später Deckung).
- **Dateiformat:** ein Ordner pro Karte unter `data/maps/<name>/`:
  - `map.toml` (Name, Größe, Zellgröße, Höhenbereich, Startpositionen),
  - `height.png` (16-bit Graustufen, ganzzahlig auf Millimeter abgebildet),
  - `terrain.png` (8-bit indiziert oder Graustufen = Geländeart).
  PNGs lassen sich mit jedem Bildprogramm bearbeiten und mit einem Skript erzeugen.
  Dekodiert wird ganzzahlig; prüfe, dass der PNG-Decoder dabei keine Gleitkommazahlen
  benutzt, sonst ein einfaches eigenes Rohformat wählen und begründen.
- **Testkarten:** ein Generator (`cli gen-map` oder Python-Skript in `tools/`) erzeugt
  aus einem Seed eine Karte mit Hügeln, einem Fluss-Bett, Feldwegen und einem flachen
  Dorfplatz. Zwei Karten committen: `test_small` (500 m) und `test_large` (4 km).

## 3. Umfang

1. Datentypen und Laden in `sim` (Laden der Bytes außerhalb, `sim` bekommt `&[u8]`).
2. Einheiten folgen der Geländehöhe (Höhe ist Teil des Snapshots und des State-Hashes).
3. Bewegungsgeschwindigkeit hängt von Geländeart und Steigung ab; unbefahrbares Gelände
   wird vorerst einfach nicht betreten (echte Umwege kommen in 006).
4. `SimConfig` referenziert eine Karte; CLI, Tests und Bridge laden sie.
5. Bridge: Höhenabfrage für die Darstellung (`height_at(x, z)`) und Kartengröße für die
   Kamera-Grenzen aus Prompt 002.
6. Mausauswahl: Strahl gegen das Höhenfeld (Schrittverfahren plus Verfeinerung) in der
   Bridge oder in GDScript, ohne Godot-Physik.
7. Bis 005 fertig ist, zeigt Godot das Höhenfeld als einfaches, grob aufgelöstes
   Debug-Mesh an.

## 4. Akzeptanzkriterien

1. `just check` grün. Neuer Golden-Hash auf `test_small` mit Höhen und Geländearten.
2. Ein Test prüft die bilineare Höhe an bekannten Punkten (exakte `Fixed`-Werte).
3. Ladezeit und Speicherbedarf von `test_large` sind gemessen und im Bericht genannt.
4. `just selftest` lässt eine Einheit einen Hügel hinauffahren und prüft ihre Höhe.
5. `docs/MAPS.md` beschreibt das Kartenformat und den Generator.

## 5. Arbeitsweise

- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe.
- **Phase 0:** Kartenformat, Rastergröße und Höhenauflösung vorschlagen und auf mein Go
  warten. Das ist schwer umkehrbar.
- Kein Test wird abgeschwächt. Pro Schritt ein Commit. Karten-PNGs über Git LFS.
- Am Ende diese Datei nach `prompts/done/` verschieben, `prompts/README.md`
  aktualisieren, committen. Abschlussbericht mit offenen Punkten.
