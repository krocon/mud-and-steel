# Prompt 055 – Fahrzeuge und Luftlande Stufe 3: Brücken, Pioniertechnik, Fallschirmjäger

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/UNIT_CATALOG.md`, `docs/ARCHITECTURE.md` (Brücken, Zerstörung, Transport, Luft), `docs/SOURCES.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Flüsse und Brücken sollen das Schlachtfeld prägen: Brücken brechen auseinander und fallen
in den Fluss, Männer können ertrinken, Brückenlegepanzer schlagen neue Übergänge. Dazu
Luftlandungen und Fahrzeugtarnung aus der Epoche.

## 2. Umfang

1. **Brücken zerstören:** Teile stürzen sichtbar ins Wasser (Darstellung), Übergang
   gesperrt; Einheiten auf der Brücke fallen ins Wasser; Infanterie im tiefen Wasser
   ertrinkt nach Zeit, wenn sie nicht ans Ufer kommt (Sim-Regel).
2. **Brückenleger:** Brückenlegepanzer je Fraktion, soweit historisch belegt (z. B.
   Brückenleger IV, Valentine Bridgelayer, Churchill ARK in US-Nutzung prüfen), legt eine
   Brücke über schmale Gewässer und Gräben und nimmt sie wieder auf. Katalog mit Quelle,
   Modell aus dem Baukasten.
3. **Fahrzeugtarnung:** Zweige und Tarnnetze an Panzern als Material-/Anbauvariante
   (verringert Entdeckungsweite im Stand); grüne Tarnung als vierte Tarnvariante prüfen.
4. **Luftlandung:** Fallschirmjäger oder Lastensegler je Fraktion als Abruf (033), Absprung
   mit Streuung; Bombenteppich per Ziehen zielen (Richtung und Länge).
5. **KI** nutzt Brückenleger an Flüssen.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Brückeneinsturz, Ertrinken, Brücke legen;
   Golden-Hash begründet.
2. `just gallery-check`, `just selftest`, `just lan-selftest` grün.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
