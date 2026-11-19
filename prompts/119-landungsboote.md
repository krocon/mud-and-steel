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
