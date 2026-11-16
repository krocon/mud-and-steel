# Prompt 033 – Flugzeuge und Flugabwehr

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Kampf, Nebel, Basisbau, KI), `docs/SCALE.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Die Flugzeug-Modelle aus 008 (Aufklärer, Jäger, Jagdbomber, Bomber je Fraktion) und die
Flak sind gebaut, im Spiel aber nicht nutzbar. Nach diesem Prompt kann ein Spieler
Luftunterstützung anfordern und sich mit Flak dagegen wehren.

## 2. Leitfrage (zuerst entscheiden)

Steuerbare Flugzeuge wären auf 1:1-Karten mit Zeitfaktor 2 in Sekunden über die Karte
hinweg. Vorschlag: **Luftunterstützung als Abruf** (Aufklärungsflug, Tiefangriff,
Bombenabwurf auf einen Zielpunkt) mit Abklingzeit und Kosten (Ressourcen aus 023),
Flugzeuge fliegen über einen festen Anflugweg vom Kartenrand und wieder hinaus.
Alternative (steuerbare Staffeln) bewerten und begründet ablehnen oder wählen.

## 3. Umfang

1. **Sim:** Lufteinheiten mit Höhe (eigene Ebene, keine Bodenkollision, keine
   Wegfindung), Anflug, Angriff (MG/Kanone als Strafing-Linie, Bomben mit Streuung wie
   032), Abflug. Sichtbarkeit: Flugzeuge sind in Sichtweite jeder Einheit sichtbar
   (große Höhe), Aufklärer decken entlang des Wegs Nebel auf.
2. **Flugabwehr:** Flak und MGs mit Flugabwehr-Eigenschaft (Katalog) schießen auf
   Flugzeuge in Reichweite; Abschuss mit Absturz (Wrack/Explosion am Boden).
3. **Bedienung:** Abruf-Leiste (mit 031), Zielwahl auf der Karte, Anzeige von Anflug
   und Abklingzeit; Warnung „feindliche Flugzeuge“ für den Gegner.
4. **KI:** ruft Unterstützung gegen Ansammlungen und baut Flak, wenn sie angegriffen wird.
5. **Darstellung:** Flugzeug-Modelle mit Schatten am Boden, Propeller, Kondens- und
   Rauchfahne beim Treffer, Bombeneinschläge über Effekte und Krater.

## 4. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Anflug, Bombenwirkung, Flak-Abschuss, Aufklärung,
   gleicher Hash bei zwei Läufen; Golden-Hash bewusst aktualisiert.
2. `just selftest`, `just lan-selftest`, `just ai-tournament` grün.
3. Doku und Katalog-Werte (Geschwindigkeit, Bewaffnung) mit Quellen; Umrechnung in
   `docs/SCALE.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach 031, 032, 038): Die zwölf Flugzeuge (je Fraktion Aufklärer,
Jäger, Schlachtflugzeug, Bomber) und die sechs Flak (`gun_aa_light`, `gun_aa_heavy`)
stehen im Katalog. `UnitType::from_catalog` könnte Flugzeuge zwar umrechnen
(`data/scale.toml` `[aircraft] speed = 1/2`), aber keine Szenario- oder Produktionsliste
nutzt sie. Gezogene Geschütze sind nicht fahrbar (Schleppen kommt mit 034); als
Stellung bauen kann man sie über `[[fortification]]` in `data/economy.toml` (Pak seit
023). Konten gibt es in jeder Partie (auch in der Schlacht ohne Basisbau, dort ohne
Einkommen). Indirektes Feuer (032) hat Einschlag, Splitter, Krater und Zerstörung in
`Sim::explode`; Sichtfelder (018) werden pro Spieler gezählt.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Leitfrage: Luftunterstützung als Abruf**, keine steuerbaren Staffeln. Vier Einsätze: **Aufklärung** (`aircraft_recon`), **Tiefangriff** mit Bordwaffen (`aircraft_fighter`), **Schlachtflug** mit Bomben und Bordwaffen (`aircraft_attack`), **Bombenteppich** (`aircraft_bomber`). Das Flugzeug kommt aus der Fraktion des Spielers. | Auch mit Faktor 1/2 kreuzt ein Jäger eine 2-km-Karte in gut 20 s: Steuern wäre Klick-Hektik ohne Entscheidung. Staffeln bräuchten Flugplätze, Treibstoff, Luftkampf, eigene Wegfindung und eine eigene Auswahl-UI; der Abruf braucht einen Befehl, keine Einheit im Einheitenarray (Sieg, Wegfindung, Trennung, KI-Gruppen, Produktion bleiben unberührt). Historisch wurde Luftunterstützung angefordert, nicht vom Bataillon geflogen. |
| 2 | Neuer Befehl `CallAir { mission, target }` am **Ende** von `Command`. Kosten (Konto aus 023) und Abklingzeit pro Spieler und Einsatz in neuem `data/air.toml` (im Hash, nicht in `SimConfig`, wie 032). Werte **[vorläufig]**: Aufklärung 40/0/10, 60 s; Tiefangriff 60/60/10, 90 s; Schlachtflug 80/100/10, 120 s; Bomber 120/160/20, 180 s (Treibstoff/Munition/Personal, Spielsekunden). | Replays alter Befehle bleiben dekodierbar; ein Ort für Spielregeln; Konto existiert in beiden Modi. |
| 3 | **Flugweg:** gerade Linie durch den Zielpunkt, Richtung vom eigenen Startschwerpunkt zum Ziel; Einflug 300 m hinter dem Kartenrand, Ausflug ebenso auf der anderen Seite. Mehrere Flugzeuge versetzt (seitlich/hintereinander je Einsatz). Keine Wegfindung, keine Bodenkollision. | „Fester Anflugweg vom Kartenrand und wieder hinaus“; deterministisch, ohne Kurvenflug. Anflug über eigene Linien wirkt plausibel. |
| 4 | **Geschwindigkeit** aus dem Katalog (Reise-, sonst Höchstgeschwindigkeit) × 1/2 (`scale.toml`, bestätigt). **Höhe** als Spielregel in Spielmetern: Reise 200–500 m, im Angriff 30 m (Tiefangriff), 150 m (Schlachtflug), 500 m (Bomber), Übergang linear über 400–500 m Strecke. **[vorläufig]** | Echte Höhen (3–4 km) lägen weit über jedem Kamerabild und jeder (1/4-)Flakreichweite; die Höhe ist hier sichtbar und für Flak-Reichweite und Mindesthöhe der schweren Flak wirksam. |
| 5 | **Bordwaffen (Strafing):** von 300 m vor bis 100 m hinter dem Ziel alle 10 Ticks ein Feuerstoß; die Einschläge liegen in einem 10 m breiten Streifen 150 m vor dem Flugzeug, so lang wie die Flugstrecke des Stoßes. Schüsse pro Stoß = Läufe × Kadenz (real, Flugzeuge ohne Zeitfaktor) × Dauer. Soldaten im Streifen: 25 % × Haltungshöhe × Deckung, Schaden wie ein Treffer des Kalibers (`combat.toml`). Fahrzeuge: je Schuss 10 %; Durchschlag des Katalog-Kalibers (Formel aus 017, ohne Entfernungsverlust) gegen Dach = 50 % der dünnsten Platte; offene Fahrzeuge (≤ 10 mm) nehmen 3 HP pro mm Kaliber. **[vorläufig]** | Ein Treffertest pro Geschoss im Flug (Projektilsystem) bräuchte einen Schützen im Einheitenarray; die Streifenregel ist billig und gibt die bekannte Wirkung (Infanterie und Weichziele stark, Panzer kaum). |
| 6 | **Bomben:** neues optionales Waffenfeld `bomb_kg` (Nennmasse aus dem Namen/der Ladung, Quelle des Flugzeugs). Abwurf so, dass die Fallzeit `sqrt(2h/g)` mit der Fluggeschwindigkeit das Ziel trifft; jedes Flugzeug wirft einen Reihenwurf (15 m Abstand). Streuung dreieckig 10 m (Schlachtflug) bzw. 25 m (Bomber). Einschlag über `explode` aus 032 (`Sim::detonate`) mit **Ersatzkaliber 25 mm × ∛kg** (SC 250 ≈ 155 mm, Splitterradius 31 m; SC 500 ≈ 198 mm, 40 m); Krater, Zerstörung, Splitter, Volltreffer wie Granaten. Zusätzlich **Nahtreffer**: gepanzerte Fahrzeuge innerhalb 1,2 m × ∛kg nehmen bis 600 HP (linear fallend). **[vorläufig]** | Eine Wirkungsregel für alles, was explodiert; Katalog bleibt bei echten Werten (Bombenmasse). |
| 7 | **Flugabwehr:** Flak (`gun_aa_*`) mit neuem Waffenfeld `range_aa_m` (wirksame Flak-Reichweite/Gipfelhöhe aus den Katalognotizen) × `range.direct_fire` (1/4), gedeckelt je Klasse: Flak 38 550 m, 61-K 750 m, Bofors 875 m, schwere Flak (88/52-K/90 mm) 1500 m (statt 2000–3325 m) **[vorläufig]**. MGs mit Fliegerlafette tragen `aa = true` (Katalogname/Notiz nennt „AA“: M2HB auf US-Panzern/Fahrzeugen, DShK auf IS-2/ISU-152) und reichen 1000 m real (250 m). Ein Schuss pro Lauf und Nachladezeit (Kadenz wie 017, automatische Waffen 25 %); Trefferchance je Klasse (MG 3 %, leicht 3 %, schwer 4 %) × (1 − d / 2R) × Spannweite/12 m; Schaden 10 HP pro mm Kaliber (2 cm 200, 3,7 cm 370, 8,8 cm 880). Langsame Flugzeuge bleiben länger in Reichweite und werden so öfter getroffen (kein eigener Geschwindigkeitsfaktor: der hätte das doppelt gezählt). Schwere Flak schießt nicht unter 100 m Höhe. Flugzeug-HP je Kategorie: Aufklärer 300, Jäger 600, Schlachtflugzeug 1000, Bomber 1500. **[vorläufig]** | Ein Abschuss braucht Wahrscheinlichkeiten, keine Kugelbahnen durch den Himmel; die Formel nutzt nur Katalogwerte (Kaliber, Kadenz, Spannweite). Schwere Flak war gegen schnelle Tiefflieger wirkungslos (Richtgeschwindigkeit). Messung: vier 61-K holen zwei Ju 87 pro Anflug herunter (3 Seeds: 6 von 6). |
| 8 | **Abschuss:** das Flugzeug fliegt 6 s brennend weiter und sinkt, Aufschlag als Explosion (Ersatzkaliber 100 mm, Krater) und Ereignis `Crash` (Wrack und Feuer in Godot). Außerhalb der Karte verschwindet es. | „Absturz mit Wrack/Explosion am Boden“; das Wrack ist Darstellung, blockiert nichts. |
| 9 | **Flak bauen:** leichte Flak als Stellung (`[[fortification]]`, braucht Kaserne), schwere Flak braucht das Fahrzeugwerk. Flak-Stellungen schießen auch auf Bodenziele (ihre Katalogwaffe, wie die Pak). | Gezogene Geschütze sind bis 034 nur als Stellung nutzbar; so wie die Pak. |
| 10 | **Sichtbarkeit:** Ein Flugzeug sieht ein Spieler, wenn es innerhalb der Sichtweite (horizontal) einer eigenen lebenden Einheit ist; eigene immer. **Aufklärer** decken entlang des Wegs auf: ein Sichtfeld aus Flughöhe (Radius 2000 m real × 1/4 = 500 m, neue Strahltabelle) zählt wie ein Einheiten-Sichtfeld in der Spielersicht und ist im Hash. | „In Sichtweite jeder Einheit sichtbar (große Höhe)“; dasselbe Strahlverfahren wie 018, Wald und Hügel verdecken weiter. |
| 11 | **Zustand im Snapshot (038):** `Air` (Regeln, Typen, Abklingzeiten, Einsätze, Flugzeuge, Bomben, Flak-Nachladezeiten, Aufklärer-Sichtfelder) leitet `Serialize/Deserialize` ab und hängt an `Sim`; Test: Speichern/Laden mitten im Einsatz, beide Läufe gleicher Hash. | Vorgabe; neuer Zustand ist automatisch im Snapshot, sobald er im `Sim` steckt. |
| 12 | **KI (Normal, Hard):** ruft höchstens alle 400 Ticks Bomber (≥ 5 Einheiten), Schlachtflug oder Jäger auf die größte gesichtete Gruppe (≥ 3 Einheiten, keine eigene Einheit im Umkreis 150 m: die Flugzeuge brauchen 10–30 s, bis dahin sind die Linien sonst ineinander), Aufklärung auf die zuletzt gesehene Feindposition, wenn 60 s kein Feind zu sehen war; im Basisbau nur mit doppeltem Preis auf dem Konto. Sieht sie feindliche Flugzeuge, baut sie eine leichte Flak (Basisbau). | „Normaler Spieler, nur Befehle“; Ansammlungen wie bei der Artillerie (032). Mit 80 m Abstand rief die KI in der Demo schon in Tick 0 Bomber auf die 120 m entfernte Front und bombte sich selbst. |
| 13 | **Bedienung:** Abruf-Leiste im HUD über der Befehlsleiste (vier Knöpfe mit Kosten, Abklingzeit, Taste Shift+1…4), Zielwahl per Linksklick, Anzeige des eigenen Anflugs (Linie Einflug → Ziel, Restzeit) und Warnung „Enemy aircraft“ im Ereignis-Feed. Darstellung: Katalog-Modelle aus 008, Schatten am Boden (Projektion), drehender Propeller (Knoten `Propeller*`, sonst Scheibe), Rauchfahne ab 50 % Schaden, Flak-Wölkchen/Leuchtspur, fallende Bomben, Einschläge und Krater über die vorhandenen `hit`-Ereignisse. | Godot zeichnet nur; vorhandene Effekte und Töne werden wiederverwendet. |
| 14 | Kein Luftkampf, keine Abfangjäger, keine Flugplätze, keine Raketen (HVAR/RS-82). | Umfang; als „offen“ dokumentiert. |

