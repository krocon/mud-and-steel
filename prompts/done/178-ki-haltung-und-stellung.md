# Prompt 178 – KI nutzt Haltungen; Verteidiger halten ihre Stellung

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach 176.
> Lies zuerst `CLAUDE.md`, `docs/AI.md` ganz (vor allem „Behavior“, „Commander“,
> „Difficulties“, „Open“: „stances, cover spots and buildings are not used by the
> AI yet“), `docs/INFANTRY.md`, `docs/GAME_DESIGN.md` („Ambush“), in
> `docs/ARCHITECTURE.md` „Terrain deformation → Infantry cover“ und „Positions“,
> dazu `crates/ai/src/lib.rs`, `crates/ai/src/commander.rs`,
> `crates/ai/src/view.rs`, `crates/sim/src/infantry.rs`,
> `crates/sim/src/sim/infantry.rs`. Wenn in `crates/ai` fremde, nicht
> eingecheckte Änderungen liegen (z. B. `flank.rs` aus 110), erst den Nutzer
> fragen.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) sagt: „KI-Aggressivität darf nicht mit KI-Kompetenz verwechselt
werden.“ Ein Verteidiger soll nicht ohne Grund seine Stellung räumen und mit
unbewaffneten Fahrzeugen in vorbereitete Feuerstellungen fahren.

Bei uns (`docs/AI.md`, „Open“):

- Angreifende Trupps der KI laufen **aufrecht** an und verlieren gegen kniende
  Verteidiger. Haltungen, Deckungspunkte und Gebäude (Prompt 022, 047) nutzt die
  KI nicht.
- Gelände (Kämme, Wälder, Häuser) spielt bei der Wahl der Achsen keine Rolle.
- Die KI hält ihre Armee bis zur Welle bei der Basis; wie sie eine einmal
  genommene Stellung verteidigt, ist nicht geregelt.

## 2. Ziel

Gekürzt am 2026-11-21: zuerst das, was mit wenig Aufwand viel bringt. Deckung zu
Deckung, Gebäude als Zwischenziele und Feuer und Bewegung sind ein späterer
Prompt (siehe 5.).

### 2.1 Haltungen im Angriff

- Unter Beschuss oder in Sicht eines bekannten Gegners: knien bzw. hinlegen nach
  denselben Regeln, die ein guter Spieler anwendet (Haltungsbefehle wie Z/X/V/B
  als `Command`s); wieder aufstehen zum Vorgehen, wenn kein Feuer mehr kommt.
- Einfache Regel, Werte in Daten (Entfernungen, Wartezeiten), keine Zuckungen
  (Mindestdauer je Haltung).

### 2.2 Stellung halten

- Verteidiger an eingenommenen Versorgungspunkten und Zielen **räumen die
  Stellung nicht** für einen einzelnen sichtbaren Gegner außerhalb ihrer
  Reichweite; nur der Kommandeur entscheidet Gegenstoß oder Rückzug.
- Trupps an solchen Punkten besetzen ein Gebäude in der Nähe, wenn eines da ist
  (vorhandener Befehl „Gebäude besetzen“); sonst knien sie.
- Unbewaffnete Fahrzeuge der KI fahren nicht in bekannte feindliche Feuerbereiche
  (Lagebild); erst prüfen, ob `transport.rs` und `supply.rs` das schon tun.
- Feuerdisziplin aus 176 für Verteidiger nutzen (z. B. Pak „nur auf Fahrzeuge“).

### 2.3 Schwierigkeit

- Normal und Schwer nutzen alles, Leicht nur Haltungen. Tabelle in `docs/AI.md`
  ergänzen.

## 3. Regeln

- Die KI bleibt ein normaler Spieler: nur `Command`s, Sicht nur über `PlayerView`
  (Nebel des Krieges), kein Wissen über verborgene Einheiten.
- Kein Zufall außerhalb des vorhandenen deterministischen Wegs der KI.
- Laufzeit der KI je Denkschritt messen und in `docs/PERFORMANCE.md` nennen.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Prüfszenarien: zwei KI-Trupps greifen ein Dorf mit drei knienden
   Spieler-Trupps an; KI verteidigt einen Versorgungspunkt gegen Panzer + Infanterie
   von der Seite. Vorher-Zahlen (Verluste, Dauer, Ergebnis).
2. 2.1 mit Tests, dann 2.2, dann 2.3.
3. `just ai-tournament` vorher/nachher; Erwartung: neue KI schlägt die alte
   gleicher Stufe deutlich in Gefechten mit viel Infanterie.
4. `just check`, `just selftest`.
5. Doku: `docs/AI.md` (Abschnitt neu, „Open“ aktualisieren), `docs/BALANCE.md`.
6. Für den Nutzer: Zahlen und ein kurzer Clip des Dorfangriffs.

## 5. Nicht Teil dieses Prompts (gekürzt am 2026-11-21)

- Von Deckung zu Deckung statt geradeaus, Gebäude und Waldränder als
  Zwischenziele, Feuer und Bewegung zweier Trupps, Stellungswahl von Panzern und
  Pak nach Anmarschrichtung. Nach diesem Prompt mit dessen Turnierzahlen
  entscheiden, ob ein eigener Prompt dafür lohnt; als offenen Punkt in
  `docs/AI.md` festhalten.
