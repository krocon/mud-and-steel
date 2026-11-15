# Prompt 011 – Terrain-Darstellung für große Karten

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` und `docs/MAPS.md`. Deren Regeln gelten unverändert.

## 1. Ziel

Das Höhenfeld aus Prompt 010 soll so aussehen wie in den Vorlagen: realistische
Bodentexturen (Gras, Erde, Feldwege mit Fahrspuren, Sand, Schlamm, Schnee), weiche
Übergänge und gute Bildrate über eine 4-km-Karte bei jeder Zoomstufe.

Die Simulation bleibt die Wahrheit. Die Darstellung liest Höhen und Geländearten über
die Bridge und muss später einzelne Bereiche neu aufbauen können (Krater in 021).

## 2. Phase 0 – Entscheidung (erst prüfen, dann berichten, dann warten)

Vergleiche zwei Wege und empfiehl einen:

- **A) Terrain3D** (GDExtension-Plugin): Clipmap-LOD, Textur-Splatting, Instancer für
  Vegetation. Prüfe: unterstützte Godot-Version (passt 4.7.x?), Lizenz, ob sich Höhen
  zur Laufzeit aus Daten setzen und bereichsweise aktualisieren lassen, Plattformen
  (macOS ARM, Linux, Windows), Pflegezustand.
- **B) Eigene Lösung:** Chunks (z. B. 64 m × 64 m) mit mehreren LOD-Stufen, Nähte
  zwischen Stufen geschlossen, Splatting-Shader in Godot.

Kriterien: Aufwand für einen Solo-Entwickler, Laufzeit-Updates für Krater, Kontrolle
über das Aussehen, Abhängigkeit von Dritten. Keine Installation ohne mein Go.

## 3. Umfang

1. Terrain-Mesh aus den Sim-Höhen, mit LOD, Frustum-Culling und Normalen.
2. **Textur-Splatting:** Gewichte aus den Geländearten der Sim, zusätzlich rein
   optische Variation (Rauschen, Steigung → Fels/Erde, Höhe). Texturen CC0 (z. B.
   ambientCG, Poly Haven), 1K oder 2K, mit Albedo, Normal und Roughness. Herkunft und
   Lizenz jeder Textur in `godot/assets/textures/CREDITS.md`.
3. Gegen sichtbare Kachelung: Triplanar oder Stochastic Sampling, Makro-Variation in
   großer Entfernung.
4. Feldwege mit Fahrspuren als eigene Geländeart beziehungsweise Decal-Schicht.
5. Bereichsweises Neuaufbauen: API „Höhen im Rechteck X geändert“ → betroffene Chunks
   neu erzeugen (für 021 vorbereitet und mit einem Test-Befehl im Debug-Menü belegt).
6. Kartenrand: sauberer Abschluss (z. B. abfallende Kante wie in der Vorlage mit
   sichtbarem Kartenrand) statt endloser Fläche.
7. Mausauswahl und Kamera-Grenzen nutzen das echte Gelände.

## 4. Akzeptanzkriterien

1. `just check` und `just selftest` grün.
2. Bildrate und Draw Calls auf `test_large` gemessen (nah, mittel, fern) und im
   Bericht genannt. Zielwert: 60 FPS auf diesem Mac bei 1080p in der Übersicht.
3. Bildvergleich (nah, mittel, fern) per `--write-movie` im Scratchpad, im Bericht
   beschrieben. Nichts davon ins Repo.
4. `docs/ARCHITECTURE.md` beschreibt die Terrain-Darstellung und den Weg der Daten.

## 5. Arbeitsweise

- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe.
- Keine Software oder Plugins ohne mein Go installieren. Kein Test wird abgeschwächt.
- Texturen über Git LFS. Pro Schritt ein Commit.
- Am Ende diese Datei nach `prompts/done/` verschieben, `prompts/README.md`
  aktualisieren, committen. Abschlussbericht mit offenen Punkten.
