# Prompt 022 – Infanterie-Verhalten

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Pfade, Kampf, Sicht), `docs/DETERMINISM.md`,
> `docs/SCALE.md`, `docs/INFANTRY.md` und `docs/UNIT_CATALOG.md`.

## 1. Ziel

Die Infanterie-Trupps des Katalogs werden spielbar: Soldaten bewegen sich in
Formation, nehmen Haltungen ein (stehend, kniend, liegend), nutzen Deckung durch
Objekte, besetzen Gebäude und werden mit den Modellen und Animationen aus 005/006/008
dargestellt. Alles Spielrelevante in `crates/sim`, deterministisch; Godot zeigt nur an.

## 2. Ausgangslage

- Ein Trupp ist heute eine `Unit` wie ein Panzer: eine Position, Fußabdruck 1 m
  (Katalog ohne Maße), eine Hitbox „Fußabdruck × 1,8 m“, Trefferpunkte
  `squad_size × 100`, Waffen aus dem Katalog (`count` Geschosse pro Salve).
- Weg, Sicht, Zielwahl und Befehle laufen pro `Unit` (O(n²) bei Separation und
  Zielwahl). Geschosse treffen Einheiten-Hitboxen, Objekte, die große Einheiten
  blockieren (Bäume, Häuser, Mauern, Hecken), und den Boden.
- Im Spiel gibt es nur Panzer; Infanterie würde als Platzhalter-Panzer erscheinen.
- Trupp-Szenen `godot/assets/models/<faction>/<id>.tscn` mit `Member_NN`-Knoten
  (Soldaten-glb mit `AnimationPlayer`, 25 Clips, `SoldierLook`).
- Prompt 020 (Gebäudezustände) und 024 (KI) laufen parallel.

## 3. Entscheidungen

