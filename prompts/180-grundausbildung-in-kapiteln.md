# Prompt 180 – Grundausbildung in Kapiteln: jede Kernmechanik einmal selbst gemacht

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach 175 (Hinweise
> bei verworfenen Befehlen helfen im Tutorial). Lies zuerst `CLAUDE.md`,
> `docs/MISSIONS.md` ganz, `docs/CONTROLS.md`, `docs/GAME_DESIGN.md`, in
> `docs/ARCHITECTURE.md` „Missions and tutorial“, dazu
> `data/maps/test_small/missions/tutorial.toml` (+ `.de.po`),
> `crates/sim/src/mission.rs`, `crates/sim/src/sim/mission.rs`,
> `crates/bridge/src/mission_funcs.rs`, `godot/scenes/hud/mission_hud.gd`,
> `prompts/done/039-*.md`.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) kritisiert stark: Informationen kommen über Hilfemenü und
Texttafeln statt über ein richtiges Tutorial; Spieler starten komplexe Missionen,
ohne Sicht, Tarnung, Nachschub oder Artillerie verstanden zu haben.
„Tutorials müssen spielerisch vermitteln.“

Bei uns gibt es ein Tutorial (Prompt 039) auf `test_small`: Kamera, Auswahl,
Bewegen, zwei Übungsziele angreifen, Trupp in den Wald, Kaserne, Trupp bauen,
Versorgungspunkt. Inzwischen hat das Spiel viel mehr: Sichtlinien und Nebel,
Deckung und Haltungen, Hinterhalt, Panzerseiten, indirektes Feuer und Rauch,
Luftunterstützung, Transport und Schleppen, Pioniere und Minen, Stellungen,
Formationen und Wegpunkte, taktische Pause, Nachschub und Reparatur, Wetter.
Nichts davon wird geübt.

## 2. Ziel

### 2.1 Kapitel

Die Grundausbildung wird zu einer Reihe **kurzer Kapitel** (je 3–6 Minuten),
jedes eine eigene Missionsdatei, in einer festen Reihenfolge im Menü
„Tutorial“; das bestehende Tutorial wird Kapitel 1. Vorschlag (vom Nutzer
bestätigen lassen, bevor alles gebaut wird):

1. Grundlagen (heutiges Tutorial, gestrafft).
2. Sehen und gesehen werden: Sichtlinie, Wald und Hecken, Höhen, Hinterhalt,
   Tarnung beim Stillstand.
3. Infanterie: Haltungen, Deckung, Gebäude besetzen, Niederhalten.
4. Panzer: Panzerseiten (frontal scheitert sichtbar, Flanke gelingt), Rumpf hinter
   Kamm, Panzerabwehr erkennen.
5. Unterstützung: Mörser und Artillerie, Rauch, Luftabruf, taktische Pause zum
   Koordinieren.
6. Bewegung im Verband: Formationen, Wegpunkte, Angriffsbewegung, Sichern,
   Transport und Schleppen.
7. Pioniere: Minen erkennen und räumen, Brücke reparieren/sprengen, Stellung
   graben.
8. Nachschub: Versorgungspunkte, Munition und Reparatur, was ein Verlust kostet.

### 2.2 Ablauf eines Kapitels

- Jede Mechanik wird **getan**, nicht gelesen: Schritt-Ziele mit Prüfung über die
  vorhandenen Auslöser; die Kamera zeigt den Ort; die passende Taste steht im
  Hinweis (aus der aktuellen Tastenbelegung, nicht fest im Text).
- Scheitern ist erlaubt und lehrreich: wer frontal gegen die Pak fährt, verliert
  den Panzer, bekommt einen Hinweis und den Panzer zurück (Skriptaktion), statt
  das Kapitel neu zu starten.
- Fehlende Skript-Bausteine (z. B. „Einheit hat Haltung X“, „Einheit steht im
  Wald“, „Taste gedrückt“, „Pause aktiv“) als neue Auslöser ergänzen,
  dokumentiert in `docs/MISSIONS.md`.
- Fortschritt je Kapitel in `user://missions.cfg`, Kapitel einzeln wählbar.

### 2.3 Nachschlagen im Spiel

- Kurze Hilfe-Karten je Mechanik (ein Satz, ein Bild, die Taste), aufrufbar aus
  dem Pausemenü und per Tooltip „?“ an Befehlskacheln; Inhalt aus denselben Texten
  wie die Kapitel.

## 3. Regeln

- Missionen sind Daten (`.toml` + `.de.po`); keine Sonderwege im Code für das
  Tutorial, neue Auslöser gelten für alle Missionen.
- Simulation: Auslöser deterministisch, Golden Hash darf sich nicht ändern (er
  nutzt keine Mission); wenn doch, begründen.
- Englische und deutsche Texte (`just mission-po`).
- Keine historischen Einheitennamen oder Abzeichen in Texten, wie bisher.

## 4. Vorgehen

1. Bestand lesen, Kapitelplan mit fehlenden Auslösern dem Nutzer vorlegen.
2. Auslöser mit Tests.
3. Kapitel nacheinander bauen; je Kapitel ein Selbsttest, der es mit Befehlen
   durchspielt (wie der Ablauf-Selbsttest).
4. Hilfe-Karten.
5. `just check`, `just selftest`.
6. Doku: `docs/MISSIONS.md` (Kapiteltabelle, neue Auslöser), `docs/CONTROLS.md`.
7. Für den Nutzer: je Kapitel ein Bild und die Spieldauer.
