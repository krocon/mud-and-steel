# Prompt 116 – Karte und Mission: Waalbrücke (Vorbild Nijmegen, September 1944)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 115
> (lange Brücken) ist erledigt (`prompts/done/115-*`). Lies zuerst `CLAUDE.md`, dann
> `prompts/done/115-lange-bruecken.md` und `prompts/done/081-karte-hafenstadt.md`
> ganz, die Entscheidungen der erledigten Prompts 112–114 (falls in
> `prompts/done/`), in `docs/MAPS.md` „Maps from real geodata (prompt 081)“,
> „Bridges“ und `harbor_city`, `docs/MISSIONS.md` ganz und eine vorhandene Mission
> (`data/maps/twin_meadow/missions/meadow_crossing.toml` mit `.de.po`). Dann
> `docs/SOURCES.md` (Karten aus Geodaten, Regeln für Abzeichen), `docs/AI.md`,
> `docs/PERFORMANCE.md` („Harbor city“), `tools/geo_import.py` ganz und
> `data/catalog/us.toml` sowie `de.toml` (welche Einheiten es gibt).
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe den Zwischenstand zeigen
> und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte um die **Waalbrücke von Nijmegen** aus echten Geodaten, Stand September
1944, und eine **Mission** dazu. Die Brücke von 1936 steht heute noch; Straßen,
Ufer und ein Teil der Stadt passen daher besser zu 1944 als in Arnhem.

Historischer Rahmen (Operation Market Garden): Amerikanische Fallschirmjäger der
82nd Airborne und britische Panzer der Guards Armoured Division nahmen die Brücke am
20. September 1944, nachdem Fallschirmjäger westlich davon die Waal in Booten unter
Feuer überquert hatten. Die Brücke fiel unbeschädigt in alliierte Hand. Briten gibt
es im Katalog noch nicht (Prompt 117); hier stehen **US-Panzer** für die
Gardetruppen. Die deutschen Verbände stellt der Katalog `de` wie überall, ohne
Abzeichen von SS-Verbänden (`docs/SOURCES.md`).

Die Karte heißt `river_crossing` (im Spiel „Flussübergang“ / „River crossing“), die
Mission ist die erste mit historischem Vorbild. Der Nutzer entscheidet, ob Mission
und Briefing den echten Ort und das Datum nennen (Frage am Ende von Stufe 1 stellen;
bis dahin wie bei `harbor_city` kein echter Name in den Dateien).

## 2. Ausschnitt und Geodaten

Richtwerte, die Session darf sie nach den Stufenbildern anpassen:

- **Ausschnitt** etwa 3,5 km × 3 km: im Süden die Altstadt mit dem Valkhof und dem
  Hunnerpark am Brückenkopf, die Waal (gut 250 m breit) mit der Brücke, im Norden
  die Polder bei Lent mit dem Deich. Die Eisenbahnbrücke westlich nur, wenn sie
  ohne Mehraufwand in den Ausschnitt fällt (Eisenbahnbrücken sind nicht Teil von
  115; dann als Kulisse ohne Fahrbahn oder draußen lassen).
- **Rahmen** so gedreht, dass die Brückenachse auf einer Kartenachse liegt
  (Prompt 115).
- **Höhen:** Copernicus DEM wie in 081 oder, wenn Prompt 114 einen Weg für
  amtliche Geländemodelle gebaut hat, das niederländische AHN (offen, RD-Koordinaten
  EPSG:28992; Lizenz prüfen und in `docs/SOURCES.md` festhalten). Die Deiche
  (wenige Meter hoch, 30–50 m breit) müssen in der Karte als Deiche erkennbar sein;
  mit 30 m DEM notfalls aus OSM (`man_made=dyke`, `embankment=yes`) aufschütten.
- **Stand 1944:** Die Innenstadt wurde im Februar 1944 bei einem Luftangriff schwer
  getroffen und nach dem Krieg teils neu bebaut. Moderne Großbauten, die neue
  Stadtbrücke („De Oversteek“, 2013) und Neubaugebiete im Norden (Lent ist heute
  stark bebaut, 1944 Dorf und Polder) weglassen. Wo das Baujahr fehlt, die
  Höhe begrenzen wie in Prompt 112 und die Entscheidung festhalten. Historische
  Topokarten der 1940er (Kadaster, Lizenz prüfen) und RAF-Luftbilder von 1944
  (Crown Copyright erloschen) als Referenz für das Gebiet am Brückenkopf und im
  Polder nutzen.

## 3. Karte (frei spielbar)

- Zwei Teams: Süd (Stadt) gegen Nord (Polder), je ein oder zwei Starts.
- Versorgungspunkte: Brückenkopf Süd (Hunnerpark), Brückenkopf Nord, Valkhof,
  Deich bei Lent, ein Punkt am Ufer westlich (Bootsübergang).
- Die Brücke ist der einzige Weg für Fahrzeuge über die Waal. Infanterie kann nicht
  schwimmen (Prompt 055: Ertrinken); Boote gibt es nicht. Das ist für die freie
  Karte zu einseitig, wenn eine Seite die Brücke hält. Die Session schlägt eine
  Lösung vor (z. B. Brückenleger und gelegte Brücken nach 055 an einer ruhigen
  Stelle zulassen, oder Starts so legen, dass beide Seiten auf beiden Ufern Fuß
  fassen) und prüft sie im KI-Turnier.
- `just map-check`, `just map-reach`, Messungen und KI-Turnier wie in 081.

## 4. Mission

- **Seite:** Alliierte (US), gegen die KI.
- **Ablauf (Vorschlag):** Phase 1 Häuserkampf in der Altstadt bis an den
  Brückenkopf (Valkhof, Hunnerpark); Phase 2 eine Gruppe Fallschirmjäger erscheint
  am Nordufer westlich der Brücke (die Überquerung in Booten wird nicht gespielt,
  Missionsaktion `spawn`, eine kurze Meldung erzählt sie); Phase 3 Panzer stoßen
  über die Brücke vor, die Fallschirmjäger nehmen den Nordkopf; Sieg, wenn beide
  Brückenköpfe gehalten werden und die Brücke steht.
- **Sprengung:** Die Verteidiger haben die Brücke zur Sprengung vorbereitet. Ein
  Missionsauslöser kann sie sprengen, wenn der Spieler zu lange braucht (Zeit oder
  Bedingung); dann ist die Mission verloren. Die genaue Regel schlägt die Session
  vor.
- Briefing, Meldungen und Ziele auf Deutsch und Englisch (`.de.po` wie die anderen
  Missionen), Missionsliste im Menü, ein Durchlauf-Test wie bei den anderen
  Missionen.

## 5. Stufen

**Stufe 1 – Geodaten, Gelände, Brücke:** Abfrage, Extrakt, Höhen, Waal, Deiche,
Straßen, Brücke nach 115, Stufenbilder, `just map-reach`. Frage an den Nutzer zur
Nennung des echten Ortes.

**Stufe 2 – Stadt, Polder, freie Karte:** Gebäude (Stand 1944), Bewuchs, Starts,
Versorgungspunkte, Menüname, Tests, Messungen, KI-Turnier, Doku.

**Stufe 3 – Mission:** Skript, Texte, Test, Doku in `docs/MISSIONS.md`. Danach
`just check`.

## 6. Nicht Teil dieses Prompts

- Britische Einheiten (Prompt 117), danach die Gardetruppen statt US-Panzern.
- Sturmboote und die Flussüberquerung als spielbarer Teil.
- Eisenbahnbrücke und Züge.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle wie in 081 eintragen
(vorläufig), mit Begründung.
