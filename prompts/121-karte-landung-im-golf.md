# Prompt 121 – Karte und Mission: Landung im Golf (Vorbild Marina di Campo, Elba, Juni 1944)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzungen:** Prompt 120
> (Mittelmeer-Biom) und Prompt 119 (Landungsboote) sind erledigt (`prompts/done/`).
> Ohne 119 nur Stufe 1 und 2 (freie Karte), die Mission als offener Punkt. Lies
> zuerst `CLAUDE.md`, dann `prompts/done/120-mittelmeer-biom.md`,
> `prompts/done/119-landungsboote.md` und `prompts/done/081-karte-hafenstadt.md`
> ganz, die Entscheidungen der erledigten Prompts 112–116 (falls in
> `prompts/done/`, besonders die Frage, ob Missionen echte Orte nennen), in
> `docs/MAPS.md` „Maps from real geodata (prompt 081)“ und `harbor_city`,
> `docs/MISSIONS.md` und eine vorhandene Mission mit `.de.po`, `docs/SOURCES.md`,
> `docs/AI.md`, `docs/PERFORMANCE.md` („Harbor city“), `tools/geo_import.py` ganz,
> `assets-src/geo/harbor_city/site.json`, `data/catalog/us.toml` und `de.toml`,
> `data/objects.toml` (`fort_*`).
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe den Zwischenstand zeigen
> und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte um den **Golf von Campo** an der Südküste von Elba aus echten Geodaten,
Stand Juni 1944, und eine **Landungsmission** dazu.

Historischer Rahmen (Operation Brassard, 17.–19. Juni 1944): Französische Truppen
(Kommandos, Kolonialinfanterie, marokkanische Goumiers) landeten mit britischer und
amerikanischer Marineunterstützung gegen die deutsche Besatzung; der Golf von Campo
war eine der Hauptlandezonen, Marina di Campo wurde hart verteidigt, Küstenbatterien
in den Hängen beschossen die Boote. Die Session prüft die Einzelheiten an Quellen
(`docs/SOURCES.md`) und erzählt nur, was belegt ist. Ereignisse an der
Zivilbevölkerung nach der Landung gehören nicht ins Spiel.

Es gibt keine französische Nation. Die Freien Franzosen waren 1944 fast ganz
amerikanisch ausgerüstet; sie werden mit **US-Einheiten** dargestellt. Die deutsche
Besatzung stellt der Katalog `de` wie überall, ohne Abzeichen (`docs/SOURCES.md`).

Die Karte heißt wie die anderen ohne echten Namen `landing_gulf` (im Spiel „Landung
im Golf“ / „Landing gulf“). Ob die Mission den
echten Ort nennt, folgt der Entscheidung aus Prompt 116; ist die noch nicht
gefallen, den Nutzer am Ende von Stufe 1 fragen.

## 2. Ausschnitt und Geodaten

Richtwerte, die Session darf sie nach den Stufenbildern anpassen:

- **Ausschnitt** etwa 4 km × 4 km um den Golf: der Strand und das Dorf Marina di
  Campo, die Ebene von Campo dahinter, die Hänge links und rechts des Golfs mit den
  Stellungen, im Nordwesten der Anstieg zum Monte Capanne (der Gipfel liegt etwa
  6 km entfernt und bleibt draußen). Im Süden ein Streifen Meer, tief genug für die
  Landungsboote (Prompt 119), vom Kartenrand bis an den Strand.
- **Rahmen** so gedreht, dass das Meer unten im Bild liegt (Blick von See wie bei
  `harbor_city`).
- **Höhen:** Copernicus DEM wie in 081 (0 bis einige hundert Meter). Die steilen
  Flanken prüfen (`just map-reach`): Die Hänge mit den Stellungen müssen für
  Infanterie erreichbar sein, für Fahrzeuge die Straßen ins Hinterland.
- **Meeresboden:** wie in 081 in Ringen, aber flacher auslaufend am Sandstrand, damit
  Landungsboote nahe am Ufer auflaufen; an den Felsküsten tiefer und steil.
  Felsküste als Kante (nicht passierbar), Sandstrand als `sand`.
- **Stand 1944:** weglassen, was später kam: der Flugplatz in der Ebene, Hotels,
  Campingplätze, Ferienanlagen, der heutige Jachthafen mit Stegen, moderne
  Straßenführungen. Das Dorf war 1944 deutlich kleiner; Häuser am Hafen und an der
  Hauptstraße behalten, Neubaugebiete am Rand weglassen (Höhengrenze und Lage wie in
  Prompt 112; als Entscheidung festhalten). Felder und Weinberge in der Ebene als
  Feldarten (`field_*`) wie in Prompt 120, Macchia als `tall_grass`.
