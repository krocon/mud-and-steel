# Prompt 177 – Geschützbedienung als verwundbare Mannschaft, Schutzschild, Gegenmittel gegen schwere Pak und Flak

> In einer frischen Session im Repo-Ordner ausführen. **Braucht 110**, empfohlen
> nach 176. Lies zuerst `CLAUDE.md`, `docs/DETERMINISM.md`, `docs/SOURCES.md`,
> `docs/SCALE.md`, in `docs/ARCHITECTURE.md` „Combat“ (Schadensmodell, „Open“:
> „the gun crew is not modelled“), „Transport and towing“, „Wounds and care“
> (ausgestiegene Besatzungen), „Indirect fire and smoke“ (Splitter), „Air support
> and anti-aircraft fire“, dazu `docs/INFANTRY.md`, `docs/BALANCE.md`,
> `data/catalog/SCHEMA.md`, die Geschütze `gun_*` in `data/catalog/*.toml`,
> `data/combat.toml`, `crates/sim/src/sim/combat.rs`, `crates/sim/src/sim/care.rs`
> und `crates/sim/src/sim/infantry.rs`. Fremde, nicht eingecheckte Änderungen an
> `crates/sim`: erst den Nutzer fragen.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) nennt zwei verwandte Schwächen: Infanterie stirbt sehr schnell,
während die Bedienung eines Geschützes unter direktem Beschuss unplausibel lange
überlebt; und einzelne schwere Pak bzw. Flakstellungen entwerten ganze taktische
Optionen, weil es kaum Gegenmittel gibt.

Bei uns sind Geschütze Fahrzeuge mit Lebenspunkten. Die Bedienung ist **nicht
modelliert** (`docs/ARCHITECTURE.md`, „Transport and towing → Open“): ein MG kann
eine Pak gar nicht beschädigen, Splitter wirken auf sie wie auf ein Fahrzeug,
Niederhalten (Prompt 047) betrifft sie nicht. Damit ist eine schwere Flak im
Erdkampf nur mit Panzergranaten oder Artillerie zu knacken, und Infanterie hat
keine Antwort.

## 2. Ziel

### 2.1 Bedienung als Zahl von Männern (Simulation)

- Jedes Geschütz (`gun_*`, auch Flak) hat eine **Bedienung** mit echter Stärke aus
  dem Katalog (neues Feld mit Quelle, z. B. Pak 40, Flak 36, 6-pdr, ZiS-3: Stärke
  der Bedienung nach Quelle). Die Männer werden **nicht** als eigene Soldaten
  simuliert, sondern als Zähler mit eigener Trefferfläche hinter dem Geschütz.
- Kleinkaliber und Splitter treffen die Bedienung, nicht das Geschütz; der
  **Schutzschild** (Katalogfeld ja/nein, mit Quelle) deckt sie von vorn innerhalb
  eines Winkels, nicht von der Seite und nicht gegen Steilfeuer.
- Weniger Männer → langsameres Laden und Richten (Tabelle in `data/combat.toml`);
  ohne Bedienung feuert das Geschütz nicht und ist ein **verlassenes Geschütz**.
- Niederhalten wirkt auf die Bedienung wie auf einen Trupp (Werte aus Prompt 047
  wiederverwenden).
- Panzergranaten und Sprenggranaten auf das Geschütz selbst wirken wie bisher auf
  dessen Lebenspunkte.

### 2.2 Verlassen, wiederbesetzen

- Verlassene Geschütze können von einem eigenen Infanterietrupp **wiederbesetzt**
  werden (Rechtsklick wie „Gebäude besetzen“); ob auch vom Gegner
  (Beutegeschütz), entscheidet der Nutzer: als Frage mit Empfehlung im Ergebnis
  festhalten, Standard vorläufig nur eigene.
- Fliehende Bedienung bei Totalverlust ist optional; wenn, dann über die
  vorhandenen ausgestiegenen Besatzungen (Prompt 049).

### 2.3 Gegenmittel sichtbar

- Prüfen, dass jede schwere Waffe mindestens zwei erkennbare Gegenmittel hat:
  Flanke (Prompt 110), Infanterie mit MG, Mörser/Artillerie, Rauch, Luftangriff
  außerhalb der Flak-Reichweite. Mit dem Gefechtssimulator aus Prompt 045 messen
  (Pak 40, Flak 36 im Erdkampf, 17-pdr, falls 117 erledigt ist).
- Flak gegen Luftunterstützung: prüfen, ob eine einzelne Flakstellung den Abruf
  fast vollständig verhindert. Wenn ja, Abrufe nicht pauschal schwächen, sondern
  Gegenmittel sichtbar machen (Flakstellung auf der Karte nach Aufklärung
  markiert, Hinweis beim Abruf „Flak im Zielgebiet bekannt“).

## 3. Regeln

- Katalog: nur echte Werte mit Quelle (`docs/SOURCES.md`), keine Daten aus anderen
  Spielen. Umrechnung nur über `data/scale.toml`. Danach `just catalog-doc` und
  `just units-web-sync`.
- Simulation: feste Kommazahlen, kein Zufall außer über `sim::Rng`, keine
  `HashMap`-Iteration. Golden Hash ändert sich beabsichtigt (Commit-Nachricht).
- Darstellung: Bedienungsstärke im Infofeld und auf der Einheitenkarte; die Männer
  am Modell nur, wenn die vorhandenen Geschützmodelle Bedienungsfiguren haben
  (sonst als Befund für einen Asset-Prompt).
- Die KI bekommt keinen Sonderweg: sie greift Geschütze mit Infanterie und
  Mörsern an, wenn die Zielwahl aus 176 das ergibt; Turnier vorher/nachher.

## 4. Vorgehen

1. Bestand lesen, Gefechtssimulator: Infanterie gegen Pak frontal/seitlich,
   Flak 36 gegen Panzer, MG gegen Pak. Vorher-Zahlen.
2. Katalogfelder mit Quellen, Simulation mit Tests (Schild vorn wirkt, von der
   Seite nicht; Bedienung null → kein Feuer; Wiederbesetzen).
3. Darstellung und Bedienung.
4. Gegenmittel messen, Abruf-Hinweis.
5. `just check`, `just selftest`, `just lan-selftest`, `just ai-tournament`,
   `just bench`.
6. Doku: `docs/ARCHITECTURE.md`, `docs/GAME_DESIGN.md` (vorläufig, Datum),
   `docs/BALANCE.md`, `docs/CONTROLS.md`, `data/catalog/SCHEMA.md`.
7. Für den Nutzer: Zahlen vorher/nachher und die offene Frage zu Beutegeschützen.
