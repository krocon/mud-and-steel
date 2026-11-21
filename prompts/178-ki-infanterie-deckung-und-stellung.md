# Prompt 178 – KI nutzt Haltung, Deckung und Gebäude; Verteidiger halten ihre Stellung

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach 110 und 176.
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

### 2.1 Infanterie im Angriff

- Unter Beschuss oder in Sicht eines bekannten Gegners: knien bzw. hinlegen nach
  denselben Regeln, die ein guter Spieler anwendet (Befehle Z/X/V/B als
  `Command`s), und **von Deckung zu Deckung** (Prompt 047) statt geradeaus.
- Gebäude und Waldränder an der Angriffsachse als Zwischenziele, wenn sie auf dem
  Weg liegen; kein langer Umweg.
- Feuer und Bewegung: ein Trupp hält nieder, der andere geht vor, wenn die Gruppe
  mindestens zwei Trupps hat.

### 2.2 Verteidigung

- Eingenommene Versorgungspunkte und Ziele: Trupps besetzen Gebäude und Deckung
  in der Nähe, Panzer und Pak beziehen Stellungen mit Blick auf die
  wahrscheinliche Anmarschrichtung (aus dem Lagebild des Kommandeurs).
- Verteidiger **räumen die Stellung nicht** für einen einzelnen sichtbaren Gegner
  außerhalb ihrer Reichweite; nur der Kommandeur entscheidet Gegenstoß oder
  Rückzug.
- Unbewaffnete Fahrzeuge der KI fahren nicht in bekannte feindliche Feuerbereiche
  (Lagebild); prüfen, ob `transport.rs` und `supply.rs` das schon tun.
- Feuerdisziplin aus 176 für Hinterhalte nutzen, wenn 176 erledigt ist.

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
