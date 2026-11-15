# Prompt 010 – Asset-Pipeline für echte Modelle

## Ziel

Vom Platzhalter zu brauchbaren Modellen: Panzer, Lkw, Geschütz, Häuser.
Eine klare, skriptgestützte Pipeline, damit ich als Einzelentwickler schnell neue Assets
ins Spiel bekomme.

## Umfang

- Konventionen erweitern: Namensschema, Maßstab, Pivot, bewegliche Teile (Turm,
  Rohr, Räder/Ketten), Anbaupunkte (Mündung, Auspuff) als Empties.
- LOD-Stufen automatisch per Blender-Skript (Decimate) und Godot-Importeinstellungen.
- PBR-Materialien, Textur-Atlas pro Fraktion, Tarnschemata als Material-Varianten.
- Zerstörte Varianten (Wrack) und Schadensstufen vorbereiten (015).
- Import-Presets in Godot automatisch (Import-Skript), Kollisions-Meshes nicht nötig.
- Lizenzregel: nur eigene oder CC0-Assets, jede Quelle in `CREDITS.md`.
- `just`-Rezepte: alle Modelle exportieren, ein Modell prüfen (Maße, Transformationen).

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Ein überarbeiteter Panzer und ein Haus mit LOD im Spiel.
- Prüfskript meldet Verstöße gegen die Konventionen (Test an einer absichtlich falschen
  Datei).
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 010“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
