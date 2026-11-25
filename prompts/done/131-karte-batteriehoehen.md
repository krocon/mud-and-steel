# Prompt 131 – Karte nach Bildvorlage: Batteriehöhen (2 gegen 2, viel Artillerie)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 124
> (Grabenlinie) ist erledigt (`prompts/done/124-*`): Er bringt die Skizzen-Bausteine
> `[[trench]]`, `[[crater_field]]`, `[[wire]]` und die Geschützringe. Lies zuerst
> `CLAUDE.md`, dann `prompts/done/124-karte-grabenlinie.md` und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[river]]`, `[[crossing]]` mit
> `stone`, `[[shoal]]`, `[[hill]]`, `[[wood]]`, `[[village]]`, `[[farm]]`,
> `[[field]]`, `[[trench]]`), „Bridges“, „Objects“ (`fort_*`) und die
> Punktsymmetrie von `city_large`; in `docs/ARCHITECTURE.md` indirektes Feuer,
> Sicht („Seeing downhill“) und Wegfindung; `data/scale.toml` (`[range]`:
> indirektes Feuer ein Achtel der echten Weite), `data/vision.toml`, die Geschütze,
> Mörser und Zugmittel in `data/catalog/*.toml` (`range_effective_m`),
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`. Sind die
> Prompts 115 (lange Brücken), 123 (Flussfront, Reichweitentabelle und
> Reichweitenprüfung), 126 (`[[mesa]]`) oder 129 (gestufte Höhen) erledigt, auch
> deren Ergebnis; deren Bausteine und Werkzeuge wiederverwenden statt neu bauen.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/batteriehoehen/batteriehoehen.webp`. Genau ansehen. Eine mit KI
> gezeichnete, frei erfundene Lagekarte. Die Beschriftungen sind verstümmelt und
> werden **nicht** übernommen (keine Orts- oder Flussnamen in Karte, Markern,
> Menüs), ebenso wenig die Legende mit den Parteien. Verwendet wird nur die
> Landschaft.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `battery_heights` (im Spiel „Batteriehöhen“ / „Battery
heights“) nach der Vorlage, gebaut für **vier Parteien, 2 gegen 2** (Team West
gegen Team Ost, auch 1 gegen 1 spielbar) und für **viel Artillerie**: Jedes Team
hält zwei bewaldete Höhen mit vielen vorbereiteten Feuerstellungen, dazwischen
liegt ein Flusstal mit Gräben auf beiden Ufern, Sumpf und drei Brücken. Wer über
den Fluss will, muss unter dem Feuer der Höhen durch.

Die Vorlage zeigt:

- einen **Fluss von Nord nach Süd** durch die Mitte, in einer weiten S-Kurve
  (oben nach Osten ausgebogen, unten nach Westen);
- **drei Steinbrücken**: Nord, Mitte (umkämpft, mit Trümmern) und Süd; über die
  mittlere läuft die Hauptstraße von West nach Ost;
- **Sumpfwiesen** in den Innenbögen: am Westufer im Norden, am Ostufer im Süden;
- **Grabensysteme im Zickzack** auf beiden Ufern von der Nord- bis zur Südbrücke,
  mit Unterständen und Laufgräben, und eine Riegelstellung quer im Westen südlich
  der Hauptstraße;
- **vier Höhen** mit Felsbändern, Höhenstufen und Nadelwald: Nordwest (die größte,
  mit Steilabbruch zum Tal), Südwest, Nordost, Südost; auf jeder Höhe kleine
  Stellungen und Lichtungen, Serpentinenwege hinauf;
- ein **umzäuntes Lager** mit Hallen am Westrand an der Hauptstraße, ein kleineres
  Lager auf der Nordosthöhe;
- **Dörfer und Weiler** an den Brücken und in den Ecken Nordost und Südost,
  **Getreidefelder** an den Rändern, Wege in alle Richtungen.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung. Nichts daraus übernehmen,
alles mit den Bausteinen des Spiels nachbauen.

## 2. Größe und Symmetrie

- **4 km × 4 km** (größte erlaubte Skizze), Biom `forest`, Wetter klar. Die
  Vorlage wäre nach ihrem Maßstabsbalken etwa 8 km breit; sie wird auf die Hälfte
  gestaucht, der Fluss dabei deutlich schmaler als im Bild. Halten Bildrate oder
  Wegsuche die 4 km nicht im Budget (`docs/PERFORMANCE.md`), alle Maße mit 0,8
  auf 3,2 km verkleinern und die Reichweitenrechnung wiederholen; als Entscheidung
  festhalten.
- **Punktsymmetrisch** um die Kartenmitte, (x, y) → (−x, −y). Das trägt die
  Vorlage: S-Kurve, Sumpf Nordwest gegen Südost, Höhen über Kreuz. Wo die Vorlage
  abweicht (Lager nur im Westen, Dörfer nur im Osten), gilt die Symmetrie. Beide
  Teams bekommen je eine große und eine kleinere Höhe, ein Lager, ein Dorf. Das
  Licht ist nicht symmetrisch; das ist in Ordnung.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0), aus der Vorlage
abgegriffen. Beschrieben ist die **Westseite**; die Ostseite ist ihr Bild unter
(x, y) → (−x, −y). Die Session übernimmt die Werte und glättet sie nach den
Stufenbildern.

- **Fluss** als `[[river]]`, Breite etwa 130 m, Mittellinie: Nordrand
  (130, −2000) → Nordbrücke (190, −1350) → weitester Ostbogen (400, −740) →
  (250, −250) → Mitte (0, 0) → (−250, 250) → weitester Westbogen (−400, 740) →
  Südbrücke (−190, 1350) → Südrand (−130, 2000). Flache Uferstreifen.
- **Drei Steinbrücken** (`[[crossing]]` `stone`), alle West-Ost: Nord bei
  (190, −1350), Mitte bei (0, 0), Süd bei (−190, 1350). Brücken sind
  achsparallel; an der Mittelbrücke läuft der Fluss schräg, er wird dort kurz
  gerade nach Süden geführt. Ist Prompt 115 erledigt: mehrbogige Steinbrücken über
  die volle Breite. Sonst verengt der Generator das Wasser an den Brücken wie
  bisher (als Entscheidung festhalten). Um die Mittelbrücke ein Trichterfeld
  (`[[crater_field]]`) auf beiden Ufern statt der Trümmer der Vorlage. Brücken
  sind sprengbar.
- **Sumpf** im Innenbogen am Westufer, etwa x −400 … 150, y −1000 … −300,
  zwischen Graben und Fluss: nasser, langsamer Boden mit Schilf, Tümpeln und
  wenigen Bäumen, für Fahrzeuge gesperrt oder sehr langsam, für Infanterie
  gangbar, ohne Deckung nach oben (von den Höhen einsehbar). Gibt es kein
  Sumpfgelände, den sumpfigen Bach aus 124 als Muster nehmen (Schlamm, Pfützen)
  und das Einfachste wählen.
- **Furten** (`[[shoal]]`) am Sumpf bei etwa (330, −600) und gespiegelt
  (−330, 600), nur für Infanterie: der zweite Weg, wenn Brücken gesprengt sind.
- **Nordwesthöhe** (die große): Fläche etwa x −2000 … −450, y −1850 … −200, Kamm
  von (−1500, −1500) nach (−750, −600), 60–70 m über dem Tal. Zum Tal hin
  (Ostflanke) ein **Steilabbruch mit Felsbändern**, für Fahrzeuge gesperrt; von
  Westen und Süden flache Hänge mit zwei Serpentinenwegen. Nadelwald auf Kamm und
  Hängen, Lichtungen an den Stellungen.
- **Südwesthöhe** (die kleinere): Fläche etwa x −2000 … −950, y 600 … 1950, Kuppe
  bei (−1400, 1250), 45–50 m, Felsbänder nach Nordosten, ein Weg von Norden
  hinauf. Mischwald.
- Höhen mit den vorhandenen Bausteinen bauen (`[[hill]]` mit Fels-`[[wood]]`,
  `env_rock_massif`; `[[mesa]]` oder die gestuften Höhen aus 126/129, falls
  erledigt). Reicht die Höhe des Generators nicht, ihn erweitern; bestehende
  Skizzenkarten müssen Bit für Bit gleich bleiben.
- **Hauptstraße** (`road`) vom Westrand (−2000, −50) durch das Lager zur
  Mittelbrücke (0, 0) und weiter zum Ostrand; je eine Uferstraße hinter der
  Grabenlinie von der Nord- zur Südbrücke; Feldwege (`track`) zu jeder
  Feuerstellung und zu den Dörfern. Leicht geschwungen wie in der Vorlage.
- **Lager** am Westrand um (−1630, 0), etwa 400 m × 300 m: Hallen, Baracken,
  Stapel, Zaun mit zwei Toren an der Hauptstraße.
- **Grabenlinie West** (`[[trench]]`, Zickzack, mit Laufgräben nach hinten und
  Draht `[[wire]]` zum Fluss hin, Gassen an den Wegen): (−100, −1300) →
  (−450, −800) → (−420, −300) → (−650, 150) → (−780, 650) → (−600, 1050) →
  (−400, 1350). Im Norden liegt sie hinter dem Sumpf, im Süden dicht am Ufer.
  **Riegelstellung** quer von (−1900, 320) nach (−750, 320). Bunker
  (`fort_bunker_mg`) an den drei westlichen Brückenköpfen.
- **Weiler** am Westkopf der Nordbrücke um (−330, −1470), vier bis sechs Häuser.
  **Dorf** mit Kirche um (−700, 1650) südwestlich der Südbrücke, mit Feldern
  (`[[field]]`, Getreide) bis zum Süd- und Westrand. Einzelne Gehöfte
  (`[[farm]]`) an den Wegen im Tal.
- **Wald** außer auf den Höhen nur als Gehölze und Hecken im Tal; das Tal bleibt
  offen, damit Beobachter etwas sehen.

## 4. Spielgedanke: viel Artillerie

- **Reichweiten zuerst rechnen** (indirekt 1/8, Sicht und direktes Feuer 1/4): mit
  den echten Katalogwerten und dem Faktor eine Tabelle in `docs/MAPS.md` (Geschütz,
  Reichweite im Spiel, was es von welcher Stellung erreicht), wie in Prompt 123.
  Richtgröße: eine leichte Feldhaubitze reicht im Spiel etwa 1,4 km. Die Abstände
  aus Abschnitt 3 danach nachziehen, sodass gilt:
  - leichte Haubitzen erreichen aus den **vorderen** Stellungen den Fluss, alle
    drei Brücken und die gegnerische Grabenlinie;
  - aus den **hinteren** Stellungen erreichen nur schwere Geschütze den Fluss;
  - die gegnerischen Feuerstellungen sind von keiner eigenen vorbereiteten
    Stellung aus erreichbar: Gegenbatteriefeuer verlangt, Geschütze ins Tal oder
    über den Fluss vorzuziehen;
  - Mörser wirken nur aus den Gräben über den Fluss;
  - die Starts liegen außerhalb der Reichweite aller Geschütze, die östlich des
    Flusses stehen.
- **Viele Feuerstellungen:** je Team **acht** vorbereitete Stellungen (Geschützring
  aus 124 bzw. `fort_sandbag_position`), jede eben, für Zugmittel mit gezogenem
  Geschütz erreichbar, mit Platz für eine Batterie von drei bis vier Geschützen,
  am **Hinterhang** (vom Gegenufer und von den gegnerischen Höhen nicht direkt
  einsehbar; Sichtfeld aus der Simulation prüfen). Westseite, Richtwerte:
  - Nordwesthöhe vorn: (−1050, −800), (−900, −450); hinten: (−1500, −1300),
    (−1700, −900);
  - Südwesthöhe vorn: (−1100, 1050), (−1150, 1500); hinten: (−1650, 1150);
  - im Tal hinter der Riegelstellung: (−1200, 450).
  So hat jeder der beiden Spieler eines Teams vier Stellungen in seiner Nähe.
- **Beobachtung:** Wer schießen will, muss sehen. Beobachtungspunkte mit Sicht
  ins Tal an der Abbruchkante der Nordwesthöhe (−700, −850) und auf der Kuppe der
  Südwesthöhe am Felsband (−1000, 1100), als Lichtung mit Unterstand; sie sind
  vom Tal aus angreifbar (Infanterie über einen Steig). Versorgungspunkte dort,
  damit um Sicht gekämpft wird.
- **Starts** (60 m eben und frei): Team West (−1750, −400) am Fuß der
  Nordwesthöhe und (−1700, 500) am Fuß der Südwesthöhe; Team Ost gespiegelt
  (1750, 400) und (1700, −500). Spielerzuordnung wie bei den anderen
  Skizzenkarten (Team 0 West: Spieler 0, 2; Team 1 Ost: 1, 3). 1 gegen 1 nutzt
  den ersten Start jedes Teams.
- **Versorgungspunkte** (elf): Mittelbrücke (0, 0), umkämpft; Nordbrücke Westkopf
  (−30, −1350) und Südbrücke Ostkopf (30, 1350); Beobachtungspunkte
  (−700, −850), (−1000, 1100) und gespiegelt; Lager (−1630, 0) und gespiegelt;
  Dorf (−700, 1650) und gespiegelt.

## 5. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/battery_heights.toml` nach Abschnitt 2
bis 4, erzeugt mit `just gen-sketch battery_heights`. Fehlende Bausteine (Sumpf,
höhere Höhen mit Steilabbruch) so einfach wie möglich ergänzen und in
`docs/MAPS.md` beschreiben.

**Stufe 2 – Prüfen:**

- `just map-check battery_heights`, `just map-reach battery_heights`: alle Starts,
  Feuerstellungen und Versorgungspunkte für Fahrzeuge mit gezogenem Geschütz
  erreichbar; die Beobachtungspunkte mindestens für Infanterie; Steilabbrüche und
  Sumpf für Fahrzeuge zu.
- **Symmetrietest:** unter Punktspiegelung gleiche Höhen, Gelände, Objekte,
  Weglängen.
- **Sichtprüfung:** Sichtfeld-Bilder von jeder Feuerstellung (sieht das Gegenufer
  nicht) und jedem Beobachtungspunkt (sieht Fluss, Brücken und Gräben).
- **Reichweitenprüfung:** Test oder Werkzeug (aus 123, falls vorhanden), das je
  Stellung zeigt, welche Brücken, Versorgungspunkte, Starts und gegnerischen
  Stellungen in Reichweite welcher Geschütze liegen; die Regeln aus Abschnitt 4
  als Test.
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, Ergebnis in `docs/AI.md`: Baut und
  nutzt die KI Artillerie, stellt sie sie auf die Höhen, greift sie über Brücken
  und Furten an, endet es im Stillstand? Probleme als offene Punkte; die KI nicht
  umbauen.
- Messung von Bildrate und Wegsuche in jeder Ansicht (`docs/PERFORMANCE.md`), bei
  Bedarf die Verkleinerung aus Abschnitt 2.

**Stufe 3 – Abschluss:** Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md` mit der Reichweitentabelle, Test wie bei den
anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
`just selftest`. Der Golden Hash ändert sich nicht (neue Karte); wenn doch,
Ursache finden.

## 6. Nicht Teil dieses Prompts

- Vorgegebene Startarmeen mit Artillerie oder ein eigenes Szenario dafür.
- KI-Verbesserungen für Gegenbatteriefeuer, Beobachter und Flussübergänge.
- Eigene Modelle für Lagerhallen, Unterstände oder eine zerstörte Brücke.
- Boote über den Fluss (Prompt 119), Winter- oder Schlammvariante.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | 4 × 4 km, nicht verkleinert | Bildrate und Wegsuche im Budget (siehe PERFORMANCE.md); die Stauchung der Vorlage auf die Hälfte reicht. |
| 2 | Abstände nach den Spielreichweiten statt nach den Richtwerten: Gräben dicht am Ufer (im Norden hinter dem Sumpf, 450 m von der Flussmitte), Höhen bis an den Talrand | Mit den Richtwerten lagen die Gräben 800–1 200 m auseinander: kein Mörser über den Fluss, keine leichte Haubitze aus den vorderen Stellungen an den Gegner. |
| 3 | „Leichte Feldhaubitze“ der Regeln = 105 mm M2A1 (1 409 m) | Der Prompt nennt etwa 1,4 km; die 75-mm-Gebirgshaubitze (1 099 m) ist kein Maßstab. „Hintere Stellungen“: keine leichte Haubitze (bis 1 541 m) erreicht den Fluss, die Hummel (1 666 m) schon. |
| 4 | Brücken: jede Brücke von mindestens einer vorderen Stellung je Seite erreichbar, nicht von jeder | 2,7 km Fluss lassen sich mit 1,4 km nicht von jeder Stellung abdecken. |
| 5 | Mörser: schwere Mörser (bis 756 m) erreichen aus jedem Grabenpunkt den gegnerischen Graben; mittlere nur den Fluss südlich der Mittelbrücke | „Mörser wirken nur aus den Gräben über den Fluss“ als Test der schweren Mörser. |
| 6 | Beobachtungspunkte als Marker (nicht eingeebnet) auf der Kante der Felsbänder; sie sehen 10–11 ha Tal unter sich (Sumpf, eigener Graben), nicht den Fluss | Sicht 250 m (+25 % bergab): der Fluss liegt 600–800 m vor der Kante. Ein eingeebneter Versorgungspunkt schiebt die Kante 20 m vor das Auge. Vorgeschobene Beobachter gehören in die Gräben [vorläufig]. |
| 7 | Sumpf = Schlamm (40 %) mit Pfützen und hohem Gras, kein eigenes Gelände; Furten für alle (flaches Wasser 30 %) | Die Regeln kennen kein Gelände nur für Infanterie; der Sumpf hinter der Furt bremst Fahrzeuge stark [offen: Gelände nur für Infanterie]. |
| 8 | Brücken als lange Steinbögen über die ganze Breite (`stone_long`, Prompt 122), sprengbar wie alle Brückenteile | Prompt 115 ist erledigt; der Generator verengt das Wasser nicht. |
| 9 | Höhen als `[[mesa]]` mit gleichmäßigem Hang (Prompt 129), Fuß über den West-/Nordrand hinaus | Sonst liegt der Hang am Kartenrand und das Plateau reicht nicht weit genug zurück für die hinteren Stellungen. |
| 10 | Wald 12 m Abstand (39 700 Bäume), nicht auf den Felsbändern zur Talseite | 8 m gab 104 000 Bäume; Bäume auf dem Hang verdecken den Blick der Beobachter. |
| 11 | Riegelstellung mit Lücke für den Weg vom südlichen Start | Ein durchgehender Graben schnitt den Start vom Lager ab. |
| 12 | Das kleinere Lager steht auf jeder kleineren Höhe, je Team ein Dorf (Symmetrie) | Abschnitt 2 des Prompts. |
