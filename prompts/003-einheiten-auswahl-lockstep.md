# Prompt 003 – Mehrere Einheiten, Auswahl und Lockstep im Spiel

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` und `docs/DETERMINISM.md`. Deren Regeln gelten unverändert.

## 1. Ziel

Vom Einzel-Panzer zum steuerbaren Trupp. Gleichzeitig läuft das Spiel ab jetzt auch im
Einzelspieler über `net::LockstepSession`, damit Einzel- und Mehrspieler denselben
Codepfad haben. Replays machen jede Partie reproduzierbar.

## 2. Umfang

### Simulation (`crates/sim`)
- Einheiten gehören einem Spieler (`PlayerId` wandert dafür nach `sim` oder wird dort
  definiert; `net` nutzt dann diesen Typ).
- `Command::MoveTo` wird zu einem Befehl für eine Liste von Einheiten. Die Sim
  verteilt die Ziele als einfache Formation (Raster um den Zielpunkt, Abstand aus den
  Einheitendaten), deterministisch und unabhängig von der Reihenfolge im Befehl.
- Befehle an fremde Einheiten werden ignoriert (Spieler-ID kommt aus dem
  Lockstep-Frame, nicht aus dem Befehl).
- `Command::Stop`.
- Die Startaufstellung kommt aus der Szenario-Konfiguration: zwei Spieler mit je acht
  Panzern.
- Golden-Hash bewusst neu festlegen und den Grund im Commit nennen.

### Replays
- Format: Seed, Konfiguration, alle Befehls-Frames pro Tick, End-Hash. Binär mit
  `postcard`, versioniert (Formatversion im Header).
- Das Spiel schreibt beim Beenden ein Replay nach `user://replays/`.
- `just replay <datei>` bzw. `cli replay <datei>` spielt es headless ab und meldet, ob
  der End-Hash stimmt.

### Bridge und Godot
- `SimHost` nutzt intern `LockstepSession` mit einem lokalen Spieler (Eingabeverzögerung
  konfigurierbar, im Einzelspieler 1 Tick).
- Snapshots enthalten Besitzer und Auswahl-relevante Daten. Die Bridge liefert sie
  weiterhin als Packed-Arrays.
- **Auswahl** (reine Darstellung, aber nur eigene Einheiten):
  - Linksklick wählt eine Einheit, Shift ergänzt, Ziehen erzeugt einen Auswahlrahmen.
  - Doppelklick wählt alle sichtbaren Einheiten desselben Typs.
  - Strg+1…9 speichert Gruppen, 1…9 ruft sie ab, zweimal drücken zentriert die Kamera.
  - Auswahlring unter ausgewählten Einheiten, Lebens- oder Statusbalken noch nicht.
- Rechtsklick sendet die Auswahl zum Zielpunkt. Kurze Zielmarkierung als Rückmeldung.
- Einheiten des zweiten Spielers sind farblich unterscheidbar und nicht auswählbar.

## 3. Nicht-Ziele

Wegfindung um Hindernisse (006), Kampf (012), echter Netzwerk-Transport (020).

## 4. Akzeptanzkriterien

1. `just check` ist grün, inklusive neuem Golden-Hash.
2. Ein Test lässt zwei Peers über Loopback das neue Szenario mit Gruppenbefehlen
   spielen und vergleicht die Hashes pro Tick.
3. Ein Test nimmt ein Replay auf, spielt es ab und bekommt denselben End-Hash.
4. `just selftest` deckt Auswahl und Gruppenbefehl ab (per Skript ausgelöst).
5. Beschreibe mir, wie ich Auswahl, Gruppen und Replays selbst teste.
6. Doku (`ARCHITECTURE.md`, `DETERMINISM.md`) ist aktualisiert.

## 5. Arbeitsweise

- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe.
- Erst kurz prüfen und einen Plan zeigen, dann umsetzen. Bei schwer umkehrbaren
  Entscheidungen (Replay-Format, Befehlsstruktur) vorher fragen.
- godot-rust-APIs in der Doku von 0.5.5 nachschlagen. Kein Test wird abgeschwächt.
- Pro Schritt ein Commit. Am Ende diese Datei nach `prompts/done/` verschieben
  (`git mv`), Status in `prompts/README.md` aktualisieren, committen.
- Abschlussbericht: was gebaut wurde, wie ich es teste, was offen ist.
