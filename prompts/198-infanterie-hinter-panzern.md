# Prompt 198 – Infanterie rückt geduckt hinter Panzern vor

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, in `docs/INFANTRY.md` „Cover“, „Tactics (prompt 047)“,
> „Transport (prompt 034)“. Im Code `crates/sim/src/sim/combat.rs` (Deckung hinter
> Objekten, um Zeile 1380), `crates/sim/src/sim/infantry.rs`, `tactics.rs`
> (`auto_stance`), `march.rs` (Verband, Prompt 187), `formation.rs`,
> `overrun.rs` (Abstand zu fahrenden Fahrzeugen), `crates/ai/src/commander.rs`,
> `flank.rs`; `godot/scenes/world/soldier_motion.gd` (`crouch_walk`).
> Empfohlen nach 196 (Haltungsregeln dort).

## 1. Problem

Der Nutzer (2026-11-22): Wenn Panzer und Fußsoldaten zusammen vorrücken, laufen
die Soldaten meist geduckt hinter dem Panzer, um in Deckung zu bleiben.

Heute bilden gemischte Gruppen eine Formation mit eigenen Plätzen
(Prompt 174/187); Soldaten laufen neben oder vor den Panzern und haben vom
Panzer keine Deckung. Ein Fahrzeug zählt nach jetzigem Stand nicht als
Deckungspunkt (prüfen).

## 2. Ziel

**Regel in `sim`:**

- Bewegen sich Trupp(s) und Panzer/Sturmgeschütz in einem gemeinsamen
  Gruppenbefehl, ordnet die Formation die Trupps **hinter** den Fahrzeugen an
  (in Fahrtrichtung, 5–15 m dahinter, seitlich versetzt, nicht im Überrollweg),
  Richtwert ein Trupp je Fahrzeug; überzählige Trupps dahinter.
- Ein fahrendes oder stehendes Fahrzeug gibt Soldaten direkt dahinter (vom
  Feind aus gesehen) **Deckung** gegen direktes Kleinkaliberfeuer, ähnlich einem
  Deckungspunkt hinter einem Objekt (Winkel und Breite aus der Fahrzeugbreite).
  Gegen Splitter von oben kaum.
- Während des Vorrückens hinter dem Fahrzeug: Haltung `Auto` → geduckt
  (Knien in Bewegung, Clip `crouch_walk`), Tempo an das Fahrzeug gebunden
  (Gleichschritt aus 187, Fahrzeug fährt im Begleittempo der Infanterie).
- Bei Feindkontakt: Panzer hält oder fährt langsam, Infanterie bleibt dahinter
  und schießt seitlich vorbei; beim Sturm (Prompt 196) lösen sie sich zum Ziel.
- Abstand zu fahrenden Fahrzeugen sicher (nicht überrollen, `overrun.rs`),
  auch beim Rückwärtsfahren.
- **KI:** gemischte Angriffsgruppen nutzen dieselbe Formation (nur `Command`s).

## 3. Nicht

- Keine Floats; Werte in `data/tactics.toml` (eigener Abschnitt), `[provisional]`.
- Kein Aufsitzen auf Panzer (eigener Prompt, falls gewünscht).

## 4. Abnahme

- Tests: gemischter Gruppenbefehl → Trupps hinter den Fahrzeugen (Positionen in
  Fahrtrichtung), Deckung hinter Fahrzeug senkt Treffer durch MG von vorn,
  niemand wird überrollt (Hin- und Rückfahrt).
- Szenario Panzer + Trupp gegen MG-Stellung vorher/nachher: Infanterieverluste
  beim Annähern.
- Golden Hash beabsichtigt neu (Regel), im Commit nennen.
- Clip aus Spielhöhe; `just ai-tournament`, `just stuck-audit` gegenprüfen;
  `just check` grün.
