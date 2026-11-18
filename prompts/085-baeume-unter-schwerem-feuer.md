# Prompt 085 – Bäume unter schwerem Feuer: umgestürzt oder kahl

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md` und
> `docs/DETERMINISM.md`, dann in `docs/ARCHITECTURE.md` die Abschnitte „Destruction“
> (Prompt 020, Regeln, Zustand, Bridge und Godot, Entscheidungen, Offen), „Indirect fire
> and smoke“ mit „Rocket launchers (prompt 062)“ und dessen „Open“, „Fire“ (Prompt 024:
> wie Feuer Objekte zerstört), „Region updates (craters, prompt 021)“ und „Rocket salvo
> (prompt 076)“. Code: `crates/sim/src/destruction.rs`, `crates/sim/src/sim/destruction.rs`,
> `crates/sim/src/sim/indirect.rs` (`explode`), `crates/sim/src/sim/strike.rs`,
> `data/destruction.toml`, `data/indirect.toml` (`[blast]`), in Godot
> `scenes/world/destruction_view.gd`, `scenes/world/map_objects.gd` und `effects.gd`
> (`object_event`). Die Baum-Einträge stehen in `data/catalog/neutral.toml`
> (`env_tree_deciduous`, `_conifer`, `_birch`, `_palm`, `env_tree_stump`).

## 1. Ausgangslage

- Bäume lassen sich heute nur durch **Direkttreffer** fällen (400 Trefferpunkte, eine
  7,5-cm-Granate genügt). Sie haben keine Stufe `Damaged` (`damaged_pct` fehlt), und
  `Remains::Fallen` lässt sie in Flugrichtung umkippen, neben einem Stumpf.
- Eine Granate oder Rakete, die **im Wald** einschlägt, beschädigt nur das eine Objekt, das
  sie getroffen hat; Splitter im Wirkradius treffen nur Einheiten. In ARCHITECTURE.md steht
  das als offen: „HE does not damage objects around the impact point (only the one hit)“.
- Folge: Nach einer vollen Salve der Raketenwerfer (BM-13 „Stalinorgel“, Panzerwerfer 42,
  Calliope, schwerer Feuerschlag aus Prompt 054) mitten in einen Wald stehen fast alle
  Bäume unverändert grün da. Das sieht falsch aus, und der Wald gibt danach dieselbe
  Deckung und Sicht wie vorher.

## 2. Ziel

Wenn große Sprengladungen (Raketen der Werfer, schwere Haubitzen, Fliegerbomben, explodierende
Munitionslager – „vergleichbar große Explosionen“) im Wald oder zwischen Bäumen einschlagen,
sieht man das danach deutlich: Bäume nahe am Einschlag **stürzen um**, Bäume etwas weiter
weg verlieren **ihr Laub** und stehen kahl (Stamm und Äste, Nadelbäume ohne Nadeln,
Palmen ohne Wedel). Eine Salve in einen Wald hinterlässt eine Schneise aus umgestürzten und
kahlen Bäumen.

Fertig ist der Prompt, wenn:

- die Wirkung in der **Simulation** entschieden wird (welcher Baum fällt, welcher kahl
  wird, in welche Richtung er fällt), deterministisch und ohne Floats, damit alle
  Lockstep-Teilnehmer dasselbe sehen und umgestürzte Bäume wie heute Wege und Sicht ändern.
  Godot zeigt nur an.
- ein Baum drei Zustände hat: `Intact` (grün), `Damaged` = **kahl** (neue Bedeutung der
  Stufe für Bäume, über `damaged_pct` und `damaged_variant` in `data/destruction.toml`, wie
  bei Häusern), `Destroyed` = umgestürzt (`Fallen`, wie heute). Ein kahler Baum kann durch
  weitere Treffer noch fallen.
- die Explosion in einem Radius um den Einschlag Bäumen Schaden zufügt, abnehmend mit der
  Entfernung: innen genug zum Fällen, weiter außen genug zum Entlauben, ganz außen nichts.
  Der Radius kommt aus der vorhandenen Regel (`blast_radius(caliber)` in `data/indirect.toml`
  oder einer neuen Größe in `[blast]` bzw. `data/destruction.toml`), nicht aus Zahlen in
  Godot. Kleine Kaliber (Granatwerfer, leichte Geschütze, Panzerkanonen) dürfen den Wald
  **nicht** spürbar lichten: eine Schwelle (Kaliber oder Sprengstoffmasse) sorgt dafür, dass
  nur schwere Ladungen flächig wirken. Richtwert: ab 10,5 cm Haubitze entlaubt einzelne
  Bäume, 13,2-cm-Rakete der BM-13 fällt im Umkreis weniger Meter und entlaubt bis etwa
  10–15 m; die Werte selbst festlegen, als `[provisional]` markieren und begründen. Echte
  Wirkradien aus Quellen nur dann, wenn eine Quelle sie hergibt (`docs/SOURCES.md`);
  ansonsten Spielwerte wie die übrigen `[blast]`-Zahlen.
- die Fallrichtung **vom Einschlag weg** zeigt (Richtung Einschlag → Baum, in ganzen Grad
  aus der Sinustabelle wie heute), nicht in Flugrichtung des Geschosses.
- ein kahler Baum für die Regeln eine **dünnere Krone** ist: entscheide und begründe, ob er
  in der Waldsicht (`VisionGrid`, Zählung der Kronenzellen) noch zählt, halb zählt oder gar
  nicht, und ob er Geschosse noch aufhält (Stamm ja, Krone nein?). Weg und Panzerblockade
  bleiben wie bei einem stehenden Baum.
- Godot je Baumart eine **kahle Variante** zeigt (neue Modelle `env_tree_*_bare` oder ein
  Materialschalter im Baumshader, der das Laub ausblendet – entscheide nach Aufwand und
  Draw-Call-Budget in `docs/ASSETS.md`; neue Modelle brauchen Katalogeinträge, Thumbnails
  `just thumbnails --only=...` und `just units-web-sync`). Beim Entlauben fällt sichtbar
  Laub bzw. Nadeln herab (kurzer Partikelstoß in `effects.gd`, nur an sichtbaren Punkten),
  beim Umstürzen wie heute Splitter und Staub. Im Winterbiom prüfen, wie Laubbäume heute
  aussehen (eventuell schon kahl), und dort nichts doppelt machen.
- die Kosten stimmen: eine Salve von 16 Raketen in einen dichten Wald kann in wenigen Ticks
  dutzende bis hunderte Bäume ändern. Die Zerstörung sammelt heute Rebuilds je Tick
  (ein Flood-Fill je Tick mit Zerstörung, rund 8 ms auf `test_large`). Messen
  (`just bench`, eine Salve in Wald auf `test_large` und `city_large`) und dafür sorgen, dass
  ein Salventick nicht über das Tickbudget von 50 ms geht; kahle Bäume ändern Wege nicht und
  brauchen keinen Flood-Fill, nur Sicht-Rebuilds. Die Objektsuche im Radius über den
  vorhandenen Objektindex, nicht über alle Objekte.
- ein Sim-Test belegt: Salve in ein Waldstück → Bäume im Innenradius `Destroyed`/`Fallen`
  mit Fallrichtung weg vom Einschlag, im Ring dahinter `Damaged`, außerhalb unverändert;
  ein 8-cm-Granatwerfer ändert keinen Baum außer bei Direkttreffer; zweimal gleich gerechnet
  ergibt denselben Hash.
- `just check`, `just selftest` und `just editor-selftest` grün sind. Der **Golden Hash** in
  `crates/sim/tests/determinism.rs` wird sich ändern (neue Regel, `data/destruction.toml`
  und `[blast]` sind im State-Hash): Ursache prüfen, nur wegen dieser Regeländerung
  aktualisieren und das im Commit-Text sagen.

## 3. Vorgehen

1. Vorher-Standbilder: eine Raketensalve (BM-13 oder schwerer Feuerschlag) und eine
   15-cm-Haubitzensalve in ein Waldstück, Kamera auf mittlerer Zoomstufe, Tag, vor und
   10 s nach dem Einschlag (z. B. mit einem kleinen Werkzeug wie `blast_shots.gd` oder
   `just salvo-shots`). Nach `build/forest_blast_shots/before_*.png`.
2. Regeln: Flächenschaden auf Objekte in `explode` (gilt damit für Granaten, Raketen und
   alles, was `explode` nutzt); prüfen, ob Fliegerbomben (`air.rs`) und explodierende
   Lager (`fire.rs`, `blast_caliber_mm`) denselben Weg gehen, sonst anschließen. Nur Bäume
   bekommen in diesem Prompt Flächenschaden; Häuser, Mauern, Hecken bleiben bei
   Direkttreffern (siehe 4.).
3. Bridge: die kahle Stufe in `object_state`/`object_instance` und den Events durchreichen
   (wahrscheinlich schon allgemein vorhanden: `Damaged` mit `damaged_variant`).
4. Darstellung: kahle Varianten, Laubfall, Umstürzen weg vom Einschlag. Bei vielen
   gleichzeitigen Änderungen die Effekte begrenzen (Partikelbudget), damit eine Salve nicht
   unter 60 FPS fällt (`just bench-view` bzw. `effects_bench.gd`, „high“).
5. Nachher-Standbilder mit denselben Kameras nach `build/forest_blast_shots/after_*.png`
   und beide nebeneinander zeigen.
6. Doku: `docs/ARCHITECTURE.md` (Destruction: Tabelle der Objekte, Entscheidungen, „Open“
   bereinigen; Indirect fire „Open“), `docs/GAME_DESIGN.md`, falls dort Wald und Artillerie
   beschrieben sind, `docs/PERFORMANCE.md` mit den Messungen, `just catalog-doc` bei neuen
   Katalogeinträgen.

## 4. Nicht Teil dieses Prompts

- Flächenschaden an Häusern, Mauern, Hecken, Zäunen und Brücken (eigener Prompt; hier nur
  so bauen, dass er sich später anschließen lässt).
- Brennende Wälder durch Raketen (gibt es über Prompt 024 schon, Zündung im Wirkradius;
  nicht verändern, nur prüfen, dass kahle und umgestürzte Bäume sich mit Feuer vertragen:
  brennt ein kahler Baum? verkohlt ein umgestürzter?).
- Nachwachsen, Jahreszeiten, Herbstlaub.
- Zersplitterte Baumstümpfe als eigenes Hindernis oder Baumkrepierer (Granaten, die in der
  Krone explodieren und Splitter nach unten werfen) als Regel für Infanterie; höchstens als
  Idee unter „Open“ notieren.

## 5. Abstimmung mit laufender Arbeit

Prompt 082 (Video „Hinterhalt am Waldrand“, Karte `ambush_meadow`, Szene
`data/trailer/ambush.toml`) nutzt eine Raketensalve am Waldrand. Die neue Regel verändert
diese Szene (Bäume fallen, Hash ändert sich). Vor dem Start prüfen, ob 082 noch läuft; falls
ja, erst danach beginnen oder die Szene von 082 nach der Änderung neu abnehmen und das im
Prompt 082 vermerken.
