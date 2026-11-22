# Prompt 192 – Sirenen nie doppelt

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> dann im Code `godot/scenes/world/strike_view.gd` (`_siren`, `_update_strikes`,
> `_siren_stream`), `godot/scenes/main.gd` und `godot/scenes/world/base_view.gd`
> (beide laden `strike_view.gd`), `godot/scenes/audio/battle_sound.gd`
> (Abstände, `per_place`, `PLACE_GAP`), `godot/scenes/audio/sound_classes.gd`,
> `web/sounds/usage.toml` und `godot/scenes/tools/blast_shots.gd` (`strike-live`).

## 1. Problem

Der Nutzer (2026-11-22): Sirenen laufen parallel; es reicht eine. Anders als
echte Gefechtsgeräusche (Explosionen, Schüsse), die sich überlagern dürfen und
sollen, ist eine Sirene ein **Signal**: zwei gleichzeitige klingen falsch
(Schwebung, doppelte Lautstärke) und sagen nichts Neues.

`strike_view.gd` hat einen `_siren`-Player, der läuft, solange irgendein Schlag
unterwegs ist. Trotzdem hört der Nutzer mehrere. Erst herausfinden, warum:

- Gibt es mehrere Instanzen von `strike_view.gd` (Hauptszene, `base_view.gd`,
  Missionen, Schlacht-Schaustück, Trailer)?
- Gibt es weitere sirenenartige Signale (Fliegeralarm, Luftabruf, Warnungen aus
  `hud/alerts.gd`, Missionsaktionen, Sounds aus `web/sounds/usage.toml`), die
  gleichzeitig mit der Raketen-Sirene oder mehrfach mit sich selbst laufen?
- Startet ein Neustart des Players (`play()` bei jedem Schlag, Szenenwechsel,
  Laden eines Spielstands) eine zweite Kopie, während die alte noch ausklingt?

Ursache mit Ort im Bericht nennen.

## 2. Ziel

- Eine **Signal-Klasse** im Ton (z. B. in `sound_classes.gd` oder
  `battle_sound.gd`): Klänge dieser Klasse (Sirenen, Alarme, Hupen, Signalpfeifen)
  spielen höchstens **einmal gleichzeitig im ganzen Spiel**. Ein zweiter Auslöser,
  während einer läuft, verlängert den laufenden höchstens (bis zum Ende des
  letzten Auslösers), startet aber keinen zweiten. Kein Neustart mitten im Ton
  (kein hörbarer Sprung): weiterlaufen lassen, am Ende des Zyklus ausklingen.
- Explosionen, Schüsse, Motoren und alle anderen Gefechtsgeräusche bleiben wie
  sie sind (mehrstimmig, mit den bestehenden Grenzen).
- Gilt für alle Szenen: Spiel, Missionen, Replay, Schaustücke, Trailer.
- `web/sounds/usage.toml` bei der Sirene vermerken („spielt nur einmal
  gleichzeitig“).

## 3. Nicht

- Keine Änderung an `crates/sim` (reiner Ton, Golden Hash unverändert).
- Keine neue Sirene, kein neuer Klang.

## 4. Abnahme

- Szenario mit zwei bis drei gleichzeitigen Raketenschlägen (eigene und
  feindliche) und, falls gefunden, den anderen Signalen: Zähler der gleichzeitig
  spielenden Signal-Player bleibt ≤ 1 (Ausgabe im Selbsttest, z. B. in
  `blast_shots.gd` `strike-live` oder `sound_self_test.gd`), Explosionen zählen
  weiter mehrfach.
- `just selftest` und `just check` grün.
