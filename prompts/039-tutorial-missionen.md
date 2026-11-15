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
