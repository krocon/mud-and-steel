# Prompts – Roadmap „Mud and Steel“

Jeder Prompt ist ein abgeschlossener Arbeitsauftrag für eine frische Coding-Agent-Session
im Repo-Ordner. Die Reihenfolge ist so gewählt, dass immer ein lauffähiges Spiel entsteht
und die Grundlagen (Determinismus, Lockstep, Karte) stehen, bevor viel Grafik darauf
aufbaut.

## Ablauf

1. Den nächsten offenen Prompt nehmen und vor dem Start kurz prüfen, ob er zum aktuellen
   Stand passt. Spätere Prompts sind bewusst knapper gehalten und werden vor der
   Ausführung konkretisiert.
2. In einer frischen Session ausführen: „Bitte führe `prompts/00X-….md` aus.“
3. Ist er fertig, verschiebt der Agent die Datei mit `git mv` nach `prompts/done/`.

## Übersicht

| Nr. | Prompt | Ziel | Status |
|-----|--------|------|--------|
| 001 | [Projekt-Setup](done/001-projekt-setup.md) | Gerüst, Sim, Lockstep, Bridge, CI | erledigt |
| 002 | [RTS-Kamera](002-rts-kamera.md) | Schwenken, Zoomen, Drehen über großem Spielfeld | offen |
| 003 | [Einheiten, Auswahl, Lockstep im Spiel](003-einheiten-auswahl-lockstep.md) | Mehrere Einheiten, Auswahlrahmen, Gruppenbefehle, Replays | offen |
| 004 | [Kartenformat und Höhenfeld](004-karte-hoehenfeld.md) | Karte als Daten, Höhen und Geländearten in der Sim | offen |
| 005 | [Terrain-Darstellung](005-terrain-darstellung.md) | Großes Gelände mit LOD und Textur-Splatting | offen |
| 006 | [Wegfindung](006-wegfindung.md) | Deterministisches A* mit Hindernissen und Gruppenbewegung | offen |
| 007 | [Vegetation und Objekte](007-vegetation-und-objekte.md) | Bäume, Büsche, Gras, Kleinkram, dicht und performant | offen |
| 008 | [Wasser, Flüsse und Brücken](008-wasser-fluesse-bruecken.md) | Flüsse, Ufer, Brücken als Engstellen | offen |
| 009 | [Licht, Atmosphäre und Biome](009-licht-atmosphaere-biome.md) | Sonne, Schatten, Nachbearbeitung, Wiese/Wüste/Winter | offen |
| 010 | [Asset-Pipeline für echte Modelle](010-asset-pipeline-modelle.md) | Modelle mit LOD, PBR-Material, Lizenzregeln | offen |
| 011 | [Performance für große Karten](011-performance-grosse-karten.md) | Benchmarks, räumliches Raster, Budgets | offen |
| 012 | [Kampf-Grundlagen](012-kampf-grundlagen.md) | Lebenspunkte, Waffen, Geschosse, Treffer | offen |
| 013 | [Nebel des Krieges](013-nebel-des-krieges.md) | Sicht pro Spieler mit Sichtlinie über Gelände | offen |
| 014 | [Effekte](014-effekte.md) | Explosionen, Rauch, Mündungsfeuer, Leuchtspur | offen |
| 015 | [Zerstörbare Gebäude und Vegetation](015-zerstoerung.md) | Schadensstufen, Trümmer, umfallende Bäume | offen |
| 016 | [Verformbares Gelände](016-gelaendeverformung.md) | Krater verändern das Höhenfeld | offen |
| 017 | [Infanterie](017-infanterie.md) | Trupps, Formationen, Animationen | offen |
| 018 | [Basisbau und Ressourcen](018-basisbau-ressourcen.md) | Bauen, Abbauen, Produzieren | offen |
| 019 | [KI-Gegner](019-ki-gegner.md) | Computerspieler über normale Befehle | offen |
| 020 | [LAN-Mehrspieler](020-lan-mehrspieler.md) | ENet-Transport, Spielsuche per Broadcast | offen |
| 021 | [Online über Steam](021-online-steam.md) | GodotSteam, Lobbys, Matchmaking | offen |
| 022 | [Menüs und Spielablauf](022-menues-spielablauf.md) | Hauptmenü, Lobby, Einstellungen, Sieg/Niederlage | offen |
| 023 | [Karteneditor](023-karteneditor.md) | Karten im Spiel bauen und speichern | offen |

## Warum diese Reihenfolge

- **002 zuerst:** Die Kamera ist reine Darstellung, klein und sofort spürbar. Sie legt
  fest, wie groß das Spielfeld wirkt und wie Zoom-Stufen aussehen; das brauchen alle
  späteren Grafik-Prompts.
- **003 vor dem Gelände:** Mehrere Einheiten und Lockstep im echten Spiel sind die
  Grundlage für alles Weitere. Replays machen Fehler reproduzierbar.
- **004 vor 005:** Das Höhenfeld gehört der Simulation (Sichtlinie, Krater, Steigungen).
  Die Darstellung baut darauf auf, nicht umgekehrt.
- **006–011:** Spielfeld fertig machen, bis es wie die Vorlagen aussieht und groß
  bleibt, ohne langsam zu werden.
- **012–019:** Gameplay. Erst danach lohnt sich echter Netzwerk-Transport (020–022).

## Stil-Referenz

Die Vorlagen in `docs/.vorlagen/` (nicht im Repo) zeigen die Ziel-Optik: hohe,
leicht perspektivische Draufsicht, realistische Texturen, dichte Vegetation, harte
Sonnenschatten, viel Rauch. Sie dienen nur als Stil-Referenz. Es werden keine Assets,
Namen oder Designs daraus übernommen; alle Assets sind selbst gebaut oder CC0.