| Frage | Entscheidung | Begründung |
|---|---|---|
| Trupp in der Sim | **Eine `Unit` mit einer Liste von Soldaten** (Position, Höhe, Trefferpunkte je Soldat). Befehle, Wege, Sicht und Zielwahl bleiben pro Trupp. | Ein Befehl bleibt ein Befehl; Pfadsuche, Sichtberechnung und O(n²)-Zielwahl kosten pro Trupp statt pro Soldat (500 Soldaten = 56 Trupps). Einzelne Soldaten sind trotzdem echte Ziele mit eigener Hitbox, eigener Deckung und eigenem Tod. Eine `Unit` pro Soldat würde Gruppen-Logik, Auswahl und Sieg-Bedingung zerreißen und die O(n²)-Teile verzehnfachen. |
| Soldaten-Positionen | Jeder Soldat folgt seinem Formationsplatz (Platz relativ zur Trupp-Position, gedreht nach Blickrichtung) mit höchstens 1,5 × Trupp-Tempo; steht der Trupp, sucht er einen Deckungsplatz in der Nähe. Ungültige Plätze (unpassierbar) fallen auf die Trupp-Position zurück. | Billig (O(Soldaten)), deterministisch, sieht aus wie eine lose Formation. Kollisionen zwischen Soldaten werden nicht gerechnet. |
| Formation | Zwei versetzte Reihen, 3 m Abstand, Führer vorne Mitte. Für Gruppen- und Separationsabstände zählt die **Trupp-Breite** (`spread`), für die Pfad-Größenklasse weiter der Fußabdruck (Infanterie bleibt `Small`). | Trupps stehen sich nicht im Weg; Infanterie passt weiter durch 4-m-Lücken. |
| Haltungen | `Stance` Stehen / Knien / Liegen pro Trupp. Befehl `SetStance { units, stance }` mit `Auto` (Standard): in Bewegung stehend, im Stand kniend, nach Beschuss (Treffer oder Einschlag ≤ 3 m in den letzten 5 s) liegend. Wechsel dauert eine Regelzeit, dabei wird nicht geschossen. | Wenig Mikromanagement, trotzdem steuerbar. |
| Wirkung der Haltung | Regeln in `data/infantry.toml`: Tempo 100 / 60 / 20 % (Liegen = Kriechen), Hitbox-Höhe 1,8 / 1,1 / 0,45 m, eigene Streuung 100 / 75 / 50 %. | Echte Kenngrößen (Körperhöhe, Kriechtempo) als Spielregeln, kein Katalogwert. |
| Deckung | Zwei Arten: **harte Deckung** entsteht von selbst, weil Geschosse Mauern, Hecken, Häuser und Bäume bis zu ihrer Höhe treffen (eine 1,2-m-Mauer schützt kniende und liegende Soldaten); **weiche Deckung** pro Objektart in `infantry.toml` (`cover_pct`: Sandsäcke, Graben, Felsen, Holzstapel, Büsche …): ein Treffer auf einen Soldaten, der hinter so einem Objekt liegt (Objekt zwischen Schütze und Soldat, ≤ 2 m), wird mit `cover_pct` Wahrscheinlichkeit (Sim-`Rng`) abgefangen. Krater (021) kommen als weitere Quelle über eine Funktion `cover_at` dazu. | Physikalisch, wo die Daten es hergeben; Regelwert, wo nicht. Richtungsabhängig, ohne Sektorregeln. |
| Deckung suchen | Steht der Trupp, verteilt er seine Soldaten auf Deckungsplätze hinter Objekten (Rückseite zur Blickrichtung, alle 1,5 m entlang der Objekt-Längsachse) innerhalb 8 m ihres Formationsplatzes, gierig in Soldatenreihenfolge. | Deterministisch, billig, sichtbar. |
| Gebäude | Befehl `Garrison { units, building }` (Objektindex). Gebäudearten mit Kapazität und Schutz in `infantry.toml`. Der Trupp läuft zum Gebäude und betritt es, solange Platz ist und kein Gegner darin ist; die Soldaten stehen dann an Fensterplätzen, Augenhöhe über dem Dach. Ein Geschoss, das ein besetztes Gebäude trifft, trifft mit `exposure_pct` (größer für Kaliber ≥ 20 mm) einen Insassen. Insassen schießen aus dem Gebäude, ihre Geschosse ignorieren das eigene Gebäude. `MoveTo` verlässt das Gebäude. | Gebäude werden Behälter, keine begehbaren Innenräume: einfach, deterministisch, klare Regeln. |
| Abstimmung mit 020 | Der Gebäudezustand kommt aus einer Funktion `Sim::building_condition(object)`, die ohne 020 immer `Intact` liefert. Ruinen: kein Besetzen, aber Deckung. 020 ersetzt nur diese Funktion. | Läuft ohne 020, eine Stelle zum Anpassen. |
| Schaden | Ein Treffer trifft einen Soldaten (`caliber × infantry_per_mm`, Soldat hat `soldier_hp`). Trupp-Trefferpunkte = Summe der Soldaten. Waffen mit `count > 1` feuern anteilig zu den lebenden Soldaten; Mündung an einem lebenden Soldaten (reihum). Ziel ist ein lebender Soldat des Zieltrupps (reihum). | Verluste wirken auf die Feuerkraft; Treffer sind sichtbar einzelnen Soldaten zuzuordnen. |
| Szenario | Die Demo bekommt pro Seite zwei Schützengruppen; Fraktionswechsel über eine neue Tabelle `[faction_infantry]` und das Gruppenfeld `role = "infantry"`. Golden-Hash wird bewusst aktualisiert. | Infanterie ist im normalen Spiel zu sehen und zu testen. |
| Darstellung | `world/infantry_view.gd`: pro Soldat ein Mitglied der Trupp-Szene, Position aus der Sim (interpoliert), Clip aus Haltung, Bewegung, Feuer und Tod (`idle/kneel/prone`, `walk/run/crawl`, `aim_*`, `fire_*`, `death_a/b`), Fortbewegungs-Clips mit `speed_scale = Tempo / Autorengeschwindigkeit`. **Animations-LOD:** nah jedes Bild, mittel jedes 2.–4. Bild manuell weitergeschaltet, fern eingefroren und ohne Schatten. Vertex-Animation (VAT) nur, wenn die Messung mit 500 Soldaten das Budget reißt. | Nutzt die fertigen Modelle sofort; VAT ist ein großer Pipeline-Umbau und lohnt sich erst, wenn gemessen. |
| Bedienung | Rechtsklick auf ein Gebäude mit ausgewählter Infanterie = Besetzen; Tasten **Z** stehen, **X** knien, **V** liegen, **B** automatisch. | Keine Kollision mit bestehenden Tasten (WASD, QE, N, H, F, C, 1–9). |

