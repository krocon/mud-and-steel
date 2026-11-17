# Prompts – Roadmap „Mud and Steel“

Jeder Prompt ist ein abgeschlossener Arbeitsauftrag für eine frische Coding-Agent-Session
im Repo-Ordner. Die Arbeit läuft in **zwei Strängen**, die weitgehend unabhängig sind
und parallel laufen können:

- **Asset-Strang (003–008):** Maßstab, Einheitenkatalog, Blender-Baukasten, Modelle.
  Er braucht nur den Katalog und die Konventionen, keinen fertigen Spielcode.
- **Spiel-Strang (002, 009–028, ab 029 Stufe 2):** Kamera, Einheitensteuerung, Karte, Gelände, Gameplay,
  Netzwerk. Er nutzt die Modelle, sobald sie da sind, und arbeitet bis dahin mit
  Platzhaltern.

## Ablauf

1. Den nächsten offenen Prompt nehmen und kurz prüfen, ob er zum aktuellen Stand passt.
   Als „Entwurf“ markierte Prompts vorher konkretisieren lassen
   („Bitte konkretisiere Prompt 0XX“).
2. In einer frischen Session ausführen: „Bitte führe `prompts/0XX-….md` aus.“
   **Parallel laufende Sessions immer in eigenen Git-Worktrees/Branches**, damit sie
   sich nicht gegenseitig Dateien überschreiben. Zusammenführen in einer kurzen
   eigenen Session.
3. Ist ein Prompt fertig, verschiebt der Agent die Datei mit `git mv` nach
   `prompts/done/` und setzt den Status hier auf „erledigt“. Bei parallelen Prompts
   (006–008) macht das erst die zusammenführende Session.

## Paralleler Fahrplan

```
Welle 1:  002 Kamera ─────────────┐        003 Maßstab + Katalog
                                  │                 │
Welle 2:  009 Einheiten/Lockstep  │   004 Baukasten    005 Infanterie-Basis
                                  │        │                 │
Welle 3:  010 Karte ─ 011 Terrain │   006 de │ 006 su │ 006 us │ 007 Umwelt
                                  │        └──── zusammenführen ────┘
Welle 4:  012 Wegfindung ─ 013 …  │   008 Stufe 2 (3 Sessions pro Fraktion parallel)
```

- **Sofort möglich:** 002 und 003 gleichzeitig.
- **Nach 003:** 004 und 005 gleichzeitig, daneben weiter der Spiel-Strang (009 …).
- **Nach 004 und 005:** vier parallele Sessions: 006 für `de`, `su`, `us` und 007.

## Übersicht

### Asset-Strang

| Nr. | Prompt | Ziel | Status |
|-----|--------|------|--------|
| 003 | [Maßstab und Einheitenkatalog](done/003-masstab-und-einheitenkatalog.md) | 1:1-Größen, Zeitfaktor 2, Katalog aller Einheiten mit Quellen, Asset-Konventionen | erledigt |
| 004 | [Blender-Baukasten](done/004-blender-baukasten.md) | Bauteile, Bauplan → Modell, Tarn-Material-System, Prüfung, Galerie | erledigt |
| 005 | [Infanterie-Basis](done/005-infanterie-basis.md) | Ein Soldat für alle Fraktionen: Skelett, Uniformen, Waffen, Animationen | erledigt |
| 006 | [Kerneinheiten Stufe 1](done/006-kerneinheiten-stufe1.md) | Je 5 Kerneinheiten pro Fraktion, 3 Sessions parallel | erledigt |
| 007 | [Umwelt-Objekte Stufe 1](done/007-umwelt-objekte-stufe1.md) | Bäume, Büsche, Gras, Häuser, Brücken, Mauern, Stacheldraht, Minen, Gräben | erledigt |
| 008 | [Einheiten Stufe 2](done/008-einheiten-stufe2.md) | Alle restlichen Einheiten inkl. Flugzeuge, 9 Sessions parallel (Vorab-Session Baukasten erledigt) | erledigt |

### Spiel-Strang

