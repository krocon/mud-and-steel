# Prompt 006 – Deterministische Wegfindung

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md`, `docs/DETERMINISM.md` und `docs/MAPS.md`.

## 1. Ziel

Einheiten fahren um unbefahrbares Gelände (tiefes Wasser, zu steile Hänge, später
Gebäude und Wald) herum und bevorzugen Straßen. Alles in `crates/sim`, deterministisch,
ohne Godot-`NavigationServer`.

## 2. Umfang

1. **Kostenraster** aus Geländeart und Steigung, gröber als das Höhenfeld (z. B. 4 m),
   inklusive Fahrzeuggrößen-Klassen (z. B. „klein“ für Infanterie, „groß“ für Panzer)
   über aufgeweitete Hindernisse.
2. **A\*** mit festen Tie-Breakern (keine Abhängigkeit von Heap-Reihenfolge bei
   gleichen Kosten), Ganzzahl-Kosten, Oktil-Heuristik. Pfadglättung per Sichtlinie
   im Raster.
3. **Budget pro Tick:** Pfadanfragen in eine deterministische Warteschlange, maximal N
   Knoten pro Tick; Einheiten warten, bis ihr Pfad fertig ist.
4. **Gruppenbewegung:** ein Pfad für die Gruppe, Formation folgt ihm; einfache
   Ausweichregeln zwischen Einheiten (Separation), deterministisch.
5. Später änderbare Hindernisse (Zerstörung in 015) berücksichtigen: Teilbereiche des
   Kostenrasters invalidierbar, betroffene Pfade neu planen.
6. Debug-Ansicht in Godot: Kostenraster und Pfade ein- und ausblendbar.

## 3. Akzeptanzkriterien

1. `just check` grün. Golden-Hash bewusst aktualisiert, Grund im Commit.
2. Tests: bekannte Labyrinth-Karte mit erwarteter Pfadlänge; zwei Läufe ergeben
   identische Pfade; Loopback-Test mit Gruppenbefehlen über Hindernisse.
3. Benchmark in der CLI: 200 Einheiten mit gleichzeitigen Pfadanfragen auf
   `test_large`, Rechenzeit pro Tick im Bericht.
4. `docs/ARCHITECTURE.md` beschreibt die Wegfindung.

## 4. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, keine Tests abschwächen, Commits pro Schritt, am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Vor schwer umkehrbaren
Entscheidungen (z. B. Flow-Fields statt A\*) fragen.
