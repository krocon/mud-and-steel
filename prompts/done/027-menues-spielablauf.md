# Prompt 027 – Menüs und Spielablauf

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md`, `docs/DETERMINISM.md`, `docs/LAN.md` und `docs/MAPS.md`.
> Läuft parallel zu 014 (Wasser) und 017 (Kampf): Änderungen an gemeinsamen Dateien
> (`crates/sim`, `crates/bridge/src/lib.rs`, `godot/scenes/main.gd`, `main.tscn`,
> `self_test.gd`, `justfile`, Doku) klein und additiv halten.

## 1. Ziel

Aus dem Prototyp wird ein Spiel mit Anfang und Ende: Das Spiel startet in ein
Hauptmenü, man wählt Karte, Biom und Fraktion, spielt, gewinnt oder verliert und
landet wieder im Menü. Sieg und Niederlage entscheidet die Simulation
(deterministisch, auf allen Rechnern gleich), Godot zeigt nur an.

## 2. Ausgangslage

- `godot/scenes/main.tscn` ist die Startszene und lädt sofort die Testpartie
  (`--map`, `--biome`, `--quality`, `--demo`, `--self-test`, Benchmarks).
- `godot/scenes/lan/lan_lobby.tscn` (Prompt 025) ist eine eigene Szene (`just lan`),
  übergibt die gestartete Partie über `lan/lan_match.gd` an `main.tscn`.
- Grafikstufen (`world/graphics_quality.gd`) und Biome (`world/atmosphere.gd`) kommen
  aus der Kommandozeile oder dem Debug-Menü (F3).
- Die Sim kennt Spieler nur als Besitzer von Einheiten. Einheiten können noch nicht
  zerstört werden (kommt mit 017, parallel).
- Tastenbelegung steht fest in `project.godot` (`camera_*`-Aktionen).

## 3. Umfang

1. **Siegbedingungen in der Sim** (`crates/sim`):
   - Neue Befehle, am Ende des `Command`-Enums angehängt (alte Replays bleiben lesbar):
     `Surrender` (der sendende Spieler gibt auf) und `RemoveUnits { units }`
     (entfernt eigene Einheiten; Testbefehl, bis 017 Einheiten zerstört).
   - Ein Spieler ist besiegt, wenn er aufgibt oder keine lebenden Einheiten mehr hat
     (`Sim::living_units(player)`, die Stelle, an der 017 seine Abfrage einhängt).
   - Die Partie ist entschieden, sobald höchstens ein Spieler unbesiegt ist: Sieger
     oder Unentschieden (alle im selben Tick besiegt). Nur bei mindestens zwei
     Spielern zum Start. Ergebnis mit Tick, Grund pro Spieler; danach unveränderlich.
   - Der Ergebnisstand geht in den State-Hash ein, aber erst sobald jemand besiegt
     ist: Der Golden-Hash laufender Partien bleibt gleich.
   - Fraktionswahl: Das Szenario nennt pro Fraktion eine Einheit
     (`[faction_units]` in `demo.toml`), `parse_scenario_with_factions` ersetzt die
     Einheiten eines Spielers durch die seiner Fraktion.
2. **Bridge:** `start_match(seed, map_dir, factions)`, `surrender()`,
   `remove_units(ids)` (nur Einzelspieler), `match_result()` (Dictionary),
   `previous_positions()` nach Einheiten-Id ausgerichtet (Einheiten können
   verschwinden).
3. **Hauptmenü** (`godot/scenes/menu/main_menu.tscn`, neue Startszene):
   Einzelspieler, LAN, Einstellungen, Beenden.
4. **Einzelspieler-Setup:** Karte (eingebaute `test_small`, alle Ordner in
   `data/maps/`), Biom (Kartenvorgabe, Wald, Wüste, Winter), eigene und gegnerische
   Fraktion (Deutschland, Sowjetunion, USA). Übergabe an `main.tscn` über eine
   statische Klasse wie `lan_match.gd`.
5. **LAN:** Die bestehende Lobby öffnet sich aus dem Menü, bekommt das Menü-Theme,
   einen Zurück-Knopf und für den Host eine Kartenwahl. `just lan` startet sie weiter
   direkt.
6. **Einstellungen** (`user://settings.cfg`, `ConfigFile`): Grafikstufe,
   Tastenbelegung der Kamera-Aktionen (Tastatur), Lautstärke (Master-Bus, für
   später), Kamera-Empfindlichkeit (Schwenk- und Zoomtempo), Randscrollen. Gilt in
   Menü, Partie und Lobby; Kommandozeile (`--quality`) geht vor.
7. **Ladebildschirm:** `main.gd` baut die Partie schrittweise über mehrere Frames auf
   (Simulation, Gelände, Atmosphäre, Objekte) und zeigt dabei Schritt und
   Fortschritt; auf `test_large` sichtbar, auf `test_small` kurz.