| Nr. | Prompt | Ziel | Status |
|-----|--------|------|--------|
| 001 | [Projekt-Setup](done/001-projekt-setup.md) | Gerüst, Sim, Lockstep, Bridge, CI | erledigt |
| 002 | [RTS-Kamera](done/002-rts-kamera.md) | Schwenken, Zoomen, Drehen über großem Spielfeld | erledigt |
| 009 | [Einheiten, Auswahl, Lockstep im Spiel](done/009-einheiten-auswahl-lockstep.md) | Mehrere Einheiten, Auswahlrahmen, Gruppenbefehle, Replays | erledigt |
| 010 | [Kartenformat und Höhenfeld](done/010-karte-hoehenfeld.md) | Karte als Daten, Höhen und Geländearten in der Sim | erledigt |
| 011 | [Terrain-Darstellung](done/011-terrain-darstellung.md) | Großes Gelände mit LOD und Textur-Splatting | erledigt |
| 012 | [Wegfindung](done/012-wegfindung.md) | Deterministisches A* mit Hindernissen und Gruppenbewegung | erledigt |
| 013 | [Vegetation und Objekte platzieren](done/013-vegetation-platzieren.md) | Dicht bestücktes Gelände mit den Modellen aus 007 | erledigt |
| 014 | [Wasser, Flüsse und Brücken](done/014-wasser-fluesse-bruecken.md) | Flüsse, Ufer, Brücken als Engstellen | erledigt |
| 015 | [Licht, Atmosphäre und Biome](done/015-licht-atmosphaere-biome.md) | Sonne, Schatten, Nachbearbeitung, Wiese/Wüste/Winter | erledigt |
| 016 | [Performance für große Karten](done/016-performance-grosse-karten.md) | Benchmarks, räumliches Raster, Budgets | erledigt |
| 017 | [Kampf-Grundlagen](done/017-kampf-grundlagen.md) | Lebenspunkte, Waffen, Geschosse, Treffer aus dem Katalog | erledigt |
| 018 | [Nebel des Krieges](done/018-nebel-des-krieges.md) | Sicht pro Spieler mit Sichtlinie über Gelände | erledigt |
| 019 | [Effekte](done/019-effekte.md) | Explosionen, Rauch, Mündungsfeuer, Leuchtspur | erledigt |
| 020 | [Zerstörbare Gebäude und Vegetation](done/020-zerstoerung.md) | Schadensstufen, Trümmer, umfallende Bäume | erledigt |
| 021 | [Verformbares Gelände](done/021-gelaendeverformung.md) | Krater verändern das Höhenfeld | erledigt |
| 022 | [Infanterie-Verhalten](done/022-infanterie-verhalten.md) | Trupps, Haltungen, Deckung, Gebäude besetzen | erledigt |
| 023 | [Basisbau und Ressourcen](done/023-basisbau-ressourcen.md) | Bauen, Ressourcen, Produzieren; Design in `docs/GAME_DESIGN.md` (vorläufig) | erledigt |
| 024 | [KI-Gegner](done/024-ki-gegner.md) | Computerspieler über normale Befehle | erledigt |
| 025 | [LAN-Mehrspieler](done/025-lan-mehrspieler.md) | ENet-Transport, Spielsuche per Broadcast | erledigt |
| 026 | [Online über Steam](done/026-online-steam.md) | Steam-Lobbys, Einladungen, Transport über Steam Networking (steamworks-rs) | erledigt |
| 027 | [Menüs und Spielablauf](done/027-menues-spielablauf.md) | Hauptmenü, Lobby, Einstellungen, Sieg/Niederlage | erledigt |
| 028 | [Karteneditor](done/028-karteneditor.md) | Karten im Spiel bauen und speichern | erledigt |

### Stufe 2: vom Prototyp zum Spiel (029–045)

Analyse nach Welle 9: Alle Grundsysteme stehen, es fehlen Sound, Bedienkomfort, die
Waffengattungen, deren Modelle schon gebaut sind (Artillerie, Flugzeuge, Transport,
Pioniere), Logistik, Teams, Speichern, Einstieg für neue Spieler, Wetter, mehr
Umgebung und der Weg zum Release. Als „Entwurf“ markierte Prompts werden vor dem
Umsetzen von ihrer Session konkretisiert.

