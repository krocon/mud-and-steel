# Prompt 042 – Infanterie: Texturen, Tarnmuster und mehr Animationen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/ASSETS.md`, `docs/SOURCES.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Soldaten haben flache Slot-Farben ohne Texturen und Tarnmuster („No textures yet“), und
manche Trupp-Mitglieder haben keine eigene Waffe (Karabiner, Pistole). Nach diesem
Prompt sehen die drei Fraktionen aus der Spielkamera unterscheidbar und glaubwürdig aus.

## 2. Umfang

1. **Texturen:** Stoff, Leder, Metall, Haut per Projektion oder einfachem UV-Layout;
   Tarnmuster als Material-Varianten (z. B. Splittertarn, Erbsentarn, US-Fleckmuster
   nach Quelle) und Wintertarn; über das bestehende Material-System, ohne Hoheitszeichen
   außer erlaubten (Balkenkreuz nur an Fahrzeugen; kein Hakenkreuz, keine SS-Runen).
2. **Ausrüstung:** fehlende Waffen (Karabiner, Pistolen, Panzerfaust/Bazooka/PTRD falls
   im Katalog) und Ausrüstung pro Fraktion.
3. **Animationen:** Laden, Werfen (Granate), Kriechen, Verwundet/Fallen in Varianten,
   Bedienung von Geschützen und MGs, Aufsitzen (für 034).
4. **Leistung:** Budgets aus 016 halten; prüfen, ob Vertex-Animationstexturen jetzt nötig
   sind (Entscheidung in `docs/INFANTRY.md`).

## 3. Akzeptanzkriterien

1. `just kit-test`, `just validate-assets`, `just gallery-check` grün.
2. `just selftest` grün; Infanterie-Benchmark nicht schlechter als 10 %.
3. Galerie-Bilder je Fraktion und Tarnung; Quellen in `docs/SOURCES.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
