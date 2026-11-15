# Prompt 016 – Performance für große Karten

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Abschnitte Wegfindung, Kampf, Sicht, Zerstörung und die
> Messungen der Darstellung), `docs/DETERMINISM.md`, `docs/INFANTRY.md`
> („Performance“) und `docs/AI.md`.

## 1. Ziel

Eine Schlacht auf einer 4-km-Karte mit einigen hundert Einheiten und Tausenden
Soldaten läuft flüssig: die Simulation bleibt weit unter ihrem 50-ms-Tick, die
Darstellung hält 60 FPS in mittlerer Grafikstufe. Dafür gibt es **ein Gesamtszenario,
eine Gesamtmessung und feste Budgets**, die in `docs/PERFORMANCE.md` stehen und
dauerhaft (CI) im Blick bleiben. Optimierungen ändern das Verhalten nicht: der
Golden-Hash und der End-Hash des Benchmarks bleiben gleich.

## 2. Ausgangslage (gemessen am 2026-11-15, Apple M1 Max)

Bisher gibt es nur Einzelmessungen (`just path-bench`, `vision-bench`,
`infantry-bench`, `terrain-bench`, `objects-bench`, `effects-bench`,
`infantry-view-bench`), jeweils für ein Teilsystem.

**Zielszenario** `data/scenarios/bench_large.toml` (neu): `test_large`, zwei Armeen
aus je 125 Panzern (Panzer IV H / T-34) und 125 Schützentrupps (2250 Soldaten),
etwa 1,3 km auseinander, beide Seiten von der KI „Hard“ geführt, 6000 Ticks
(5 min Spielzeit): Anmarsch, Wegsuche, Sicht, Kampf, Wracks, überrollte Objekte.

**Simulation** (`cli bench`, Release, Minimum pro Tick aus 3 Läufen, ms):

| | Mittel | Median | p99 | Max |
|---|---|---|---|---|
| Sim-Tick | 5,21 | 5,20 | 31,7 | 65,4 |
| davon Wegsuche (Budget 32 768 Knoten) | 2,23 | 3,61 | 4,65 | 6,5 |
| davon Sicht (Budget 400 000 Strahlschritte) | 1,41 | 1,07 | 2,86 | 3,0 |
| davon Projektile/Überrollen | 0,63 | 0,01 | 24,6 | 58,5 |
| davon Zerstörung | 0,09 | 0 | 0 | 27,1 |
| KI (beide) | 0,09 | 0,08 | 0,30 | 0,47 |
| Snapshot | 0,04 | 0,04 | 0,05 | 0,05 |

Die Spitzen von 25–65 ms kommen daher, dass jedes Wrack und jedes überrollte Objekt
die Zusammenhangskomponenten des ganzen Wegerasters neu berechnet (rund 8 ms pro
Komponentenraster). Bewegung mit Trennung (O(n²)) kostet 0,46 ms, Zielwahl 0,2 ms.

**Darstellung** (`just bench-view`, neu, 1920 × 1080, Qualität „high“, Debug-Bridge):
siehe Tabelle „Vorher“ in `docs/PERFORMANCE.md`. Bekannt aus den Einzelmessungen:
ein Panzermodell besteht aus rund 40 Mesh-Teilen (je eigener Draw Call plus
Schattenkaskaden), 16 Panzer in der Übersicht kosteten schon 849 Draw Calls; 504
Soldaten liegen bei 57–90 FPS (Prompt 022), das Zielszenario hat 2250.

## 3. Umfang

1. **Gesamtmessung `cli bench`:** spielt ein Szenario mit zwei KIs über eine
   Lockstep-Session (wie `ai-tournament`) und misst pro Tick Sim-Schritt, jede
   Phase des Ticks (`Sim::step_observed` ruft nach jeder Phase einen Beobachter auf,
   die Sim selbst liest keine Uhr), KI und Snapshot; Fenster über die Zeit, die
   langsamsten Ticks mit Phasen, End-Hash. `--runs N` nimmt das Minimum pro Tick
   über N gleiche Läufe (die Maschine ist oft durch andere Prozesse belastet) und
   bricht ab, wenn die Läufe verschiedene Hashes haben. `just bench`.
2. **Bildratenmessung `just bench-view`** (`godot/scenes/tools/perf_bench.gd`): das
   Zielszenario mit KI „Hard“ als Gegner; drei Kameradistanzen (40, 150, 600 m) über
   der stehenden Armee, dann Vormarsch und dieselben Distanzen in der Schlacht mit
   Effekten. Ausgabe: FPS, Frame-Zeit (Mittel, p99, schlechtester), Anteil der Sim
   pro Frame (`SimHost.process_usec`), GDScript-Zeit, Render-CPU, Draw Calls.
3. **Simulation:**
   - Komponenten des Wegerasters nach Änderungen lokal neu berechnen (Fenster um die
     geänderten Zellen; echte Abtrennungen durch verschränkte Flutfüllung), mit
     Test gegen die volle Berechnung.
   - **Räumliches Raster** (Uniform Grid) für Nachbarschafts- und Bereichsabfragen
     (Trennung, Zielwahl, Projektiltreffer, Schusslinie), wo es messbar hilft;
     deterministische Iteration (Kandidaten in Index-Reihenfolge, wo die Reihenfolge
     das Ergebnis beeinflussen kann). Messen auch mit doppelter Einheitenzahl.
   - Budgetierte Arbeit (A\*, Sicht) pro Einheit schneller machen, ohne die
     Reihenfolge oder die Budgets (Regelkonstanten) zu ändern, wo es sich lohnt.
