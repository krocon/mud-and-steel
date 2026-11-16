# Prompt 049 – Verwundung und Kameradenhilfe

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/ARCHITECTURE.md` (Infanterie, Kampf, Transport aus 034).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Ein Treffer ist bisher Lebenspunktabzug oder Tod. Nach diesem Prompt werden Soldaten
verwundet, versorgen sich, helfen einander und werden geborgen; das macht Infanterie
menschlicher und gibt Sanitätern eine Aufgabe.

## 2. Umfang

1. **Zustände (Sim):** leicht verwundet (humpelt, langsamer, bis er sich verbindet),
   schwer verwundet (liegt, kann nicht kämpfen, stirbt ohne Hilfe nach Zeit),
   versorgt (sichtbarer Verband, humpelt eine Weile). Regeln mit Begründung.
2. **Kameradenhilfe:** Nachbarn ziehen Verwundete aus der Schusslinie in Deckung (an der
   Schulter/am Kragen, Seil aus Deckung als Variante), helfen nach Explosionen auf.
3. **Sanitäter:** versorgen schwer Verwundete, tragen sie mit Trage aus Gräben/zum
   Sammelpunkt; Sanitäter mit weißer Armbinde und Tasche, **kein Rotes Kreuz**
   (`docs/SOURCES.md`).
4. **Ausgestiegene Besatzungen:** bei zerstörten Panzern steigen Überlebende aus (Anzahl
   nach Katalog-Besatzung und Zufall aus `sim::Rng`) und sind ein kleiner Trupp mit
   Pistolen/MPs; sie können ein Fahrzeug gleicher Art wieder besetzen (optional).
5. **Gefangene** [provisional]: umzingelte, niedergehaltene Soldaten ohne Ausweg ergeben
   sich; eigene Gefangene können befreit werden. Nur umsetzen, wenn es einfach bleibt.
6. **HUD:** Verwundete in der Einheitenanzeige, Ereignis „Mann verwundet“.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für alle Zustände, Bergung, Besatzung steigt aus,
   gleicher Hash; Golden-Hash begründet.
2. `just selftest`, `just lan-selftest`, `just ai-tournament` grün.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