- **Biom:** `mediterranean` (Prompt 120). Der Importer setzt damit Pinien,
  Steineichen, Olivenhaine, Macchia und die mediterranen Häuser; Trockenmauern und
  Terrassen an den Hängen, wo OSM sie zeigt (`barrier=wall`, `retaining_wall`) oder
  wo Olivenhaine und Weinberge am Hang liegen. Die Kirche des Dorfes mit Campanile.

## 3. Befestigungen

- Küstenbatterien und Stellungen in den Hängen beiderseits des Golfs und am Dorf:
  Orte aus OSM (`military=bunker`, `historic=*`) und Quellen, sonst in sinnvoller
  Lage festgehalten. Platzhalter: `fort_bunker_mg`, `fort_trench_section`,
  `fort_sandbag_position`, Minen- und Panzerhindernisse am Strand
  (`fort_hedgehog`, `fort_mine_*`) dort, wo Quellen sie belegen.
- Die Geschütze der Batterien als deutsche Geschütze aus dem Katalog in
  Stellung (passende Kaliber; schwere Küstengeschütze gibt es nicht, als offener
  Punkt).

## 4. Karte (frei spielbar)

- Zwei Teams: West gegen Ost über den Golf, oder Hinterland gegen Küste. Die Session
  wählt, was nach `just map-reach` und Weglängen ausgeglichener ist, und begründet
  es.
- Versorgungspunkte: Dorf/Hafen (Mitte), Strand, die Höhen über dem Golf, ein Punkt
  in der Ebene, die Straßenkreuzung ins Hinterland.
- Wenn Boote im Gefecht ohne Mission verfügbar sind (Entscheidung in Prompt 119),
  den Strand so anlegen, dass Landungen möglich sind.
- `just map-check`, Tests wie bei `harbor_city`, Messungen, KI-Turnier.

## 5. Mission

- **Seite:** Alliierte (US-Ausrüstung), gegen die KI.
- **Ablauf (Vorschlag):** In der Dämmerung fahren Landungsboote vom Kartenrand in
  den Golf (Prompt 119, Boote mit Ladung über das Missionsskript), die Batterien
  eröffnen das Feuer; erste Welle Infanterie an den Strand, zweite Welle mit Jeeps
  und Geschützen, dritte mit Panzern (LCM). Ziele: Strand halten, die Batterien in
  den Hängen ausschalten (eine nach der anderen, bis dahin beschießen sie die
  späteren Wellen), das Dorf nehmen, die Straße ins Hinterland öffnen.
- Eine Gruppe Kommandos landet vorab an einer Felsbucht seitlich und greift eine
  Batterie von hinten an (historisch landeten Kommandos vor der Hauptlandung an den
  Flanken; nach Quelle prüfen).
- Gegner: Festungstruppen in Stellungen, ein Gegenstoß mit wenigen Fahrzeugen aus
  dem Hinterland.
- Briefing, Meldungen, Ziele auf Deutsch und Englisch, Missionsliste, Durchlauf-Test.
  Schwierigkeit so abstimmen, dass die erste Welle nicht chancenlos ist.

## 6. Stufen

**Stufe 1 – Geodaten, Gelände, Küste, Straßen:** Abfrage, Extrakt, `site.json`,
Höhen, Strand, Felsküste, Meeresboden, Straßen, Stufenbilder, `just map-reach`.

**Stufe 2 – Dorf, Bewuchs, Befestigungen, freie Karte:** Stand 1944, Gebäude,
Felder, Macchia, Wald, Befestigungen, Starts, Versorgungspunkte, Menüname, Tests,
Messungen, KI-Turnier, Doku (`docs/MAPS.md` mit Tabelle „was die Vereinfachung
behielt“, `docs/SOURCES.md`, `docs/PERFORMANCE.md`, `docs/AI.md`).

**Stufe 3 – Mission** (nur mit Prompt 119): Skript, Texte, Test, Doku in
`docs/MISSIONS.md`. Danach `just check`.

Golden Hash: Ein reiner Kartenzusatz ändert ihn nicht.

## 7. Nicht Teil dieses Prompts

- **Französische Nation** und eigene Modelle (Uniformen der Kolonialtruppen,
  Goumiers).
- **Schwere Küstengeschütze** und Schiffsartillerie.
- Die übrigen Landezonen und Portoferraio.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle wie in 081 eintragen
(vorläufig), mit Begründung.
