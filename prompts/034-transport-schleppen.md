# Prompt 034 – Transport, Aufsitzen und Geschütze schleppen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Einheiten, Infanterie, Wegfindung, Kampf), `docs/INFANTRY.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

LKW und Schützenpanzerwagen fahren bisher leer herum, Pak und Feldgeschütze stehen fest.
Nach diesem Prompt sitzen Infanterietrupps auf und ab, gezogene Geschütze werden von
Zugmaschinen verlegt und abgeprotzt, Panzer können Infanterie aufsitzen lassen
(Tank-Desant, z. B. sowjetisch).

## 2. Umfang

1. **Sim:** Transportkapazität aus dem Katalog (Sitzplätze, Zugkraft), neue
   `Command`s `Embark`, `Disembark`, `Tow`, `Unlimber` (hinten ans Enum).
   Aufgesessene Einheiten bewegen sich mit dem Fahrzeug, sind unsichtbar/geschützt
   je nach Fahrzeug (offen vs. geschlossen), sterben ganz oder teilweise bei Zerstörung.
   Aufsitz-, Absitz- und Abprotzzeiten (Spielregeln mit Begründung, Zeitfaktor 2).
2. **Geschütze:** gezogene Geschütze sind ohne Zugmaschine nur langsam von der
   Bedienung verschiebbar; abgeprotzt schussbereit, aufgeprotzt nicht.
3. **Bedienung:** Rechtsklick mit Infanterie auf eigenes Fahrzeug = aufsitzen; Taste für
   Absitzen; Anzeige der Insassen (mit 031).
4. **KI:** fährt Infanterie mit LKW an die Front, verlegt Geschütze.
5. **Darstellung:** Soldaten auf offenen Fahrzeugen sichtbar sitzend, Geschütz an der
   Anhängerkupplung (Sockel aus dem Baukasten prüfen/ergänzen).

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests: Auf- und Absitzen, Verluste bei Zerstörung, Schleppen
   und Abprotzen, deterministischer Hash; Golden-Hash bewusst aktualisiert.
2. `just selftest`, `just lan-selftest`, `just gallery-check` grün.
3. Doku: `docs/ARCHITECTURE.md`, `docs/INFANTRY.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
