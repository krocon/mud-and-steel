# Prompt 194 – Große Explosionen schleudern Soldaten weg

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, in `docs/INFANTRY.md` „Knocked down by blasts
> (prompt 048)“. Im Code `crates/sim/src/sim/knockdown.rs` (`blast_reach`,
> `knock_down`, `Thrown`), `[knockdown]` in `data/infantry.toml`, `[blast]` in
> `data/indirect.toml`, `crates/sim/src/sim/strike.rs` (Raketenschlag,
> Munitionsexplosion), `godot/scenes/world/ragdolls.gd` (`MAX_ACTIVE`,
> `MAX_DISTANCE_M`), `blast_effects.gd` (`big_blast`), `infantry_view.gd`.

## 1. Problem

Der Nutzer (2026-11-22): Bei einer großen Explosion sollen nah stehende Soldaten
weggeschleudert werden.

Es gibt schon das Umwerfen aus Prompt 048: Radius 60 mm je mm Kaliber,
höchstens 2,5 m Wurf, 6 s am Boden; Munitionsexplosionen werfen in 12 m um.
Das wirkt bei großen Ladungen (15-cm-Granate, 38-cm-Sturmtiger-Rakete, Bomben,
Raketenschlag, Munitionslager, explodierender Panzer) zu klein: die Soldaten
kippen um, statt durch die Luft zu fliegen. Die Ragdolls sind auf 8 gleichzeitig
und 60 m Kameraabstand begrenzt.

## 2. Ziel

- **Wurfweite nach Ladung, nicht fest:** Wurf wächst mit der Sprengladung
  (Kaliber bzw. Sprengstoffmasse, soweit im Katalog mit Quelle vorhanden) und
  fällt zum Rand ab; nahe einer großen Ladung mehrere Meter (Richtwert 4–8 m bei
  15 cm und mehr, Bomben und Raketen mehr), kleine Ladungen bleiben wie heute.
  Werte als `[provisional]` in `[knockdown]`, mit kurzer Begründung; echte
  Druckwirkung wo eine Quelle existiert (`docs/SOURCES.md`).
- **Bahn statt Versetzen:** die Simulation liefert Start, Ziel und Flugzeit
  (ganzzahlig, deterministisch); der Soldat landet nicht in Wänden, Wasser oder
  Gebäuden (Zielpunkt auf begehbare Zelle kürzen). Wer im Flug getötet wird,
  liegt am Landepunkt.
- **Darstellung:** Ragdoll bekommt einen Impuls nach oben und vom Zentrum weg,
  der zur Flugbahn der Simulation passt (landet am Punkt der Simulation, nicht
  irgendwo); Waffe und Helm können wegfliegen (`helmets.gd`). Für große Blasts
  dürfen kurzzeitig mehr Ragdolls laufen (eigene Obergrenze, Rest als
  Fallen-Clip mit Bogen).
- Gebäude- und Fahrzeuginsassen bleiben unberührt, Liegende und Deckung
  dämpfen weiter (`prone_pct`, `cover_pct`).

## 3. Nicht

- Keine Godot-Physik für Spielrelevantes: Ort und Zeit kommen aus `sim`.
- Keine Floats in `crates/sim`.

## 4. Abnahme

- Tests in `crates/sim`: Wurfweite je Ladung (8 cm, 15 cm, 38 cm, Bombe),
  Landepunkt nie in gesperrten Zellen, Deckung dämpft.
- Golden Hash ändert sich beabsichtigt (Regel), im Commit nennen.
- Clip/Bilder: Sturmtiger-Treffer und Munitionslager neben einem Trupp
  (`just trailer-shots --only=…` oder `blast_shots.gd`), Bildrate beim Blast.
- `just selftest`, `just check` grün; Turnier kurz gegenprüfen (`just ai-tournament`).