4. **Bridge:** Snapshots ohne unnötige Kopien: Snapshots nur für die zuletzt
   gezeichneten Ticks bauen, Ereignisse direkt aus der Sim sammeln, Arrays pro Frame
   nicht mehrfach erzeugen (z. B. `unit_type_ids()` pro neuer Einheit).
5. **Godot:**
   - Einheitenmodelle: statische Teile pro Einheitentyp zu wenigen Meshes
     zusammenfassen (Rumpf, Turm), Markierungen und Schatten mit Distanz ausblenden.
   - Infanterie: ferne Soldaten billig zeichnen (z. B. MultiMesh statt einzelner
     Skelette jenseits einer Distanz).
   - Sichtbarkeits-Culling und LOD prüfen, `main.gd`-Schleife pro Frame entlasten
     (nur bewegte Einheiten anfassen, Knoten-Referenzen cachen).
   - Profiler-Auswertung der Szenen (Draw Calls, Skripte) im Bericht.
6. **Dokumentation und CI:** `docs/PERFORMANCE.md` mit Budgets, Rezepten,
   Vorher-/Nachher-Tabellen und Entscheidungen; ein eigener, nicht blockierender
   CI-Job, der `cli bench` ausführt und die Tabelle in die Job-Zusammenfassung
   schreibt.

## 4. Budgets (Akzeptanzkriterien)

Gemessen auf dem Entwicklungsrechner (Apple M1 Max) mit dem Zielszenario:

| Größe | Budget | Begründung |
|---|---|---|
| Sim-Tick, Mittel | ≤ 5 ms | 10 % des 50-ms-Ticks; bei 60 FPS läuft ein Tick in jedem dritten Frame. |
| Sim-Tick, p99 | ≤ 10 ms | Ein Tick darf einen Frame nicht spürbar verlängern. |
| Sim-Tick, Max | ≤ 20 ms | Keine Ruckler durch einzelne Ereignisse (Wracks, Zerstörung). |
| KI pro Tick (beide), Mittel | ≤ 0,5 ms | Läuft im Spiel im selben Frame. |
| Snapshot pro Tick | ≤ 0,2 ms | |
| Darstellung, Qualität „medium“, alle Ansichten | ≥ 60 FPS | Zielwert aus dem Entwurf. |
| Darstellung, Qualität „high“, stehend 150 m | ≥ 60 FPS | |
| Darstellung, Qualität „high“, Schlacht | ≥ 45 FPS, Ziel 60 | „high“ hat MSAA und volles SSAO; 2250 animierte Soldaten. |

Weitere Kriterien:

1. Der Golden-Hash in `crates/sim/tests/determinism.rs` bleibt unverändert, der
   End-Hash von `cli bench` (Seed 1, 6000 Ticks) auch.
2. `just check`, `just selftest` (mit Flow-Test), `just lan-selftest`,
   `just editor-selftest`, `just gallery-check` und der Demo-Lauf sind grün.
3. `docs/PERFORMANCE.md` enthält Vorher-/Nachher-Tabellen für Sim und Darstellung
   und die Budgets; `docs/ARCHITECTURE.md` verweist darauf, betroffene Abschnitte
   sind aktualisiert.
4. CI hat einen nicht blockierenden Benchmark-Job mit Ausgabe der Werte.

## 5. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Zielszenario | 504 Einheiten (250 Panzer, 250 Trupps mit 2250 Soldaten) statt der 2000 Einheiten des Entwurfs | Ein Gefecht mit dieser Zahl ist für das Spiel schon groß; die KI führt jede Rolle als eine Gruppe. 2000 Einheiten (doppelte Zahl, mit `--scenario`) werden zusätzlich gemessen, um Skalierung (O(n²)) zu sehen, aber ohne Budget. |
| Messung auf belasteter Maschine | Minimum pro Tick über mehrere identische Läufe | Die Sim ist deterministisch, jeder Lauf rechnet dieselben Ticks; das Minimum filtert Verdrängung durch andere Prozesse heraus. |
| Phasenzeiten | Beobachter-Callback in `Sim::step_observed` | Die Sim darf keine Uhr lesen (DETERMINISM.md); der Callback kann den Zustand nicht ändern. |
| Budgets der Wegsuche und Sicht | bleiben (Regelkonstanten) | Eine Änderung änderte das Verhalten und den Golden-Hash. Schneller wird nur die Arbeit pro Knoten bzw. Strahlschritt. |
| Komponenten-Labels | dürfen anders nummeriert sein als bei voller Berechnung | Sie werden nur auf Gleichheit verglichen und nicht gehasht; entscheidend ist die gleiche Partition (Test). |
| CI-Benchmark | Ubuntu-Runner, `continue-on-error`, Ausgabe in die Job-Zusammenfassung | Geteilte CI-Maschinen sind zu verrauscht für harte Grenzen. |

## 6. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, keine Tests abschwächen, ein Commit pro Schritt. Erst messen, dann
optimieren, jede Optimierung mit `cli bench` (End-Hash gleich) und den Tests
belegen. Parallel laufende Prompts (021, 023, 026) ändern Sim und Darstellung:
vor dem Abschluss den aktuellen `main` übernehmen, erneut messen und neue Teile bei
Bedarf mit optimieren. Am Ende diese Datei nach `prompts/done/` verschieben und
`prompts/README.md` aktualisieren.
