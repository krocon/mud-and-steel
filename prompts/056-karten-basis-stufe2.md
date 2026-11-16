# Prompt 056 – Karten und Basis Stufe 2

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/MAPS.md`, `docs/GAME_DESIGN.md`, `docs/ASSETS.md`, Umwelt-Objekte aus 041.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Mehr und lebendigere Karten: eine Winterkarte mit Dorf, eine Landschaft mit Höfen, eine
Flusskarte; Versorgungspunkte an echten Orten der Karte statt abstrakten Markern; eine
Basis, die nach Fabrik und Feldlager aussieht.

## 2. Umfang

1. **Karten** (mit dem Editor 028 und Werkzeugen): „Winterdorf“, „Höfe“, „Flussland“
   (Namen frei wählbar, keine Orte mit politischer Bedeutung), je 2 und 4 Spieler.
2. **Versorgungspunkte an Orten:** Dorfplatz, Grabenstellung, Gehöft, Brückenkopf;
   Gebäude und Objekte dort gehören zum Punkt.
3. **Abwechslung:** jedes Dorfhaus mit eigener Farbe und Dachvariante (Material-Varianten,
   deterministisch nach Position), verschneite Bäume, die in 3D umfallen, Heuhaufen,
   Strohreste, Schnee auf Objekten.
4. **Basis-Optik:** Fahrzeugwerk als große Stahlhalle, Kaserne, Depot mit Fässern und
   Kisten; Bauvorschau gibt es schon (grün/rot).
5. **Ressourcen:** Personal als begrenzende Größe prüfen (Einkommen sinkt ohne Punkte);
   in `GAME_DESIGN.md` mit Begründung, Abgleich mit 045.
6. **Hinterhalt** [provisional]: Infanterie kann sich in Wald/Gebäuden verstecken und
   erst auf kurze Distanz feuern (Tarnung bis zum ersten Schuss).

## 3. Akzeptanzkriterien

1. `just map-check` für jede neue Karte, `just editor-selftest`, `just selftest`,
   `just gallery-check` grün; Benchmark je Karte.
2. Doku in `docs/MAPS.md` und `GAME_DESIGN.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
