# Prompt 007 – Vegetation und Objekte

## Ziel

Das Gelände so dicht bestücken wie in den Vorlagen: Bäume, Büsche, Grasbüschel, Felsen,
Heuballen, Zäune, Mauerreste, Telegrafenmasten. Trotzdem flüssig auf einer 4-km-Karte.

## Umfang

- **Zwei Klassen trennen:**
  - *Spielrelevant* (Bäume, Hecken, Mauern, Felsen): statische Objekte in der Sim mit
    Grundfläche, blockieren Wegfindung, später Sicht und Deckung, zerstörbar (015).
    Gespeichert in der Karte (`objects.toml` oder eigenes Binärformat).
  - *Deko* (Gras, kleine Büsche, Steine, Blumen): nur in Godot, aus Dichte-Karten und
    Geländeart erzeugt, mit festem Seed reproduzierbar.
- Darstellung über `MultiMeshInstance3D` pro Chunk (oder den Instancer aus 005),
  Sichtweite pro Objektart, LOD und Impostor/Billboards für Bäume in der Ferne.
- Wind-Bewegung im Shader (dezent), Schatten nur für größere Objekte.
- Platzhalter-Modelle per Blender-Skript (Laub- und Nadelbaum, Busch, Heuballen,
  Mauerstück) oder CC0-Modelle mit Credits-Datei.
- Generator aus 004 erweitern: Wäldchen, Hecken an Feldwegen, Obstgärten am Dorf.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Bildrate und Instanz-Zahlen auf `test_large` (nah, mittel, fern) gemessen.
- Wegfindung umfährt Bäume und Mauern (Test).
- Bildvergleich per `--write-movie` im Scratchpad, im Bericht beschrieben.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 007“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
