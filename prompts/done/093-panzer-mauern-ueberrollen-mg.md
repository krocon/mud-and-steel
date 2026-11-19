# Prompt 093 – Panzer: Mauern umfahren, Infanterie überrollen, MG gegen Soldaten, Häuser beschießen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, `docs/CONTROLS.md`, dann `data/destruction.toml` (Kopf und
> die Klassen `env_wall_stone`, `env_wall_mud`, `env_fence_wood`, `fort_barbed_wire`),
> `crates/sim/src/destruction.rs`, `crates/sim/src/sim/destruction.rs`
> (`crush_objects`), in `crates/sim/src/path.rs` den Umgang mit zerstörbaren und
> zerdrückbaren Objekten, `crates/sim/src/sim/combat.rs` (`CombatState`,
> `choose_target`, Feuern pro Waffe mit `reload`), `crates/sim/src/combat.rs`
> (`can_damage`, `infantry_damage`), `crates/sim/src/sim/infantry.rs` und
> `crates/sim/src/sim/care.rs` (Verwundete, Bergen), `Command::AttackGround` in
> `crates/sim/src/command.rs` und `crates/sim/src/sim.rs`,
> `crates/bridge/src/combat_funcs.rs`, `godot/scenes/selection/selection.gd`,
> `godot/scenes/hud/command_bar.gd` und `godot/scenes/tools/destruction_self_test.gd`.

## 1. Ausgangslage

Auf der Wüstenkarte (Bild des Nutzers: ein M4 Sherman zwischen Lehmhöfen) fühlt sich
der Panzer zu zahm an. Der Nutzer wünscht vier Dinge:

1. **Mauern umfahren.** Lehmmauern (`env_wall_mud`) und einfache Steinmauern
   (`env_wall_stone`) soll ein Panzer einfach umfahren, also niederwalzen und eine
   Lücke hinterlassen. Heute haben beide Klassen nur Trefferpunkte (800 und 1200) und
   kein `crush_min_mass_kg`. Panzer fahren deshalb außen herum, und nur Beschuss legt
   eine Mauer nieder. Zerdrücken gibt es schon für Holzzäune (ab 2,5 t) und
   Stacheldraht (ab 8 t): `crush_objects`, `ObjectCause::Crushed`, Rest `flat`.
2. **Infanterie überrollen.** Feindliche Soldaten, über die ein Panzer fährt, sollen
   sterben. Das gibt es heute gar nicht; Fahrzeuge und Soldaten durchdringen sich.
3. **MG gegen Soldaten.** Ein Panzer soll sein Maschinengewehr auf feindliche
   Soldaten richten, sobald welche in Reichweite sind. Der Katalog führt die MGs
   schon. Beim M4A3(75)W sind das z. B. `Browning M1919A4 (coaxial, bow)`, 2 Stück,
   und das `Browning M2HB (AA)`. Jede Waffe hat ihren eigenen `reload`, aber eine
   Einheit hat nur **ein** Ziel (`attack` / `auto_target`). `choose_target` bevorzugt
   Ziele der Hauptwaffe. Steht ein Feindpanzer in Reichweite, schweigen die MGs
   deshalb, auch wenn feindliche Infanterie daneben steht. Zuerst im Spiel
   nachprüfen, wann die MGs heute schießen und wann nicht, und das kurz festhalten.
4. **Haus beschießen.** Der Nutzer will mit einer eigenen Taste und einem Klick auf
   ein Haus genau dieses Haus beschießen. Es gibt schon „Bodenziel“
   (`AttackGround`, Taste L, Strg/Cmd + Rechtsklick, Knopf „Ground“). Der
   Zerstörungs-Self-Test nutzt es auf Gebäude. Das zielt aber auf einen Bodenpunkt,
   nicht auf das Objekt. Der Spieler sieht nicht, dass das Haus gemeint ist, und die
   Einheit hört nicht von selbst auf, wenn das Haus Ruine ist. Prüfen, wie gut
   Bodenziel auf Häuser heute trifft, bevor etwas Neues gebaut wird.

## 2. Ziel

### 2.1 Mauern niederwalzen

- `env_wall_mud` und `env_wall_stone` bekommen ein `crush_min_mass_kg`. Lehmmauern
  walzen alle Kettenfahrzeuge und schweren Fahrzeuge nieder. Bei Trockenmauern ist
  die Grenze höher: mittlere und schwere Panzer ja, leichte Panzer und Halbketten
  nein. Die Werte begründen (Masse, Mauerhöhe und -dicke laut Katalog) und als
  `[provisional]` kennzeichnen. Andere Mauerarten (Ziegel, Stadtmauern, Beton,
  Bunker) bleiben unzerdrückbar.
- Der Rest ist eine Lücke. Damit kein neues Modell nötig wird, als Rest `gone`
  verwenden oder, wenn es leicht geht, ein flacher Schutthaufen in der Lücke
  (Darstellung in `destruction_view.gd`), Staubwolke und Geräusch (vorhandene
  Klassen in `sounds.json`, z. B. `object_collapse` o. ä., nachsehen).
