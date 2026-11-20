# Prompt 102 – Taktische Pause: anhalten und trotzdem Befehle geben

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, `docs/CONTROLS.md`, `prompts/done/027-*.md`,
> `godot/scenes/menu/game_menu.gd` (`open_pause`, `get_tree().paused`),
> `godot/scenes/replay/replay_bar.gd` und `game_files.gd` (`sim_host.speed`,
> `SPEEDS`), in `crates/bridge` die Stelle, an der der Takt der Simulation
> vorangetrieben und Befehle einem Tick zugeordnet werden, `docs/LAN.md` und
> `prompts/095-hud-neu-anordnen.md` (Pause-Knopf neben der Minimap).

## 1. Ausgangslage

Im Test zum Referenzvideo (`docs/.vorlagen/vorlage.mp4`, nur als Anregung) lobt der
Tester die **taktische Pause auf der Leertaste**: Spiel anhalten, in Ruhe Befehle
geben, weiterlaufen lassen. Für ein Spiel mit vielen Einheiten und langsamem,
schwerem Tempo (Prompt 097) passt das gut.

Bei uns heute:

- Esc öffnet das Pausenmenü; im Einzelspiel hält `get_tree().paused` alles an,
  auch Auswahl, Kamera und HUD, und ein dunkler Schleier liegt über dem Bild.
  Befehle sind in der Pause nicht möglich.
- Im Spiel gibt es keine Pausentaste und keine Spielgeschwindigkeit. Die
  Wiederholung hat Pause und 1/2/4/8-fach über `sim_host.speed`.
- Prompt 095 sieht einen Pause-Knopf vor, legt aber kein Verhalten fest.
- Die Leertaste ist mit `event_jump` belegt.

## 2. Ziel

- **Taktische Pause im Einzelspiel** (Gefecht gegen KI und Missionen): eine
  belegbare Taste und der Knopf aus 095 halten die Simulation an. Kamera, Auswahl,
  HUD, Minimap, Tooltips, Kreise und Vorschauen laufen weiter.
- **Befehle in der Pause** sind erlaubt und werden gesammelt: Sie gelten ab dem
  ersten Tick nach dem Weiterlaufen, in der Reihenfolge der Eingabe. Wegpunkte,
  Routen und Zielmarken sind schon in der Pause zu sehen.
- **Deutlich sichtbar**, dass pausiert ist: ein ruhiger Hinweis am Bildrand
  (kein Schleier über dem Bild), Partikel und Animationen stehen.
- **Ton:** Gefechts- und Fahrgeräusche halten an oder blenden weich aus, Musik
  läuft leiser weiter. Kein hängender Dauerton.
- **Geschwindigkeit** (nur Einzelspiel): zusätzlich halbe und doppelte
  Geschwindigkeit, wenn `sim_host.speed` das ohne Umbau hergibt. Sonst nur Pause,
  und im Ergebnis sagen, was fehlt.
- **Mehrspieler (LAN, online):** keine taktische Pause. Taste und Knopf sind dort
  ausgeblendet bzw. ohne Wirkung; das Pausenmenü verhält sich wie heute.

## 3. Umsetzung

- Die Simulation bleibt unberührt und deterministisch: In der Pause wird schlicht
  kein Tick gerechnet. Befehle bekommen den nächsten Tick wie sonst auch. Der
  Golden Hash und Wiederholungen dürfen sich nicht ändern: Eine Aufzeichnung mit
  Pausen muss beim Abspielen denselben Zustand ergeben (Pause ist keine Eingabe
  der Simulation).
- Nicht `get_tree().paused` für die taktische Pause benutzen, sonst steht das HUD.
  Stattdessen den Takt der Simulation anhalten und alles, was von Bildzeit lebt
  (Partikel, Animationen, Interpolation zwischen Ticks, Rauch, Wasser, Wind),
  gezielt anhalten. Vorher auflisten, was heute an Bildzeit hängt, und prüfen, dass
  nach dem Weiterlaufen nichts springt (Einheiten dürfen nicht zum Zielpunkt
  „teleportieren“, weil die Interpolation weiterlief).
- Missionsuhr, Auslöser, Einkommen, Bauzeiten, Abklingzeiten der Luftleiste stehen
  mit (sie laufen in Ticks; nur prüfen, dass keine HUD-Anzeige mit Wanduhr zählt).
- Das Pausenmenü (Esc) bleibt wie es ist. Öffnet man es aus der taktischen Pause
  und schließt es wieder, ist weiter pausiert.
- Taste: Vorschlag machen, ob die Leertaste die Pause bekommt und `event_jump`
  umzieht, oder umgekehrt. Nicht ohne Rückfrage eine bestehende Belegung
  ändern; beide Varianten vorbereiten und den Nutzer entscheiden lassen.

## 4. Vorgehen

1. Bestand lesen, Liste „was läuft mit Bildzeit“ aufstellen.
2. Pause des Takts, Hinweis, Ton.
3. Befehle in der Pause, Vorschau der gesammelten Befehle.
4. Geschwindigkeit halb/doppelt, falls einfach.
5. Selftest: Pause → Tickzähler steht, Kamera bewegt sich, Befehl wird angenommen
   und ab dem nächsten Tick ausgeführt; Aufzeichnung mit Pause ergibt denselben
   Hash wie ohne; im LAN-Selbsttest ist die Pause nicht verfügbar.
6. `just check`, `just selftest`, `just lan-selftest`, `just export-selftest`.
7. Doku: `docs/CONTROLS.md`, `docs/ARCHITECTURE.md`, `docs/GAME_DESIGN.md`
   (Entscheidung mit Datum).
8. Für den Nutzer: die offene Frage zur Tastenbelegung und ein kurzer Clip.

## 5. Nicht Teil dieses Prompts

- Pause im Mehrspieler (Abstimmung, Pausenzahl je Spieler).
- Zeitlupe als Effekt oder automatische Pause bei Ereignissen.
