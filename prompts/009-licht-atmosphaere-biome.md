# Prompt 009 – Licht, Atmosphäre und Biome

## Ziel

Den Look der Vorlagen treffen: kräftige Sonne mit harten Schatten, weiche
Verschattung in Senken, leichter Dunst in der Ferne, stimmige Farben. Dazu drei Biome:
gemäßigte Wiese, Wüste, Winter.

## Umfang

- `DirectionalLight3D` mit an die Zoomstufe angepassten Kaskaden-Schatten.
- `WorldEnvironment`: SSAO, ggf. SSIL/SDFGI nach Messung, Tonemapping (Filmic/AgX),
  Farbkorrektur per LUT, Höhennebel/Dunst, leichte Vignette, optional Tiefenunschärfe
  nur in der Nahansicht.
- Biom-Presets als Ressourcen (Licht, Himmel, Nebel, Texturset, Vegetations-Set);
  die Karte wählt ihr Biom in `map.toml`.
- Grafikqualitäts-Stufen (niedrig, mittel, hoch) für schwächere Rechner.
- Wetter optional vorbereiten (Schneefall-Partikel im Winter-Biom).

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Drei Biome per `--write-movie` gerendert (Scratchpad), im Bericht verglichen.
- Bildrate pro Qualitätsstufe gemessen.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 009“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
