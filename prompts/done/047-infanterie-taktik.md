# Prompt 047 – Infanterie-Taktik: Feuer und Bewegung

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/ARCHITECTURE.md` (Infanterie, Kampf, indirektes Feuer).
>
> Konkretisiert vor der Umsetzung (siehe „Konkretisierung“ unten).

## 1. Ziel

Trupps bewegen sich bisher als Block und suchen Deckung erst am Ziel (022). Nach diesem
Prompt kämpfen sie wie eine Infanteriegruppe: Sie teilen sich in zwei Feuertrupps, einer
gibt Feuerschutz, der andere springt vor, von Deckung zu Deckung.

## 2. Umfang

1. **Feuertrupps:** Trupp teilt sich automatisch (z. B. MG-Gruppe und Schützen),
   abwechselnd Feuerschutz und Sprung, sobald Feindkontakt besteht oder Angriffsbewegung
   (046) befohlen ist.
2. **Deckung zu Deckung:** Wegpunkte entlang Deckungspunkten (Mauern, Krater, Gräben,
   Ruinen) statt der kürzesten Linie; Bewegung unter Beschuss geduckt/kriechend.
3. **Kämpfen unterwegs:** auf dem Marsch erwidern sie Feuer, statt stur weiterzulaufen.
4. **Nahe Feuergefechte liegend**, Blindfeuer um Ecken und über Deckung (geringe
   Treffgenauigkeit, Niederhalten).
5. **Niederhalten** (Suppression): Beschuss senkt Genauigkeit und Bewegung, Trupp geht
   zu Boden; wichtig für Feuer und Bewegung. Spielregel mit Begründung.
6. **Mörsertrupps** bauen selbständig auf und bekämpfen erkannte Ziele (032).
7. **Ladehemmung** als seltene Spielregel (optional, abschaltbar), aus `sim::Rng`.
8. Darstellung über vorhandene Haltungen und Animationen; neue Animationen kommen in 048.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests: Trupp springt abwechselnd vor, nutzt Deckung,
   Niederhalten wirkt, gleicher Hash; Golden-Hash begründet.
2. `just selftest`, `just ai-tournament` grün; Infanterie-Benchmark aus 016 hält.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach Welle 13): ein Trupp ist eine Sim-Einheit mit Soldaten
(eigene Position, Trefferpunkte), Haltung pro Trupp (`StanceOrder::Auto`: stehend in
Bewegung, kniend im Stand, liegend „unter Feuer“ 5 s nach Treffer oder Einschlag
≤ 3 m), Deckungspunkte hinter Objekten, in Kratern und Grabenstellungen nur im Stand
(022, 021, 052), Gebäude. Angriffsbewegung (046, `sim/drill.rs`): Einheit mit
`Advance` hält bei automatischem Ziel an und fährt danach weiter. Mörser (032)
schießen automatisch nur auf selbst gesehene Ziele und bauen nach jeder Bewegung auf
(`setup_ms`). Umwerfen und neue Clips (048, `soldier_motion.gd`: `crouch_walk`,
`dive`, Rollen, Deckungsposen, `reload_*`). Spielstände (038) speichern `Sim` per
serde. Niederhalten wirkte bisher nur auf die automatische Haltung.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Eigene Regeldatei** `data/tactics.toml` (`sim::TacticsRules`), von `Sim::new` geladen wie `indirect.toml`, im State-Hash, nicht in `SimConfig`; Logik in `sim/tactics.rs`, nur kleine Haken in `infantry.rs`, `combat.rs`, `drill.rs`, `indirect.rs`, `air.rs`, `sim.rs`. | Gemeinsame Dateien klein halten (parallele Prompts 049/050/054/055); Replay-/Config-Format unverändert. |
| 2 | **Feuertrupps nach Kaderreihenfolge:** erste Hälfte (Führer + MG-Gruppe / BAR) = Feuerschutz, Rest = Bewegungstrupp; ab 6 lebenden Soldaten, beide Hälften mit ≥ 2. MG-, Mörser-, Scharfschützentrupps bleiben ein Trupp. | Die Truppdateien (006) ordnen Führer und MG vorn ein (DE: Führer, MG 1, MG 2, Munitionsschütze); die Sim kennt keine Rollen pro Soldat. |
| 3 | **Sprung-Modell:** die Truppposition (Sim-Einheit) bleibt zwischen den Teams; der Bewegungstrupp läuft auf eine Linie einen halben Sprung vor ihr, während sie 10–20 m die Route entlang vorrückt; der andere Trupp bleibt liegen und schießt; dann hält die Truppposition, der Bewegungstrupp nimmt Deckungspunkte an seiner Linie ein, 2 s real Einrichten, Wechsel. Laufende schießen nicht, Haltende ohne Bewegungsmalus. **[provisional]** Sprungweiten | Treffertests suchen Soldaten nur im Umkreis der Truppbreite (`spread`) um die Einheit; so bleibt jeder Soldat darin, ohne die Suche (Leistung) zu vergrößern. Real: Sprünge von wenigen Sekunden. |
| 4 | **Wann gesprungen wird:** bei Angriffsbewegung (zwei Teams statt Anhalten) und mit Feuer frei auf normaler Bewegung bei Feindkontakt (Ziel innerhalb 60 % der größten Reichweite). Feuer von unsichtbarem Feind: nur geduckt weiter. Feuer halten: kein Springen, der Trupp läuft. **[provisional]** 60 % | Spieler behalten mit „Feuer halten“ einen schnellen Rückzug; Rückfeuer am Rand der Reichweite bringt nichts und ließ Angreifer im KI-Test verbluten. |
| 5 | **Feuerüberlegenheit bei Angriffsbewegung:** der nächste Sprung beginnt erst, wenn das Ziel ein Trupp mit Niederhalten ≥ 30 ist (oder kein Ziel); bis dahin feuern beide Teams aus Deckung. Normale Bewegung springt ohne diese Bedingung. **[provisional]** | Lehre „erst niederhalten, dann bewegen“; ohne sie liefen Angreifer im KI-Turnier halb feuernd in Verteidiger (Easy schlug Passive nicht mehr sicher). |
| 6 | **Deckung zu Deckung:** die Sprungweite ist die Stichprobe (alle 2,5 m zwischen 10 und 20 m), an der die Linie des Bewegungstrupps die meisten Deckungspunkte (Objekte, Krater, Grabenstellungen) im Umkreis 6 m findet; der Pfad selbst bleibt die geplante Route. Bewegung im Kontakt kniend/geduckt (`crouch_walk`), niedergehalten liegend. | Pfadplanung mit Deckungskosten wäre ein Umbau von 012; das Ende des Sprungs in Deckung ist der sichtbare Teil. |
| 7 | **Niederhalten als Wert 0..100 pro Trupp:** Einschlag ≤ 3 m (bestehender Radius): +8 (ab 20 mm +20), Treffer +10, Granate/Bombe im Splitterradius +35; Liegende bekommen die Hälfte. Erholung 8/s real nach 3 s Ruhe. Wirkung linear: bei 100 Streuung +150 %, Tempo −50 % (nicht kriechend). Ab 60 **niedergehalten**: liegend, Sprünge pausieren, nur die Hälfte schießt ungezielt über die Deckung; unter 30 wieder frei. **[provisional]** alle Werte | MG-Feuer soll einen Trupp in wenigen Sekunden zu Boden zwingen (Test: MG auf 150 m: 77), Gewehrfeuer langsamer; Hysterese gegen Flackern. Erfahrung (050) multipliziert eigene Faktoren dazu (getrennte Streuungs- und Zuwachsfunktion). |
| 8 | **Nahe Feuergefechte:** Ziel ≤ 60 m → automatische Haltung liegend. **[provisional]** | Spec. |
| 9 | **Blindfeuer:** geht das Ziel aus der Sicht (Ecke, Deckung, Rauch), feuert der Trupp 6 s real auf den letzten gesehenen Punkt, mit halb so vielen Soldaten und dreifacher Streuung. | Spec: geringe Treffer, aber Niederhalten. |
| 10 | **Mörsertrupps:** automatisches Ziel ist jeder Feind, den ihr Spieler aufgeklärt hat (nicht nur selbst gesehen); bei Angriffsbewegung halten sie nur für Mörserziele (nicht für Ziele ihrer Gewehre), bauen auf (`setup_ms`) und feuern, danach weiter. Haubitzen und Selbstfahrlafetten unverändert. | Spec 032-Erweiterung; Geschütze sollen nicht ohne Befehl auf weit entfernte Aufklärungsziele wirken. |
| 11 | **Ladehemmung:** jede abgegebene Sim-Kugel eines Soldaten hemmt mit 1 %, 5 s real Beheben (spielt `reload_*`), aus `sim::Rng`; `[jams] enabled = 0` schaltet ab; Fahrzeugwaffen hemmen nicht. **[provisional]** Rate und ob es eine Partie-Option wird | Eine Sim-Kugel steht für mehrere echte Schüsse (Feuerstoß); so tritt sie etwa zwei- bis dreimal pro Gefecht auf. |
| 12 | **Zustand:** pro Trupp `Squad::tactics` (serde, Hash), damit im Spielstand (038); Darstellung über `Sim::squad_tactics`/`soldier_tactics` und die Bridge-Datei `tactics_funcs.rs`; keine neuen `SoldierSnapshot`-Felder, kein neues Kommando. | Additiv, keine Formatänderung des Snapshots für Godot. |
| 13 | **KI:** keine neuen Befehle; leere Transporter fahren aus der Feuerzone zurück (nach dem Abladen standen sie sonst bis zur Zerstörung). | Der KI-Test „rides trucks and tows guns“ hing an diesem Zufall, sobald Gefechte länger dauern. |

## Ergebnis

Umgesetzt wie in der Tabelle oben; Details in `docs/INFANTRY.md` („Tactics (prompt
047)“), Regeln in `data/tactics.toml`.

- **Sim:** `crates/sim/src/tactics.rs` (Regeln), `crates/sim/src/sim/tactics.rs`
  (Feuertrupps, Sprünge, Niederhalten, Blindfeuer, Mörser, Ladehemmung); Haken in
  `infantry.rs` (Haltung, Soldaten folgen Linie/Halt, Deckungspunkte nur für den
  Bewegungstrupp), `combat.rs` (Truppfeuer: Laufende und Gehemmte schießen nicht,
  Streuung × Niederhalten × Erfahrung (050), Blindfeuer, Treffer), `drill.rs`,
  `indirect.rs`, `air.rs`, `sim.rs` (Halt ohne „Stall“: eine Einheit mit Tempo 0
  gibt ihren Wegpunkt nicht auf). Zustand `Squad::tactics` im Hash und im Spielstand.
- **Bridge:** `crates/bridge/src/tactics_funcs.rs` (`soldier_tactics`,
  `squad_tactics`). **Godot:** `infantry_view.gd` spielt bei Ladehemmung
  `reload`/`reload_kneel`/`reload_prone`; geduckt laufen, Hinwerfen, Deckungsposen
  kommen aus den vorhandenen Clips (048); `unit_panel.gd` zeigt „suppressed N %“,
  „pinned down“, „bounding“.
- **KI:** leere Transporter fahren nach dem Abladen aus der Feuerzone zurück.
- **Tests:** `crates/sim/tests/tactics.rs` (Teams wechseln sich ab, Sprünge enden in
  Deckung, MG hält nieder und es klingt ab, Niederhalten bremst, Niedergehaltene
  treffen weniger, Rückfeuer auf dem Marsch, liegend im Nahkampf, Blindfeuer in
  Rauch, Mörser hält und feuert, Mörser trifft aufgeklärte Ziele hinter Häusern,
  gleicher Hash, Ladehemmungen selten), `crates/net/tests/snapshot_tactics.rs`
  (Spielstand mitten im Sprung läuft identisch weiter).
- **Golden-Hash:** `0x9ca9_6477_eff5_75ef` nach dem Merge mit 054 (Wirtschaft `0x745c_45c6_528b_58ee`; vor 054: `0x3d9a_b414_771f_2276`):
  Regeln und Truppzustand im Hash, Gefechte laufen anders. Die Demo-Schlacht ist
  erst in Tick 1017 statt 412 entschieden (gleicher Sieger); der Test
  `demo_battle_is_decided` läuft deshalb 1100 Ticks (Golden-Hash weiter nach 1000).
- **Prüfungen nach dem Merge von `main` (050, 055, 059):** `just check`,
  `just selftest`, `just lan-selftest`, `just infantry-selftest` grün;
  `just ai-tournament --matrix --games 8`: keine Unentschieden/Zeitüberschreitungen,
  gegen Passiv 100 %, Turnier-Hash `0x2c73_15a7_daa5_9825`.
- **Messung** (`just infantry-bench`, Apple M1 Max): stehend 0,027 ms, Marsch
  0,22 ms (vorher 0,22), erste 10 s Feuer 0,16 ms (vorher 0,24) im Mittel – hält.

Offen: der Pfad selbst sucht keine Deckung (nur das Sprungende); Trupps eines
Spielers stimmen ihre Sprünge nicht ab; kein Rückzug/Moral; Fahrzeugwaffen hemmen
nicht; ob Ladehemmungen eine Partie-Option werden; alle Werte vorläufig bis 045.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