## Ergebnis

Umgesetzt wie oben; Details in `docs/ARCHITECTURE.md` „Air support and anti-aircraft
fire“, `docs/SCALE.md` (Flugzeuge), `docs/GAME_DESIGN.md` (Preise, Flak-Stellungen),
`docs/AI.md` (Luftunterstützung), `docs/CONTROLS.md` (Abruf-Leiste). Regeln in
`data/air.toml`, Katalogfelder `range_aa_m`, `aa`, `bomb_kg` (`data/catalog/SCHEMA.md`).

Tests: `crates/sim/tests/air.rs` (Anflug vom Kartenrand über das Ziel und hinaus,
Kosten und Abklingzeit, Bomben auf Infanterie und LKW mit Kratern, Flak-Abschuss mit
Absturz, Feuerhalt der Flak, Aufklärung deckt verdeckten LKW auf und wieder zu,
Sichtbarkeit feindlicher Flugzeuge, gleicher Hash bei zwei Läufen), Einheitentests in
`crates/sim/src/air.rs`, `crates/net/tests/air_snapshot.rs` (Speichern während eines
Luftangriffs, 3 Zeitpunkte, Bomben im Fall), `crates/ai/tests/air.rs`, Godot-Selbsttest
`scenes/tools/air_self_test.gd` (Bomber über die Abruf-Leiste mit Shift+4 und Klick).
Golden-Hashes (Demo, Basisbau) bewusst aktualisiert: Luft-Regeln und -Zustand im Hash,
zwei neue Stellungstypen; die Demo-Schlacht ist unverändert (Tick 480, Spieler 0 gewinnt).

Offen: Darstellung nur headless geprüft (Modelle, Schatten, Propeller, Rauch nicht
visuell begutachtet); kein Triebwerksgeräusch; kein Luftkampf, keine Raketen; alle Werte
in `data/air.toml` und die Tastenbelegung Shift+1…4 sind vorläufig; Anflugrichtung folgt
dem eigenen Start; die KI nutzt keine schwere Flak.
