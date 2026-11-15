# Prompt 031 – HUD: Minikarte, Einheitenanzeige, Befehlsleiste, Wegpunkte

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Auswahl, Kamera, Nebel, Basisbau-HUD, Menüs).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Auf großen Karten fehlt die Übersicht: Es gibt keine Minikarte, keine Anzeige der
gewählten Einheiten und keine Befehlsleiste; alle Befehle laufen über Tasten und
Rechtsklick. Nach diesem Prompt sieht der Spieler jederzeit, wo was los ist, was er
ausgewählt hat und was er damit tun kann.

## 2. Umfang

1. **Minikarte** (Ecke unten): Gelände (Höhe, Wasser, Wald, Straßen) aus den
   Kartendaten vorab gerendert, eigene und sichtbare fremde Einheiten als Punkte in
   Spielerfarbe, Nebel des Krieges, Kamerasichtfeld als Trapez. Klick springt,
   Ziehen schwenkt, Rechtsklick gibt einen Bewegungsbefehl. Kampf-Pings bei Beschuss
   eigener Einheiten. Performance: Punkte per MultiMesh/Shader, nicht pro Node.
2. **Einheitenanzeige:** eine Einheit: Name, Bild (Galerie-Thumbnail), Lebenspunkte,
   Panzerung, Waffen, Haltung, Feuermodus. Mehrere: Raster mit Symbolen und Lebensbalken,
   Klick wählt eine, Shift-Klick entfernt sie. Lebensbalken über beschädigten
   Einheiten im Spiel (Option „immer / bei Auswahl / nie“).
3. **Befehlsleiste:** Schaltflächen für Bewegen, Angreifen, Bodenziel, Stopp,
   Feuermodus, Haltungen, Besetzen, Bauen/Produzieren (das Basisbau-HUD aus 023 geht
   darin auf); zeigt die Tastenbelegung aus den Einstellungen.
4. **Befehlsketten:** Shift+Rechtsklick hängt Wegpunkte an, sichtbar als Linie mit
   Markern. Dafür braucht die Sim eine Befehlswarteschlange pro Einheit (neuer
   `Command` oder Flag `queue`, hinten an das Enum, damit alte Replays lesbar bleiben;
   Golden-Hash prüfen). Optional: Patrouille, Formationswahl (Linie, Keil, Kolonne),
   Ausrichtung am Ziel per Rechtsklick-Ziehen.
5. **Ereignisanzeige:** knappe Meldungen (Einheit verloren, Bau fertig, Gegner
   gesichtet) mit Klick zum Hinspringen; Leertaste springt zum letzten Ereignis.
6. **Selbsttest:** Minikarte klicken/ziehen, Auswahl über die Anzeige ändern,
   Wegpunktkette fahren, Befehl über die Leiste geben.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Befehlsketten (deterministisch, im Replay).
2. `just selftest`, `just lan-selftest` grün; Befehlsketten auch im LAN-Test.
3. Schlacht-Benchmark: HUD kostet höchstens 1 ms pro Frame.
4. Doku: Steuerung in `docs/ARCHITECTURE.md` und eine Spieler-Steuerungsübersicht
   (`docs/CONTROLS.md`), die auch das Tutorial (039) nutzt.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
