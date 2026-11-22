# Prompt 195 – Sturmtiger nie allein und nie vorn

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/AI.md`, den Katalogeintrag `de_sturmtiger` in `data/catalog/de.toml`
> (Quellen, Reichweite, Ladezeit) und den Commit „Sturmtiger as a new German unit“
> (`git show 7a43255 --stat`). Im Code `crates/ai/src/commander.rs`,
> `flank.rs`, `artillery.rs`, `posture.rs`, `economy.rs`; `data/economy.toml`
> (Bauliste); in `crates/sim/src/sim/` `march.rs` (Verband) und `orders.rs`.

## 1. Problem

Der Nutzer (2026-11-22): Ein Sturmtiger greift nie allein an, er steht nie an
vorderster Front. Historisch war er ein Spezialfahrzeug gegen Bunker und
befestigte Häuser: dicke Front, aber eine einzige langsam nachgeladene 38-cm-
Rakete, kaum Nahschutz, wenig Munition. Er wurde hinter Infanterie und Panzern
geführt und schoss aus der zweiten Linie auf erkannte Ziele.

## 2. Ziel

**KI (Computerspieler, nur über `Command`s):**

- Der Sturmtiger bekommt eine eigene Rolle „Unterstützung, zweite Linie“: er
  rückt nur im Verband mit mindestens N anderen Kampfeinheiten vor (Richtwert:
  ein Trupp und ein Panzer oder zwei Trupps) und hält Abstand X hinter der
  vordersten eigenen Einheit (Richtwert 80–150 m, kleiner als seine Reichweite).
- Fehlt der Verband (Begleiter tot oder weg): zurückziehen bzw. warten, bis neue
  Begleiter da sind, nicht allein weiterfahren.
- Ziele: Gebäude mit Besatzung, Bunker, Pak-Stellungen, Ansammlungen; nicht auf
  Einzelschützen oder schnelle Fahrzeuge verschwenden (an `[targeting]` aus
  Prompt 188 andocken, nichts doppelt bauen).
- Nach dem Schuss in Deckung zurück, bis nachgeladen.

**Spieler (optional, wenn einfach):** keine Automatik, die den Spieler
bevormundet. Höchstens ein Hinweis im Infofeld/Feed, wenn ein Sturmtiger ohne
Begleitung in Feindnähe steht.

Prüfen, ob die Regel als allgemeine Eigenschaft passt (z. B. Katalogfeld oder
`ai`-Tabelle „support_rear“ für Sturmtiger, Sturmpanzer IV, Calliope, Wespe),
statt einen Sonderfall nur für eine Id zu bauen.

## 3. Nicht

- Keine Regeländerung in `crates/sim`, wenn es ohne geht (Golden Hash dann
  unverändert); die KI bleibt ein normaler Spieler (`CLAUDE.md` Regel 5).

## 4. Abnahme

- Szenario (KI-Angriff mit Sturmtiger und Begleitung auf ein besetztes Dorf):
  Abstand des Sturmtigers zur vordersten eigenen Einheit über die Zeit
  (Tabelle), nie vorn; Begleiter getötet → Sturmtiger zieht sich zurück.
- `just battle valley_fields/all_units` angesehen: Sturmtiger bleibt hinten.
- `just ai-tournament` vorher/nachher, Denkzeit unverändert; `just check` grün.