| Nr. | Prompt | Ziel | Status |
|-----|--------|------|--------|
| 029 | [Sound-Effekte](done/029-sound-effekte.md) | Gefecht, Motoren, UI; Busse, Stimmenlimit, Quellen | erledigt |
| 030 | [Musik und Umgebungsklang](done/030-musik-ambiente.md) | Biom-Ambiente, Musik ruhig/Gefecht, Menü | erledigt |
| 031 | [HUD, Minikarte, Befehle](done/031-hud-minimap-befehle.md) | Minikarte, Einheitenanzeige, Befehlsleiste, Wegpunktketten; Steuerung in `docs/CONTROLS.md` | erledigt |
| 032 | [Indirektes Feuer und Rauch](done/032-indirektes-feuer-rauch.md) | Mörser, Haubitzen, Splitter, Rauch blockiert Sicht | erledigt |
| 033 | [Flugzeuge und Flak](done/033-luftwaffe-flak.md) | Luftunterstützung als Abruf (Aufklärung, Jäger, Schlachtflug, Bomber), Flak und Fliegerabwehr-MGs | erledigt |
| 034 | [Transport und Schleppen](done/034-transport-schleppen.md) | Aufsitzen, Geschütze ziehen und abprotzen | erledigt |
| 035 | [Pioniere, Minen, Stellungen](done/035-pioniere-minen-stellungen.md) | Minen, Draht, Gräben, Brücken reparieren/sprengen | erledigt (Werte vorläufig, siehe Ergebnis) |
| 036 | [Nachschub und Reparatur](done/036-nachschub-reparatur.md) | Verbrauch pro Einheit, Versorgungs-LKW, Reparatur, Bergung | erledigt (Design vorläufig, Rest offen, siehe Ergebnis) |
| 037 | [Teams und KI-Plätze](done/037-teams-ki-slots.md) | 2v2/3v3, KI in der Lobby, Moduswahl im LAN; `docs/TEAMS.md` | erledigt (vorläufige Entscheidungen in `docs/TEAMS.md` zu bestätigen) |
| 038 | [Speichern und Replays](done/038-speichern-replays.md) | Spielstände, Replay-Ansicht mit Vorspulen | erledigt |
| 039 | [Tutorial und Missionen](done/039-tutorial-missionen.md) | Skript-System, Tutorial, Beispielmissionen; `docs/MISSIONS.md` | erledigt (vorläufige Entscheidungen in `docs/MISSIONS.md` zu bestätigen) |
| 040 | [Wetter, Schlamm, Tageszeit](done/040-wetter-schlamm-tageszeit.md) | Regen weicht Boden auf, Nebel, Nacht | erledigt (Werte vorläufig, Rest offen, siehe Ergebnis) |
| 041 | [Umwelt Stufe 2 und Feuer](done/041-umwelt-stufe2-feuer.md) | Stadt, Industrie, Bahn, Bunker; Brände und Lager-Explosionen | erledigt (Rest offen, siehe Ergebnis) |
| 042 | [Infanterie-Texturen](done/042-infanterie-texturen.md) | Prozedurale Uniform-Texturen, Tarnmuster, Pistolen, zehn neue Clips | erledigt |
| 043 | [Sprache und Barrierefreiheit](done/043-sprache-barrierefreiheit.md) | Deutsch/Englisch, Farbpaletten, UI-Skalierung, Grafikoptionen | erledigt (vorläufige Entscheidungen im Prompt zu bestätigen) |
| 044 | [Release-Builds und Steam](done/044-release-builds-steam.md) | Export für drei Systeme, Release-CI, SteamPipe, Credits | erledigt (erster Export mit Godot-Export-Templates steht aus, `docs/RELEASE.md`) |
| 045 | [Balancing und Playtest](done/045-balancing-playtest.md) | Statistik, Turniere, Gefechtssimulator, bestätigtes Design | erledigt (Bestätigung durch Playtests offen) |

```
Welle 10:  029 Sound │ 031 HUD │ 032 Indirektes Feuer │ 038 Speichern │ 042 Infanterie │ 044 Release
Welle 11:  030 Musik (nach 029) │ 033 Flugzeuge (nach 032) │ 034 Transport │ 037 Teams │ 041 Umwelt 2
Welle 12:  035 Pioniere │ 036 Nachschub (nach 034) │ 040 Wetter │ 043 Sprache (nach 031) │ 057 Wiese │ 058 Produktionsbilder │ 059 Wüstendorf (nach 041) │ 060 Berge und Täler
Welle 13:  039 Tutorial/Missionen (nach 031, 038) │ 046 Befehle │ 048 Animationen │ 053 Sound 2 │ 056 Karten 2
Welle 14:  047 Infanterie-Taktik (nach 046) │ 049 Verwundung │ 050 Erfahrung │ 052 Stellungen 2 │ 054 Explosionen 2 │ 055 Brücken/Luftlande
Welle 15:  051 KI-Kommandeur (nach allem, was die KI nutzen soll)
Welle 16:  045 Balancing (zuletzt)
```

