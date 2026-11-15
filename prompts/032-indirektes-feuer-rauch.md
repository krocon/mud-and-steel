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

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach Welle 9): `CombatType::from_catalog` gibt Mörsern,
Haubitzen und Selbstfahrlafetten *keine* Waffen (`indirect` → leere Liste), die KI
behandelt sie als unbewaffnet, und keine Produktionsliste enthält sie. `VisionGrid`
kennt feste Hindernisse (Häuser, Hecken) und Wald, aber nichts Vorübergehendes.
`AttackGround` gibt es schon (Direktfeuer). Krater (021), Zerstörung (020),
Haltung/Deckung (022) und Effekte (019) reagieren auf `Hit`-Ereignisse.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Die **erste Katalogwaffe** einer Indirektfeuer-Kategorie (`inf_mortar_*`, `mortar_heavy`, `gun_howitzer`, `self_propelled`) wird `CombatType::indirect`; ihre übrigen Waffen (Karabiner, MPi der Bedienung) werden normale Direktwaffen. | Katalog unverändert; die Bedienung wehrt sich im Nahbereich, ohne dass die Direktfeuer-Logik (Hauptwaffe = `weapons[0]`) angepasst werden muss. |
| 2 | Reichweite × `range.indirect_fire` (1/8, `data/scale.toml`), Kadenz und Geschwindigkeit wie Direktwaffen. Fehlt die Mündungsgeschwindigkeit (50-PM 40), gilt die Vakuum-Mindestgeschwindigkeit `sqrt(Reichweite · g)`. | Eine Umrechnungsstelle; keine erfundenen Werte. |
| 3 | **Mindestreichweite** und **Aufbauzeit** sind Spielregeln pro Kategorie in neuem `data/indirect.toml` (Katalog hat keine Felder dafür): Mörser 5–10 %, Geschütze 10 % der Höchstreichweite; Aufbau Mörsertrupps 10 s, schwere Mörser 30 s, Haubitzen 60 s, Selbstfahrlafetten 10 s (real, Zeitfaktor gilt). **[vorläufig]** | Quellenlage im Katalog fehlt; als Regel bleibt es balancierbar. Gezogene Geschütze sind bis 034 unbeweglich, die Aufbauzeit greift vorerst nur bei Mörsertrupps und SFL. |
| 4 | **Ballistik:** Flugzeit aus realer Distanz (Spieldistanz ÷ 1/8), bevorzugter Erhöhung (Mörser 60°, Geschütze 25°, Ladung so gewählt, dass die Mündungsgeschwindigkeit reicht, sonst volle Ladung mit passendem Winkel) und `t = 2·v·sin θ / g`, geteilt durch den Zeitfaktor. Die Flugbahn ist eine Parabel zwischen Mündung und Einschlagpunkt mit maßstäblich verkleinertem Scheitel (Scheitel × 1/8); kein Treffertest im Flug. | Reale Flugzeiten mit Zeitfaktor 2 (8 cm GrW 34 auf 300 m Spieldistanz ≈ 14 s, leFH 18M auf 1540 m ≈ 15 s); alles in `Fixed`, ohne Trigonometrie außer der vorhandenen Grad-Tabelle. |
| 5 | **Streuung:** dreieckverteilt je Achse, `3 mrad × reale Distanz` (mind. 2 m), × 3 bei Blindfeuer (Zielzelle für den Spieler nicht sichtbar). **[vorläufig]** | Flächenwaffe; Splitterradien sind 1:1, daher Streuung in echten Metern. |
| 6 | **Splitter:** Radius 0,2 m pro mm Kaliber (8 cm ≈ 16 m, 10,5 cm ≈ 21 m). Soldaten: Trefferchance 60 % im Zentrum, linear auf 0 am Rand, × Haltungshöhe/Stehhöhe (liegend 25 %), × 50 % in Deckung, Kraterrand (021) wie bei Kugeln; ein Treffer kostet 100 HP. Soldaten in Gebäuden nur bei Treffer aufs Gebäude (Exposition wie 022). Offene Fahrzeuge (dünnste Platte ≤ 10 mm): 4 HP pro mm Kaliber im Zentrum, linear fallend. Panzer nur bei **Volltreffer** (Einschlag im Rumpf-Grundriss): HE-Durchschlag = 25 % Kaliber gegen Dach = 50 % der dünnsten Platte (Katalog hat kein Dach). **[vorläufig]** | Infanterie stark, offene Fahrzeuge mittel, Panzer nur von oben – wie gefordert, mit Katalogwerten als Grundlage. |
| 7 | **Feuerleitung:** `Attack` auf eine Einheit braucht Sicht des Spielers (beliebiger eigener Beobachter, wie 018); verliert er sie, endet der Befehl. `AttackGround` feuert bis `Stop` (blind mit Streuungszuschlag). Neuer Befehl `Barrage { units, target, radius_m, salvos, smoke }` (am Enum-Ende) für Sperrfeuer über eine Fläche und Rauch. Außer Reichweite fährt die Einheit heran; unter der Mindestreichweite feuert sie nicht. Ohne Befehl feuern Indirektwaffen „frei“ nur auf Ziele in eigener Sicht, die HE verletzen kann (keine Panzer). | Klein und additiv; `Command` bleibt serialisierbar. Indirekte Befehle liegen in eigenem Zustand, nicht in `combat.attack`, damit die Direktfeuer-Logik unverändert bleibt. |
| 8 | **Rauch:** nur Waffen, deren Katalog-Munition „smoke“ oder „WP“ nennt (DE 8 cm, 10,5 cm; US 60/81 mm, 4.2 in, 105 mm; SU-Einträge haben keine Rauchmunition, Panzer keine Nebelwerfer). Wolken sind Sim-Objekte (Mittelpunkt, Radius 0,15 m pro mm Kaliber, Lebensdauer 90 s real, Drift mit festem Wind aus `data/indirect.toml`). Zellen unter einer Wolke tragen eine Rauch-Oberkante (Boden + 10 m) im `VisionGrid`: Sichtlinien darunter enden dort, Ziele in Rauchzellen sind (außer direkt daneben) unsichtbar; geänderte Zellen stellen die betroffenen Sichtfelder neu in die Warteschlange. Wolken und belegte Zellen sind im Hash. **[vorläufig: Wind fest bis Wetter (040)]** | Höhe statt reinem Flag: Beobachter auf Hügeln sehen über Rauch. Der „Zeitstempel“ ist das Ablauf-Tick der Wolke. |
| 9 | **KI (Hard):** eigene Rolle „Artillerie“; feuert Sperrfeuer auf die größte gesichtete Gruppe in Reichweite und legt Rauch zwischen eigene vorrückende Gruppe und Feind, bevor sie angreift. Mörser-Trupps kommen in die Infanterie-Produktionsliste (`data/economy.toml`), Hard baut sie. | Ohne Produktion hätte die KI keine Artillerie. |
| 10 | **Darstellung:** Granaten als dunkle Körper auf der Bahn, Mündungsfeuer/Einschlag über die vorhandenen `shot`/`hit`-Ereignisse (gleiche Ids), Rauchwolken als Billboard-Puffs, Einschlagmarker für eigene Granaten, Signal `shell_incoming` für das Pfeifen (029 hängt sich an). Tasten: **G** Sperrfeuer (3 Salven, 25 m) und **N** Rauch (2 Salven) auf den Bodenpunkt unter dem Mauszeiger. | Godot zeichnet nur; Ton kommt aus 029, das parallel entsteht. |
| 11 | Regeln in eigener Datei `data/indirect.toml`, von `Sim::new` geladen (wie Krater), nicht in `SimConfig`. | Ändert das Replay-/Config-Format nicht (038 läuft parallel) und berührt `CombatRules` nicht. |
