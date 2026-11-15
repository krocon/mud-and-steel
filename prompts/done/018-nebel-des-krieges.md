# Prompt 018 – Nebel des Krieges

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Abschnitte Pfadfindung, Kampf), `docs/DETERMINISM.md`,
> `docs/MAPS.md` und `docs/SCALE.md`.

## 1. Ziel

Jeder Spieler sieht nur, was seine Einheiten sehen. Hügel, Wald, Hecken, Mauern und
Gebäude verdecken die Sicht. Die Sicht ist Spielregel (Zielauswahl im Kampf hängt
davon ab), liegt deshalb deterministisch in `crates/sim`; Godot zeigt nur an: fremde
Einheiten außerhalb der Sicht verschwinden, das Gelände liegt je nach Zustand im
Nebel.

Stand vorher: Jeder sieht alles. Einheiten wählen automatisch jedes Ziel in
Waffenreichweite, auch hinter einem Hügel (das Geschoss trifft dann den Hügel). Der
Katalog hat keine Sichtweiten; `data/scale.toml` hat schon den Faktor
`range.sight = 1/4` (`Scale::sight_range_m`). Kartenobjekte haben Katalog-Höhen
(`ObjectKind::height`).

## 2. Umfang

1. **Sichtregeln** in einer neuen Datei `data/vision.toml` (Ganzzahlen, eingebaut wie
   `combat.toml`, neues Feld `SimConfig::vision`, Teil des Zustands-Hashes):
   Zellgröße, Zielhöhe, Rechenbudget pro Tick, Sichtweite in **echten Metern** pro
   Kategorie (umgerechnet nur über `range.sight`), Laub-Eindringtiefe und welche
   Objekt-Ids die Sicht blockieren (`solid`) oder dämpfen (`foliage`).
2. **Sichtraster** (`crates/sim/src/vision.rs`): Zellen von 8 m über die ganze Karte.
   Pro Zelle vorab: Bodenhöhe in der Zellmitte (mm), Oberkante fester Hindernisse
   (Häuser, Scheunen, Kirche, Mauern, Hecken) und Laub (Zellen mit mindestens zwei
   Bäumen).
3. **Sichtfeld pro Einheit** (Viewshed): Strahlen vom Ankerpunkt (Zelle der Einheit,
   Augenhöhe = Bodenhöhe + Einheitenhöhe aus dem Katalog bzw. Infanteriehöhe) zu
   allen Randzellen des Quadrats mit Sichtweiten-Radius. Entlang jedes Strahls wird
   die größte Steigung (Horizont) geführt, rein ganzzahlig (Kreuzmultiplikation statt
   Division): Eine Zelle ist sichtbar, wenn ihr Punkt auf Bodenhöhe + Zielhöhe nicht
   unter dem Horizont liegt. Boden und feste Hindernisse heben den Horizont sofort,
   Laub erst nach der Eindringtiefe (2 Zellen = 16 m in den Wald hinein sieht man).
   Ergebnis: Bitfeld über das Quadrat, nur Zellen im Kreis.
4. **Spielerraster:** pro Spieler Referenzzähler je Zelle (wie viele eigene
   Sichtfelder sie enthalten) und „erkundet“-Bits. Zustände für die Darstellung:
   nie gesehen, erkundet, sichtbar.
5. **Inkrementell und mit Budget:** Ein Sichtfeld wird neu gerechnet, wenn die
   Einheit die Zelle wechselt (oder zerstört wird: Sichtfeld weg). Neu zu rechnende
   Einheiten kommen in eine FIFO-Warteschlange; pro Tick werden höchstens
   `budget` Strahlschritte abgearbeitet (mindestens ein Sichtfeld), so dass der Tick
   unabhängig von der Einheitenzahl begrenzt bleibt. Beim Start werden alle
   Sichtfelder ohne Budget gerechnet.
6. **Sichtbarkeit von Einheiten:** Eine fremde Einheit ist für Spieler P sichtbar,
   wenn ihre Zelle in P's Raster sichtbar ist. Pro Tick nach der Bewegung als
   Bitmaske je Einheit bestimmt (`Sim::visible_to`). Wracks sind Gelände und für
   alle sichtbar.
7. **Kampf:** Automatische Ziele nur, wenn die Zielzelle im **eigenen** Sichtfeld
   des Schützen liegt (keine Schüsse in Hügel auf Ziele, die nur ein Kamerad sieht).
   Ein Angriffsbefehl auf eine Einheit feuert nur bei eigener Sicht; sieht der
   Spieler das Ziel nicht mehr, fährt die Einheit zur zuletzt gesehenen Position und
   gibt den Befehl dort auf, wenn das Ziel weiter unsichtbar ist. Bodenangriff
   unverändert.
8. **Snapshot pro Spieler:** `Sim::snapshot_for(player)` enthält nur eigene, sichtbare
   fremde und zerstörte Einheiten. Geschosse und Kampf-Ereignisse bleiben
   ungefiltert (Mündungsfeuer und Leuchtspur sieht man von weitem; Prompt 019 baut
   darauf). `Sim::vision_states(player)` liefert das Raster für den Nebel.
9. **Bridge:** Der lokale Spieler bekommt `snapshot_for`; neues Modul
   `vision_funcs.rs` mit Rastergröße, Ursprung und Bild-Bytes (nur bei Änderung neu)
   und einem Debug-Schalter „alles aufdecken“ (nur Darstellung).
