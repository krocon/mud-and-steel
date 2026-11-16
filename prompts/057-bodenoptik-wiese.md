# Prompt 057 – Bodenoptik: Wiesen und Rasen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` („Terrain rendering“, Vegetation, Licht/Biome), `docs/PERFORMANCE.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Aus der hohen Spielkamera soll eine Wiese wie eine echte Wiese aussehen: dichtes, feines
Gras mit sichtbarer Halmstruktur, Farbwechsel zwischen sattem Grün, trockenen gelblichen
Flecken und dunkleren feuchten Stellen, Mähstreifen und Trampelpfade, weicher Übergang
zu Feldwegen, Grasbüschel und Blumen als Akzente. Referenz ist ein Screenshot des
Nutzers (nicht im Repo): nur die Wirkung zählt, nichts wird übernommen.

## 2. Ausgangslage

- Eigener Terrain-Renderer (011): Splatting von 8 Geländearten, `terrain_albedo.png`,
  `terrain_normal.png`, `terrain_macro.png`; Spuren-Richtung für Feldwege.
- Grasbüschel und Blumen als Instanzen (013) bis 100 m, bis zu 70 000 nahe der Kamera.
- Budgets aus 016 (`docs/PERFORMANCE.md`).

## 3. Umfang

1. **Vergleich zuerst:** Screenshots der aktuellen Wiese aus 3–4 Zoomstufen (Movie-Writer
   oder Screenshot-Tool) in `build/` ablegen und Unterschiede zur Zielwirkung notieren.
2. **Mikrodetail im Shader:** prozedurale Halm-/Sprenkelstruktur in hoher Frequenz
   (stochastisches Tiling gegen Kachelmuster), Detail-Normalen, Ausblenden nach Distanz.
3. **Farbvariation in mehreren Skalen:** große Flecken (trocken/feucht), mittlere
   Wolken, kleine Sprenkel; nach Biom und Jahreszeit (015) parametrisiert;
   Mähstreifen/Wiesenstruktur optional als Kartenebene oder Rauschen.
4. **Gras-Schicht nah:** prüfen, ob Shell-Texturing oder dichtere, kleinere Halm-Instanzen
   (GPU-Partikel/MultiMesh mit Wind) die Wiese nah besser machen als die Büschel aus 013.
5. **Übergänge:** Wege mit ausgefransten Rändern, Fahrspuren mit plattgedrücktem Gras,
   Krater (021) mit Erdauswurf.
6. **Vorher/Nachher-Bilder** in den Abschlussbericht.

## 4. Akzeptanzkriterien

1. `just selftest`, `just gallery-check` grün; Golden-Hash unverändert.
2. Terrain-Benchmark aus 016: höchstens 1 ms pro Frame mehr auf `test_large`.
3. Doku: Abschnitt „Terrain rendering“ ergänzt.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
