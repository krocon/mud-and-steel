# Prompt 110 – Geschütze und stehende Panzer reagieren auf Beschuss von der Seite

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, `docs/AI.md` ganz (Rolle `Gun`, Flankensuche des
> Angreifers, Abschnitt zu Geschützen und Zugmaschinen), `docs/BALANCE.md`,
> `prompts/done/017-*.md` (Panzerseiten), `prompts/done/051-*.md` (KI sucht die
> Flanke), `crates/sim/src/sim/combat.rs` (`turn_to` um Zeile 997–1017: dreht nur
> zu einem Ziel, das die Einheit schon sieht; Trefferseite um 1560–1610),
> `crates/sim/src/tactics.rs`, `data/tactics.toml`, `data/vision.toml`,
> `crates/ai/src/commander.rs`, `crates/ai/src/lib.rs` (Rolle `Gun`) und
> `crates/ai/src/transport.rs`. Wenn im Arbeitsverzeichnis fremde, nicht
> eingecheckte Änderungen an `crates/sim` liegen, erst den Nutzer fragen.

## 1. Ausgangslage

Im Test zum Referenzvideo (`docs/.vorlagen/vorlage.mp4`, nur als Anregung) nennt
der Tester eine deutliche Schwäche: Greift man eine Stellung von der Seite an,
**dreht der Gegner seine Geschütze nicht** und lässt eingegrabene Panzer einfach
abschießen. Zusammen mit zu billigem Nachschub macht das den Panzersturm zur
besten Taktik.

Bei uns heute:

- `turn_to` dreht Turm bzw. bei Geschützen und Kasemattfahrzeugen die Wanne nur zu
  einem Ziel, das die Einheit **sieht**. Wird sie von einer Seite getroffen, auf
  der sie nichts sieht, passiert nichts. Es gibt kein Gedächtnis für die
  Trefferrichtung.
- Die KI sucht als Angreifer selbst die Flanke (Prompt 051), verteidigt die eigene
  aber nicht: Geschütze sind nur für das Ziehen mit Zugmaschinen eingeplant.

Das betrifft **alle** stehenden Einheiten, auch die des Spielers. Deshalb gehört
die Grundreaktion in die Simulation (gilt für jeden gleich), die Entscheidung
„Stellung wechseln“ in die KI.

## 2. Ziel

### 2.1 Grundreaktion jeder Einheit (Simulation)

- Wird eine stehende Einheit ohne eigenes Ziel getroffen oder knapp verfehlt,
  merkt sie sich die **Richtung des Beschusses** (aus der getroffenen Seite bzw.
  der Schussrichtung, die die Trefferrechnung schon kennt) für kurze Zeit und
  dreht sich dorthin: Panzer den Turm, Geschütze und Fahrzeuge mit begrenztem
  Schwenkbereich die Lafette bzw. Wanne.
- Sie sieht den Schützen dadurch **nicht automatisch**. Die Sichtregeln bleiben;
  die Einheit schaut nur in die richtige Richtung und kann feuern, sobald sie ihn
  nach den normalen Regeln sieht.
- Das Drehen kostet Zeit nach den vorhandenen Dreh- und Richtgeschwindigkeiten
  (ein Pak-Geschütz herumzuschwenken dauert). Niedergehaltene Bedienungen drehen
  langsamer oder gar nicht (Prompt 047).
- **Befehle des Spielers gehen vor:** Feuerverbot, ein ausdrücklicher
  Angriffsbefehl oder eine befohlene Blickrichtung werden nicht überstimmt.
  Festlegen, wie lange die befohlene Richtung „hält“, und dokumentieren.
- Bei Beschuss aus mehreren Richtungen: die gefährlichere (durchschlagender
  Treffer vor Abpraller, näher vor ferner). Kein Hin-und-her-Zucken: Mindestzeit,
  bevor die Richtung wieder wechselt.

### 2.2 KI

- Geschütze und stehende Panzer der KI, die von der Seite oder von hinten
  beschossen werden und sich nicht wehren können (Gegner außerhalb des
  Schwenkbereichs, nicht sichtbar, oder die Stellung ist verloren): Stellung
  wechseln, zurückziehen oder Nebel werfen, soweit die Einheit das kann.
  Geschütze nur, wenn eine Zugmaschine da ist; sonst Bedienung in Deckung.
- Die KI stellt Geschütze so auf, dass sie sich gegenseitig die Flanke decken,
  wenn das mit kleinem Aufwand geht; sonst als Befund für später.
- Nicht übertreiben: Die KI soll nicht bei jedem Schuss alles umgruppieren.

## 3. Regeln

- Regeländerung in `crates/sim`: feste Kommazahlen, kein versteckter Zufall, keine
  Iteration über `HashMap`. Der Golden Hash ändert sich **beabsichtigt**; in der
  Commit-Nachricht sagen.
- Werte (Merkdauer, Mindestzeit, Schwellen) in `data/tactics.toml`.
- Die KI bleibt ein normaler Spieler und gibt nur `Command`s.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Bestand lesen. `just ai-tournament` als Vorher-Stand, dazu ein kleines
   Prüfszenario: drei Geschütze in Stellung, Angriff frontal und von der Seite;
   Verluste und Dauer messen.
2. Grundreaktion in der Simulation mit Tests: Treffer von links → Einheit steht
   nach der erwarteten Zeit nach links; mit Feuerverbot dreht sie, feuert aber
   nicht; befohlene Richtung bleibt; Determinismus.
3. KI-Reaktion mit Tests.
4. Prüfszenario und Turnier nachher. Erwartung: Der Flankenangriff bleibt klar
   besser als der frontale, aber er ist kein Freischuss mehr. Zahlen nennen.
5. `just check`, `just selftest`, `just lan-selftest`, `just bench`.
6. Doku: `docs/AI.md`, `docs/BALANCE.md`, `docs/GAME_DESIGN.md` (Entscheidung mit
   Datum, vorläufig bis zur Bestätigung durch den Nutzer).
7. Für den Nutzer: Vorher/Nachher-Zahlen und ein Clip des Prüfszenarios.

## 5. Nicht Teil dieses Prompts

- Moral und Rückzug der Infanterie (`docs/INFANTRY.md`, offen).
- Eine Bremse für den Panzersturm über Sprit und Versorgung (Lkw mit eigenem
  Vorrat, `docs/GAME_DESIGN.md` „Open“). Das ist die zweite Hälfte der Kritik des
  Testers und verdient einen eigenen Prompt nach Prompt 097, wenn Tempo und
  Reichweite neu stehen.
