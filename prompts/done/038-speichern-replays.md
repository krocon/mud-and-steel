# Prompt 038 – Speichern, Laden und Replay-Ansicht

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Replays, Spielablauf), `docs/DETERMINISM.md`.
> Läuft parallel zu 029, 031, 032, 042, 044: gemeinsame Dateien (`sim.rs`,
> `command.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`, `justfile`,
> Menü, Doku) klein und additiv ändern.

## 1. Ziel

Replays werden geschrieben, aber man kann sie im Spiel nicht ansehen; Partien lassen
sich nicht speichern. Nach diesem Prompt kann man Einzelspieler-Partien speichern und
laden und jedes Replay im Spiel abspielen, vorspulen und aus jeder Perspektive ansehen.

## 2. Ausgangslage (geprüft am 2026-11-15)

- `net::replay` (Format 5): Seed, aufgelöste `SimConfig`, zusammengeführte Befehle
  pro Tick (nur nicht leere), End-Hash. Keine Spielversion, kein Regel-Hash, keine
  Metadaten für eine Liste. `main.gd` schreibt beim Verlassen
  `user://replays/<datum-zeit>.msreplay`; `just replay` spielt headless nach.
- `Sim` ist `Clone`, aber nicht serialisierbar. Zustand in vielen Teilstrukturen
  (Pfadraster mit inkrementellen Änderungen, Sicht, Infanterie, Zerstörung, Krater,
  Wirtschaft). Die Karte (`Arc<Map>`, 20 MB bei 4 km) ändert sich im Spiel
  (Krater per Copy-on-write, zerstörte Brücken).
- Einzelspieler läuft über eine `LockstepSession` mit `LocalTransport`; die KI ist
  ein gehosteter Spieler mit eigenem Zustand (`ai::Ai`, Gedächtnis, Gruppen,
  Wirtschaftsplan). Befehle sind `input_delay` Ticks unterwegs.
- Darstellung: Gelände, Objektzustände (`changed_objects`) und Krater liest `main.gd`
  beim Start aus dem Sim-Zustand; danach kommen Änderungen als Ereignisse.
- Messung (`just save-bench`, Release, Apple M1 Max): große Schlacht
  (`bench_large`, 504 Einheiten, 4 km, zwei harte KIs, 6000 Ticks = 5 min):
  Nachspielen 32,4 s (5,4 ms/Tick; eine Stunde ≈ 6½ min), Schnappschuss schreiben
  162 ms, wiederherstellen 113 ms, 9,1 MB komprimiert. Basisbau auf `test_small`
  (12 000 Ticks = 10 min): Nachspielen 0,27 s, Schnappschuss 8 / 5 ms, 452 KB.
  Regel-Hash: 60 ms einmal pro Prozess.

## 3. Umfang

1. **Schnappschüsse** (`crates/sim/src/snapshot.rs`, `crates/net/src/snapshot.rs`):
   kompletter Sim-Zustand per serde (postcard + deflate), Karte als Differenz zur
   geladenen Karte, Hash-Prüfung beim Wiederherstellen.
2. **Spielstände** (`net::save`, `.mssave`): Replay bis zum Speichertick +
   Schnappschuss + ausstehende Lockstep-Frames + KI-Zustand. Laden stellt den
   Schnappschuss her (kein Nachspielen) und prüft den Hash; die Partie läuft exakt
   weiter wie ohne Speichern, das Replay am Ende umfasst die ganze Partie.
   Speichern im Pausenmenü (nur Einzelspieler), Laden im Hauptmenü („Load game“).
3. **Replay-Ansicht:** Hauptmenü „Replays“ mit Liste (Datum, Karte, Spieler,
   Dauer, Ergebnis; inkompatible ausgegraut mit Grund), Abspielen mit Pause,
   1×/2×/4×/8×, Zeitleiste zum Springen, Perspektive (ein Spieler mit dessen Nebel
   oder alles sichtbar).
4. **Versionierung:** Replay-Format 6 und Spielstand-Format 1 tragen Spielversion
   und Regel-Hash; abweichende werden mit klarer Meldung abgelehnt.
5. **Mehrspieler:** kein Speichern; das Replay wird wie bisher geschrieben (jetzt mit
   Kopf) und ist in der Replay-Ansicht abspielbar.

## 4. Akzeptanzkriterien

1. `just check` grün; Tests: Speichern/Laden ergibt denselben Hash wie Durchspielen
   (`crates/ai/tests/save.rs`, mit KI und Wirtschaft), Schnappschuss-Rundreise Tick
   für Tick (`crates/net/tests/snapshot.rs`, mit Kratern), alte Replays (Format 5)
   und fremde Regeln sauber abgelehnt (`net::replay` Tests).