- **Wegfindung:** Für Fahrzeuge ab der Massengrenze gilt eine zerdrückbare Mauer als
  befahrbar, mit Zusatzkosten. Das Fahrzeug fährt durch, wenn der Umweg deutlich
  länger ist, und nimmt sonst lieber das Tor. Nachsehen, wie Zäune und Stacheldraht das
  heute lösen, und dasselbe Muster nutzen. Kein Fahrzeug darf an einer Mauer hängen
  bleiben (Prompt 088 hatte genau solche Fälle an Brücken und Furten).
- Das Fahrzeug wird beim Durchbrechen kurz langsamer (Verlangsamung pro Mauerstück,
  `[provisional]`).
- Leichte Fahrzeuge (Jeep, Krad, Lkw unter der Grenze) fahren weiter außen herum.

### 2.2 Infanterie überrollen

- Ein fahrendes Kettenfahrzeug oder schweres Radfahrzeug tötet **feindliche**
  Soldaten, deren Position in seiner Grundfläche liegt (Rechteck aus Länge und
  Breite des Katalogs, gedreht nach Fahrtrichtung). Eine Mindestgeschwindigkeit
  verhindert, dass ein stehender Panzer beim Drehen Soldaten tötet.
- Soldaten sterben nicht stumm. Wer eine gefährliche Annäherung bemerkt (Fahrzeug
  kommt mit Tempo auf ihn zu, ist in Sicht und nicht weiter als ein paar Meter
  entfernt), versucht seitlich auszuweichen, wenn er nicht liegt oder in Stellung
  ist. Liegende und Verwundete erwischt es eher. Das macht das Überrollen
  glaubwürdig und nicht übermächtig. Ausweichweite und Reaktionszeit sind
  `[provisional]`.
- **Eigene und verbündete Soldaten werden nie überrollt.** Fahrzeuge verlangsamen
  oder halten vor ihnen, oder die Soldaten treten zur Seite. Den vorhandenen Umgang
  mit Einheiten im Weg nutzen.
- Infanterie in Gebäuden, Gräben und Stellungen (`positions.rs`, `works.rs`) ist
  geschützt: In einem Graben wird man nicht überrollt.
- Statistik und Erfahrung: Überrollte zählen als Abschüsse des Fahrzeugs
  (`stats.rs`, `experience.rs`). Ein Ereignis oder eine Meldung ist nicht nötig.
- Darstellung: Der Soldat fällt mit der vorhandenen Todesanimation, ohne Blut oder
  Splatter. Ein kurzes Geräusch ist willkommen, wenn eine passende Klasse da ist.

### 2.3 MG gegen Soldaten

- Waffen eines Fahrzeugs, die gegen Infanterie taugen (MG, Kaliber unter 20 mm,
  nicht nur `aa = true`), wählen ihr **eigenes** Ziel unter feindlichen Soldaten in
  Reichweite und Sicht, unabhängig vom Ziel der Hauptwaffe. Die Hauptwaffe bleibt auf
  ihrem Ziel (Befehl oder `auto_target`).
- Einschränkungen nach Lafette, aus dem Waffennamen abgeleitet: Ein Koaxial-MG
  schießt nur in Turmrichtung. Es darf deshalb nur auf Infanterie feuern, wenn die
  Hauptwaffe kein Ziel hat oder das Infanterieziel im Turmwinkel liegt. Ein
  Bug-MG deckt nur einen Kegel nach vorn (etwa ±30°), das MG auf dem Turmdach
  (`M2HB`, `aa = true`) ringsum. Wo der Name nichts verrät, die einfachste Regel
  wählen und notieren. Neue Katalogfelder nur, wenn es ohne nicht geht, mit Quelle.
- Ohne Feindpanzer in der Nähe soll ein Panzer feindliche Infanterie mit MG **und**
  Sprenggranaten bekämpfen, wie heute. Das muss erhalten bleiben.
- „Feuer halten“ (`HoldFire`) schweigt alle Waffen. Befiehlt der Spieler einen
  Angriff auf einen Feindpanzer, schießen die MGs weiter auf Soldaten.
- Der Turm dreht nur für die Hauptwaffe, sonst wäre die Ausrichtung gegen
  Panzer verloren.
- Die Darstellung zeigt Mündungsfeuer und Leuchtspur am richtigen Sockel (Koax am
  Turm, Bug-MG an der Wanne). Nachsehen, welche Sockel die Modelle haben
  (`docs/ASSETS.md`, `Socket_*`). Fehlt einer, den nächsten passenden nehmen und
  notieren.

### 2.4 Haus beschießen

- Ein Befehl „Gebäude beschießen“ (Knopf in der Befehlsleiste und frei belegbare
  Taste, Vorschlag: die vorhandene Bodenziel-Taste auf Gebäude erweitern, oder eine
  freie Taste; in `docs/CONTROLS.md` und den Einstellungen eintragen). Nach dem Druck
  zeigt der Mauszeiger an, ob unter ihm ein zerstörbares Objekt liegt. Das Objekt
  wird hervorgehoben (Umriss oder Tönung). Der Klick befiehlt den Beschuss.