## 4. Umfang

1. **Sim** (`crates/sim/src/sim/infantry.rs`, Regeln `crates/sim/src/infantry.rs` +
   `data/infantry.toml`, in `SimConfig` und Hash): Soldaten, Formation, Haltungen,
   Deckung, Gebäude, neue Befehle `SetStance` und `Garrison` (ans Enum angehängt, alte
   Replays bleiben lesbar), Snapshot mit Soldaten, Haltung und Gebäude.
2. **Kampf:** Treffer pro Soldaten-Hitbox, Deckungswurf, Gebäudetreffer, Feuer aus
   Gebäuden, Feuerkraft nach Verlusten.
3. **Sicht:** Augenhöhe im Gebäude über dem Dach.
4. **Bridge:** Soldaten-Arrays (Positionen aktuell/vorher, Trupp, Lebenspunkte),
   Haltung, Gebäude, Befehle `set_stance`, `garrison_units`, `building_at`;
   Szenario-Start für den Benchmark. Änderungen additiv in eigener Datei.
5. **Godot:** `infantry_view.gd`, kleine Ergänzungen in `main.gd` (Trupp ohne
   Platzhalter-Panzer) und `selection.gd` (Gebäude, Tasten).
6. **Szenario:** zwei Schützengruppen pro Seite in der Demo.
7. **Messung:** `just infantry-bench` (CLI: Sim-Tick mit 504 Soldaten) und
   `just infantry-view-bench` (Godot: Bildzeit mit 504 Soldaten).
8. **Doku:** `docs/INFANTRY.md` (Verhalten, Darstellung, Messung),
   `docs/ARCHITECTURE.md` (Befehle, Kampf, Entscheidungen), Golden-Hash-Kommentar.

## 5. Akzeptanzkriterien

1. `just check` grün, Golden-Hash bewusst aktualisiert, Grund im Commit.
2. Tests (fester Seed): Trupp in Deckung erleidet unter gleichem Beschuss weniger
   Verluste als im Freien; liegender Trupp weniger als stehender; Haltung ändert das
   Tempo; Trupp besetzt ein Gebäude bis zur Kapazität, Gegner kommt nicht hinein,
   `MoveTo` verlässt es; Insassen nehmen weniger Schaden; zwei Läufe identisch.
3. Performance mit 500 Soldaten gemessen (Sim und Darstellung), Werte in
   `docs/INFANTRY.md`.
4. Alle Selbsttests grün: `just selftest` (mit Flow-Test), `just lan-selftest`,
   `just editor-selftest`, `just gallery-check`, Demo-Lauf.

## 6. Arbeitsweise

Deutsch mit dem Nutzer, Englisch im Repo, kein Push, keine Installation ohne Go, kein
Test wird abgeschwächt. Änderungen an gemeinsamen Dateien (`sim.rs`, `bridge/lib.rs`,
`main.gd`, `self_test.gd`, `justfile`, Doku) klein und additiv. Am Ende `git mv` nach
`prompts/done/`, Status in `prompts/README.md`, committen; nicht mergen.
