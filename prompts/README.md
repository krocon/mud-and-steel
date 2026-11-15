# Prompts – Roadmap „Mud and Steel“

Jeder Prompt ist ein abgeschlossener Arbeitsauftrag für eine frische Coding-Agent-Session
im Repo-Ordner. Die Arbeit läuft in **zwei Strängen**, die weitgehend unabhängig sind
und parallel laufen können:

- **Asset-Strang (003–008):** Maßstab, Einheitenkatalog, Blender-Baukasten, Modelle.
  Er braucht nur den Katalog und die Konventionen, keinen fertigen Spielcode.
- **Spiel-Strang (002, 009–028):** Kamera, Einheitensteuerung, Karte, Gelände, Gameplay,
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
| 014 | [Wasser, Flüsse und Brücken](014-wasser-fluesse-bruecken.md) | Flüsse, Ufer, Brücken als Engstellen | Entwurf |
| 015 | [Licht, Atmosphäre und Biome](done/015-licht-atmosphaere-biome.md) | Sonne, Schatten, Nachbearbeitung, Wiese/Wüste/Winter | erledigt |
| 016 | [Performance für große Karten](016-performance-grosse-karten.md) | Benchmarks, räumliches Raster, Budgets | Entwurf |
| 017 | [Kampf-Grundlagen](done/017-kampf-grundlagen.md) | Lebenspunkte, Waffen, Geschosse, Treffer aus dem Katalog | erledigt |
| 018 | [Nebel des Krieges](018-nebel-des-krieges.md) | Sicht pro Spieler mit Sichtlinie über Gelände | Entwurf |
| 019 | [Effekte](019-effekte.md) | Explosionen, Rauch, Mündungsfeuer, Leuchtspur | Entwurf |
| 020 | [Zerstörbare Gebäude und Vegetation](020-zerstoerung.md) | Schadensstufen, Trümmer, umfallende Bäume | Entwurf |
| 021 | [Verformbares Gelände](021-gelaendeverformung.md) | Krater verändern das Höhenfeld | Entwurf |
| 022 | [Infanterie-Verhalten](022-infanterie-verhalten.md) | Trupps, Haltungen, Deckung, Gebäude besetzen | Entwurf |
| 023 | [Basisbau und Ressourcen](023-basisbau-ressourcen.md) | Bauen, Abbauen, Produzieren | Entwurf |
| 024 | [KI-Gegner](024-ki-gegner.md) | Computerspieler über normale Befehle | Entwurf |
| 025 | [LAN-Mehrspieler](done/025-lan-mehrspieler.md) | ENet-Transport, Spielsuche per Broadcast | erledigt |
| 026 | [Online über Steam](026-online-steam.md) | GodotSteam, Lobbys, Matchmaking | Entwurf |
| 027 | [Menüs und Spielablauf](027-menues-spielablauf.md) | Hauptmenü, Lobby, Einstellungen, Sieg/Niederlage | Entwurf |
| 028 | [Karteneditor](028-karteneditor.md) | Karten im Spiel bauen und speichern | Entwurf |

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
