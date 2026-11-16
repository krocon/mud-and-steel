# Prompt 058 – Bilder in der Bau- und Produktionsauswahl

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (HUD aus 031, Basisbau-HUD aus 023), `docs/ASSETS.md` (Galerie).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Bau- und Produktionsmenü zeigen bisher Textschaltflächen. Nach diesem Prompt sieht man in
der Auswahl rechts ein Bild jeder Einheit und jedes Gebäudes als Kachel mit Name, Kosten,
Personal und Bauzeit; nicht verfügbare Kacheln sind abgedunkelt mit Grund, die Auswahl
ist in Reiter gegliedert (Basis, Verteidigung, Infanterie, Panzer, Geschütze, Luft).

## 2. Umfang

1. **Bilder vorab rendern** (statt zur Laufzeit wie die Porträts aus 031): ein
   Werkzeug (`just thumbnails`) rendert jede Katalog-Einheit und jedes baubare Objekt
   einheitlich (Licht, Winkel von schräg vorn, Hintergrund nach Fraktion/Tarnung) als
   PNG nach `godot/assets/thumbnails/` (Git LFS), aus der Modellgalerie oder Blender
   headless. Die Porträts der Einheitenanzeige (031) nutzen dieselben Bilder, wenn
   vorhanden.
2. **Kacheln:** Bild, Name, Ressourcenkosten, Personal, Bauzeit, Tastenkürzel,
   Warteschlangen-Zähler, Fortschritt als Überlagerung; Tooltip mit Werten aus dem
   Katalog (Panzerung, Waffe, Reichweite).
3. **Reiter** nach Kategorie, scrollbar, Größe passend zu UI-Skalierung (043).
4. **Prüfung:** `just gallery-check` (oder eigener Check) meldet fehlende Bilder.

## 3. Akzeptanzkriterien

1. `just selftest` mit Bauen und Produzieren über die Kacheln grün.
2. Jede baubare Einheit hat ein Bild; Check grün.
3. Doku: HUD-Abschnitt und `docs/ASSETS.md` (Thumbnails).

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
