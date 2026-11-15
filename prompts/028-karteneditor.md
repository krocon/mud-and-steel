# Prompt 028 – Karteneditor

## Ziel

Karten im Spiel (oder als Godot-Editor-Plugin) bauen: Höhen formen, Geländearten
malen, Objekte setzen, speichern im Kartenformat aus 010.

## Umfang

- Werkzeuge: Anheben/Absenken/Glätten/Ebnen, Geländearten malen, Flüsse ziehen,
  Objekte und Vegetation platzieren (Pinsel mit Dichte).
- Speichern und Laden im Kartenformat; Validierung (Startpositionen, erreichbare Wege).
- Rückgängig/Wiederholen.
- Vorher entscheiden: eigener Editor-Modus im Spiel oder Godot-Editor-Plugin.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Eine im Editor gebaute Karte lässt sich spielen und erzeugt auf zwei Peers
  denselben Hash.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 028“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
