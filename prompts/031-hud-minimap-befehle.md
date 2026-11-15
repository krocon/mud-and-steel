# Prompt 031 – HUD: Minikarte, Einheitenanzeige, Befehlsleiste, Wegpunkte

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Auswahl, Kamera, Nebel, Basisbau-HUD, Menüs).
>
> Konkretisiert am 2026-11-15 (Abschnitt 4), danach umgesetzt.

## 1. Ziel

Auf großen Karten fehlt die Übersicht: Es gibt keine Minikarte, keine Anzeige der
gewählten Einheiten und keine Befehlsleiste; alle Befehle laufen über Tasten und
Rechtsklick. Nach diesem Prompt sieht der Spieler jederzeit, wo was los ist, was er
ausgewählt hat und was er damit tun kann.

## 2. Umfang

1. **Minikarte** (Ecke unten): Gelände (Höhe, Wasser, Wald, Straßen) aus den
   Kartendaten vorab gerendert, eigene und sichtbare fremde Einheiten als Punkte in
   Spielerfarbe, Nebel des Krieges, Kamerasichtfeld als Trapez. Klick springt,
   Ziehen schwenkt, Rechtsklick gibt einen Bewegungsbefehl. Kampf-Pings bei Beschuss
   eigener Einheiten. Performance: Punkte per MultiMesh/Shader, nicht pro Node.
2. **Einheitenanzeige:** eine Einheit: Name, Bild (Galerie-Thumbnail), Lebenspunkte,
   Panzerung, Waffen, Haltung, Feuermodus. Mehrere: Raster mit Symbolen und Lebensbalken,
   Klick wählt eine, Shift-Klick entfernt sie. Lebensbalken über beschädigten
   Einheiten im Spiel (Option „immer / bei Auswahl / nie“).
3. **Befehlsleiste:** Schaltflächen für Bewegen, Angreifen, Bodenziel, Stopp,
   Feuermodus, Haltungen, Besetzen, Bauen/Produzieren (das Basisbau-HUD aus 023 geht
   darin auf); zeigt die Tastenbelegung aus den Einstellungen.
4. **Befehlsketten:** Shift+Rechtsklick hängt Wegpunkte an, sichtbar als Linie mit
   Markern. Dafür braucht die Sim eine Befehlswarteschlange pro Einheit (neuer
   `Command` oder Flag `queue`, hinten an das Enum, damit alte Replays lesbar bleiben;
   Golden-Hash prüfen). Optional: Patrouille, Formationswahl (Linie, Keil, Kolonne),
   Ausrichtung am Ziel per Rechtsklick-Ziehen.
5. **Ereignisanzeige:** knappe Meldungen (Einheit verloren, Bau fertig, Gegner
   gesichtet) mit Klick zum Hinspringen; Leertaste springt zum letzten Ereignis.
6. **Selbsttest:** Minikarte klicken/ziehen, Auswahl über die Anzeige ändern,
   Wegpunktkette fahren, Befehl über die Leiste geben.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Befehlsketten (deterministisch, im Replay).
2. `just selftest`, `just lan-selftest` grün; Befehlsketten auch im LAN-Test.
3. Schlacht-Benchmark: HUD kostet höchstens 1 ms pro Frame.
4. Doku: Steuerung in `docs/ARCHITECTURE.md` und eine Spieler-Steuerungsübersicht
   (`docs/CONTROLS.md`), die auch das Tutorial (039) nutzt.

## 4. Konkretisierung (Ausgangslage und Entscheidungen)

**Ausgangslage im Code:** Auswahl und Befehle in `scenes/selection/selection.gd`
(feste Tasten H, F, C, Z/X/V/B, Gruppen 1–9); Lebensbalken über beschädigten und
gewählten Einheiten in `world/combat_view.gd` (`HealthBars`); Basisbau-HUD
(`world/base_hud.gd`: Ressourcenleiste oben, Baumenü unten links,
Produktionsleiste unten Mitte); Einstellungen (`menu/settings.gd`) können nur die
sieben Kamera-Aktionen umbelegen; Sichtzustände pro 8-m-Zelle kommen als L8-Bild
(`SimHost.vision_image`); Kampfereignisse verteilt `combat_view.combat_event`.
Die Sim kennt pro Einheit genau ein Ziel (`target` + `route`), keine
Warteschlange. Galerie-Thumbnails gibt es nicht als Dateien.