8. **Pause** (Esc): im Einzelspieler hält das Spiel an (Weiter, Einstellungen,
   Aufgeben, Zurück ins Menü); im Mehrspieler läuft es weiter, das Menü bietet nur
   Aufgeben und Verlassen. Keine Pause-Abstimmung.
9. **Ergebnisbildschirm:** Sieg, Niederlage oder Unentschieden, Grund, Spieldauer,
   verbliebene Einheiten pro Spieler, „Zurück ins Menü“. Das Replay wird wie bisher
   geschrieben.
10. **Startpfade bleiben:** `just run` öffnet das Menü. Das Menü springt direkt in
    die Partie bei `--skip-menu` und bei allen bisherigen Direktstart-Argumenten
    (`--map`, `--demo`, `--camera-shots`, Benchmarks, `--path-debug`). `just lan`,
    `just lan-selftest`, `just gallery` bleiben unverändert.

## 4. Akzeptanzkriterien

1. `just check` grün. Golden-Hash unverändert (Ergebnis wird erst gehasht, wenn
   jemand besiegt ist); falls doch nicht, Grund im Commit.
2. Rust-Tests: Aufgeben, Auslöschung (per `RemoveUnits`), Unentschieden, fremde
   Einheiten nicht entfernbar, Ergebnis bleibt nach Entscheidung fest, Ein-Spieler-
   Konfiguration wird nie entschieden, Fraktionswahl im Szenario, ein Replay mit
   Aufgabe re-simuliert zum gleichen Hash.
3. `just selftest` startet im Hauptmenü und deckt den ganzen Ablauf ab: Einstellungen
   öffnen, ändern, speichern, laden (eigene Testdatei, nicht die des Nutzers);
   Einzelspieler-Setup, Ladebildschirm, bisherige Partie-Tests, Pause hält die Sim an,
   Aufgeben → Niederlage; zweite Partie mit anderer Fraktion, eigene Einheiten
   entfernen → Niederlage durch Auslöschung; Ergebnisbildschirm → zurück ins Menü;
   LAN-Lobby öffnen und zurück.
4. `just lan-selftest` grün und erweitert: Nach Tick 400 gibt der Client auf, der
   Host sieht Sieg, der Client Niederlage.
5. `just run -- --demo` (bzw. `godot --path godot -- --demo`), `just run-map`,
   `just gallery` funktionieren weiter.
6. Doku: `docs/ARCHITECTURE.md` (Siegbedingungen, Spielablauf, neue Befehle),
   `docs/LAN.md` (Lobby aus dem Menü, Aufgeben), `CLAUDE.md` falls nötig.

## 5. Entscheidungen

| Frage | Entscheidung | Begründung |
|---|---|---|
| Sieg ohne Kampf (017 parallel) | Aufgeben + Auslöschung über `living_units`, Testbefehl `RemoveUnits` | Funktioniert sofort; 017 ersetzt nur die Zählung lebender Einheiten. |
| `RemoveUnits` nur für eigene Einheiten | ja | Sonst wäre es im LAN ein Schummelbefehl. Ein Sieg durch Auslöschung lässt sich im Einzelspieler-Selbsttest so nur als Niederlage prüfen; den Sieg prüft der LAN-Selbsttest (Gegner gibt auf) und die Rust-Tests. |
| Ergebnis im Hash nur ab erster Niederlage | ja | Hält den Golden-Hash (und die Zusammenführung mit 014/017) stabil; der Zustand ist trotzdem vollständig gehasht, weil er vorher leer ist. |
| Teams | keine, jeder gegen jeden | Es gibt zwei Spieler; Teams kommen mit dem KI-Gegner (024) oder mehr Spielern. |
| Startszene | neues Hauptmenü, Direktstart-Argumente überspringen es | Bestehende Rezepte und Aufrufe laufen unverändert. |
| Einstellungen | statisches Skript per `preload`, kein Autoload | Wie `lan_match.gd`; keine Änderung der Projektkonfiguration nötig. |
| Ladebildschirm | schrittweiser Aufbau in `main.gd` mit `await` | Die teure Arbeit (Pfadraster, Gelände, Objekte) liegt in `_ready`, nicht im Laden von Ressourcen; Threads wären für die Sim nicht erlaubt. |
| Pause im Mehrspieler | keine, nur Menü | Lockstep müsste alle anhalten; eine Abstimmung lohnt erst mit längeren Partien. |
| Stil | dunkles Feldgrau/Oliv, Sand als Akzent, Godot-Standardschrift, schlichte Rahmen | Passend zum WW2-Thema, gut lesbar, keine Downloads; keine Hoheitszeichen oder verbotenen Symbole, Fraktionen nur als Name. |
| UI-Sprache | Englisch wie die bestehende Lobby | Einheitlich; Übersetzung später. |

## 6. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, keine Tests abschwächen, Commits pro Schritt, `just check`, `just selftest`
und `just lan-selftest` grün. Am Ende nach `prompts/done/` verschieben und
`prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde, wie man es
testet, was offen ist.