### Stufe 3: aus der Wunschliste (046–056)

Allgemeine Spielmechaniken aus einer Wunschliste, auf WW2 übertragen. Moderne Inhalte
(Drohnen, Atomwaffe, moderne Fahrzeuge und Hubschrauber, Laser) sind bewusst ersetzt
oder weggelassen; es werden keine Inhalte anderer Spiele übernommen.

| Nr. | Prompt | Ziel | Status |
|-----|--------|------|--------|
| 046 | [Befehle und Formationen](done/046-befehle-formationen.md) | Angriffsbewegung, Gebiet sichern, Auto-Formation, Ausrichtung per Ziehen | erledigt (Schwellen und Tasten vorläufig, siehe Abschnitt 4) |
| 047 | [Infanterie-Taktik](done/047-infanterie-taktik.md) | Feuertrupps, Deckung zu Deckung, Niederhalten, Blindfeuer | erledigt (Sprungweiten, Niederhalt-Werte, Ladehemmung vorläufig, siehe Ergebnis) |
| 048 | [Animationen und Ragdolls](done/048-animationen-ragdolls.md) | Deckungs-, Kriech- und Rollanimationen, Ragdolls nur als Darstellung | erledigt (Umwerf-Werte, Ragdoll-Budget und Trefferdarstellung vorläufig, siehe Ergebnis) |
| 049 | [Verwundung und Kameradenhilfe](done/049-verwundung-kameradenhilfe.md) | Verwundete, Bergen, Sanitäter mit Trage, ausgestiegene Besatzungen | erledigt (Werte vorläufig; Gefangene und Wiederbesetzen offen, siehe Ergebnis) |
| 050 | [Erfahrung und Ränge](done/050-erfahrung-raenge.md) | Veteranenstufen mit moderaten Boni und Abzeichen | erledigt (Werte vorläufig, siehe Ergebnis) |
| 051 | [KI-Kommandeur Stufe 2](done/051-ki-kommandeur.md) | Aufklären, Lagebild, Konter, Versorgungskrieg, Aufgeben | erledigt (Ziele nur teilweise erreicht, Werte vorläufig, siehe Ergebnis) |
| 052 | [Stellungen Stufe 2](done/052-stellungen-stufe2.md) | Gräben, Sperrlinien, eingegrabene Panzer, Tarnnetze, Bunker | erledigt (Werte vorläufig, siehe Ergebnis) |
| 053 | [Sound Stufe 2](done/053-sound-stufe2.md) | Überschallknall, Schallverzögerung, Hall, Zoom-Akustik, Flugzeuge, Brände, Stimmen-Struktur | erledigt |
| 054 | [Explosionen Stufe 2](done/054-explosionen-stufe2.md) | Druckwelle, Blitzlicht, Munitionsexplosion, schwerer Feuerschlag | erledigt (Werte vorläufig, siehe Ergebnis) |
| 055 | [Brücken, Pioniertechnik, Luftlande](done/055-bruecken-pioniertechnik-luftlande.md) | Brückeneinsturz, Ertrinken, Brückenleger, Fallschirmjäger | erledigt (Werte vorläufig, siehe Ergebnis) |
| 056 | [Karten und Basis Stufe 2](done/056-karten-basis-stufe2.md) | Winterdorf, Höfe, Flussland, Punkte an Orten, Hinterhalt | erledigt |
| 057 | [Bodenoptik: Wiesen](done/057-bodenoptik-wiese.md) | Feines Gras, Farbvariation, Halmbüschel, Blüten, Übergänge, Tilt-Shift | erledigt |
| 058 | [Bilder in der Produktionsauswahl](done/058-bilder-produktionsauswahl.md) | Vorgerenderte Bilder als Kacheln mit Kosten, Reiter; `just thumbnails` | erledigt |
| 059 | [Wüstendorf](done/059-wuestendorf.md) | Flachdachhäuser mit Hofmauern, Palmenhaine, Dorfkarte Nordafrika | erledigt |
| 060 | [Berge und Täler](done/060-berge-taeler.md) | Gebirgs-Profile im Generator, Pässe, Hangregeln, Gebirgskarte | erledigt |
| 061 | [Recherche: freie Soundeffekte](done/061-sound-recherche.md) | Geprüfte CC0/CC-BY-Aufnahmen, Pakete, Lücken, Ordnerstruktur in `docs/AUDIO_LIBRARY.md` | erledigt |
| 062 | [Raketenwerfer und Sturmpanzer](done/062-raketenwerfer-sturmpanzer.md) | Fahrbare Raketenwerfer je Fraktion mit Salven-Regel, deutscher Sturmpanzer; Katalog, Modelle, KI | erledigt (Panzerwerfer 42, BM-13-16, T34 Calliope, Sturmpanzer IV; Entscheidungen vorläufig (Nachtlauf, Vorschläge übernommen); vorläufig auch Salvenabstand, Streuung 8 mrad, Mindestreichweite 5 %, Dauerfeuerraten und mehrere Fahrzeugmaße (geschätzt, in `uncertain`); Raketendarstellung bewusst einfach, kommt mit 076; Balance der KI-Mischungen offen) |