| Frage | Entscheidung | Grund |
|---|---|---|
| Befehlsketten in der Sim | Zwei neue `Command`-Varianten hinten am Enum: `QueueMove { units, target }` (Wegpunkt anhängen) und `Patrol { units, target }`. Pro Einheit eine Warteschlange von Formationsplätzen (`sim/orders.rs`). | Ein eigener Befehl statt eines `queue`-Flags an `MoveTo`: alte Replays und der `MoveTo`-Aufbau bleiben unverändert, postcard liest alte Dateien weiter. |
| Formation in der Kette | Plätze werden beim Befehl wie bei `MoveTo` vergeben (Raster um den Wegpunkt, Zuordnung nach Position); jede Einheit fährt ihren Platz mit eigenem Pfad an. Sind alle Einheiten untätig und ohne Kette, wird `QueueMove` zu einem normalen `MoveTo` (ein Gruppenpfad). | Einheiten erreichen Wegpunkte nicht im selben Tick; ein gemeinsamer Gruppenpfad pro Etappe bräuchte Synchronisation. Kurze Etappen kosten wenig Suchbudget. |
| Wann geht es weiter | Eine lebende Einheit ohne Ziel, Route und Angriffsbefehl nimmt den nächsten Wegpunkt, direkt nach den Befehlen im selben Tick. Jeder andere Bewegungs-/Angriffs-/Stopp-/Besetzen-Befehl löscht die Kette. | Einfache, reihenfolgeunabhängige Regel; ein beendeter Angriff setzt die Kette fort. |
| Patrouille | `Patrol`: wie `MoveTo` zum Ziel, danach pendelt die Einheit zwischen Startpunkt und Platz am Ziel (zyklische Kette). Feuert nach Feuermodus. | Billig auf derselben Warteschlange; optionaler Punkt aus 2.4. |
| Golden-Hash | Ketten gehen nur in den Hash, wenn eine Einheit eine hat (am Ende, nach Einheiten-ID). Ohne Ketten bleibt der Hash gleich. | Der Golden-Hash (Demo ohne Ketten) bleibt stabil, parallele Prompts bekommen keinen Konflikt; Ketten sind trotzdem vollständig im Hash. |
| Replay-Version | Bleibt 4. | Neue Enum-Varianten hängen hinten an; alte Dateien bleiben lesbar. Alte Builds melden bei neuen Dateien einen Dekodierfehler. |
| Formationswahl, Ausrichtung per Ziehen | Nicht umgesetzt (offen). | Optional; ändert die Formationsregeln (Golden-Hash) und kollidiert mit 032. |
| Minikarte: Gelände | Rust (`hud_funcs.rs`) rendert ein Bild mit einem Pixel pro Sichtzelle (8 m): Geländefarbe, Schummerung aus der Höhe, Wasser, Straßen (jede Stichprobe der Zelle), Wald und Gebäude aus dem Sichtraster. Einmal pro Partie, nach Kratern nicht neu. | Gleiches Raster wie der Nebel: ein Shader multipliziert beide Texturen; 4 km = 500 × 500 Pixel. |
| Minikarte: Einheiten | Ein `MultiMeshInstance2D`, dessen Puffer die Bridge in einem Aufruf füllt (`minimap_dots`), Farbe nach Spieler; nur Einheiten des (nebelgefilterten) Snapshots. | Ein Draw-Call, keine Schleife in GDScript (Vorgabe „MultiMesh/Shader, nicht pro Node“). |
| Minikarte: Bedienung | Unten rechts, 220 px. Links klicken/ziehen setzt die Kamera, Rechtsklick Bewegung (Shift: Wegpunkt), Kamerasichtfeld als Trapez (Bildschirmecken auf den Boden), Kampf-Pings bei Treffern auf eigene Einheiten (höchstens einer pro 2 s und 60 m). | Übliche RTS-Bedienung. |
| Einheitenanzeige | Unten Mitte. Eine Einheit: Name, Porträt, Lebenspunkte, Panzerung (Wanne/Turm vorn/Seite/hinten), Waffen (Kaliber, Reichweite), Haltung, Feuermodus. Mehrere: Raster mit Kurznamen und Lebensbalken, Klick wählt eine, Shift-Klick entfernt sie. | Wie im Prompt. |
| Porträt [vorläufig] | Zur Laufzeit gerendert: ein `SubViewport` pro Einheitentyp (einmal, zwischengespeichert) mit dem Modell. Keine Bilddateien. | Es gibt keine Galerie-Thumbnails als Dateien; so passt das Bild immer zum Modell. Ein Thumbnail-Export kann es später ersetzen. |
| Lebensbalken-Option | Einstellung „Health bars“: `damaged` (beschädigte und gewählte, bisheriges Verhalten, Standard), `selected` (nur gewählte), `never`. | „immer“ = immer, wenn beschädigt. |
| Befehlsleiste | Unten links, Schaltflächen im Raster mit Taste: Bewegen (M), Angriff (T), Bodenziel (G), Patrouille (P), Stopp (H), Feuermodus (F), Stehen/Knien/Liegen/Auto (Z/X/V/B), Besetzen (U), Bauen. Gezielte Befehle (Bewegen, Angriff, Bodenziel, Patrouille, Besetzen) warten auf den nächsten Linksklick; Rechtsklick bricht ab. | Rechtsklick bleibt der schnelle Weg; die Leiste macht alles sichtbar. |
| Tastenbelegung | Die Befehlstasten werden Aktionen in `project.godot` und in den Einstellungen umbelegbar (Steuerungs-Tab scrollt). Gruppen 1–9 bleiben fest. | „zeigt die Tastenbelegung aus den Einstellungen“. |
| Basisbau-HUD [vorläufig] | Baumenü und Produktionsleiste hängen in der Befehlsleiste: „Bauen“ klappt das Baumenü auf, ein gewähltes Produktionsgebäude zeigt seine Produktion darüber. Ressourcenleiste, Vorschau und Sammelpunkt bleiben in `base_hud.gd`. | „geht darin auf“ ohne die geprüfte Logik aus 023 neu zu schreiben. |
| Ereignisanzeige | Links über der Befehlsleiste, höchstens 6 Meldungen, je 12 s: Einheit verloren, Gebäude fertig, Einheit produziert, Gegner gesichtet (höchstens alle 15 s), unter Beschuss (höchstens alle 10 s). Klick springt hin, Leertaste (`event_jump`) zur letzten. | Alles aus Snapshot und Ereignisstrom, keine Sim-Änderung. |
| Wegpunktlinien | Für gewählte Einheiten: Linie Einheit → aktuelles Ziel → Kette, Marker an den Wegpunkten, auf dem HUD gezeichnet (projizierte Punkte). | Gerade Linien bleiben unter Projektion gerade; billig. |
| HUD-Messung | Jedes HUD-Teil misst seine `_process`-Zeit; Selbsttest und `--perf-bench` geben die Summe aus. | Akzeptanzkriterium 3. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
