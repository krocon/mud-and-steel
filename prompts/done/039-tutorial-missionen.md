# Prompt 039 – Tutorial, Missionen und Skript-System

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md`, `docs/MAPS.md`, `docs/AI.md` und `docs/CONTROLS.md` (aus 031).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Neue Spieler stehen bisher ohne Anleitung auf einer großen Karte. Nach diesem Prompt gibt
es ein geführtes Tutorial und ein deterministisches Missions-Skript-System, mit dem sich
Einzelspieler-Missionen (und später eine Kampagne) bauen lassen.

## 2. Umfang

1. **Missions-Skripte** als Daten (TOML) neben der Karte: Auslöser (Tick, Gebiet
   betreten, Einheit zerstört, Objekt erobert), Aktionen (Einheiten erscheinen lassen,
   Ziel setzen/erfüllen, Text, Kamerafahrt, Sieg/Niederlage). Was das Spiel verändert,
   läuft deterministisch in der Sim (oder als normale Befehle eines Skript-Spielers wie
   die KI); reine Darstellung (Texte, Kamera) in Godot. Entscheidung begründen.
2. **Tutorial** in kurzen Schritten: Kamera, Auswahl, Bewegen, Angreifen, Infanterie und
   Deckung, Bauen und Produzieren, Versorgungspunkte; jeder Schritt wartet auf die
   Handlung des Spielers.
3. **Zwei bis drei Missionen** als Beispiel (je Fraktion eine kleine Lage, ohne
   historische Einheitenabzeichen), mit Zielanzeige im HUD.
4. **Editor (028):** Auslöser-Gebiete und Spawn-Punkte setzen; Skript als Datei.
5. **Menü:** „Tutorial“ und „Missionen“ im Hauptmenü, Fortschritt in `user://`.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Auslöser und Aktionen, Hash-Stabilität.
2. Selbsttest spielt das Tutorial automatisch durch und jede Mission bis zum Sieg
   (mit KI als Spieler oder Skript).
3. Doku: `docs/MISSIONS.md` mit Skript-Referenz.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## 4. Konkretisierung (Ausgangslage und Entscheidungen)

Stand vor dem Umsetzen: Steuerung in `docs/CONTROLS.md` (031 HUD, 046
Formationen/Angriffsbewegung, 058 Bildkacheln, 033 Luftleiste, 034
Transport, 035 Pioniere, 036 Nachschub); KI als normaler Spieler mit
Befehlen (024, `crates/ai`); Szenarien als Startaufstellung
(`data/scenarios`); Sieg/Niederlage in `sim::outcome` (027); Speichern und
Replays über `SimConfig` + Befehle + Snapshots (038); Teams (037); Editor
mit Markern und Startpunkten, Karten in `data/maps/<name>/` (028).

