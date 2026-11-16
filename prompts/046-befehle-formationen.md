# Prompt 046 – Befehle und Formationen Stufe 2

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, `docs/ARCHITECTURE.md` (Auswahl, Befehlsketten aus 031, Wegfindung).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Steuergruppen, Wegpunkte, Patrouille und Formationsbewegung gibt es (009, 031). Es fehlen
die Befehle, mit denen man eine Truppe wirklich führt: vorrücken und dabei kämpfen, ein
Gebiet sichern, Formationen bewusst wählen und Panzer gezielt ausrichten.

## 2. Umfang

1. **Angriffsbewegung** (`AttackMove`): zum Ziel fahren, unterwegs auf Feinde in
   Reichweite reagieren, danach weiter. Standard für Infanterie, wählbar für Fahrzeuge.
2. **Gebiet sichern** (`Secure`): Gebiet (Kreis) halten, Feinde darin bekämpfen, nicht
   weiter als ein Radius verfolgen, danach zurück in Stellung.
3. **Formationen:** Linie, Keil, Kolonne, Breitkeil, aufgelockert; **Auto** wählt nach
   Gelände vor der Gruppe (Straße/Engstelle → Kolonne, offenes Feld → Keil/Linie, Wald →
   aufgelockert). Deterministisch in der Sim, Wahl im HUD (031).
4. **Ausrichtung per Ziehen:** Rechtsklick-Ziehen legt Zielpunkt und Blickrichtung fest;
   die Formation wird senkrecht dazu aufgestellt, Panzer zeigen die Frontpanzerung.
5. **Eigene Trupps:** prüfen, ob die Steuergruppen (031) als „Trupps“ mit Namen und
   gemeinsamer Formation reichen; sonst schlank ergänzen.
6. **KI** nutzt Angriffsbewegung und Ausrichtung zum Gegner.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für AttackMove, Secure, Auto-Formation, Ausrichtung;
   neue `Command`s hinten ans Enum; Golden-Hash begründet.
2. `just selftest`, `just lan-selftest` grün; `docs/CONTROLS.md` aktualisiert.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