10. **Godot:** `world/fog_of_war.gd` mit Screen-Space-Shader: ein Quad vor der
    Kamera liest die Tiefe, rekonstruiert die Weltposition und legt den Nebel über
    Terrain, Vegetation, Objekte und Wasser (Textur linear gefiltert, weiche Ränder).
    Nie gesehen: fast schwarz, erkundet: abgedunkelt. Einheiten, die nicht mehr im
    Snapshot sind, blendet `main.gd` aus. F3-Debugmenü: Nebel an/aus.
11. **Benchmark** in der CLI (`just vision-bench`): 500 Einheiten auf `test_large`
    fahren gleichzeitig, Rechenzeit pro Tick mit und ohne Sicht; Werte im Bericht und
    in `docs/ARCHITECTURE.md`.

## 3. Akzeptanzkriterien

1. `just check` grün. Golden-Hash bewusst aktualisiert, Grund im Commit;
   Replay-`FORMAT_VERSION` erhöht (neues Feld in `SimConfig`).
2. Sim-Tests: Einheit hinter einem Hügel unsichtbar, auf dem Hügel sichtbar; Wald
   verdeckt ab der Eindringtiefe, Waldrand sichtbar; Gebäude verdeckt; Zustände nie
   gesehen → sichtbar → erkundet; Ziel hinter Hügel wird nicht automatisch
   beschossen, auf dem Hügel schon; Angriff auf unsichtbares Ziel fährt zur letzten
   Position; Budget verteilt Neuberechnungen über Ticks; gleiche Seeds ergeben
   gleiche Sicht.
3. Messung mit 500 Einheiten (`just vision-bench`), Werte im Bericht.
4. `just selftest` (prüft Nebel-Overlay und Ausblenden), Flow-Selbsttest,
   `just lan-selftest`, `just gallery-check` grün; `-- --demo` läuft.
5. Doku: `docs/ARCHITECTURE.md` (Abschnitt Sicht), `docs/DETERMINISM.md`
   (Sicht, Golden-Hash-Historie), `docs/SCALE.md` (Sichtweiten).

## 4. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Rasterauflösung | 8 m | Bei 500 m Sichtweite 62 Zellen Radius, ein Sichtfeld ≈ 31 000 Strahlschritte. 4 m wäre 4× teurer bei 500 Einheiten; 8 m entspricht etwa einer Panzerlänge und reicht für Hügelkämme und Waldränder. Die Pfadzellen (4 m) bleiben getrennt. |
| Aktualisierung | Beim Zellwechsel, FIFO mit Strahlschritt-Budget pro Tick | Stehende Einheiten kosten nichts; Last ist pro Tick gedeckelt wie bei der Wegfindung; deterministisch, weil das Budget eine Regelkonstante ist. Bei Überlast hinkt die Sicht etwas nach statt den Tick zu sprengen. |
| Sichtlinie | Strahlen zum Rand des Quadrats, Horizont-Steigung, Ganzzahl-Kreuzmultiplikation | Standardverfahren für Viewsheds; deckt jede Zelle ab, ohne Division und ohne Floats. |
| Zielhöhe | Fester Wert (2 m) statt Höhe des Ziels | Ein Raster pro Spieler statt Sichtlinie pro Paar; 2 m liegt zwischen stehender Infanterie und Panzern. |
| Wald | Laub-Zellen (≥ 2 Bäume) heben den Horizont erst nach 2 Zellen | Man sieht 16 m in den Wald, Einheiten tiefer drin sind versteckt; von einem Hügel sieht man über die Baumkronen. Einzelne Bäume verdecken nichts. |
| Hecken, Mauern, Gebäude | Feste Hindernisse mit Katalog-Höhe | Hecken (Bocage) sollen blockieren, sind aber nur eine Zelle dick. |
| Kampf | Automatisches Ziel und Feuer nur mit eigener Sichtlinie; Angriffsbefehl folgt der letzten bekannten Position | Keine Schüsse in Hügel; kein Wissen über unsichtbare Einheiten im Verhalten. |
| Schützen aufdecken | Nicht als Regel, aber Leuchtspur/Mündungsfeuer sind sichtbar | Hält die Sim einfach; die Darstellung zeigt trotzdem, woher geschossen wird. |
| Map-Hack | Nicht verhinderbar bei Lockstep (jeder Peer hat den ganzen Zustand) | Gefiltert wird nur die Darstellung; im Bericht und in `docs/ARCHITECTURE.md` eingeordnet. |
| Darstellung | Screen-Space-Overlay aus Tiefenpuffer | Ein Shader für Terrain, MultiMesh-Vegetation, Objekte und Wasser, ohne die bestehenden Materialien anzufassen (Prompt 019 und 028 laufen parallel). |
| Wracks | Immer sichtbar, blockieren keine Sicht | Wracks sind Gelände; Sicht-Blocker durch Wracks folgen mit Prompt 020. |

## 5. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, keine Tests abschwächen, Commits pro Schritt, `just check` und
`just selftest` grün. Prompt 019 und 028 laufen parallel: gemeinsame Dateien
(`crates/bridge/src/lib.rs`, `godot/scenes/main.gd`, `main.tscn`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern; Bridge-Funktionen in ein eigenes
Modul. Am Ende nach `prompts/done/` verschieben und `prompts/README.md`
aktualisieren.