2. `just selftest` mit Speichern, Laden und Replay-Abspielen grün.
3. Doku: `docs/ARCHITECTURE.md` und `docs/DETERMINISM.md`.

## 5. Entscheidungen

| Frage | Entscheidung | Begründung |
|---|---|---|
| Laden durch Nachspielen oder Schnappschuss | Schnappschuss; das Replay im Spielstand dient nur der Prüfung (`cli save-check`) und dem vollständigen Replay am Ende | Gemessen: eine Stunde große Schlacht bräuchte ≈ 6½ min Nachspielen, der Schnappschuss 0,1 s. Für die Sprünge im Replay braucht es Schnappschüsse ohnehin. |
| Was in den Schnappschuss | Alle Felder von `Sim` per serde-Ableitung, auch abgeleitete (Pfadraster, Sichtraster, Objektindex) | Neu aufbauen müsste jede inkrementelle Änderung (lokale Pfad-Neubeschriftung nach Kratern, Sicht-Budget) exakt nachbilden; Mitspeichern kann nicht danebengehen. Neuer Zustand (031, 032 …) ist automatisch dabei: der Compiler verlangt die Ableitung. |
| Karte | Nur als Differenz (`MapDelta`: geänderte Höhen-/Geländeproben, Brückenzustände) zur geladenen Karte, die per Name und Inhalts-Hash gefunden wird | Wie Replays: die Karte liegt ohnehin vor, ein Schnappschuss ohne Krater bleibt klein. |
| Kodierung | postcard + deflate (miniz_oxide, Stufe 1), eigener Kopf mit Magic, Version und State-Hash | postcard ist schon da, miniz_oxide steckt über png schon im Lockfile (kein Download); Stufe 1 hält das Speichern unter 0,2 s. |
| Exaktes Weiterlaufen | Ausstehende Frames der Session (`PendingFrames`) und der KI-Zustand (`ai::Ai` per serde) kommen in den Spielstand | Sonst gingen die schon gegebenen Befehle der Eingabeverzögerung verloren und die KI finge neu an; die geladene Partie wiche von der nie gespeicherten ab. |
| Regel-Hash | Hash der Schnappschuss-Bytes nach einem festen Demo-Lauf (400 Ticks, ≈ 60 ms, einmal pro Prozess) **[vorläufig]** | Jede Regeländerung ändert den Zustand, jedes neue Zustandsfeld die Bytes; keine Konstante, die man vergessen kann. Nebenwirkung: auch Katalogänderungen (Demo-Einheiten) ändern ihn und machen alte Replays unabspielbar, obwohl Replays die aufgelöste Config tragen. Alternative: Referenzlauf mit fest im Code gebauter Config. |
| Spielversion | Workspace-Version (`CARGO_PKG_VERSION`), nur zur Anzeige | Ausschlaggebend ist der Regel-Hash; die Version erklärt dem Spieler, woher eine Datei stammt. |
| Alte Replays (Format 5) | Klar abgelehnt („older game version“), in der Liste ausgegraut | Format 5 kennt keinen Regel-Hash; Abspielen könnte still eine andere Partie zeigen. |
| Springen im Replay | Schnappschüsse im Speicher alle 30 s Spielzeit beim Abspielen; Sprung = Szene neu aufbauen ab dem nächstfrüheren Schnappschuss, Rest headless vorspulen | Gelände, Krater und Objektzustände baut die Darstellung beim Start aus der Sim auf; laufende Darstellungen (Krater-, Zerstörungs-, Effektansicht) müssten sonst alle zurückgesetzt werden können. Der Neuaufbau hinter dem Ladebildschirm ist bei `test_small` kurz. |
| Perspektivwechsel | Wie ein Sprung zum aktuellen Tick; Perspektive = lokaler Spieler der Darstellung (Nebel, Gegnerfärbung), „Everything“ = alles sichtbar | Nutzt die bestehende Darstellung unverändert. |
| Geschwindigkeiten | 1×/2×/4×/8× über `SimHost.speed`, im Replay bis 20 Ticks pro Frame | 8× = 160 Ticks/s; mit 5 Ticks pro Frame wäre das erst ab 32 FPS erreichbar. |
| Speichern | Pausenmenü „Save game“, Datei `user://saves/<datum-zeit>.mssave`, beliebig viele; kein Schnellspeichern | Einfach; Schnellspeichern/Autosave später. |
| Laden | Hauptmenü „Load game“ mit Liste (Datum, Karte, Spielzeit, Spieler); Biom und Modus kommen aus dem Spielstand | Gleicher Weg wie der Start aus dem Setup (`MatchSetup.pending`). |
| Datum in Dateien | Von Godot übergeben (lokale Zeit als Text) | Sim und net kennen keine Uhr. |
| UI-Sprache | Englisch wie die übrigen Menüs | Einheitlich. |

## 6. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
