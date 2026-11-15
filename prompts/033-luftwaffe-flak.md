# Prompt 033 – Flugzeuge und Flugabwehr

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Kampf, Nebel, Basisbau, KI), `docs/SCALE.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Die Flugzeug-Modelle aus 008 (Aufklärer, Jäger, Jagdbomber, Bomber je Fraktion) und die
Flak sind gebaut, im Spiel aber nicht nutzbar. Nach diesem Prompt kann ein Spieler
Luftunterstützung anfordern und sich mit Flak dagegen wehren.

## 2. Leitfrage (zuerst entscheiden)

Steuerbare Flugzeuge wären auf 1:1-Karten mit Zeitfaktor 2 in Sekunden über die Karte
hinweg. Vorschlag: **Luftunterstützung als Abruf** (Aufklärungsflug, Tiefangriff,
Bombenabwurf auf einen Zielpunkt) mit Abklingzeit und Kosten (Ressourcen aus 023),
Flugzeuge fliegen über einen festen Anflugweg vom Kartenrand und wieder hinaus.
Alternative (steuerbare Staffeln) bewerten und begründet ablehnen oder wählen.

## 3. Umfang

1. **Sim:** Lufteinheiten mit Höhe (eigene Ebene, keine Bodenkollision, keine
   Wegfindung), Anflug, Angriff (MG/Kanone als Strafing-Linie, Bomben mit Streuung wie
   032), Abflug. Sichtbarkeit: Flugzeuge sind in Sichtweite jeder Einheit sichtbar
   (große Höhe), Aufklärer decken entlang des Wegs Nebel auf.
2. **Flugabwehr:** Flak und MGs mit Flugabwehr-Eigenschaft (Katalog) schießen auf
   Flugzeuge in Reichweite; Abschuss mit Absturz (Wrack/Explosion am Boden).
3. **Bedienung:** Abruf-Leiste (mit 031), Zielwahl auf der Karte, Anzeige von Anflug
   und Abklingzeit; Warnung „feindliche Flugzeuge“ für den Gegner.
4. **KI:** ruft Unterstützung gegen Ansammlungen und baut Flak, wenn sie angegriffen wird.
5. **Darstellung:** Flugzeug-Modelle mit Schatten am Boden, Propeller, Kondens- und
   Rauchfahne beim Treffer, Bombeneinschläge über Effekte und Krater.

## 4. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Anflug, Bombenwirkung, Flak-Abschuss, Aufklärung,
   gleicher Hash bei zwei Läufen; Golden-Hash bewusst aktualisiert.
2. `just selftest`, `just lan-selftest`, `just ai-tournament` grün.
3. Doku und Katalog-Werte (Geschwindigkeit, Bewaffnung) mit Quellen; Umrechnung in
   `docs/SCALE.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