| Frage | Entscheidung | Begründung |
|---|---|---|
| Wo läuft das Skript? | In der Simulation (`sim::mission`, `sim/mission.rs`), als Teil von `SimConfig::mission` | Auslöser lesen den vollen Zustand (Gebiete, Zerstörung, Versorgungspunkte), Aktionen erzeugen Einheiten und entscheiden das Spiel. Ein Skript-Spieler mit Befehlen bräuchte dafür neue Schummel-Befehle (Einheiten erzeugen, Sieg setzen) und sähe nur seine Nebel-Sicht. In der Sim ist das Skript automatisch deterministisch, im Replay, im Spielstand (Snapshot) und im Hash. |
| Was die Sim nicht weiß (Kamera, Auswahl) | Neuer Befehl `Command::MissionSignal { name }` (am Ende des Enums), gesendet vom Missions-HUD nur für Signale, auf die ein scharfer Auslöser wartet | Läuft durch Lockstep und Replay wie jeder Befehl; die Sim bleibt die einzige Wahrheit. Nur der Missionsspieler darf signalisieren. |
| Reine Darstellung | Text und Kamerafahrt sind Zustand der Mission (letzte Nachricht/Kamera mit Tick), Godot zeigt sie an | Zustand statt Ereignis: übersteht mehrere Ticks pro Frame, Spielstände und Replays ohne Ereignis-Sammeln in `lib.rs`. |
| Gegner in Missionen | Normale KI-Spieler (`[[computer]]`, Beispiele: `passive`) plus Skript-Aktion `order` (FormationMove als Befehl des Besitzers, in der Sim) | KI bleibt normaler Spieler; Gegenstöße sind geskriptet und dadurch planbar. [vorläufig] |
| Sieg/Niederlage | `victory`/`defeat` entscheiden die Wertung mit `DefeatReason::Mission`; `elimination = false` lässt Gegner ohne Einheiten nicht verlieren (der Spieler verliert weiterhin mit der letzten Einheit) | Missionen mit späteren Verstärkungen und das Tutorial mit Übungszielen. |
| Dateien | Skript `data/maps/<karte>/missions/<id>.toml` (Handarbeit), Orte `data/maps/<karte>/mission_points.toml` (Editor) | „Neben der Karte“; mehrere Missionen pro Karte; der Editor schreibt nie in das handgeschriebene Skript (Kommentare bleiben). [vorläufig] |
| Format | Flache TOML-Tabellen mit `when = "…"` und Aktionen als Inline-Tabellen `{ do = "…" }`, Namen statt Indizes, Fehler mit Trigger-Namen | Gut von Hand schreibbar, `deny_unknown_fields` findet Tippfehler. |
| Auslöser | `time`, `area`, `destroyed`, `eliminated`, `captured`, `signal`, `units`; Ketten über `after`, `delay_s` | Deckt die geforderten (Tick, Gebiet betreten, Einheit zerstört, Punkt erobert) und das Tutorial (Bauen, Produzieren) ab. |
| Aktionen | `text`, `objective`, `spawn`, `order`, `fire`, `camera`, `victory`, `defeat` | Wie gefordert; `order`/`fire` für Gegenstöße und Übungsziele. Gebäude können nicht gespawnt werden (die Wirtschaft müsste sie registrieren). |
| Hash | Missionszustand nur im Hash, wenn es eine Mission gibt | Golden-Hash und Gefechte unverändert. |
| Formate | Replay-Format 9, Lobby-Protokoll 5 | Neues `SimConfig`-Feld und neuer Befehl ändern das postcard-Layout. |
| Tutorial | Eigene Mission auf `test_small` mit 8 Schritten (Kamera, Auswahl, Bewegen, Angreifen, Infanterie und Deckung, Bauen, Produzieren, Versorgungspunkt) | Jeder Schritt wartet per Auslöser auf die Handlung; kleine, schnell ladende Karte. |
| Beispielmissionen | Sowjetunion auf `twin_meadow`, USA auf `desert_village`, Deutschland auf `town_small` (Winter-Look) | Je Fraktion eine kleine Lage auf vorhandenen Karten, keine neuen Kartendaten. Keine historischen Verbände oder Abzeichen. |
| Editor | Werkzeuge „Trigger area“ und „Spawn point“ mit Namensfeld, Klick ins Gebiet entfernt; ohne Undo | Einfach; die Orte sind kein Teil der Kartendaten (Karten-Hash bleibt). [vorläufig] |
| Menü/Fortschritt | Hauptmenü „Tutorial“ und „Missions“ (Liste mit Briefing, „(won)“); `user://missions.cfg` | Wie gefordert; der Selbsttest nutzt eine eigene Datei. |
| Selbsttest | Autopilot folgt den Zielen (Angriff auf den nächsten sichtbaren Gegner beim Zielgebiet, sonst Bewegung dorthin; Bauen über die Platzierung der Basis-HUD); dieselbe Logik als Rust-Test in `crates/ai/tests/missions.rs` | Prüft jede Mission bis zum Sieg, ohne Missionsspezifisches im Test. |

## 5. Ergebnis

Umgesetzt wie oben, Referenz in `docs/MISSIONS.md`. Offen: Mehrspieler-/
Koop-Missionen, Kampagne mit Freischalten, Zielmarker auf der Minikarte,
Skript-Editor im Spiel, Übersetzung der Missionstexte (043).
