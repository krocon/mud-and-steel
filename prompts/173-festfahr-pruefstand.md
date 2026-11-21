# Prompt 173 – Festfahr-Prüfstand: Einheiten kommen an, wo sie hinsollen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, in `docs/ARCHITECTURE.md` die Abschnitte „Pathfinding“
> (Weggitter, Suche, Gruppen, „Movement and separation“, Regeln gegen Festfahren,
> „Lead-in“, „Changing obstacles“, „Open“), „Bridges“ und „Transport and towing“,
> dazu `docs/PERFORMANCE.md`, `crates/sim/src/path.rs`, `crates/sim/src/sim.rs`
> (Bewegung), `crates/cli/src/bench.rs` (`stuck_spawns`) und `docs/AI.md`
> („Tournament“). Wenn im Arbeitsverzeichnis fremde, nicht eingecheckte
> Änderungen an `crates/sim` liegen, erst den Nutzer fragen.

## 1. Ausgangslage

Quelle: Auswertung von Tests und Spielermeinungen zu einem anderen
WW2-Echtzeit-Taktikspiel (Nutzer-Datei, 2026-11-21; nur Anregung, nichts wird
übernommen). Der mit Abstand häufigste Kritikpunkt dort: Einheiten bleiben an
Bäumen, Flüssen und Brücken hängen, blockieren sich gegenseitig oder nehmen
unsinnige Wege. „Wegfindung ist Teil des Gameplays“: wer nicht ankommt, macht jede
Planung wertlos.

Bei uns gibt es Regeln gegen Festfahren (Wegpunkt überspringen nach 2 s ohne
Fortschritt, Gleitschritt, 45°-Schritt, bis zu 3 Neuplanungen, Lead-in seit 088).
Was fehlt, ist eine **Messung**: Niemand weiß, wie oft auf unseren gut 25 Karten
eine Einheit tatsächlich liegen bleibt, wo und warum. `stuck_spawns` zählt nur
Startplätze auf gesperrten Zellen. Gemeldet werden Festfahrer heute nur, wenn der
Nutzer sie zufällig sieht.

## 2. Ziel

### 2.1 Prüfstand `just stuck-audit`

- Neuer CLI-Befehl (headless, deterministisch): für jede Karte in `data/maps/`
  (oder `--map=<name>`) Gruppen typischer Einheiten (Infanterie, leichtes und
  schweres Fahrzeug, Geschütz im Schlepp, gemischte Gruppe) mit Bewegungsbefehlen
  quer über die Karte, **gezielt über Brücken, Furten, Engstellen, durch Wald- und
  Stadtränder** (Ziele aus Brücken, Versorgungspunkten und Startplätzen der Karte
  ableiten), dazu die Einheiten aus `just ai-tournament`-Läufen.
- Erfasst wird je Einheit: kein Fortschritt zum Ziel über `N` Sekunden
  (Vorschlag 10 s Spielzeit, in einer Datei einstellbar), Ort, Einheitentyp,
  Zellart (Brückendeck, Ufer, Wald, Gebäude, Wrack, andere Einheit), Zahl der
  Neuplanungen, und ob das Ziel überhaupt erreichbar war.
- Ausgabe: Tabelle je Karte (Befehle, angekommen, festgefahren, Anteil, häufigste
  Ursache) und eine CSV-Liste aller Fälle mit Koordinaten; optional
  `--shots` mit Bildern der schlimmsten Stellen (über die vorhandenen
  Aufnahmewerkzeuge, wenn das ohne großen Aufwand geht).
- Vorher-Stand für alle Karten festhalten (`docs/PERFORMANCE.md` oder ein neues
  Kapitel in `docs/ARCHITECTURE.md`).

### 2.2 Ursachen beheben

- Die häufigsten Ursachen nach Anteil abarbeiten, nicht nach Vermutung. Kandidaten
  aus „Open“: Bäume und Objekte werden für große Einheiten nicht um den Rand
  verbreitert; Gruppen an Brückenköpfen; geschleppte Geschütze in Kurven; Wracks
  auf dem einzigen Weg.
- Jeder Fix bekommt einen kleinen Regressionstest (Szenario aus dem Prüfstand
  nachgebaut).
- Ziel: auf jeder Karte unter 1 % festgefahrene Befehle, keiner länger als 30 s;
  wo das nicht erreichbar ist, Grund und Zahlen nennen.

### 2.3 Letzte Rettung und Meldung

- Eine Einheit, die trotz aller Regeln länger als `M` Sekunden (Vorschlag 20 s)
  nicht vorankommt, **stoppt sauber** (kein Zucken auf der Stelle) und erzeugt ein
  Ereignis „kommt nicht weiter“ in der Simulation.
- Darstellung: Meldung im Ereignis-Feed bzw. als Alarm nach Prompt 101 (springt
  zur Einheit), kleine Zustandsmarke an der Einheit (`state_marks.gd`).
- Kein Teleportieren und kein „Freischieben“ durch Hindernisse: das wäre eine
  versteckte Regel. Wenn eine Rettung nötig ist, dann eine sichtbare, z. B. der
  nächste sicher erreichbare Punkt als neues Ziel.

## 3. Regeln

- Änderungen in `crates/sim`: feste Kommazahlen, kein versteckter Zufall, keine
  Iteration über `HashMap`. Der Golden Hash ändert sich, wenn sich Bewegung ändert:
  begründen und in der Commit-Nachricht sagen.
- Die Weg-Budgets aus `docs/PERFORMANCE.md` bleiben; `just path-bench` und
  `just bench` vorher und nachher.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Bestand lesen, Prüfstand bauen, Vorher-Stand für alle Karten.
2. Ursachen nach Häufigkeit beheben, je Fix Test und neue Prüfstand-Zahlen.
3. Ereignis „kommt nicht weiter“ und Darstellung.
4. `just check`, `just selftest`, `just lan-selftest`, `just path-bench`, `just bench`.
5. Doku: `docs/ARCHITECTURE.md` (Pathfinding, „Open“ aktualisieren),
   `docs/CONTROLS.md` (Alarm), `justfile`-Kommentar, CLAUDE.md-Befehlsliste.
6. Für den Nutzer: Vorher/Nachher-Tabelle je Karte und die fünf schlimmsten
   verbliebenen Stellen.

## 5. Nicht Teil dieses Prompts

- Formationsplätze in Fahrtrichtung (Prompt 174), Gleichschritt und Kolonne
  (Prompt 187).
- Rückmeldung zu unerreichbaren Zielen beim Befehl (Prompt 175).