- Sim: entweder ein neuer `Command::AttackObject { units, object }` oder
  `AttackGround` mit Zielobjekt. Die Einheit fährt in Reichweite und Sicht und zielt
  auf die Mitte des Objekts (bzw. den nächsten sichtbaren Teil). Sie hört von
  selbst auf, wenn es Ruine oder weg ist. Mit Shift lassen sich mehrere Ziele
  einreihen, wenn die Befehlsschlange das schon kann.
- Nur Waffen, die Objekten schaden (`min_caliber_mm` in `data/destruction.toml`),
  schießen auf das Haus. MGs suchen sich weiter Soldaten (2.3), auch die im Haus.
- Rechtsklick mit Panzern auf ein **feindlich besetztes** Haus darf ebenfalls
  Beschuss auslösen, wenn das zur bestehenden Bedienung passt (heute: Angriff auf
  die Besatzung?). Das zuerst prüfen und nichts Bestehendes brechen.
- Die KI (`crates/ai`) darf den neuen Befehl nutzen, um besetzte Häuser zu
  beschießen. Das ist kein Muss, aber als Folgeaufgabe notieren, wenn es fehlt.

## 3. Regeln und Technik

- Alles Spielrelevante in `crates/sim`: `Fixed`/`Vec2`, keine Floats, keine
  `HashMap`-Iteration, feste Reihenfolge, z. B. nach Einheiten-ID. Die
  Überrollprüfung muss billig sein: vorhandene Raumaufteilung
  (`crates/sim/src/sim/spatial.rs`) nutzen, nur fahrende Fahrzeuge über der
  Massengrenze prüfen. `just bench` vorher und nachher. Der Tick darf messbar nicht
  langsamer werden.
- Neue Regelwerte in `data/*.toml` (z. B. `data/destruction.toml`,
  `data/combat.toml` oder eine neue Tabelle `[overrun]`), nicht im Code. Echte
  Werte mit Quelle, wo es welche gibt, sonst `[provisional]` und eine Begründung.
- Der **Golden Hash** in `crates/sim/tests/determinism.rs` ändert sich (neue Regeln).
  Das ist eine beabsichtigte Regeländerung: Ursache nachvollziehen, aktualisieren
  und im Commit sagen.
- Neue Befehle: Protokoll in `crates/net` (Serialisierung), Wiedergabe (Replays),
  `docs/CONTROLS.md`, Texte in `godot/locale/hud.csv` (de/en), Tooltip.
- Godot nur Darstellung: keine Godot-Physik für Überrollen oder Mauern.

## 4. Vorgehen

1. Bestand prüfen und kurz festhalten: Wann schießen Panzer-MGs heute? Wie gut
   trifft Bodenziel ein Haus? Wie umfahren Panzer heute Lehmhöfe?
   (`just run --skip-menu` oder ein kleiner Self-Test-Shot)
2. Mauern zerdrückbar machen, mit Wegfindung, Verlangsamung, Lücke, Staub und Ton.
3. Überrollen mit Ausweichen und Schutz für eigene Soldaten.
4. Eigene Ziele für die MGs mit Lafettenregeln, dazu Mündungsfeuer am richtigen
   Sockel.
5. Befehl „Gebäude beschießen“ mit Zeiger, Hervorhebung, Knopf und Taste.
6. Tests, siehe unten. `just check`, `just selftest`, `just bench`, `just catalog-doc`,
   falls Katalogfelder dazukommen, `just units-web-sync`, falls der Katalog sich
   ändert.

## 5. Tests

- Sim-Unit-Tests:
  - Ein M4 fährt durch eine Lehmmauer und hinterlässt eine Lücke; ein Jeep fährt
    außen herum.
  - Ein leichter Panzer durchbricht keine Trockenmauer.
  - Ein feindlicher Soldat in der Fahrbahn stirbt.
  - Ein eigener Soldat in der Fahrbahn stirbt nie.
  - Ein Soldat im Graben überlebt.
  - Ein Panzer im Duell mit einem Feindpanzer schießt mit dem MG gleichzeitig auf
    Infanterie im Turmwinkel bzw. im Bugkegel.
  - Beschuss eines Hauses endet, wenn es Ruine ist.
- Determinismus: zwei Läufe gleicher Hash; Lockstep-Test mit den neuen Befehlen.
- Godot-Self-Test (`just selftest`), z. B. in `destruction_self_test.gd`:
  - Ein Panzer durchbricht eine Lehmmauer.
  - Ein Panzer überrollt eine feindliche Gruppe.
  - Ein Panzer beschießt auf Befehl ein Haus bis zur Ruine.
- Vorher/Nachher-Bild oder kurzes Video auf der Wüstenkarte nach
  `build/tank-overrun/` für den Nutzer.

## 6. Abschluss

`docs/CONTROLS.md`, `docs/GAME_DESIGN.md` (Panzer gegen Infanterie und Objekte) und
`docs/ARCHITECTURE.md` (Tick-Reihenfolge, falls geändert) nachziehen. Ist der Prompt
fertig: `git mv prompts/093-… prompts/done/` und Status in `prompts/README.md` auf
„erledigt“.
