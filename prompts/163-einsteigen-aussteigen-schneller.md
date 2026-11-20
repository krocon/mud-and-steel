# Prompt 163 – Ein- und Aussteigen geht zu langsam

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/SCALE.md` (Abschnitt zu Prompt 097: Fahrzeuge fahren echtes Tempo,
> Infanterie läuft mit Zeitfaktor 2), `docs/ARCHITECTURE.md` „Transport and
> towing“, `data/transport.toml` ganz, `prompts/done/034-*.md` (Transport),
> `prompts/109-mitfahrer-einzeln-absetzen.md` (offen, berührt dieselben Stellen),
> `crates/sim/src/sim/transport.rs` (`Task::Embark`/`Disembark`, `order_embark`,
> `order_disembark`, die Nähe-Prüfung um Zeile 268 mit `reach`),
> `crates/sim/src/transport.rs`, `godot/scenes/selection/transport_orders.gd`,
> die Darstellung der Mitfahrer in `godot/scenes/world/` (Soldaten, die auf die
> Ladefläche klettern) und `crates/ai/src/transport.rs` (`JOB_TICKS`,
> `PICKUP_M`, `UNLOAD_M`).

## 1. Ausgangslage

Der Nutzer im Spiel (Bildschirmfoto 2026-11-20: Schützentrupp neben einem
offenen Lkw, einige Männer schon auf der Ladefläche, einer klettert noch):
**„Es dauert ewig, bis die Soldaten einsteigen.“** und **„Das Aussteigen dauert
auch zu lang.“**

Heute (vor der Arbeit im Code nachprüfen):

- `data/transport.toml`: Lkw `embark_ms = 20000`, `disembark_ms = 10000` (echte
  Zeit, durch Zeitfaktor 2 im Spiel 10 s / 5 s); andere Träger 8–12 s echt.
  Alle Werte `[provisional]`, Regelwerte, keine Katalogwerte.
- Einsteigen beginnt erst, wenn der **ganze Trupp** (Mitte plus Ausbreitung) in
  `reach_mm = 4000` am Fahrzeug steht; vorher läuft er in Formation hin. Das
  Einsteigen selbst ist dann ein fester Zähler für den ganzen Trupp.
- Seit Prompt 097 fahren Fahrzeuge halb so schnell; Infanterie nicht. Ein
  Hin- und Herfahren zum Aufsammeln dauert dadurch länger als vorher.

## 2. Ziel

Ein- und Aussteigen fühlen sich zügig an: ein Trupp steht nach dem Befehl
**sichtbar schnell** auf bzw. neben dem Fahrzeug, ohne dass Transport zum
Teleport wird. Richtwert für den Nutzer: Einsteigen eines Trupps, der schon
neben dem Lkw steht, in **3–4 s Spielzeit**, Aussteigen in **2–3 s**; das
Hinlaufen kommt dazu.

## 3. Vorgehen

1. **Messen:** In einer kleinen Szene (Headless-Test oder `just run`) den
   Ablauf in Ticks zerlegen: Befehl → Trupp losgelaufen → in Reichweite → im
   Fahrzeug, und Befehl → Fahrzeug steht → alle draußen. Für Lkw, Halbkette
   und Panzer (Aufsitzen außen). Ergebnis als Tabelle; daran sehen, ob die
   Zeit im Hinlaufen, im Warten auf den letzten Mann oder im Zähler steckt.
2. **Hebel in dieser Reihenfolge prüfen** und den einfachsten nehmen, der
   reicht:
   - `embark_ms` / `disembark_ms` in `data/transport.toml` senken (Regelwerte,
     begründen; Belege für echte Auf- und Absitzzeiten in `docs/SOURCES.md`
     suchen, sonst als Spielregel markieren).
   - Einsteigen beginnt, sobald der **erste** Mann am Fahrzeug ist, und jeder
     Soldat steigt einzeln ein, wenn er ankommt (Zähler pro Soldat oder
     gestaffelt), statt dass der Trupp auf den letzten wartet.
   - Fahrzeug und Trupp gehen aufeinander zu (der Träger fährt dem Trupp
     entgegen), wenn der Spieler den Trupp auf ein Fahrzeug schickt, das
     weiter als z. B. 30 m weg ist.
   - Aussteigen: alle Männer springen gleichzeitig bzw. kurz gestaffelt ab;
     der Trupp ist sofort wieder ein normaler Trupp und kann Befehle annehmen.
3. **Darstellung** an die neuen Zeiten anpassen (Klettern/Abspringen der
   Soldaten, keine Männer, die nach dem Einsteigen noch neben dem Lkw stehen
   oder durch ihn laufen).
4. **KI** (`crates/ai/src/transport.rs`): Zeitgrenzen (`JOB_TICKS`) und
   Abstände an die neuen Zeiten anpassen; `just ai-tournament` Schwer gegen
   Normal vorher/nachher, kurz in `docs/AI.md` notieren.
5. Mit Prompt 109 (Mitfahrer einzeln absetzen) abstimmen: wenn 109 schon
   eingecheckt ist, darauf aufbauen; sonst nichts vorwegnehmen, was 109
   regelt.

## 4. Regeln

- Simulation bleibt Quelle der Wahrheit (`crates/sim`, keine Floats, keine
  Godot-Physik). Zeiten in `data/transport.toml`, nicht im Code.
- Golden Hash ändert sich vermutlich (Regeländerung): erst nach der Umsetzung
  aktualisieren und in der Commit-Nachricht nennen. Keinen Test abschwächen;
  Tests mit festen Zeiten an die neuen Regeln anpassen und die Herleitung im
  Kommentar zeigen.
- Batch-Sessions teilen sich das Arbeitsverzeichnis: vor Turnieren und
  Messungen nachsehen, ob andere Prompt-Sessions laufen.

## 5. Abschluss

- `just check`, `just selftest`, `just lan-selftest`.
- Doku: `docs/ARCHITECTURE.md` (Transport), `docs/CONTROLS.md` falls sich die
  Bedienung ändert, Kommentare in `data/transport.toml`.
- Für den Nutzer: Tabelle vorher/nachher (Sekunden Spielzeit für Ein- und
  Aussteigen je Fahrzeugart) und wo er die Zeiten selbst nachstellen kann.

## 6. Nicht Teil dieses Prompts

- Fahrzeugtempo und Reichweiten (Prompt 097).
- Mitfahrer im Infofeld einzeln absetzen (Prompt 109).
