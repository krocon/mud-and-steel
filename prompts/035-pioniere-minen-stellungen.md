# Prompt 035 – Pioniere, Minen, Hindernisse und Stellungen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Kartenobjekte, Zerstörung, Krater, Basisbau, Infanterie),
> `docs/MAPS.md`, `docs/GAME_DESIGN.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Minen, Stacheldraht, Panzersperren und Gräben gibt es als Modelle (007) und als
Kartenobjekte, aber niemand kann sie im Spiel anlegen oder räumen. Nach diesem Prompt
legen Pioniere Minenfelder und Hindernisse, heben Schützenlöcher und Gräben aus,
räumen Minen und reparieren oder sprengen Brücken.

## 2. Umfang

1. **Minen in der Sim:** Panzer- und Schützenminen als verdeckte Objekte (nur für den
   Besitzer sichtbar, für den Gegner erst nach Entdeckung durch Pioniere oder
   Auslösung), Auslösung nach Masse, Schaden an Ketten (Fahrzeug bewegungsunfähig) oder
   Infanterie. Im State-Hash, aber nicht im `PlayerView` des Gegners.
2. **Pionierbefehle:** `Lay` (Mine, Draht, Panzersperre an Position/Linie), `Clear`,
   `Dig` (Schützenloch, Graben: Deckung für 022, über Höhenfeld aus 021 oder als
   Objekt), `Repair`/`Demolish` (Brücken aus 014). Bauzeiten und Kosten (023).
   Mauern und Minen als baubare Objekte stehen in `GAME_DESIGN.md` unter „Open“.
3. **Wegfindung:** bekannte Minenfelder meiden (pro Spieler!), ohne das gemeinsame
   `PathGrid` spielerabhängig zu machen; Lösung entscheiden und begründen.
4. **KI:** legt Minen an Engstellen, räumt sie mit Pionieren.
5. **Darstellung:** Minenmarker nur für den Besitzer/Entdecker, Grabungseffekt, Kette
   abgesprungen am Fahrzeug.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests: Mine löst nur nach Masse aus, Gegner sieht sie nicht
   vorher, Räumen entfernt sie, Graben gibt Deckung, Brücke repariert/gesprengt
   ändert Wege; gleicher Hash; Golden-Hash bewusst aktualisiert.
2. `just selftest`, `just lan-selftest` (Minen dürfen keinen Desync auslösen),
   `just editor-selftest` (Minenfelder im Editor platzierbar) grün.
3. Doku aktualisiert, `GAME_DESIGN.md` „Open“ bereinigt.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach 029–034, 037, 038, 041): Stacheldraht, Igel, Gräben,
Sandsack-Stellungen und Bunker gibt es nur als statische Kartenobjekte (`data/objects.toml`,
Teil des Karten-Hashs, Zerstörung/Deckung/Feuer hängen am Objektindex). Die Karte ist
unveränderlich bis auf Krater (Höhen-Delta-Raster aus 021, copy-on-write) und den
Intakt-Zustand von Brücken (`Sim::set_bridge_intact`, bisher nur von Tests benutzt;
Brücken sind in keiner Zerstörungsklasse). Krater geben Infanterie Deckung
(`crater_catches`, `crater_spots`) und Godot aktualisiert das Gelände über
Krater-Ereignisse. Pioniertrupps (`inf_engineer`) stehen für alle drei Fraktionen im
Katalog (mit Modell), werden aber nirgends produziert. Minenfelder stehen als
Fraktionseinträge `*_minefield_at/ap` (20 × 20 m, Minenzahl als `weapon.count`) und
Einzelminen `fort_mine_at/ap` im Katalog, ohne Spielfunktion. Es gibt keine
Bewegungsunfähigkeit von Fahrzeugen. Der Wegfinder hat ein gemeinsames `PathGrid` mit
Blockier-Flags pro Zelle (zwei Bits frei). 038 speichert den ganzen `Sim` per serde,
Kartenänderungen als `MapDelta` (Höhen, Geländearten, Brücken). `PlayerView` (024) baut
nur auf `Sim::snapshot_for` auf; Teams (037) teilen Sicht.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Eigene Sim-Schicht **„Feldbefestigungen“** (`crate::works` mit Regeln in neuem `data/works.toml`, Zustand in `crate::sim::works`), **keine** neuen Kartenobjekte zur Laufzeit. | Kartenobjekte sind Teil des Karten-Hashs und über ihren Index mit Zerstörung, Deckung und Feuer verknüpft; Laufzeit-Objekte hätten all das umgebaut. Wie 034 (`data/transport.toml`): kein neues Config-/Replay-Format, im Hash, im Spielstand. |
| 2 | **Befehle** am Enum-Ende: `Lay { units, kind, from, to }` (Panzerminen, Schützenminen, Draht, Panzersperren), `Dig { units, kind, from, to }` (Schützenloch, Graben), `Clear { units, target }`, `Demolish { units, bridge }`, `Repair { units, bridge }`. `from == to` ist ein Punkt, sonst eine Linie. Nur Pioniertrupps (`inf_engineer`) führen sie aus. | Entspricht den Pionierbefehlen des Prompts; Linien sind schon da, damit 052 „Sperren als Linie ziehen“ nur erweitern muss. |
| 3 | Eine Linie wird in **Stücke** zerlegt: Draht und Graben je ≤ 4 m (eine Wegzelle), Panzersperren (Igel) alle 3 m, Minen in Abschnitten von 20 × 20 m entlang der Linie (Katalogmaß des Fraktions-Minenfelds). Jedes Stück ist eine Baustelle mit eigener Bauzeit; mehrere Trupps arbeiten zusammen schneller. | Stückweise Zerstörung (Panzer walzen Draht platt), stückweises Räumen und einfache Darstellung (ein Modell pro Stück). 052 kann Arten (Fahrzeugstellung, Tarnnetz) an `WorkKind` anhängen. |
| 4 | **Wegfindung:** zwei neue Flags im `PathGrid` (bisher freie Bits): Draht sperrt kleine Einheiten (wie `fort_barbed_wire`), Igel sperren große. Kettenfahrzeuge und schwere LKW walzen Draht beim Überfahren platt (wie die Zerstörungsregel für `fort_barbed_wire`). | Gleiches Verhalten wie die Kartenobjekte aus 007/020; additive Änderung in `path.rs`. |
| 5 | **Bekannte Minen meiden pro Spieler ohne spielerabhängiges Grid:** jede Wegsuche bekommt eine sortierte Liste „gemiedener“ Zellen (bekannte feindliche Minen des Teams des Auftraggebers); A* rechnet dort das Zehnfache der Kosten (nicht gesperrt, damit Einheiten aus einem Feld herausfinden), die Glättung kürzt nicht über solche Zellen ab. Wird ein Feld neu bekannt, planen Einheiten dieses Teams, deren Route es berührt, neu. **[vorläufig: Faktor 10]** | Das gemeinsame Grid und seine Komponenten bleiben unverändert; die Liste ist klein, deterministisch und reist im gespeicherten Suchzustand mit. |
| 6 | **Minen:** einzelne Minen mit Position, Besitzer (keiner = Kartenminen aus dem Editor) und „bekannt für“-Maske. Panzermine löst bei Masse ≥ 150 kg (Fahrzeug, dessen Wanne die Mine überdeckt) aus, Schützenmine bei ≥ 5 kg (Soldat innerhalb 0,5 m oder Fahrzeugwanne). Minen lösen unter Einheiten des eigenen Teams nicht aus. **[vorläufig]** | „Auslösung nach Masse“; Auslösegewichte in der Größenordnung von Tellermine (≈ 90–180 kg) und S-Mine (≈ 3–7 kg). Eigene Minen nicht auszulösen erspart Gassen-Logik (Pioniere markieren Gassen). |
| 7 | **Wirkung:** Panzermine: leicht gepanzerte Fahrzeuge (≤ `open_armor_mm` aus 032) sind zerstört, sonst 30 % der Trefferpunkte Schaden und **Kette ab**: Fahrzeug bewegungsunfähig (Tempo 0, Befehle zum Fahren werden ignoriert, Turm und Waffen arbeiten), die Besatzung montiert die Kette nach 90 s Spielzeit wieder. Schützenmine: der auslösende Soldat fällt, dazu Splitter über die Detonationsregeln aus 032 (Kaliber 60 mm). Krater nach den Regeln aus 021. **[vorläufig]** | „Schaden an Ketten (Fahrzeug bewegungsunfähig) oder Infanterie“; Wiederherstellung durch die Besatzung, weil Instandsetzung (036) parallel entsteht. |
| 8 | **Sichtbarkeit:** Minen sind keine Einheiten und erscheinen nie in `snapshot_for`. Neue Abfrage `Sim::mines_for(player)` liefert nur Minen, die das Team des Spielers kennt (eigene/verbündete, entdeckte); `PlayerView` bekommt daraus `mines`. Entdeckung: Pioniertrupps erkennen feindliche Minen im Umkreis von 10 m, eine Explosion verrät dem Opfer die Minen im Umkreis von 15 m. Wissen gilt fürs ganze Team. Im State-Hash ist alles. | „Im State-Hash, aber nicht im `PlayerView` des Gegners“, Teams aus 037. |
| 9 | **Graben über das Höhenfeld aus 021:** Schützenloch (eine Stützstelle) und Graben (Stützstellen entlang der Linie) werden in vier Schritten auf 0,9 m vertieft, daneben eine Brustwehr von 0,3 m; gleiche Maschinerie wie Krater (Delta-Raster, Wegzellen, Sicht, Einheitenhöhen, `MapDelta` im Spielstand, Godot-Gelände über Krater-Ereignisse mit Kaliber 0). Gegrabene Stützstellen geben stehenden Trupps Deckung wie ein Krater, aber mit eigenen Werten (Graben 60/75/85 % stehend/kniend/liegend). **[vorläufig]** | „Über Höhenfeld aus 021 oder als Objekt“: das Höhenfeld liefert Deckung, Sicht und Darstellung ohne neue Objektarten; 052 macht die eigentliche Grabenoptik. 0,9 m + 0,3 m bleibt auf 2-m-Raster unter der Steigungsgrenze von 60 %, Infanterie kann hinein. |
| 10 | **Brücken:** `Demolish` (Sprengladungen, 30 s) zerstört eine intakte Brücke über `set_bridge_intact` mit einer Explosion, `Repair` (90 s) stellt eine zerstörte wieder her; Pioniere arbeiten am nächsten Brückenende. Godot blendet zerstörte Brücken aus. **[vorläufig]** | Brücken aus 014; mehr (Behelfsbrücken, Pontons) kommt mit 055. |
| 11 | **Kosten und Zeiten** in `data/works.toml` (Spielregeln, Spielsekunden wie in 023): Panzerminenabschnitt 60 Munition/40 s, Schützenminen 40/30 s, Draht 6 Personal/6 s je Stück, Igel 10 Personal/10 s, Schützenloch 15 s, Graben 20 s je Stück, Räumen 3 s je Mine, 5 s je Draht, 8 s je Igel, Sprengen 40 Munition, Reparieren 40 Treibstoff + 80 Personal. Bezahlt wird bei der Anlage der Baustelle; verlassene, nicht begonnene Baustellen werden erstattet und entfernt. Höchstens 24 offene Baustellen pro Spieler. **[vorläufig]** | Kosten über das Konto aus 023 (auch ohne Basisbau vorhanden). |
| 12 | **Editor:** neue Objektarten `fort_mine_at`/`fort_mine_ap` (Variante `Field`) in `data/objects.toml`, im Editor platzierbar wie andere Objekte; beim Spielstart wird jedes zu einem verdeckten Abschnitt **neutraler** Minen (20 × 20 m, Ausrichtung des Objekts, 20 bzw. 40 Minen, Lage aus einem Hash der Position, nicht aus dem Sim-RNG). Im Spiel zeichnet Godot sie nicht. **[vorläufig: neutral statt einem Spieler zugeordnet]** | „Minenfelder im Editor platzierbar“; Kartenobjekte haben keinen Besitzer. |
| 13 | **Produktion:** Pioniertrupps der drei Fraktionen kommen ans Ende der Infanterie-Liste der Kaserne (`data/economy.toml`). | Ohne Produktion gäbe es im Basisbau keine Pioniere; Listenende lässt die KI-Mischungen (Indizes) unverändert. |
| 14 | **KI** (Normal, Hard): baut einen Pioniertrupp; legt Panzerminen an der eigenen Seite der Brücke, die der Linie zum Gegner am nächsten liegt (sonst auf 30 % der Strecke dorthin), höchstens zwei Abschnitte; räumt bekannte feindliche Minen in der Nähe der eigenen Armee. Der Pioniertrupp gehört nicht zur Infanteriegruppe. **[vorläufig]** | „Legt Minen an Engstellen, räumt sie mit Pionieren“, nur über normale Befehle und den `PlayerView`. |
| 15 | **Darstellung:** Minenmarker (eigene/verbündete gelb, entdeckte feindliche rot) nur für Spieler, die sie kennen; fertiger Draht und Igel als Modelle aus 007 für alle sichtbar (wie Kartenobjekte), Baustellen halbtransparent nur für den Besitzer; Staub an Grabungsstellen; abgesprungene Kette als Kettenstück neben dem Fahrzeug. Befehlsleiste: Schaltflächen für die Pionierbefehle (aktiv mit Pionieren), Linien mit zwei Klicks. | Wie gefordert; keine neuen Modelle nötig. |
