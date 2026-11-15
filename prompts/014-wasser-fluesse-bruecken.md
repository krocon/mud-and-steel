# Prompt 014 – Wasser, Flüsse und Brücken

## Ziel

Flüsse und Bäche wie in den Vorlagen: Wasser mit Strömung, Böschungen mit
Steinen und Schilf, Brücken als taktische Engstellen.

## Umfang

- Wasser als Geländearten aus 010 (flach = durchfahrbar langsam, tief = unpassierbar).
- Wasser-Shader: Fließrichtung aus der Karte (Flow-Map), Schaum an Ufern und Steinen,
  Tiefe-abhängige Farbe, Reflexion/Spiegelung soweit performant.
- Ufer: Übergangstexturen, Uferbewuchs über das Vegetationssystem aus 013.
- Brücken als Objekte der Sim: befahrbare Fläche über Wasser, Tragfähigkeit
  (Fahrzeugklassen), zerstörbar vorbereitet. Wegfindung kennt Brücken.
- Modelle: Holzbrücke und Steinbogenbrücke als Platzhalter per Blender-Skript.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Einheiten queren nur über Brücken oder Furten (Test mit Golden-Hash-Update).
- Bildvergleich im Scratchpad; Bildrate gemessen.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 014“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
