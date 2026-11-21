# Prompt 176 – Feuerdisziplin für Hinterhalte, Unbewaffnete weichen aus

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, `docs/CONTROLS.md`, in `docs/ARCHITECTURE.md` „Combat“,
> „Vision and fog of war → Ambush“, dazu `docs/GAME_DESIGN.md` („Ambush“, „Fire
> from the flank“), `prompts/done/056-*.md`, `prompts/done/110-*.md`,
> `crates/sim/src/combat.rs` (`FireMode`), `crates/sim/src/command.rs`
> (`SetFireMode`), `crates/sim/src/sim/combat.rs` (`choose_target`, Reaktion auf
> Flankenfeuer aus 110), `data/tactics.toml`, `godot/scenes/hud/state_marks.gd`.
> Fremde, nicht eingecheckte Änderungen an `crates/sim`: erst den Nutzer fragen.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) kritisiert Besatzungen, die auf Beschuss nicht reagieren, und zu
viel Mikromanagement.

Bei uns gibt es nur `FireMode::FireAtWill` und `FireMode::HoldFire` (Taste F).
Ein Hinterhalt nach `docs/GAME_DESIGN.md` braucht aber: **erst feuern, wenn das
Ziel nah genug ist oder ein Panzer kommt**, sonst verrät sich die Stellung an der
ersten Spitze. Unbewaffnete Fahrzeuge (Lkw, Zugmaschinen) bleiben unter Beschuss
stehen, wenn sie keinen Befehl haben.

Die neue Zielwertung nach Rolle und Bedrohung ist Prompt 188.

## 2. Ziel

### 2.1 Feuerdisziplin (erweiterter Befehl)

- Drei Stufen statt zwei: **Feuer frei**, **Feuer erwidern** (schießt nur auf
  Einheiten, die sie oder ihren Trupp beschossen haben; die Beschussrichtung aus
  110 ist schon da), **Feuerverbot**.
- Für „Feuer frei“ eine optionale **Eröffnungsentfernung** (Vorschlag: fest halbe
  Reichweite als Schalter; Klick-Entfernung nur, wenn es einfach geht) und
  **„nur auf Fahrzeuge“** für Panzerabwehr.
- Erweiterung von `FireMode` bzw. `SetFireMode`, gilt für Spieler und KI gleich;
  Taste F schaltet weiter um (Belegung in `docs/CONTROLS.md`), Zustandsmarke und
  Kachel zeigen die Stufe.
- Mit Tarnung und Hinterhalt aus Prompt 056 prüfen: Wer nicht feuert, bleibt
  verborgen; der erste Schuss deckt auf wie bisher.
- KI: mindestens ein einfacher Fall über normale `Command`s (Verteidiger-Pak „nur
  auf Fahrzeuge“ oder Eröffnungsentfernung); mehr als Befund für 178.

### 2.2 Unbewaffnete unter Feuer

- Unbewaffnete Fahrzeuge ohne Befehl, die getroffen werden, fahren aus der
  Schusslinie (kurzer Rückzug weg von der Beschussrichtung aus Prompt 110), statt
  stehenzubleiben. Abschaltbar, Werte in `data/tactics.toml`.

## 3. Regeln

- Feste Kommazahlen, kein Zufall, keine `HashMap`-Iteration; Golden Hash ändert
  sich beabsichtigt (Commit-Nachricht).
- `just bench` vorher/nachher, keine spürbare Verschlechterung.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Prüfszenarien: Hinterhalt im Wald (Pak + MG-Trupp) gegen eine Kolonne mit
   Spähwagen vorne und Panzer dahinter; Lkw ohne Befehl unter MG-Feuer.
   Vorher-Ergebnis festhalten.
2. Feuerdisziplin: Befehl, Simulation mit Tests (je Stufe, Eröffnungsentfernung,
   nur Fahrzeuge), Bridge, HUD, Taste.
3. Unbewaffnete Fahrzeuge mit Test.
4. `just ai-tournament`, `just check`, `just selftest`, `just lan-selftest`,
   `just bench`.
5. Doku: `docs/ARCHITECTURE.md` (Combat), `docs/GAME_DESIGN.md` (vorläufig, mit
   Datum), `docs/CONTROLS.md`, `docs/AI.md`.
6. Für den Nutzer: Ergebnis der Szenarien vorher/nachher.

## 5. Nicht Teil dieses Prompts

- Zielwertung nach Wirkung, Bedrohung und Rolle; Grenzen für indirekte Waffen
  ohne Befehl (Prompt 188).
