# Prompt 197 – Mehrstöckige Gebäude stürzen von oben nach unten ein

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ASSETS.md` (Gebäude, Schadensstufen, Blueprints), in
> `docs/ARCHITECTURE.md` die Teile zu Zerstörung (Prompt 020) und Gebäuden.
> Im Code `crates/sim/src/destruction.rs` (`ObjectStage`, `Remains`),
> `crates/sim/src/sim/destruction.rs`, `godot/scenes/world/map_objects.gd`
> (`set_object_state`, `swapped`), `godot/scenes/world/effects.gd` (Trümmer und
> Staub beim Einsturz, `"ruin"`), `tools/blender/kit/buildings.py`,
> `base_buildings.py`, `blocks.py` (Geschosse), `godot/scenes/tools/destruction_self_test.gd`
> und `just building-preview`.

## 1. Problem

Der Nutzer (2026-11-22): Mehrstöckige Gebäude sollen in Zeitlupe einstürzen:
erst die oberste Etage, dann die darunter usw.; ein Gebäude fällt von oben nach
unten.

Heute wechselt ein zerstörtes Gebäude in einem Schritt auf die Ruine, mit
Trümmern und Staub.

## 2. Ziel

Reine Darstellung (die Simulation bleibt bei ihren Stufen; Zeitpunkt der
Zerstörung kommt aus `sim`):

- **Geschosse am Modell:** Die Gebäudemodelle werden beim Export nach Geschossen
  getrennt (Dach, OG n … EG, Ruinenrest), oder der Einsturz schneidet zur Laufzeit
  per Shader auf Höhenbänder. Zuerst entscheiden, was mit den Kits und Budgets
  einfacher ist (Vorschlag mit Bildern, dann bauen). Geschosshöhe aus dem
  Blueprint (etwa 3 m).
- **Ablauf:** Dach bricht zuerst ein, dann das oberste Geschoss: Wände kippen
  nach innen/außen, sacken ab, zerfallen in Brocken; dann das nächste darunter.
  Jedes Geschoss etwa 0,6–1,2 s, mit kurzem Zögern dazwischen; insgesamt bewusst
  langsam („Zeitlupe“), am Ende die bestehende Ruine. Staubwolke wächst von oben
  nach unten mit, Trümmer fallen außen am Gebäude herab.
- Richtung und kleine Unterschiede deterministisch aus Objekt-Index und
  Trefferrichtung (keine Godot-Zufälle für Spielrelevantes; hier nur Optik, aber
  Replays sollen gleich aussehen).
- Einstöckige Gebäude: kürzere Fassung (Dach, dann Wände).
- `animate == false` (Laden, Spulen im Replay): sofort Ruine.
- Qualitätsstufen: Niedrig ohne Brocken, nur Absinken je Geschoss.
- Besatzung: stirbt bzw. springt raus nach den Regeln der Simulation; die
  Figuren verschwinden nicht vor dem Geschoss, in dem sie sind.

## 3. Nicht

- Keine Godot-Physik für Spielrelevantes; Brocken dürfen Physik nur als Optik
  auf eigener Kollisionsebene nutzen (wie Ragdolls) oder vorberechnet fallen.
- Golden Hash unverändert.

## 4. Abnahme

- Clip aus Spielhöhe und nah: dreistöckiges Stadthaus, zweistöckiges
  Bauernhaus, eingeschossiges Haus; Bildfolge alle 0,25 s.
- Bildrate beim gleichzeitigen Einsturz von drei Gebäuden (`just bench-view`
  oder eigener Lauf), Kosten je Qualitätsstufe in `ARCHITECTURE.md`.
- Thumbnails geänderter Modelle neu, falls Modelle neu exportiert wurden.
- `destruction_self_test.gd` prüft den Ablauf (Reihenfolge der Geschosse,
  Ende = Ruine); `just selftest`, `just check` grün.
