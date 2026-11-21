# Prompt 171 – Karte nach Bildvorlage: Felseninsel (Steilküste, Kessel, sechs Lager)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt
> 126 (Tafelberge: Skizzen-Tabelle `[[mesa]]` mit Abbruchkanten und Rampen) ist
> erledigt (`prompts/done/126-*`). Empfohlen nach 133 (Wüstenplateau: Auffahrten
> für alle Fahrzeuge mit Fahrprobe), 149 (Höhenstufen: mehrere Ebenen), 122
> (Inselreich: Meer und Look je Karte) und 113 (Steilküste aus Geodaten).
>
> Lies zuerst `CLAUDE.md`, dann `prompts/done/088-karten-nach-bildvorlagen.md`
> ganz und von `126-*` und, falls erledigt, `133-*`, `149-*`, `122-*` Ziel,
> Entscheidungen und Ergebnis. In `docs/MAPS.md` „Maps from picture sketches
> (prompt 088)“ ganz (mit `[[mesa]]`), den Abschnitt zum Gebirge (Felsbänder,
> Geröll, Steigungen), „Objects“; in `docs/ARCHITECTURE.md` Wegfindung
> (Größenklassen, Steigungsgrenzen je Fahrzeug), Sicht über Gelände,
> Wasser/Meer; `data/scale.toml` (`[range]`), `data/terrain.toml`,
> `data/objects.toml`, `docs/TEAMS.md`, `docs/SOURCES.md`,
> `crates/cli/src/gen_sketch.rs` und die Skizzen aus 126/133 als Muster.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/felseninsel/img.webp`: ein mit KI (ChatGPT) erzeugtes, frei
> erfundenes Bild, 1672 × 941 px. **Schrägansicht** von Süden: Die Insel ist in
> Wahrheit etwa rund, vorne ist alles größer und die Felswände der Nordseite
> verdecken, was dahinter liegt. Die Session entzerrt die Anordnung zu einem
> Grundriss und zeigt ihn in Stufe 1 neben dem Bild. Es gibt **keine SVG und
> keine Beschreibung**: Maße und Regeln kommen aus diesem Prompt.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Skizzenkarte `cliff_island` (im Spiel „Felseninsel“ / „Cliff island“):
eine runde, grüne **Felseninsel im offenen Meer**, rundum von **senkrechten
Klippen** umgeben, ohne Strand und ohne Hafen. Oben ist sie in **Stufen**
gegliedert: ein hoher **Randring**, auf dem die Lager stehen, fällt mit
Felswänden nach innen zu einem tieferen, grünen **Kessel** mit Tümpeln,
Wäldchen und einer Ruine in der Mitte ab; dazwischen liegen einzelne
**Zwischenplateaus**. Fahrwege führen über **Rampen**, die schräg in die
Felswände geschnitten sind. Wer die Rampen hält, bestimmt, wer in den Kessel
und wieder heraus kommt.

Die Vorlage zeigt (Lage im Bild; die Meter ergeben sich erst aus der
Entzerrung):

- **Steilküste** rundum, 30–60 m hoch, mit Brandung, vorgelagerten
  **Felsnadeln** und kleinen Felsinseln (vorne links, vorne rechts, rechts
  oben, links);
- **sechs Lager** auf dem Randring, jedes mit Holzwachtürmen, Baracken und
  einer runden **Geschützstellung** im Erdwall: oben Mitte, oben rechts,
  rechts, vorne Mitte (das größte), links, links oben (klein);
- **Felswände nach innen**, 10–25 m hoch, die den Ring vom Kessel trennen;
  besonders hoch oben (Nordseite), wo die Wand eine lange Schlucht bildet;
- **Zwischenplateaus**: ein großes oben Mitte mit einem **Steinkreis aus
  Ruinen**, weitere kleine Felsköpfe mit Bäumen;
- **Kessel** in der Mitte, grüne Wiese mit gelben Blumen, mehrere **Tümpel**,
  Nadelwäldchen, eine **Burgruine** (Mauerreste, Turmstumpf) etwas links der
  Mitte, Felsbrocken;
- **Rampen**: etwa sieben schräg in die Wände geschnittene Fahrwege
  (links oben, oben Mitte zum Ruinenplateau, rechts oben, links, rechts, vorne
  rechts lang zum vorderen Lager, vorne links), dazu Pfade, die nur für
  Infanterie sind;
- **Nadelwald** in Gruppen auf Ring und Plateaus, einzelne blühende Bäume.

## 2. Größe, Symmetrie, Spieler

- **Größe:** Insel nach der Entzerrung etwa **2400–2800 m** Durchmesser
  (Lager etwa 50–80 m groß als Maßstab), Karte **3200 × 3200 m** mit Meer
  rundum, mindestens 150 m Wasser zwischen Klippe und Kartenrand. Biom
  `forest`, sommerlicher Look, Meer wie in 122, Wetter klar.
- **Spieler:** sechs Starts, einer je Lager. Spielarten **jeder gegen jeden
  (bis 6)**, **2 gegen 2** und **3 gegen 3**, dazu 1 gegen 1 auf zwei
  gegenüberliegenden Lagern. Die Lobby erlaubt heute bis 6 Spieler; mehr ist
  nicht Teil dieses Prompts (143).
- **Symmetrie:** Weil alle Lager gleich viel wert sein müssen, wird die Insel
  **sechsfach drehsymmetrisch** um die Mitte gebaut (Drehung um 60°): Die
  Session zeichnet ein Sechstel (ein Lager, sein Stück Ring, seine Rampe, sein
  Stück Kessel) nach dem Bild und dreht es fünfmal. Der Höhenraster ist
  quadratisch, die Drehung also nicht zellgenau: Abweichungen bis eine Zelle
  sind erlaubt, Weglängen und Sichtflächen werden geprüft (Stufe 2). Folgen,
  die gewollt sind: Aus den ungleich großen Lagern im Bild werden sechs gleiche;
  das Ruinenplateau oben und die Burgruine in der Mitte werden zu **einem
  Mittelstück** (Burgruine im Kessel auf einem flachen Mittelhügel) und
  **sechs kleinen Zwischenplateaus** mit Steinkreis-Ruinen, je eines zwischen
  zwei Lagern. Ist sechsfache Drehung im Generator unverhältnismäßig teuer,
  fällt die Session auf **dreifache Drehung (120°) mit Punktsymmetrie**
  zurück (gleiches Ergebnis für sechs Lager, wenn das Sechstel spiegelbar
  ist), begründet das und schreibt es in die Entscheidungen.
- **Teams:** Für 2 gegen 2 und 3 gegen 3 die Lager so zuteilen, dass
  Teammitglieder nebeneinander liegen (3 gegen 3: drei benachbarte Lager je
  Team; 2 gegen 2: zwei Paare gegenüber, zwei Lager leer). In `docs/TEAMS.md`
  festhalten, falls die Startzuteilung je Spielart neu ist.
- **Starts:** je Lager 60 m flach und frei, **zwei Rampen** in den Kessel in
  Reichweite (eigene und eine geteilte mit dem Nachbarn), ein Ringweg zu beiden
  Nachbarlagern.
- **Versorgungspunkte:** 13: je einer in jedem Lager (6), je einer auf jedem
  Zwischenplateau (6), einer an der Burgruine in der Mitte.

## 3. Gelände und Wege

- **Ebenen:** Ring etwa 50–70 m über dem Meer, Zwischenplateaus 15–25 m unter
  dem Ring, Kessel 25–40 m unter dem Ring, Mittelhügel 5–10 m über dem Kessel.
  Werte so wählen, dass gezogene Geschütze die Rampen schaffen (Steigungsgrenzen
  aus `data/terrain.toml` und dem Katalog).
- **Klippen und Wände** als `[[mesa]]`-Kanten bzw. Felsbänder: **unpassierbar**
  für alle Fahrzeuge, für Infanterie nur über die eingezeichneten Pfade.
  Die Außenklippe ist für niemanden passierbar; es gibt keine Landung.
- **Rampen:** je Lager eine eigene Rampe in den Kessel und zu jedem
  Zwischenplateau mindestens zwei Rampen (vom Ring und vom Kessel), alle
  **mindestens 12 m breit** und mit **Fahrprobe** wie in 133 (jede
  Größenklasse, auch Zugmaschine mit Geschütz, hinauf und hinunter).
- **Ringweg:** auf dem Randring ein Fahrweg rund um die Insel, der alle Lager
  verbindet; an jedem Zwischenplateau eine Engstelle, aber keine Sackgasse.
- **Sicht:** Vom Ring sieht man in den Kessel hinab (Höhe gegen Weite), aber
  nicht über den Kessel bis ins übernächste Lager: Mittelhügel, Wäldchen und
  die Zwischenplateaus brechen die Sicht. Größte freie Fläche ohne Deckung
  höchstens 500 m.
- **Reichweiten:** Abstände rechnen und als Tabelle festhalten: Lager zu
  Nachbarlager über den Ring und über den Kessel, Lager zur Mitte. Leichte
  Artillerie aus einem Lager erreicht die Mitte, aber kein anderes Lager.
- **Tümpel** im Kessel flach (Infanterie langsam, Fahrzeuge nicht), so
  verteilt, dass sie keine Rampe sperren.

## 4. Stufen

### Stufe 1: Entzerren und Skizze

Zuerst den entzerrten Grundriss (Küste, Ebenen, Wände, Rampen, Lager,
Plateaus, Tümpel) und das gewählte Sechstel als Bild neben der Vorlage zeigen.
Dann `data/map_sketches/cliff_island.toml`, erzeugt mit
`just gen-sketch cliff_island` (Drehung im Generator oder als Rechenhilfe: die
Session wählt und begründet). Bilder aus dem Spiel: Übersicht, ein Lager mit
Rampe, ein Zwischenplateau, die Mitte, die Steilküste, neben dem Ausschnitt
der Vorlage.

### Stufe 2: Prüfen

- `just map-check cliff_island`, `just map-reach cliff_island`: jedes Lager,
  jeder Versorgungspunkt und die Mitte von jedem Start für jede Größenklasse
  erreichbar; Fahrprobe auf jeder Rampe.
- Symmetrietest (Drehung: Höhen, Objekte, Versorgung innerhalb einer Zelle),
  Test wie bei den anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien).
- Gerechtigkeit als Tabelle: Weglänge jedes Starts zur Mitte, zu den beiden
  nächsten Zwischenplateaus und zu den Nachbarlagern (höchstens 3 %
  Unterschied).
- Sichtfeld-Bilder von einem Lager, von einem Zwischenplateau, von der Mitte;
  Reichweitentabelle.
- Wegsuche und Bildrate (Wasser rundum, viele Kanten) wie in
  `docs/PERFORMANCE.md`.
- **KI-Turnier** jeder gegen jeden zu sechst, 3 gegen 3 und 1 gegen 1, über
  viele Seeds, Ergebnis in `docs/AI.md`: Gewinnt jedes Lager ungefähr gleich
  oft (Ziel: kein Lager über 1,5-mal seinem Anteil)? Findet die KI die
  Rampen? Probleme als offene Punkte; die KI nicht umbauen.

### Stufe 3: Look und Abschluss

Sommerlicher Look mit gelben Blumen, helle Kalkfelsen an den Wänden,
Brandung an der Küste, Felsnadeln im Meer als Objekte ohne Spielwirkung.
Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
`docs/MAPS.md` (mit Zuordnung Vorlage → Baustein, Entzerrung, Ebenen-,
Reichweiten- und Sichttabelle), `docs/SOURCES.md` (Vorlage frei erfunden,
kein reales Vorbild), `just selftest`, `just lan-selftest`, `just check`. Der
Golden Hash ändert sich durch eine neue Karte nicht.

## 5. Nicht Teil dieses Prompts

- Landungen von See, Boote, Hafen (119 Landungsboote ist ein eigener Prompt).
- Mehr als sechs Spieler (143).
- Neue Modelle (Holzwachturm, Geschützstellung im Erdwall, Steinkreis, falls
  nicht vorhanden): vorhandene Objekte als Ersatz, offene Punkte.
- KI-Verbesserungen für Rampen und jeder-gegen-jeden.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| Frage | Entscheidung | Grund |
|---|---|---|
| Vorlage | nur das Bild (Schrägansicht, wird entzerrt); Maße und Regeln aus diesem Prompt | keine SVG vorhanden |
| Spieler | sechs Lager, jeder gegen jeden, 1 gegen 1, 2 gegen 2, 3 gegen 3 | sechs Lager im Bild; Lobby bis 6. Vom Prompt vorgeschlagen, nicht vom Nutzer entschieden |
| Symmetrie | sechsfach drehsymmetrisch (Rückfall: dreifach mit Punktsymmetrie) | sechs gleichwertige Lager für jeder gegen jeden |
| Größe | Insel etwa 2,4–2,8 km, Karte 3,2 × 3,2 km | Lagergröße als Maßstab, Meer rundum |
| Landung | keine | Steilküste rundum, wie im Bild |

## Ergebnis

(von der Session auszufüllen)
