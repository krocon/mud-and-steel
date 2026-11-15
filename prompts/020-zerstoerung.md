# Prompt 020 – Zerstörbare Gebäude und Vegetation

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Wegfindung, Kampf, Nebel des Krieges, Effekte, Kartenobjekte),
> `docs/DETERMINISM.md` und `docs/MAPS.md` („Objects“, „Bridges“).

## 1. Ziel

Häuser, Scheunen, die Kirche, Mauern, Hecken und Bäume lassen sich mit Kanonen
beschädigen und zerstören, Fahrzeuge walzen Zäune und Stacheldraht nieder. Das ändert
Wege, Sicht und Deckung: Eine zerschossene Mauer öffnet Panzern einen Weg, ein gefällter
Wald wird lichter, eine Ruine bleibt als niedrige Deckung stehen. Alles ist eine
deterministische Regel in `crates/sim`; Godot tauscht nur Modell-Varianten und spielt
Effekte ab.

## 2. Ausgangslage

- Kartenobjekte (013) liegen in `Map::objects()` mit Art (Katalog-Id + Variante), Form
  (Kreis oder gedrehte Box) und Blockier-Klasse aus `data/objects.toml`. Gebäude haben
  die Modell-Varianten `Intact`, `Damaged`, `Ruin` (007); Karten setzen sie schon so ein.
- Wegfindung (012): Objekte werden beim Bau des `PathGrid` als Flags gerastert;
  `PathGrid::rebuild(rect)` und `Sim::replan_through` erneuern Teilbereiche und planen
  betroffene Einheiten neu (genutzt von Wracks und `set_bridge_intact`).
- Kampf (017): Geschosse treffen Objekte, die Panzer blockieren, bis zu ihrer
  Katalog-Höhe (`ObjectIndex`); der Treffer richtet keinen Schaden an.
- Nebel (018): `VisionGrid` wird einmal gebaut (feste Hindernisse bis Katalog-Höhe,
  Wald ab zwei Bäumen pro 8-m-Zelle); Sichtfelder werden nur beim Zellwechsel neu
  berechnet.
- Effekte (019): `effects.gd` reagiert auf Kampf-Ereignisse (Einschlag, Explosion,
  Trümmer, Staub).

## 3. Umfang

1. **Regeln** in `data/destruction.toml` (eingebaut, `sim::DestructionRules`, im
   State-Hash): pro Gruppe von Katalog-Ids Lebenspunkte, Schwelle der Stufe
   „beschädigt“, was nach der Zerstörung bleibt (`ruin`: Ruine mit eigener
   Blockier-Klasse und Resthöhe in Prozent; `fallen`: umgestürzter Baum; `gone`: Lücke)
   und ob Fahrzeuge ab einer Masse das Objekt niederwalzen (`crush_min_mass_kg`).
   Mindestkaliber für Schaden an Objekten. Spielregeln, keine Katalogwerte; mit
   Begründung im Kommentar.
2. **Zustand pro Objekt** in der Sim: Lebenspunkte, Stufe (`Intact`, `Damaged`,
   `Destroyed`), Fallrichtung (ganze Grad). Gespeichert werden nur Objekte, die vom
   Anfangszustand abweichen (sortierte `BTreeMap`), damit Hash und Speicher bei
   98 000 Objekten klein bleiben. Karten-Varianten `Damaged` und `Ruin` legen den
   Anfangszustand fest. Die Karte selbst bleibt unverändert.
3. **Schaden:** Ein Geschoss ab dem Mindestkaliber, das ein Objekt trifft, kostet
   Kaliber² × Faktor wie bei Fahrzeugen (`combat.toml`, 7,5 cm: 562). Bei 0 wird das
   Objekt zerstört: Bäume fallen in Flugrichtung des Geschosses, Mauern und Hecken
   verschwinden, Gebäude werden Ruine. Fahrzeuge ab der Mindestmasse walzen Zäune
   (nur Darstellung, sie blockieren nichts) und Stacheldraht (öffnet den Weg für
   Infanterie) in Fahrtrichtung nieder.
4. **Wirkung**, über eine Sicht „wirksame Eigenschaften je Stufe“ (Blockier-Klasse,
   Sicht-Art, Höhe für Sicht und Geschosse):
   - **Wegfindung:** Objekt-Flags im Kostenraster werden für den betroffenen Bereich
     neu gerastert (alle Objekte, die ihn überdecken), Kosten und Komponenten neu
     berechnet, Pfade durch den Bereich neu geplant. Alle Änderungen eines Ticks
     gesammelt, die Komponenten nur einmal pro Tick.
   - **Nebel:** betroffene Sichtzellen neu berechnet (Ruinen niedriger, gefallene
     Bäume kein Wald mehr, Lücken frei), Sichtfelder, die diese Zellen enthalten,
     neu in die Warteschlange.
   - **Geschosse:** Ruinen halten Geschosse bis zu ihrer Resthöhe auf, gefallene
     Bäume und Lücken nicht mehr. Ein Geschoss, das in einer Ruine startet, trifft
     diese Ruine nicht (Infanterie kann aus Ruinen schießen).
5. **Schnittstelle für 022** (Infanterie-Verhalten, parallel): `ObjectStage`,
   `Sim::object_stage(index)`, `Sim::object_hp(index)` und die wirksame
   Blockier-Klasse, damit Besetzen und Deckung den Gebäudezustand nutzen können.