Viele dieser Prompts ändern `sim.rs`, `command.rs`, `main.gd` und den Golden-Hash.
Parallel laufende Sessions übernehmen vor dem Abschluss den aktuellen `main`.

### Stufe 4: Fahrzeugdaten (063–069)

Die Vorlagen in `docs/.vorlagen/fahrzeuge/` (nicht im Repo, geladen mit
`just vehicle-refs`) enthalten technische Daten und Zeichnungen zu 141 Panzerfahrzeugen.
Daraus entstehen fehlende Katalog-Einträge und Modelle, eine JSON-Datenbank und die
Anzeige der Werte auf der Webseite und im Spiel. Übernommen werden nur Fakten; die
Zeichnungen bleiben Referenz auf dem Rechner.

| Nr. | Prompt | Ziel | Status |
|-----|--------|------|--------|
| 063 | [Fahrzeug-Vorlagen: Modelle](done/063-fahrzeug-vorlagen-modelle.md) | Bestandsliste aller 141 Vorlagen, fehlende Fahrzeuge in Katalog und als Modell, vorhandene Modelle gegen Maße und Zeichnungen prüfen; Vorab-Session, dann 9 Sessions parallel | erledigt (Nachtlauf 17.11.2026, Gruppen nacheinander auf `main`: 141 Ordner = 17 existing / 111 new / 13 skipped; alle 128 Fahrzeuge mit Modell, Bild, Prüfzeile in `docs/VEHICLE_REFS.md` und `just ref-compare`; geprüft und korrigiert u. a. Wespe, Panzerjäger I, StuG III G, Pz III L, Pz IV H, Sturmpanzer IV, Tiger I, T-70M, SU-76M, T-34/76 1943, SU-85, IS-2, ISU-152, M10, M26, M4A3(75)W (Mündung am Bug, Länge über die Sandbleche); Baukasten rückwärtsverträglich erweitert (`early_tanks`, `plate_turret`/`cast_turret` mit `x`, `spokes`, `top_inset`). Vorläufig: alle Entscheidungen aus Abschnitt 7 autonom (Auswahl/skipped, `tier = 3`, nicht baubar, Zweitquellen-Prüfung erst bei Baubarkeit); Fahrzeuge ohne eigene Zeichnung nach Verwandten gebaut (Hummel, Nashorn, IV/70 V, Pz IV A, Elefant als Ferdinand, PaK 40/H39, T-34-85 1943, T-35 1935, SU-76, OT-130, KV-1 1939/KV-1E/KV-2/SU-152/IS-1/ISU-122/122S, M36, M5A1); Katalog-Widersprüche gemeldet statt geraten (IV/70 V Wanne, SU-85/SU-100-Höhen, Lee M2-Kanone mit M3-Länge, M4A3(76)W HVSS Mündungsbremse M1A2 statt M1A1, M4A3E2 Breite über Ketten → Spurweite zu breit); Sherman-Familie: 75-mm-Varianten mit Mündung am Bug (data.json Überstand 0), M4A2-Zeichnung mit 47°-Wanne; M24 ohne Bug-/Koax-MG und IS-3 ohne DShK (Katalog). Offen: Zweitquellen der neuen Einträge, Baubarkeit (Prompt 045); `just bench-view` (hoch) Schlacht 600 m nur 40 FPS gegen Budget 45 und viele Godot-Fehler „Too many instances using shader instance variables“ (Bench-Szenario ohne `tier = 3`-Modelle, also nicht durch 063; nicht untersucht) |
| 064 | [Fahrzeug-Datenbank](done/064-fahrzeug-datenbank.md) | `data/vehicles/*.json` mit Schema, Quelle und Prüfung gegen den Katalog; Zugriff für Spiel und Webseite | erledigt (130 Einträge; dabei aus 063 vorgezogen: `tools/vehicle_refs.py`, `data/vehicles/index.json`, `docs/VEHICLE_REFS.md`) |
| 065 | [Webseite: Fahrzeugdaten und Spielwerte](done/065-webseite-fahrzeugdaten-spielwerte.md) | Technische Daten in der Detailansicht, Spielwerte aus der Sim (`cli unit-stats`), fünf Kennzahlen | erledigt (offen: Hosting-Anbieter in der Datenschutzerklärung eintragen) |
| 066 | [Einheitenwerte im Spiel](done/066-einheitenwerte-im-spiel.md) | Kennzahlen im Picker, Tooltip, Einheitenkarte, Nachschlagewerk im Hauptmenü | erledigt (Taste Umschalt + I statt I, weil I „Sichern“ ist; offen: Vorhersage beim Zielen, auf kleinen Fenstern verdeckt die Karte die Einheitenanzeige) |
| 067 | [Typische Kampfentfernungen](done/067-typische-kampfentfernungen.md) | Typische statt maximale Reichweiten im Katalog, mit Quellen; Balance neu messen | erledigt (`range_typical_m` je Waffe, Klassenwerte mit Quellen; Direktfeuer × 1/4 bleibt; Gefecht 49 / 48 / 46 % statt 48 / 41 / 47 %; neuer Preisfaktor Mündungsgeschwindigkeit vorläufig; offen: USA im Basis-Modus 15 %, siehe `docs/BALANCE.md`) |
| 068 | [Wiese: Artefakte bei Kamerabewegung](done/068-wiese-artefakte-bei-kamerabewegung.md) | Flimmern und Moiré des Rasens beim Bewegen des Bildausschnitts nachstellen, messen, Ursache eingrenzen; kleine Ursachen beheben | erledigt (Nachtlauf 17.11.2026: Hauptursache in 071 behoben; Nahbereich 15/35/55 m mit `just ground-motion --near` und `--off=` gemessen: Textur haftet am Boden (0,15–0,17 Pixel/Bild), Rest im 2–4-Pixel-Band 0,72–0,82 ist Detail an der Pixelgrenze ohne Einzelursache, nur TAA hebt es auf 0,93–0,95; keine Änderung an der Darstellung. Vorläufig: Entscheidungen aus Abschnitt 7 autonom (Ort `test_large`, keine Rückfrage). Offen: TAA als Standard (Vorschlag, Kosten nicht gemessen), Straße 2–4 px aus 071. Nebenbei: Instanz-Uniform-Puffer auf 1 048 576, keine „Too many instances“-Fehler mehr; Schlacht 600 m 38–40 FPS schon vor 062/063) |
| 069 | [Sound-Verbesserungen](done/069-sound-verbesserungen.md) | Kettenfahrzeuge (Motor, Kettenrasseln), MG-Feuer, Panzerschüsse, Treffer und Explosionen mit besseren Aufnahmen; Pixabay-, Epidemic- und ElevenLabs-Kandidaten auf Lizenz und Klang prüfen; ElevenLabs-API als Quelle bewerten | erledigt (Panzerschüsse, Durchschlag, Abpraller, Explosionen, Nachhall und Kettenrasseln aus Freesound, CC0 / CC BY; MG und Motoren schwer/leicht mit ElevenLabs über das private Repo `mud-and-steel-sfx`, `priv_*.wav`; `just sfx-generate`; Pixabay nur lokal, Epidemic ausgeschlossen; offen: MG-Treffer auf Panzerung und Halbkette bleiben synthetisch, MG 42 weiter Lubini, kein Begrenzer auf den Bussen, Extremfall übersteuert) |

```
063 Vorab-Session ─┬─ 063 Modell-Sessions (9 parallel) ─ zusammenführen
                   └─ 064 Datenbank ─ 065 Webseite + Spielwerte ─ 066 Anzeige im Spiel
```

### Außer der Reihe: Öffentlichkeit (070)

| Nr. | Prompt | Ziel | Status |
|-----|--------|------|--------|
| 070 | [YouTube-Demo-Video](periodic/070-youtube-demo-video.md) | Demo-Video von 1 bis 4 Minuten: geskriptete Spielszenen aus wechselnden Blickwinkeln (`just trailer-shots`, Movie Maker), Schnitt als Promo im Video-Projekt `videoserien`; Freigabe vor dem Rendern, kein Upload | erledigt (16.11.2026: 1:39, 17 Einstellungen auf 5 Karten; wiederholbar, liegt in `periodic/`) |

### Außer der Reihe: Darstellung, Umwelt und Karten (071–081)

| Nr. | Prompt | Ziel | Status |
|-----|--------|------|--------|
| 071 | [Bodentextur gleitet bei Kamerabewegung](done/071-bodentextur-gleitet-bei-kamerabewegung.md) | Wandernde Schlieren auf Wiese und Straße beheben: der Maßstab der zweiten Texturabtastung in `terrain.gdshader` hängt vom Kameraabstand ab; zwei feste Maßstäbe überblenden, mit `tools/ground_flicker.py` nachmessen | erledigt (zwei feste Maßstäbe überblendet, `just ground-motion`; Wiese gleitet 4,7 → 0,2 Pixel/Bild; offen: Straße im 2–4-Pixel-Band 0,52, Nahbereich aus 068, Demo-Video neu aufnehmen) |
| 072 | [Explosionen und Rauch nach Videovorlage](072-explosionen-nach-videovorlage.md) | Video `.temp/rts-explosionen.mp4` Bild für Bild auswerten (Phasen, Zeiten, Größen, Farben, Rauchverlauf), Explosionen und Rauch in `effects.gd` / `blast_effects.gd` danach nachstellen, Bildreihen Video gegen Spiel, `just effects-bench` über 60 FPS; nur Darstellung, nichts aus dem Video ins Repo | offen (Video liegt vor; nur als Vorbild für Ablauf und Rauchverhalten, eigene Texturen) |
| 073 | [Hohes Gras nach Bildvorlage](073-hohes-gras.md) | Dichter, hoher Bewuchs wie in den Tropen nach `.temp/dschungel-grass.png`: neue Geländeart `tall_grass` (malbar im Editor, Bewegung wie Gras, keine Tarnung), Büschel, Kräuter und Sträucher aus einzelnen Halmen, Shader-Fassung für die Ferne, Windwellen, flach unter Fahrspuren; ohne Flimmern, über 60 FPS | offen |
| 074 | [Felsen nach Bildvorlage](074-felsen.md) | Felsplatten im Boden, Felsgruppen und Felsrücken nach `.temp/felsen*.png` als erzeugte Umwelt-Objekte (Baukasten, Shader-Muster ohne Texturen), eingebettet in den Boden mit Bewuchs am Fuß; Gruppen und Rücken halten alle auf und verdecken die Sicht; Editor-Vorlagen, Karten, Vorschaubilder | offen |
| 075 | [Hausdächer mit Patina](075-dach-patina.md) | Dächer nach `.temp/haus-daecher.png` altern lassen: Alter je Haus, Ton je Ziegel, Ausbesserungen, Schmutzfahnen entlang der Neigung, Verlauf First zu Traufe, Moos und Flechten, für Tonziegel, Schiefer, Stroh, Bretter und Wellblech in `env.gdshader`; ohne Flimmern, Vorschaubilder der Gebäude neu | offen |
| 076 | [Raketensalve: Abschuss, Flamme und Rauchspur](076-stalinorgel-effekte.md) | Salve des schweren Feuerschlags nach `.temp/stalinorgel*.png`: Feuerschein und Bodenrauch am Werfer je Rakete, Raketenkörper mit Flamme statt dunkler Kugel, stehende helle Rauchspur; nur Darstellung, für alle Fraktionen, wiederverwendbar für die fahrbaren Werfer aus 062; zwei Salven zugleich über 60 FPS | offen (nicht gleichzeitig mit 072 im selben Arbeitsverzeichnis) |
| 077 | [Mündungsfeuer aus dem Panzerrohr](077-muendungsfeuer-panzerrohr.md) | Feuerstoß nach `.temp/blitz-aus-rohr.png`: Kern, lange Feuerzunge in Rohrrichtung (aus `Socket_Muzzle`, mit Rohrerhöhung), glühende Gaswolke, Pulverrauch, Bodenstaub, Licht; abgestuft nach Waffe, Leuchtspur ab der Mündung, Rohrrücklauf und Wippen; nur Darstellung, 40 Schüsse je Sekunde über 60 FPS | offen (nicht gleichzeitig mit 072 oder 076 im selben Arbeitsverzeichnis) |
| 078 | [Hochhäuser: mehrgeschossige Wohnblöcke](078-hochhaeuser.md) | Wohnblöcke mit vier bis sechs Geschossen nach `.temp/hochhäuser*.png`, als Bautyp der Zeit mit Quellen: zwei Flachdach-Blöcke für Wüstenstädte, ein Mietshausblock für Europa, je `Intact` / `Damaged` / `Ruin` mit sichtbarem Inneren; Regeln wie Gebäude (Weg, Sicht, Zerstörung, Besetzen, gedeckelt); Kamera und verdeckte Einheiten an hohen Häusern; Stadtviertel zum Ansehen | offen |
| 079 | [Große Karte mit Großstadt im Zentrum](079-karte-grossstadt.md) | Neue Karte `city_large` (3 km, Winter, vier Starts in zwei Teams) aus dem Generator `just gen-city` nach `.temp/map_grossstadt1.png`: Zentrum mit Platz, Blockviertel, Vorstadt, Land; punktsymmetrisches Straßennetz mit mindestens drei Fahrzeugrouten und zwei Umgehungen; Bildrate, Tickzeit, Wegfindung, Sicht und KI auf der Karte gemessen | offen (braucht 078) |
| 080 | [Schnee auf Dächern](080-schnee-auf-daechern.md) | Dachschnee nach `.temp/schnee_auf_dächern.png`: Decke mit klarem Rand statt Farbmischung, liegt an First und Traufe, rutscht dazwischen ab, vorgetäuschte Dicke, je Dachart und je Haus anders, in `env.gdshader`; zweite Stufe mit eigenem Schnee-Teil im Modell nur bei Bedarf; ohne Flimmern | offen (nicht gleichzeitig mit 075 im selben Arbeitsverzeichnis) |
| 081 | [Karte nach echten Geodaten: Hafenstadt](081-karte-hafenstadt.md) | Importwerkzeug `just gen-geo` schreibt aus OpenStreetMap und einem freien Höhenmodell die Karte `harbor_city` nach dem heutigen Monaco, kräftig vereinfacht: echtes Gelände, Küste und tragendes Straßennetz 1:1, Gebäude aus dem Katalog an den echten Standorten; kein echter Ortsname im Spiel; das steile Gelände bleibt und muss nicht befahrbar sein (Fahrzeuge an Hafen und Küste, Infanterie am Hang); drei Stufen ohne Halt; Namensnennung und Lizenz in `docs/SOURCES.md` und `CREDITS.md` | offen (Downloads freigegeben; besser nach 078) |

## Festgelegte Grundsätze

- **Größen 1:1** in echten Metern, auch Infanterie (1,80 m).
- **Zeitfaktor 2:** Geschwindigkeiten × 2, Dauern ÷ 2; echte Werte im Katalog,
  Umrechnung nur über `data/scale.toml`.
- **Ein Modell pro Einheit**, Tarnung Wald/Wüste/Winter über das Material-System.
- **Quellen:** Fakten und Maße frei, Zeichnungen und Fotos Dritter nur als Referenz,
  nichts aus anderen Spielen. Balkenkreuz statt Hakenkreuz, kein Rotes Kreuz.

## Stil-Referenz

Die Vorlagen in `docs/.vorlagen/` (nicht im Repo) zeigen die Ziel-Optik: hohe,
leicht perspektivische Draufsicht, realistische Texturen, dichte Vegetation, harte
Sonnenschatten, viel Rauch. Sie dienen nur als Stil-Referenz. Es werden keine Assets,
Namen oder Designs daraus übernommen; alle Assets sind selbst gebaut oder CC0.
