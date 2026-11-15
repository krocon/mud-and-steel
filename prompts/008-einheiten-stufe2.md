# Prompt 008 – Restliche Einheiten Stufe 2 (parallel)

## Ziel

Alle übrigen Katalog-Einheiten (`tier = 2`) bauen: restliche Infanterie-Rollen
(Mörser, Aufklärer, Scharfschütze, Sanitäter, Pionier, Panzerabwehr, Flammenwerfer,
Offizier/Funker), restliche Fahrzeuge (leichte und schwere Panzer, Panzerjäger,
Sturmgeschütze, Aufklärer, Selbstfahrlafetten), Geschütze (Flak, Haubitzen, schwere
Mörser), Befestigungen und Flugzeuge (Jäger, Schlachtflugzeug, Bomber, Aufklärer).

## Umfang

- Läuft wie Prompt 006 parallel, aber feiner aufgeteilt, z. B. pro Fraktion und
  Kategorie: „Bitte führe 008 für `de`, Kategorie `vehicles` aus.“ Jede Session in
  einem eigenen Worktree/Branch `assets/<fraktion>-<kategorie>`.
- Vorher prüfen, welche Baukasten-Erweiterungen alle brauchen (z. B. Flugzeug-Rümpfe,
  Schachtellaufwerk), und diese in einer kurzen Vorab-Session gemeinsam umsetzen,
  damit die parallelen Sessions nur additiv arbeiten.
- Flugzeuge: Fahrwerk ein/aus, Propeller als drehbares Teil, Bombenschächte und
  Abwurfpunkte als Sockets, Schatten-Proxy für große Flughöhen.
- Zusammenführen wie in Prompt 006, Abschnitt 5.

## Akzeptanzkriterien

- `just check`, `just validate-assets`, `just selftest` grün nach dem Zusammenführen.
- Gesamt-Aufstellung aller Einheiten im Scratchpad, Größenverhältnisse bewertet.
- Jeder Katalog-Eintrag hat ein Modell oder eine begründete Lücke im Bericht.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren („Bitte konkretisiere Prompt 008“).

- Regeln für parallele Arbeit wie in Prompt 006, Abschnitt 3.
- Mit mir Deutsch, im Repo Englisch. Kein Push, keine Installation ohne mein Go. Mein
  laufendes Blender nicht anfassen. Kein Test wird abgeschwächt. Binärdateien über
  Git LFS.