6. **Ereignisse:** `Sim::object_events()` (abgeleitet, nicht im Hash) mit Objekt,
   neuer Stufe, Fallrichtung, Ort und Ursache (Treffer, Niederwalzen). Die Bridge
   sammelt sie pro Frame (`SimHost.take_object_events`) und liefert den Zustand
   aller veränderten Objekte (`changed_objects`, für den Aufbau der Darstellung).
7. **Darstellung** (`godot/scenes/world/destruction_view.gd`, `map_objects.gd`):
   - Gebäude tauschen die Instanz gegen die Variante `Damaged` bzw. `Ruin`.
   - Bäume kippen animiert in die Fallrichtung um, ein Stumpf (`env_tree_stump`)
     bleibt stehen.
   - Mauern und Hecken verschwinden, Zäune und Stacheldraht liegen flach.
   - Effekte aus 019: Trümmer, Staubwolke und Funken beim Einsturz, Splitter und
     Staub beim Fällen, Staub beim Niederwalzen; nur an sichtbaren Orten (Nebel).
8. **Selbsttest** (`tools/destruction_self_test.gd`, nach dem Kampftest): Panzer
   beschießen per Bodenangriff ein Gebäude und einen Baum, bis sie zerstört sind;
   Stufe in der Sim, getauschte Darstellung und Effekte werden geprüft. Der folgende
   Replay-Test spielt die Zerstörung deterministisch nach.
9. **Doku:** `docs/ARCHITECTURE.md` (neuer Abschnitt „Destruction“, Verweise in
   Wegfindung, Kampf, Nebel), `docs/MAPS.md` (Objekte), Kommentar in
   `data/objects.toml`.

Nicht in diesem Prompt: zerstörbare Brücken durch Beschuss (die Funktion
`set_bridge_intact` gibt es; das Schicksal von Einheiten auf der Brücke ist offen),
Splitterwirkung (HE-Radius), Bunker und Panzersperren (bleiben unzerstörbar),
Trümmer als eigenes Hindernis, Feuer in Gebäuden, Geländeverformung (021).

## 4. Akzeptanzkriterien

1. `just check` grün. Golden-Hash bewusst aktualisiert (neuer Zustand im Hash), Grund
   im Commit.
2. Tests in `crates/sim`:
   - zerstörte Mauer öffnet einen Weg: Panzerpfad vorher um die Mauer herum, nachher
     kürzer durch die Lücke;
   - Baum fällt in Flugrichtung, Wald-Zelle wird frei, Sichtfeld neu berechnet;
   - Ruine bleibt: blockiert Panzer, nicht Infanterie, hält niedrige Geschosse auf;
   - Panzer walzt Stacheldraht nieder, Infanterie kommt danach durch;
   - zwei Läufe mit Zerstörung ergeben denselben Hash.
3. `just selftest` (mit Flow-Test), `just lan-selftest`, `just editor-selftest`,
   `just gallery-check` und der Demo-Lauf sind grün.
4. Doku aktualisiert.

## 5. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Wo lebt der Zustand | In der Sim neben der unveränderten Karte, nur Abweichungen | Die Karte (und ihr Hash) bleibt die geladene Karte; Copy-on-write einer 4-km-Karte pro Treffer wäre teuer, und die Varianten-Arten einer Karte enthalten oft keine Ruine. |
| Regeln | Eigene Datei `data/destruction.toml`, von `Sim::new` eingebaut geladen und gehasht, nicht in `SimConfig` | Wie `data/objects.toml` an die Kartenobjekte gebunden; hält `SimConfig` (geteilt mit 022/024) unverändert. Der State-Hash meldet abweichende Regeln. |
| Schaden | Direkter Treffer, Kaliber² wie bei Fahrzeugen, ab 20 mm | Nachvollziehbar und mit 017 konsistent; Gewehrfeuer fällt keine Bäume. Splitterwirkung kommt mit indirektem Feuer. |
| Lebenspunkte | Baum 400, Hecke 600, Mauer 1200, Lehmhaus 1800, Scheune 2000, Bauernhaus 2500 (Ziegel 3000, Stein 3500), Kirche 6000 | 7,5 cm fällt einen Baum mit einem Treffer, eine Mauer mit drei, ein Bauernhaus wird nach drei Treffern beschädigt und nach fünf zur Ruine. |
| Ruine | Blockiert nur Panzer, Resthöhe 40 % (Kirche 30 %) | Schutthaufen und Mauerreste: Infanterie geht hinein und hat Deckung, Panzer fahren außen herum; Sicht über niedrige Reste. |
| Gefallener Baum | Blockiert nichts, kein Wald mehr | Panzer überfahren Stämme; „Wald wird lichter“ ergibt sich aus der Waldzählung. |
| Fallrichtung | Flugrichtung des Geschosses in ganzen Grad, aus der Sinustabelle | Deterministisch ohne Floats und ohne Zufall. |
| Neuberechnung | Am Ende jedes Ticks gesammelt: Raster-Rechtecke, Sichtzellen, Komponenten einmal | Mehrere Treffer pro Tick kosten nur eine Komponenten-Flutung. |
| Darstellung | Instanz in der MultiMesh verstecken, Einzelknoten mit neuer Variante | Kein Neuaufbau der Gruppen; wenige zerstörte Objekte kosten wenig. |

## 6. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Gemeinsame Dateien (`sim.rs`,
`bridge/src/lib.rs`, `main.gd`, `self_test.gd`, `justfile`, Doku) nur klein und additiv
ändern, weil 022 und 024 parallel laufen. Am Ende nach `prompts/done/` verschieben und
`prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde, wie ich es teste,
Messwerte, was offen ist.
