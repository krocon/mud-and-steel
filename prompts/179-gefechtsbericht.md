# Prompt 179 – Gefechtsbericht: warum gewonnen, warum verloren

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/BALANCE.md` (Ergebnisbildschirm und `user://stats/*.json`),
> `docs/PLAYTEST.md`, in `docs/ARCHITECTURE.md` „Victory conditions and game
> flow“, „Saved games and replay viewer“, „Replays“, dazu `crates/sim/src/stats.rs`
> (`MatchStats`), die Kampf-Ereignisse in `crates/sim/src/combat.rs`, den
> Ergebnisbildschirm unter `godot/scenes/` und `prompts/done/045-*.md`.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung): Missionen sind anspruchsvoll, und das ist gut. Frust entsteht,
wenn Spieler nicht verstehen, **warum** ein Angriff gescheitert ist und welche
Gegenmaßnahme geholfen hätte. „Niederlagen sollen sich auf nachvollziehbare
Entscheidungen zurückführen lassen.“

Bei uns sammelt `MatchStats` Verluste, Abschüsse und Abschusswert je
Einheitentyp, Produktion, Versorgungspunkte, Konto und alle 30 s den Verlauf.
Der Ergebnisbildschirm zeigt Zahlen, aber keine Ursachen.

## 2. Ziel

### 2.1 Mehr erfassen (Simulation, nur Statistik)

Je Verlust zusätzlich, soweit die Simulation es beim Ereignis weiß:

- Ursache: direktes Feuer (Waffe und Typ des Schützen), indirektes Feuer, Mine,
  Luftangriff, Überrollen, Brand, Ertrinken/Brückeneinsturz;
- Trefferseite (vorn, Seite, hinten, oben) bei Fahrzeugen;
- ob der Schütze im Moment des Treffers für den Verlierer **sichtbar** war
  (Hinterhalt, unbekannte Stellung);
- Ort (grob, für eine Karte).

Nicht Teil des Zustands-Hashs, wenn das geht (wie die vorhandene Statistik);
sonst begründen.

### 2.2 Bericht (Darstellung)

- Neuer Reiter „Bericht“ auf dem Ergebnisbildschirm, auch für Missionen:
  - Verlustkarte: Minikarte mit Punkten je Verlust, Farbe nach Ursache;
  - „Was dich am meisten gekostet hat“: die drei teuersten Ursachen (z. B.
    „6 Panzer durch Pak aus unbekannter Stellung, 4 davon von der Seite“);
  - kurze, regelbasierte **Hinweise** zu diesen Ursachen aus einer Tabelle in
    `data/` (Beispiele: unbekannte Pak → „vorher aufklären, Rauch, Infanterie
    vor“; Minen → „Pioniere räumen lassen“; Verluste von hinten → „Flanke
    sichern“). Keine KI-Texte, nur Tabelle; Deutsch und Englisch.
  - Verlauf der Stärke beider Seiten aus den 30-s-Werten (vorhanden) mit
    Markierung der größten Einbrüche; Klick darauf öffnet, wenn ein Replay
    gespeichert ist, den Replay-Betrachter an dieser Stelle.
- Gegnerische Einheiten im Bericht nur, soweit der Spieler sie gesehen hat; nach
  Spielende darf der Bericht aufdecken, was unbekannt war („du wurdest von 2 Pak
  beschossen, die du nie gesehen hast“). Entscheidung dokumentieren.

## 3. Regeln

- Die Simulation entscheidet nichts Neues; nur zusätzliche Statistik.
- Daten der Hinweise in `data/`, Texte über die Übersetzung (Prompt 043).
- Lesbar mit Farbschwäche-Paletten und UI-Skalierung.
- Netzwerkspiele: beide Seiten sehen ihren eigenen Bericht; nichts, was der
  Gegner während des Spiels nicht wissen durfte, wird vor Spielende gezeigt.

## 4. Vorgehen

1. Bestand lesen; Liste der Verlust-Ursachen, die die Simulation unterscheidet.
2. Statistik erweitern mit Tests (jede Ursache einmal).
3. Bericht im Ergebnisbildschirm, Hinweistabelle, Sprung ins Replay.
4. Selbsttest: Bericht erscheint, enthält mindestens eine Ursache.
5. `just check`, `just selftest`, `just lan-selftest`.
6. Doku: `docs/BALANCE.md`, `docs/ARCHITECTURE.md`, `docs/PLAYTEST.md`.
7. Für den Nutzer: Bilder des Berichts nach einer verlorenen Mission und nach
   einem Gefecht gegen die KI.
