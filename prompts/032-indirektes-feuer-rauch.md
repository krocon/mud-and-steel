# Prompt 032 – Indirektes Feuer, Mörser, Artillerie und Rauch

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Kampf, Nebel, Effekte, Krater), `docs/SCALE.md`,
> `docs/DETERMINISM.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Mörser, Haubitzen und Selbstfahrlafetten stehen im Katalog und als Modelle bereit, haben
aber noch keine Waffen („indirect fire is a later prompt“). Nach diesem Prompt schießen
sie im Bogen über Hindernisse auf Bodenziele, mit Streuung, Splitterwirkung,
Kratern und Rauchgranaten. Rauch blockiert erstmals die Sichtlinie.

## 2. Umfang

1. **Ballistik in der Sim:** Bogenflug mit Festkomma (Flugzeit aus Mündungsgeschwindigkeit
   und Distanz, Zeitfaktor 2), Mindest- und Höchstreichweite aus dem Katalog,
   Streuung aus `sim::Rng`, Einschlag am Boden. Kein Treffertest gegen Objekte im Flug
   (Bogen), nur am Einschlagpunkt.
2. **Wirkung:** Splitter im Radius nach Kaliber (Infanterie stark, offene Fahrzeuge
   mittel, Panzer nur bei Volltreffer von oben), Deckung und Haltung (022) dämpfen.
   Krater aus 021 und Zerstörung aus 020 wie bei Kanonen.
3. **Feuerleitung:** Bodenziel-Befehl (`AttackGround`) für Indirektfeuer; Ziel nur im
   Sichtbereich eines eigenen Beobachters oder mit Streuungszuschlag blind; Sperrfeuer
   über eine Fläche (Anzahl Salven). Aufbauzeit für gezogene Geschütze vorbereiten
   (Schleppen kommt mit 034).
4. **Rauch:** Rauchgranaten (Mörser, Haubitzen, Nebelwerfer der Panzer falls im
   Katalog) erzeugen Rauchwolken als Sim-Objekte mit Lebensdauer und Wind-Drift
   (deterministisch). Rauch blockiert die Sicht in `VisionGrid` (Zellen-Flag mit
   Zeitstempel) und ist im Hash. Darstellung über die Effekte aus 019.
5. **KI (024):** setzt Artillerie gegen gesichtete Gruppen und Rauch vor dem Angriff ein
   (mindestens `Hard`).
6. **Darstellung:** Mündungsfeuer, Bogen-Leuchtspur optional, Pfeifen im Anflug
   (Sound aus 029, falls vorhanden), Einschlagmarker des Ziels für den Spieler.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests: Mörser trifft hinter einer Mauer, Mindestreichweite
   greift, Splitter treffen Infanterie im Radius, Rauch blockiert Sicht und verschwindet,
   zwei Läufe gleicher Hash. Golden-Hash bewusst aktualisiert.
2. `just selftest`, `just lan-selftest` grün; `just ai-tournament` läuft.
3. Doku: Kampf-Abschnitt und `docs/SCALE.md` (Umrechnung der Ballistik).

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
