# Prompt 119 – Landungsboote und Sturmboote

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/034-transport-schleppen.md`, `prompts/done/055-bruecken-pioniertechnik-luftlande.md`
> (Schwimmen und Ertrinken, Luftlandung als Muster für einen eigenen Bewegungsraum)
> und `prompts/done/014-wasser-fluesse-bruecken.md` (Ziel, Entscheidungen,
> Ergebnis). In `docs/ARCHITECTURE.md` „Path grid“, „Transport and towing“,
> „Passengers and towed guns“, „Water (presentation only)“, den Abschnitt zu
> Flugzeugen (`air`) und zu Zerstörung. In `docs/MAPS.md` Geländearten, Höhen und
> Küste (`harbor_city`: Meeresboden in Ringen). Dann `crates/sim/src/path.rs`,
> `crates/sim/src/transport.rs`, `crates/sim/src/sim/transport.rs`,
> `crates/sim/src/sim/crossings.rs`, `crates/sim/src/air.rs`,
> `crates/sim/src/command.rs`, `data/transport.toml`, `data/catalog/SCHEMA.md`,
> `docs/MISSIONS.md`, `docs/ASSETS.md` und `docs/SOURCES.md`.
>
> **Dieser Prompt läuft in drei Stufen** (Abschnitt 7). Nach jeder Stufe den
> Zwischenstand zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Boote als echte Einheiten: Sie fahren auf dem Wasser, nehmen Infanterie, Geschütze
und Fahrzeuge auf, laufen auf einen Strand oder ans Flussufer auf, öffnen die
Bugklappe und setzen ihre Ladung ab. Sie können beschossen und versenkt werden.

Wofür das gebraucht wird:

- **Landungen von See** (Elba 1944, Normandie, Mittelmeer): Landungsboote fahren
  vom Kartenrand an den Strand.
- **Flussübergänge unter Feuer** (die Waal bei Nijmegen, Prompt 116; Rhein,
  Dnepr): kleine Sturmboote, gepaddelt oder mit Außenborder, setzen Infanterie über.

## 2. Bootsarten (erste Ausbaustufe)

Echte Werte mit Quellen wie in jedem Katalog (`docs/SOURCES.md`). Vorschlag, die
Session prüft Maße, Zuladung und Tiefgang nach Quelle:

| Nation | Boot | Rolle | Ladung (Richtwert) |
|---|---|---|---|
| us | LCVP (Higgins-Boot) | Landungsboot | ein Zug Infanterie (~36 Mann) **oder** ein Jeep / ein leichtes Geschütz |
| us | LCM(3) | Landungsboot | ein mittlerer Panzer **oder** ~60 Mann |
| us | M2 Sturmboot (Sperrholz) | Flussübergang, gepaddelt | eine Gruppe (~12 Mann inkl. Pioniere als Paddler) |
| de | Sturmboot 39 | Flussübergang, Außenborder, schnell | ~6 Mann |
| de | Pionierlandungsboot 39 | Landungsboot | Infanterie oder ein leichtes Fahrzeug |
| su | Pontonboot bzw. Sturmboot | Flussübergang | eine Gruppe |

Das LCT (mehrere Panzer) und der Marinefährprahm nur, wenn Zeit bleibt; sonst als
offener Punkt. Britische Boote (LCA, Faltboot „Goatley“) kommen mit Prompt 117 oder
danach; die Datenfelder sollen das ohne Umbau tragen.

## 3. Regeln (Simulation)

Alles in `crates/sim`, deterministisch, keine Floats, Werte in Daten
(`data/transport.toml` oder eine neue `data/boats.toml`), Zeiten real und über den
Zeitfaktor umgerechnet.

- **Bewegungsraum Wasser:** Boote sind Einheiten einer eigenen Größenklasse im
  Weggitter (neben `Small` und `Large`): offen sind Wasserzellen mit genug Tiefe
  für den **Tiefgang** des Bootes (neues Katalogfeld), Land ist zu. Die Wassertiefe
  ergibt sich aus Wasserspiegel und Boden; prüfen, wie Karten den Wasserspiegel
  heute kennen (Meer, Flüsse, Seen) und das einfachste deterministische Modell
  wählen. Ufer und Ränder: ein Boot hält Abstand zum Land außer beim Anlanden.
- **Anlanden:** Befehl „anlanden“ an einer Uferstelle: Das Boot fährt bis zur
  ersten Zelle, an der es aufläuft (Tiefgang erreicht oder Sand/Land vor dem Bug),
  bleibt stehen und öffnet die Klappe (`ramp_ms`). Dann ist es **aufgelaufen** und
  kann ent- und beladen werden. Ablegen: rückwärts frei fahren (`retract_ms`).
- **Ladung:** Infanterie und Geschütze über das vorhandene `Embark`/`Disembark`
  (Plätze aus dem Katalog). **Fahrzeuge als Ladung** sind neu: ein Fahrzeug auf
  einem Boot ist `Aboard` wie ein Passagier (nicht sichtbar für den Gegner außer
  über das Boot, kann nicht schießen). Tragfähigkeit nach Masse und Anzahl
  (Katalogfelder). Ein- und Aussteigen nur, wenn das Boot aufgelaufen ist oder an
  einem Kai liegt (Kai: Kartenkante mit genug Tiefe direkt am Land; als
  Entscheidung festhalten, ob das in diesem Prompt kommt).
- **Absetzen:** Die Ladung geht über die Bugklappe nach vorn, Infanterie watet
  durch flaches Wasser (`shallow_water` wie Furten), Fahrzeuge fahren wie durch
  eine Furt. Steht vor dem Bug noch tiefes Wasser (zu früh aufgelaufen, Sandbank),
  ertrinken Fahrzeuge bzw. muss Infanterie schwimmen (Regeln aus 055).
- **Treffer:** Boote haben Panzerung nach Quelle (LCVP: Sperrholz, gepanzerte
  Bugklappe) und Lebenspunkte wie Fahrzeuge. Ein versenktes Boot: die Ladung
  schwimmt oder ertrinkt nach 055, Fahrzeuge an Bord gehen verloren. Das Wrack
  bleibt in flachem Wasser als Hindernis liegen und sinkt in tiefem Wasser.
  Insassen eines offenen Bootes können von Splittern getroffen werden
  (Vorschlag: Splitterwirkung auf das Boot geht anteilig auf die Insassen).
- **Sicht:** Boote sehen und werden gesehen wie Fahrzeuge (Augenhöhe nach Modell);
  Wasser verdeckt nichts.
- **Gepaddelte Boote:** Geschwindigkeit aus Daten (gepaddelt etwa 3–5 km/h real),
  Paddler sind die Insassen selbst; fällt ein Teil der Insassen aus, wird das Boot
  langsamer (Vorschlag, einfache Regel).
- **Strömung, Wellen, Gezeiten:** nicht in diesem Prompt.
- **Neue Befehle** am Ende des `Command`-Enums (Hash-Stabilität). Bestehende
  Partien und Karten ohne Boote verhalten sich exakt wie vorher: der Golden Hash
  ändert sich nicht. Tut er es doch, Ursache finden und beheben.

## 4. Woher kommen Boote?

- **Missionen und Karteneditor:** Boote lassen sich in Missionen setzen und
  erscheinen lassen (auch „fährt ein“ vom Kartenrand, wie Prompt 108, falls schon
  erledigt). Das ist der Hauptweg.
- **Gefecht ohne Mission:** Vorschlag: Sturmboote baut die Pionierkompanie bzw. es
  gibt sie in der Fahrzeugfabrik nur auf Karten mit Wasser; Landungsboote nur in
  Missionen. Die Session entscheidet nach Aufwand und hält es fest.

## 5. KI

- Die KI schießt auf Boote und verteidigt Strände (Ziele wie Fahrzeuge).
- Die KI selbst benutzt Boote in dieser Ausbaustufe nur, wenn ein Missionsskript
  es befiehlt. Eine eigene Planung für Übersetzen und Landungen ist ein offener
  Punkt.

## 6. Darstellung

- Modelle aus dem Baukasten (headless Blender) nach `docs/ASSETS.md`, Bugklappe
  beweglich, versenkter Zustand. Vorschaubilder (`just thumbnails --only=…`),
  `just units-web-sync`, `just catalog-doc`.
- Bugwelle und Kielwasser, leichtes Schaukeln (reine Darstellung), Spritzer bei
  Nahtreffern, Untergang. Motor- bzw. Paddelgeräusch aus der freien Tonbibliothek
  (`docs/AUDIO_LIBRARY.md`).
- Bedienung: Befehl „anlanden“ in der Befehlsleiste, Rechtsklick auf Ufer mit
  ausgewähltem Boot landet an, Rechtsklick auf ein Boot mit ausgewählter
  Infanterie steigt ein (wie bei Lkw); Infofeld zeigt Ladung (Prompt 109, falls
  erledigt). Steuerung in `docs/CONTROLS.md`.

## 7. Stufen

**Stufe 1 – Regeln:** Weggitter für Boote, Bewegung, Anlanden und Ablegen,
Ladung inkl. Fahrzeuge, Treffer und Untergang; Testkarte mit Küste und Fluss (klein,
von Hand oder per Generator); Tests: LCVP fährt an den Strand, setzt einen Zug ab;
LCM setzt einen Panzer ab; Boot wird versenkt, Insassen schwimmen oder ertrinken;
Sturmboot setzt über einen Fluss; zwei Läufe gleicher Hash; Golden Hash unverändert.

**Stufe 2 – Katalog und Darstellung:** Bootsarten aus Abschnitt 2 mit Quellen,
Modelle, Effekte, Ton, Bedienung, Vorschaubilder.

**Stufe 3 – Einsatz:** eine kleine Beispiel-Mission (Landung an einem Strand einer
vorhandenen Karte, z. B. `harbor_city` oder einer Küstenkarte), Missionsaktionen für
Boote in `docs/MISSIONS.md`, Selbsttest-Teil, Messung (Wegfindung mit Booten) in
`docs/PERFORMANCE.md`, Doku in `docs/ARCHITECTURE.md`. Danach `just check`,
`just selftest`.

## 8. Nicht Teil dieses Prompts

- LCT, Marinefährprahm, Schiffe, Schiffsartillerie.
- Strömung, Wellen, Gezeiten, Wetter auf See.
- KI-Planung von Landungen und Flussübergängen.
- Amphibienfahrzeuge (DUKW, LVT, Schwimmwagen) als eigene Art, die an Land und im
  Wasser fährt. Die Datenfelder sollen sie später zulassen.
- Die Waal-Überquerung in Prompt 116 und eine Elba-Mission: Sie bauen danach auf
  diesen Booten auf.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Wasserspiegel** je Wegzelle aus dem Ufer: jede Wasserprobe (Geländeart `*water*`) übernimmt per Breitensuche (zeilenweise, ganzzahlig) die Höhe des nächsten Ufers, höchstens 0,9 m über dem Grund daneben (wie `WaterField` der Darstellung); Tiefe = Spiegel − Grund, kleinster Wert über Zelle und Probenring (2 m Abstand zum Ufer). Nur in Partien mit einem Boot-Typ gebaut. **[vorläufig]** | Karten kennen keinen Wasserspiegel; das ist das einfachste deterministische Modell, das Meer (Spiegel ≈ 0) und Flüsse gleich behandelt und andere Partien nicht berührt. |
| 2 | **Größenklasse `Boat`** im bestehenden Weggitter (dritte Klasse neben `Small`/`Large`): offen, wo die Tiefe ≥ Tiefgang; eine Komponentenkarte je Tiefgang; Boote fahren unter Brücken durch (keine Geländer); Gruppen aus Booten und Landeinheiten planen getrennt; eigene Bewegungsfunktion ohne Geländetempo und Steigung. | Wiederverwendet Suche, Warteschlange, Glättung, Trennung und Feststecken-Regeln; das Gitter-Hash enthält nur Größe und Version, der Golden Hash bleibt. |
| 3 | **Anlanden** als Phasen landing → run_in → ramp → beached → retract; aufgelaufen, wenn das Wasser unter dem Bug (halbe Länge voraus) flacher als der Tiefgang ist; höchstens 80 m Anlauf über das Wegende hinaus. Ein Bewegungsbefehl auf Land landet an; ein aufgelaufenes Boot bleibt bei Befehlen an Land und Angriffen liegen, ein Befehl aufs Wasser legt zuerst ab. **[vorläufig: Zeiten Klappe 6 s / 2 s, Ablegen 20 s / 8 s real, Anlauf 50–60 % Tempo]** | Keine Quelle nennt Klappen- oder Entladezeiten; das Verhalten bei Bewegungsbefehlen macht Rechtsklick-Bedienung und Missionsskripte (und den Test-Autopiloten) robust. |
| 4 | **Ladung:** Infanterie über `Embark`/`Disembark` mit Boots-Trägerregeln aus `data/boats.toml` (nur in Boot-Partien zu den Transportregeln addiert); Fahrzeuge und Geschütze neu als `Aboard` über `Embark` (neue Aufgabe `Board`) nach `cargo_vehicles` und `cargo_kg` (+20 % Überladung); Fahrzeuge kosten Sitzplätze nach ihrem Anteil an der Nutzlast. Ein- und Aussteigen nur aufgelaufen; **kein Kai** in diesem Prompt. **[vorläufig: Überladung 20 %, Laden 30 s, Entladen 15 s je Fahrzeug]** | „Ein 30-ton-Panzer“ des LCM(3) sind Short Tons (27,2 t); Shermans von 30–32 t wurden 1944 trotzdem gelandet. Kai-Erkennung bräuchte eigene Kartenregeln. |
| 5 | **Absetzen** über den Bug: Trupps an der ersten Stelle vor dem Bug, an steilem Ufer weiter oben (`boat_door`), in tiefem Wasser schwimmen sie (055); ein Fahrzeug in tiefem Wasser (unpassierbares Gelände) geht verloren, sonst fährt es den Strand hinauf. | Wie gefordert, mit den vorhandenen Regeln. |
| 6 | **Treffer:** Boote sind Fahrzeuge (Panzerung nach Katalog); Insassen offener Boote fallen mit `hit_pct` je schädigendem Treffer (auch Splitter); versenkt: Soldaten mit `destroyed_pct`, Rest schwimmt; Fahrzeuge an Bord verloren; Wrack sinkt ab 1,5 m Tiefe, sonst Hindernis. **[vorläufig: 40/50 %, 50/40 %]** | Nutzt die Verlustregeln von 034. |
| 7 | **Gepaddelte Boote:** Tempo = (Besatzung + Soldaten an Bord) / (Besatzung + halbe Sitzplätze), mindestens 25 %. **[vorläufig]** | „Fällt ein Teil der Insassen aus, wird das Boot langsamer.“ |
| 8 | **Katalog:** neue Kategorien `boat_landing`, `boat_assault` (Gruppe Fahrzeuge), Felder `draft_mm`, `cargo_kg`, `cargo_vehicles`, `paddled`; `speed_water_kmh_x10` als Wassertempo. Boote: LCVP, LCM(3), Sturmboot M2 (us), Sturmboot 39, Pionierlandungsboot 39 (de), **Schlauchboot A-3** (su, statt DL-10: keine Daten zum DL-10 gefunden). Rumpfhöhen, gepaddelte Tempi, Masse/Tiefgang A-3 geschätzt und `uncertain`. | Quellen in `docs/SOURCES.md`; Briten (LCA, Goatley) passen ohne Umbau in die Felder. |
| 9 | **Woher Boote kommen:** nur Missionen (und Karteneditor-Missionen); **kein Bau im Gefecht**. Neue Missionsaktionen `land` und `unload`; `spawn … ride` setzt auch Fahrzeuge auf Boote; Boote erscheinen (`appear`), „fährt ein“ nur für Landeinheiten. **[vorläufig]** | Bau im Gefecht bräuchte Wasser-Erkennung der Karten und KI-Nutzung; Missionen sind laut Prompt der Hauptweg. |
| 10 | **KI:** schießt auf Boote wie auf Fahrzeuge; plant keine Landungen (offen). | Wie gefordert. |
| 11 | **Darstellung:** Modelle aus dem Baukasten (neue Baugruppe `boat`, Teile `boat_ramp`, `boat_tube`), Bugklappe als eigener Knoten `Ramp`, Sitz-Sockel im Laderaum; `boat_view.gd`: Klappe, Schaukeln, Schaum als Bugwelle/Kielwasser, Untergang. Das Modell wird 60 % des Tiefgangs höher gezeichnet, damit im Laderaum kein Wasser erscheint. Ton: Landungsboote mit dem Halbketten-Motor, Außenborder mit dem Radfahrzeug-Motor, gepaddelte Boote stumm (keine neuen Aufnahmen geladen). **[vorläufig]** | Keine Downloads ohne Rückfrage; die freie Bibliothek hat noch keine Boots- oder Paddelgeräusche. |
| 12 | **Beispiel-Mission** „Landung am Strand“ (`beach_landing`) auf `dune_coast` (nicht historisch): LCVP mit Zug und LCM mit Sherman landen, dann Dünen säubern. Selbsttest-Teil als Gefechtsdatei `data/battles/dune_coast/boat_test.toml` (Szenarien verschieben Gruppen auf Startpunkte). **[vorläufig]** | Langer Sandstrand ohne Bauten; Elba kommt mit 121. |

## Ergebnis

Umgesetzt wie oben; Details in `docs/ARCHITECTURE.md` „Boats (prompt 119)“,
`docs/MISSIONS.md` (`land`, `unload`, „Landing on the beach“),
`docs/CONTROLS.md` (Boote), `docs/ASSETS.md` (Baugruppe `boat`, `Ramp`),
`data/catalog/SCHEMA.md`, `docs/SOURCES.md` (Boats), `docs/PERFORMANCE.md`
(Boats).

- **Sim:** `data/boats.toml`, `crates/sim/src/boats.rs`,
  `crates/sim/src/sim/boats.rs`, Wasser in `crates/sim/src/path.rs`;
  Befehle `Land`, `Retract` am Enum-Ende; Aufgabe `Board`; Zustand im
  Spielstand (`#[serde(default)]`), Regeln und Zustand nur in Boot-Partien
  im Hash. **Golden Hash unverändert.**
- **Tests:** `crates/sim/tests/boats.rs` (LCVP landet einen Zug, LCM einen
  Sherman, Zuladung, versenkte Boote: Panzer verloren, Männer schwimmen
  oder ertrinken, kein Wrack in tiefem Wasser; Sturmboot setzt über den
  Fluss; gepaddelt langsamer; gleicher Hash),
  `crates/net/tests/snapshot_boats.rs`, Mission im Autopiloten-Test,
  Selbsttest-Teil `godot/scenes/tools/boat_self_test.gd`.
- **Messung:** Weggitter `dune_coast` 110 → 225 ms mit Boot-Typ; 4,3 km
  Bootsweg 0,43 ms.
- **Bilder:** `build/boats/` (`just boat-shots`), Vorschaubilder der sechs
  Boote.

Offen: Kai, Strömung/Wellen/Gezeiten, KI-Planung von Landungen, Boote im
Gefecht bauen, LCT/Marinefährprahm, Amphibienfahrzeuge, eigene Boots- und
Paddelgeräusche, Rumpfhöhen und Zeiten ohne Quelle, Sherman der Mission
kommt die Düne nicht hinauf.
