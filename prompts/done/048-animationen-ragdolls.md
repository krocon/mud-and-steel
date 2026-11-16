# Prompt 048 – Soldaten-Animationen, Ragdolls und Treffer

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/ASSETS.md`, `docs/PERFORMANCE.md`.
>
> Konkretisiert im Abschnitt „Konkretisierung“ (Ausgangslage, Entscheidungen).

## 1. Ziel

Soldaten sollen sich glaubwürdig bewegen und fallen: Deckungshaltungen an Mauern, Ecken
und Gräben, Drehen „Kopf zuerst“, Hinwerfen, Kriechen, Rollen, Hocken und geduckt gehen,
Fallen mit Physik und von Explosionen geworfen werden.

## 2. Leitregel

Ragdolls und Physik sind **nur Darstellung** (Regel 2 in `CLAUDE.md`): Die Sim
entscheidet, wer fällt, umgeworfen wird oder wieder aufsteht und wo er danach liegt;
Godot-Physik bewegt nur den sichtbaren Körper dahin und darf keine spielrelevante
Position zurückmelden.

## 3. Umfang

1. **Animationen** (Blender, gemeinsames Skelett aus 005, selbst erstellt; kein
   Motion-Capture-Material Dritter): an Mauer/Ecke/Grabenwand lehnen und herausschießen,
   Hinwerfen, Kriechen, seitlich Rollen, Hocken, geduckt gehen, Aufstehen, Drehen mit
   Kopf und Oberkörper zuerst.
2. **Ragdolls** in Godot (`PhysicalBone3D`/`PhysicalBoneSimulator3D`) für Tod und
   Umgeworfen-Werden; Explosionsimpuls aus dem Sim-Ereignis; nur im Nahbereich der
   Kamera und mit Budget (Anzahl gleichzeitiger Ragdolls).
3. **Umgeworfen (Sim):** Explosionen und Munitionsexplosionen von Panzern werfen
   Infanterie im Radius um (kurz handlungsunfähig), danach Waffe aufheben und weiter.
4. **Helm fliegt weg** als Darstellungs-Effekt bei Kopftreffer/Explosion (Zufall aus
   Darstellungs-RNG, nicht aus der Sim).
5. **Treffer-Darstellung:** zurückhaltend (Staub, Stoff, kurzer dunkler Puff) statt
   Blut-Effekten; [provisional], der Nutzer entscheidet über mehr Härte (Altersfreigabe).

## 4. Akzeptanzkriterien

1. `just kit-test`, `just validate-assets`, `just gallery-check` grün.
2. `just check` grün (Umgeworfen-Regel getestet), `just selftest` grün.
3. Infanterie-Benchmark: höchstens 10 % schlechter; Ragdoll-Budget dokumentiert.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach Welle 12): ein gemeinsames Skelett mit Godot-Humanoid-Namen
und 35 prozeduralen Clips (005, 042: `tools/blender/soldier/animation.py`, Posen in
Code, Blender-IK, gebacken); Trupps aus Soldaten mit eigener Position, Haltung pro
Trupp, Deckungspunkte hinter Objekten und in Kratern, Gebäude (022, 020, 021);
Spreng-Splitter für Granaten, Bomben und explodierende Lager in
`Sim::explode` (032, 033, 041); Panzer explodieren in der Sim bisher nicht (nur
`Destroyed`). Darstellung: `world/infantry_view.gd` wählt pro Soldat einen Clip
(tot, gehen/laufen/kriechen, feuern/zielen, Ruhe) mit Animations-LOD (022, 016);
`soldier.gdshader` zeichnet Muster im Ruhepose-Raum (UV), Helme sind `Paint` über
1,55 m. In Godot läuft keine Physik für das Spiel; Treffer zeigen bisher nur
Einschläge am Boden.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Umgeworfen (Sim):** Sprengladungen (`Sim::explode`: Granaten, Bomben, Lager) werfen Soldaten im Radius 60 mm/mm Kaliber um (8 cm: 4,8 m; kniend 75 %, liegend 40 %, an einem Deckungspunkt ×50 %), **ohne Zufall**. Der Soldat wird bis 2,5 m vom Explosionspunkt weggeworfen (zum Rand hin weniger; nur auf begehbare Punkte), liegt dort 6 s real (3 s Spiel: benommen, aufstehen, Waffe aufheben) und kann in der Zeit weder gehen noch schießen; getroffen wird er wie ein Liegender. Von derselben Explosion Getötete werden mitgeworfen, damit die Leiche dort liegt, wo die Sim sie hinlegt. Regeln in `data/infantry.toml` `[knockdown]`. **[provisional]** | Regel 2: die Sim entscheidet wer, wohin und wie lange; Godot zeigt nur den Fall. Kein Zufall bei der Explosion: weniger Zufallszüge, leicht testbar. |
| 2 | **Munitionsexplosion:** ein zerstörtes Fahrzeug mit Kanone ab 20 mm fliegt mit 40 % in die Luft und wirft Soldaten im Umkreis von 12 m um (gleiche Haltungs- und Deckungsfaktoren, kein Schaden). Der Wurf nutzt einen eigenen `Rng` aus dem Zustand des gemeinsamen Generators und der Einheiten-Id, ohne ihn weiterzudrehen. **[provisional]** | Spec „Munitionsexplosionen von Panzern“; die Demo-Schlacht bleibt gleich (entschieden in Tick 467), nur Regeln und Soldatenzustand gehen neu in den Hash. |
| 3 | **Godot erfährt den Zustand über den Snapshot:** pro Soldat `knocked` (Ticks übrig), `knock_from` (Explosionspunkt) und `cover` (Objekt des Deckungspunkts); Bridge `soldier_knocked`, `soldier_knock_from`, `soldier_cover_objects`, `cover_object_info` in einer eigenen Datei `knockdown_funcs.rs`. Kein neues `CombatEvent`. | Kleine, additive Änderungen an gemeinsamen Dateien (parallele Prompts). |
| 4 | **Neue Clips (20, selbst in Code erstellt, kein Motion-Capture):** `crouch`, `crouch_walk` (1,0 m/s), `dive` (Hinwerfen), `roll_left`/`roll_right` (0,68 m/s seitlich), `knockdown`, `get_up_back`, `get_up_front` (aufstehen und Waffe vom Boden aufheben), `cover_low`/`_aim`/`_fire` (niedrige Deckung: kniend anlehnen, halb aufgerichtet darüber schießen), `cover_tall`, `cover_peek_r/l_aim/_fire` (hohe Deckung: Ecke, Baum, Mauer: herauslehnen), `trench_idle`/`_aim`/`_fire` (an der Grabenwand, Brustwehr 1,35 m), `turn_in_place` (Schritte beim Drehen auf der Stelle). Gleiches Skelett, gleiche Pipeline. | Bestehender Weg von 005/042: Posen in Code + IK; alle Rollen bekommen dieselben Clips. |
| 5 | **Clip-Wahl:** kniende Trupps gehen geduckt (`crouch_walk`); Liegende, die sich vor allem seitlich bewegen, rollen; Soldaten am Deckungspunkt wählen nach Objekt: Graben → `trench_*`, Objekt bis 1,5 m → `cover_low*`, höher → `cover_tall`/`cover_peek_*` (Seite nach der näheren Objektkante; kniend: `crouch` + kniend zielen); Wechsel zu „liegend“ unter Feuer → `dive`. **Drehen „Kopf zuerst“:** ein `SkeletonModifier3D` (`world/turn_lead.gd`) dreht Kopf, Hals und Oberkörper sofort Richtung Ziel, der Körper folgt langsamer, im Stand mit `turn_in_place`. Nur im Nahbereich (70 m). | Wirkt ohne zusätzliche Clips für jede Haltung; im Nahbereich, weil man es weiter weg nicht sieht. |
| 6 | **Ragdolls (nur Darstellung):** `world/ragdolls.gd` baut für einen Soldaten bei Bedarf 11 `PhysicalBone3D` (Kapseln, Kegelgelenke) unter einem `PhysicalBoneSimulator3D`, auf eigener Kollisionsebene mit einer Bodenebene in Höhe der Sim-Position. Auslöser: Tod (Impuls vom Schützen weg) und Umgeworfen (Impuls vom `knock_from` weg, so bemessen, dass der Körper etwa an der Sim-Position landet). Nach ≤ 1,8 s wird die Pose eingefroren (Tote) bzw. per `influence` in `get_up_back`/`get_up_front` übergeblendet (je nachdem, ob er auf Rücken oder Bauch liegt). **Budget: höchstens 8 gleichzeitig, nur bis 60 m Kamera-Abstand;** sonst Clips (`death_*`, `knockdown` + Aufstehen). Zufall (Impulsstärke, Drehung) nur aus einem Darstellungs-`RandomNumberGenerator`. **[provisional]** | Regel 2; Budget hält den Benchmark (≤ 10 %); `PhysicalBoneSimulator3D` ist in Godot 4.7 der vorgesehene Weg. |
| 7 | **Helm fliegt weg:** bei Explosionstod/Umgeworfen (50 %) und Kopftreffer (Einschlag in den oberen 15 % der Haltungshöhe) im Nahbereich: Helm/Mütze im Shader per Instanz-Uniform `helmet_off` ausgeblendet (Gear-Ecken über 1,62 m Ruhehöhe), ein `RigidBody3D`-Helm fliegt davon und bleibt liegen (höchstens 12, älteste zuerst weg). Zufall aus Darstellungs-RNG. **[provisional]** | Gear ist ein Mesh pro Soldat (Draw-Calls); ein Instanz-Uniform kostet nichts und braucht keine Modelländerung. |
| 8 | **Treffer-Darstellung zurückhaltend:** Treffer an Soldaten zeigen einen kleinen Staub-/Stoffpuff und einen kurzen dunklen Puff (kein Blut, keine Wunden), über die vorhandene Effekt-Bibliothek. **[provisional]** – der Nutzer entscheidet über mehr Härte (Altersfreigabe). | Spec. |
| 9 | **Gräben:** Soldaten stehen in 022 0,7 m *hinter* einem Grabenstück; ob sie *im* Graben stehen, entscheidet 035. Die Graben-Clips werden schon gewählt, die Figur aber nicht abgesenkt. **[provisional]** | Paralleler Prompt 035 baut Stellungen; keine doppelte Regel. |
| 10 | **Kein neues Katalogfeld, kein neuer Mesh-Slot, Speicherformat unverändert** (Spielstände prüfen Regel-Hash und Spielversion wie bei 040/041). | Klein halten. |

## Ergebnis

Umgesetzt wie in der Tabelle oben; Details in `docs/INFANTRY.md` („Knocked down by
blasts (prompt 048)“, „Prompt 048“ unter Presentation und Performance).

- **Sim:** `data/infantry.toml` `[knockdown]`, `crate::infantry::KnockdownRules`,
  `crates/sim/src/sim/knockdown.rs` (`blast_reach`, `knock_down`, `ammo_explosion`);
  Haken in `Sim::explode` (vor den Splittern berechnet, danach angewendet) und
  `Sim::destroy`; Umgeworfene schießen nicht, folgen ihrem Platz nicht und haben die
  liegende Trefferbox. `SoldierSnapshot` mit `knocked`, `knock_from`, `cover`.
  Golden-Hash geändert (Regeln und Soldatenzustand im Hash; Demo-Schlacht unverändert).
- **Bridge:** `crates/bridge/src/knockdown_funcs.rs` (`soldier_knocked`,
  `soldier_knock_from`, `soldier_cover_objects`, `cover_object_info`).
- **Blender:** 20 neue Clips in `tools/blender/soldier/animation.py` (`_clips_048`),
  alle 57 Soldaten neu gebaut (55 Clips); Posen als Kontaktbögen gerendert und
  angesehen (Seiten- und Schrägansicht).
- **Godot:** `world/soldier_motion.gd` (Clip-Wahl, Fallen und Aufstehen, Helme,
  Trefferrichtung), `world/ragdolls.gd`, `world/turn_lead.gd`, `world/helmets.gd`;
  kleine Haken in `infantry_view.gd`, `effects.gd` (Soldatentreffer: Staub- und
  dunkler Puff statt Funken), `soldier.gdshader` (Instanz-Uniform `helmet_off`).
- **Tests:** `crates/sim/tests/knockdown.rs` (Wurf weg von der Explosion, liegt still,
  steht nach 60 Ticks auf; Liegende seltener umgeworfen; Munitionsexplosion wirft den
  Trupp neben dem Panzer um, ohne Explosion niemand; deterministisch), Regeltest in
  `infantry.rs`; Infanterie-Vorschau-Selbsttest: Ragdoll fällt, ruht, friert ein;
  Umgeworfener wird ins Aufstehen übergeblendet; Kopf führt die Drehung; Helm fliegt
  und landet; Spiel-Selbsttest: Trupp wirft sich hin (`dive`).
- **Selbsttests nach dem Merge von `main`** (035, 046, 053): `just check`, `kit-test`,
  `validate-assets`, `gallery-check`, `infantry-selftest`, `lan-selftest` grün.
  `just selftest` scheitert ab dem Befehlsleisten-Test von 046 (die Partie ist in
  Tick 4198 schon entschieden: „move button left mode ''“, Transport, Formationen,
  Pause, Aufgeben). Derselbe Fehler tritt mit abgeschalteter Umwerf-Regel *und* den
  Godot-Dateien von `main` auf, liegt also nicht an 048; vor dem Merge war
  `just selftest` mit 048 grün.
- **Messung:** Infanterie-Benchmark A/B höchstens 3 % langsamer (Tabelle in
  INFANTRY.md), Simulation unverändert. Ragdoll-Budget: 8 gleichzeitig, bis 60 m.

Offen: Ragdolls kollidieren nur mit einer flachen Bodenplatte (keine Hänge, Mauern,
anderen Körper); Soldaten in gegrabenen Gräben (035) bekommen die Graben-Clips nicht
(der Snapshot unterscheidet Krater und Graben nicht), Grabenstücke senken die Figur
nicht ab; Panzer-Direktfeuer mit Sprenggranaten hat in der Sim keine Sprengwirkung,
wirft also niemanden um; Werte und Härte der Trefferdarstellung vorläufig.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
