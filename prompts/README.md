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
| 034 | [Transport und Schleppen](034-transport-schleppen.md) | Aufsitzen, Geschütze ziehen und abprotzen | Entwurf |
| 035 | [Pioniere, Minen, Stellungen](035-pioniere-minen-stellungen.md) | Minen, Draht, Gräben, Brücken reparieren/sprengen | Entwurf |
| 036 | [Nachschub und Reparatur](036-nachschub-reparatur.md) | Verbrauch pro Einheit, Versorgungs-LKW, Reparatur, Bergung | Entwurf |
| 037 | [Teams und KI-Plätze](done/037-teams-ki-slots.md) | 2v2/3v3, KI in der Lobby, Moduswahl im LAN; `docs/TEAMS.md` | erledigt (vorläufige Entscheidungen in `docs/TEAMS.md` zu bestätigen) |
| 038 | [Speichern und Replays](done/038-speichern-replays.md) | Spielstände, Replay-Ansicht mit Vorspulen | erledigt |
| 039 | [Tutorial und Missionen](039-tutorial-missionen.md) | Skript-System, Tutorial, Beispielmissionen | Entwurf |
| 040 | [Wetter, Schlamm, Tageszeit](040-wetter-schlamm-tageszeit.md) | Regen weicht Boden auf, Nebel, Nacht | Entwurf |
| 041 | [Umwelt Stufe 2 und Feuer](done/041-umwelt-stufe2-feuer.md) | Stadt, Industrie, Bahn, Bunker; Brände und Lager-Explosionen | erledigt (Rest offen, siehe Ergebnis) |
| 042 | [Infanterie-Texturen](done/042-infanterie-texturen.md) | Prozedurale Uniform-Texturen, Tarnmuster, Pistolen, zehn neue Clips | erledigt |
| 043 | [Sprache und Barrierefreiheit](043-sprache-barrierefreiheit.md) | Deutsch/Englisch, Farbpaletten, UI-Skalierung, Grafikoptionen | Entwurf |
| 044 | [Release-Builds und Steam](done/044-release-builds-steam.md) | Export für drei Systeme, Release-CI, SteamPipe, Credits | erledigt (erster Export mit Godot-Export-Templates steht aus, `docs/RELEASE.md`) |
| 045 | [Balancing und Playtest](045-balancing-playtest.md) | Statistik, Turniere, Gefechtssimulator, bestätigtes Design | Entwurf |

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
| 046 | [Befehle und Formationen](046-befehle-formationen.md) | Angriffsbewegung, Gebiet sichern, Auto-Formation, Ausrichtung per Ziehen | Entwurf |
| 047 | [Infanterie-Taktik](047-infanterie-taktik.md) | Feuertrupps, Deckung zu Deckung, Niederhalten, Blindfeuer | Entwurf |
| 048 | [Animationen und Ragdolls](048-animationen-ragdolls.md) | Deckungs-, Kriech- und Rollanimationen, Ragdolls nur als Darstellung | Entwurf |
| 049 | [Verwundung und Kameradenhilfe](049-verwundung-kameradenhilfe.md) | Verwundete, Bergen, Sanitäter mit Trage, ausgestiegene Besatzungen | Entwurf |
| 050 | [Erfahrung und Ränge](050-erfahrung-raenge.md) | Veteranenstufen mit moderaten Boni und Abzeichen | Entwurf |
| 051 | [KI-Kommandeur Stufe 2](051-ki-kommandeur.md) | Aufklären, Lagebild, Konter, Versorgungskrieg, Aufgeben | Entwurf |
| 052 | [Stellungen Stufe 2](052-stellungen-stufe2.md) | Gräben, Sperrlinien, eingegrabene Panzer, Tarnnetze, Bunker | Entwurf |
| 053 | [Sound Stufe 2](053-sound-stufe2.md) | Überschallknall, Schallverzögerung, Zoom-Akustik, Stimmen | Entwurf |
| 054 | [Explosionen Stufe 2](054-explosionen-stufe2.md) | Druckwelle, Blitzlicht, Munitionsexplosion, schwerer Feuerschlag | Entwurf |
| 055 | [Brücken, Pioniertechnik, Luftlande](055-bruecken-pioniertechnik-luftlande.md) | Brückeneinsturz, Ertrinken, Brückenleger, Fallschirmjäger | Entwurf |
| 056 | [Karten und Basis Stufe 2](056-karten-basis-stufe2.md) | Winterdorf, Höfe, Flussland, Punkte an Orten, Hinterhalt | Entwurf |
| 057 | [Bodenoptik: Wiesen](057-bodenoptik-wiese.md) | Feines Gras, Farbvariation, Mähstreifen, Übergänge | Entwurf |
| 058 | [Bilder in der Produktionsauswahl](058-bilder-produktionsauswahl.md) | Vorgerenderte Bilder als Kacheln mit Kosten, Reiter | Entwurf |
| 059 | [Wüstendorf](059-wuestendorf.md) | Flachdachhäuser mit Hofmauern, Palmenhaine, Dorfkarte Nordafrika | Entwurf |
| 060 | [Berge und Täler](060-berge-taeler.md) | Gebirgs-Profile im Generator, Pässe, Hangregeln, Gebirgskarte | Entwurf |
| 061 | [Recherche: freie Soundeffekte](done/061-sound-recherche.md) | Geprüfte CC0/CC-BY-Aufnahmen, Pakete, Lücken, Ordnerstruktur in `docs/AUDIO_LIBRARY.md` | erledigt |

Viele dieser Prompts ändern `sim.rs`, `command.rs`, `main.gd` und den Golden-Hash.
Parallel laufende Sessions übernehmen vor dem Abschluss den aktuellen `main`.

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
