# Prompt 193 – Artillerie mit sichtbarer Bedienung

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> dann `prompts/done/177-geschuetzbedienung-verwundbar.md` (Bedienung als Zähler;
> offen geblieben: „keine Figuren an den Modellen“), `docs/ASSETS.md`
> (`Socket_Crew_NN`, `crew_sockets`, Budgets), `docs/INFANTRY.md` (Abschnitte
> „Presentation (prompt 022)“, „Knocked down by blasts“, „Performance“). Im Code
> `crates/sim/src/sim/crew.rs` (`Crew`, `crew_reload_pct`, `crew_silenced`),
> `crates/bridge` (was über die Bedienung schon herausgegeben wird),
> `godot/scenes/world/infantry_view.gd`, `soldier_motion.gd`, `ragdolls.gd`,
> `kill_sequence.gd`, die Ansicht der Geschütze (Mündungsfeuer, Rücklauf:
> `fire_view.gd`, `artillery_view.gd`) und die Blender-Kits unter `tools/blender/kit/`
> mit `crew_sockets`.

## 1. Problem

Der Nutzer (2026-11-22): Die Artillerie soll eine Besatzung bekommen. Seit
Prompt 177 hat jedes gezogene Geschütz und jeder schwere Mörser eine Bedienung
als Zähler (Katalog `crew`, Treffer, Niederhalten, ohne Männer stumm), aber am
Modell steht niemand. Man sieht nicht, dass die Bedienung schrumpft, und ein
verlassenes Geschütz sieht aus wie ein feuerndes.

## 2. Ziel

Figuren an allen Geschützen mit `Crew` (zuerst `gun_howitzer`, dann `gun_at`,
`gun_aa_light`, `gun_aa_heavy`, `mortar_heavy`, `rocket_launcher`), reine
Darstellung:

- **Plätze:** `Socket_Crew_NN` der Modelle (fehlen sie, in den Blender-Kits
  ergänzen und exportieren). Rollen je Platz: Richtschütze, Ladeschütze,
  Munitionsträger, Geschützführer (mit Fernglas), übrige knien an der Munition.
  Nationalität und Uniform wie die Infanterie (Fraktion, Biom-Tarnung).
- **Zahl folgt dem Zähler:** fällt ein Mann in der Simulation, fällt eine Figur
  (Ragdoll bzw. Clip wie bei Trupps, Leiche bleibt liegen); welcher Platz frei
  wird, deterministisch aus Unit-Id und Zahl (Richtschütze zuletzt). Weniger
  Männer: die übrigen rücken auf die wichtigen Plätze.
- **Tätigkeit:** Laden im Rhythmus der Ladezeit (Granate tragen, einsetzen),
  Abfeuern (Ducken, Ohren zuhalten bei großen Kalibern), Richten beim Schwenken,
  niedergehalten: alle liegen/kauern hinter dem Schild, Ladetakt langsamer
  (wie in der Simulation). Verlassen: niemand mehr am Geschütz.
- **Schild:** Figuren stehen im Schutz des Schilds, das mit dem Rohr dreht.
- **Marsch:** angehängt am Zugfahrzeug sitzen sie im Fahrzeug bzw. sind
  unsichtbar; beim Abprotzen steigen sie aus und laufen an ihre Plätze
  (wie Aussteigen aus Prompt 163).
- Sichtbar nur, wo die Einheit sichtbar ist (Nebel des Krieges wie Trupps).
- Performance: Figuren über dieselben MultiMesh-/Skelett-Wege wie die Infanterie;
  ab Entfernung X nur noch statische Posen. Budget aus `docs/PERFORMANCE.md`.

Fehlt der Bridge etwas (Zahl der Männer, Ladefortschritt, niedergehalten),
nur lesend herausgeben, nichts in den Hash.

## 3. Nicht

- Keine Regeländerung in `crates/sim` (Golden Hash unverändert). Wiederbesetzen
  verlassener Geschütze bleibt ein eigener Prompt.
- Keine Godot-Physik außer den Ragdolls im bestehenden Rahmen.

## 4. Abnahme

- Bilder aus Spielhöhe und nah: le.FH 18 mit voller Bedienung beim Laden und
  Feuern, nach zwei Verlusten, niedergehalten, verlassen; Pak und Flak je eins;
  jede Fraktion einmal.
- Thumbnails der Geschütze mit Bedienung neu (`just thumbnails --only=…`),
  `just units-web-sync`.
- `just bench-view` vorher/nachher mit einer Batterie von 6 Geschützen im Bild.
- `just selftest` (Zahl der Figuren = Zähler nach Verlusten) und `just check` grün.
