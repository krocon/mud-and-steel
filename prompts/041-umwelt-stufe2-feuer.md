# Prompt 041 – Umwelt-Objekte Stufe 2, Feuer und Brände

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ASSETS.md`, `docs/SOURCES.md`, `docs/MAPS.md`, `docs/ARCHITECTURE.md`
> (Kartenobjekte, Zerstörung, Effekte, Vegetation).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Die Karten haben Bauernhäuser, Scheunen, eine Kirche, Mauern und Hecken (007). Für
abwechslungsreiche Schlachtfelder fehlen Dörfer und Kleinstädte, Industrie, Eisenbahn
und Befestigungen, und für Wüste und Winter passende Bauten. Gebäude stürzen beim
Zerstören ein, brennen aber nie. Nach diesem Prompt gibt es eine zweite Welle
Umwelt-Objekte aus dem Blender-Baukasten und Feuer als Spielregel.

## 2. Umfang

1. **Neue Objekte** (je mit Intact/Damaged/Ruin, Budgets aus `docs/ASSETS.md`, echte
   Maße mit Quelle): Stadthäuser in Reihe, Marktplatz, Rathaus, Fabrikhalle mit Schlot,
   Lagerhaus, Bahnhof, Gleise, Bahnübergang, Güterwaggons, Wasserturm, Windmühle,
   Tankstelle, Bunker und MG-Nest, Panzergraben, Sandsackstellung; Wüste: Lehmhäuser,
   Oase, Brunnen; Winter: Blockhäuser, verschneite Varianten. Telegrafenmasten, Zäune,
   Wegweiser (ohne echte Ortsnamen mit politischer Bedeutung). Viele Gebäude können
   parallel in mehreren Sessions gebaut werden (wie 008).
2. **Katalog und Regeln:** `data/objects.toml` und `data/destruction.toml` erweitern,
   besetzbare Gebäude für 022 (Fensterplätze), Kartenobjekt-Gruppen für den Editor.
3. **Feuer in der Sim:** Gebäude, Wald und Wracks können brennen (Auslöser: Treffer mit
   Brandwirkung, Explosionen), Feuer schadet über Zeit, breitet sich deterministisch auf
   Nachbarn aus (Wald, Holzgebäude; Stein kaum), erlischt; Munitions- und
   Treibstofflager explodieren (Flächenschaden). Rauch aus 032 blockiert Sicht, falls
   vorhanden. Im Hash.
4. **Darstellung:** Flammen, Glut, schwarzer Rauch, verkohlte Varianten, Licht bei Nacht.
5. **Karten:** mindestens eine Dorf-/Kleinstadt-Karte und eine Karte mit Bahnlinie.

## 3. Akzeptanzkriterien

1. `just kit-test`, `just validate-assets`, `just gallery-check` grün.
2. `just check` grün; Sim-Tests: Feuer breitet sich im Wald aus, nicht über eine
   Straße, Lager explodiert, gleicher Hash; Golden-Hash bewusst aktualisiert.
3. `just selftest`, `just editor-selftest` grün; Benchmark auf der Stadtkarte.
4. Quellen in `docs/SOURCES.md`; Doku aktualisiert.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
