# Prompt 196 – Rennen beim Sturm, Hinlegen unter Feuer

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, in `docs/INFANTRY.md` „Stances“, „Cover“, „Tactics
> (prompt 047)“, „Presentation (prompt 022)“ (Clips `walk`, `run`, `crawl`).
> Im Code `crates/sim/src/sim/infantry.rs` (`StanceOrder::Auto`, Haltung beim
> Bewegen), `crates/sim/src/sim/tactics.rs` (`auto_stance`, `SquadTactics`,
> `pinned`, `close`, `bound`), `[suppression]` und `[discipline]` in
> `data/tactics.toml`, `crates/ai/src/posture.rs`, `godot/scenes/world/soldier_motion.gd`.

## 1. Problem

Der Nutzer (2026-11-22): Beim Sturm sollen die Soldaten rennen; bei nahen
Einschlägen und MG-Salven sollen sie sich hinlegen, wenn es keine andere
Deckung gibt.

Heute: in `Auto` stehen bewegte Trupps (unter Feuer knien sie); liegen nur bei
`pinned` oder `close`. Die Geschwindigkeit hängt nicht davon ab, ob der Trupp
stürmt. Ein naher Einschlag ohne Niederhalten-Schwelle bringt niemanden zu Boden.

## 2. Ziel

**Rennen (Regel in `sim`):**

- „Sturm“ festlegen: Angriffsbewegung auf einen erkannten Feind in kurzer
  Entfernung (Richtwert unter 80–100 m), Sprung über offenes Gelände unter Feuer
  (die Bewegung der Feuer-und-Bewegung, `bound`), Laufen zur nächsten Deckung.
- Dann laufen die Männer mit Lauftempo (echte Werte mit Quelle bzw. Richtwert
  aus `docs/INFANTRY.md`: `run` 3,6 m/s, über `scale.toml`) statt Gehtempo,
  höchstens für eine Strecke/Zeit, danach Erschöpfung (langsamer, schlechter
  zielen), Erholung im Stand. Wer rennt, schießt nicht (oder stark verschlechtert).
- Darstellung: Clip `run`, vorgebeugt; kein neuer Sonderclip nötig.

**Hinlegen (Regel in `sim`):**

- Naher Einschlag (HE innerhalb eines Radius je Kaliber) oder MG-Salve in der
  Nähe (Garbe trifft in wenigen Metern) → betroffene Soldaten werfen sich sofort
  hin, auch unter der Niederhalten-Schwelle, für eine kurze Zeit.
- **Nur ohne andere Deckung:** wer an einem Deckungspunkt steht (hinter Objekt,
  im Krater, im Graben, im Gebäude, hinter Fahrzeug, falls 198 das einführt),
  bleibt in der Deckung und duckt sich dort (Knien bzw. was die Deckung vorsieht).
- Gilt nur für `StanceOrder::Auto`; feste Haltungsbefehle des Spielers gelten
  weiter. Nicht während eines Sturms im letzten Stück (sonst bleibt jeder
  Angriff liegen): dort eine klare, getestete Ausnahme.
- KI-Haltung (`posture.rs`) mit der neuen Auto-Regel abgleichen, nichts doppelt.

## 3. Nicht

- Keine Floats, keine Zufälle außer `sim::Rng`.
- Wert-Tabellen in `data/tactics.toml`/`infantry.toml`, nicht im Code.

## 4. Abnahme

- Tests in `crates/sim`: Trupp stürmt → Lauftempo, danach Erschöpfung; naher
  Mörsereinschlag/MG-Salve im Freien → liegt; dasselbe hinter einer Mauer →
  bleibt in Deckung; fester Haltungsbefehl unverändert.
- Szenario „Dorfangriff“ vorher/nachher: Verluste beim Sturm, Dauer.
- Golden Hash beabsichtigt neu (Regel), im Commit nennen.
- `just ai-tournament` gegenprüfen, Clip aus Spielhöhe, `just check` grün.
