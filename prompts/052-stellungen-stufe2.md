# Prompt 052 – Stellungen Stufe 2: Gräben, Sperren, Tarnung

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Pioniere aus 035, Krater, Infanterie), `docs/MAPS.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

035 bringt Minen, Draht, Gräben und Pionierbefehle. Dieser Prompt macht Stellungen zum
Kernstück der Verteidigung: Gräben sehen von Hand gegraben aus, Soldaten kämpfen von der
Grabenwand, Sperren werden als Linie gezogen, Panzer graben sich ein, Tarnnetze
verbergen Stellungen vor Luftaufklärung.

## 2. Umfang

1. **Gräben:** unregelmäßige, handgegrabene Optik mit Brustwehr; Zickzack-Verlauf; als
   Linie zeichnen wie in 035; Soldaten besetzen Feuerplätze an der Wand und an Ecken.
2. **Sperren als Linie zeichnen:** Stacheldraht, Panzersperren (Höcker, Igel) entlang
   einer gezogenen Linie, Kosten pro Meter.
3. **Panzer eingraben:** Pioniere heben eine Fahrzeugstellung aus; nur Turm sichtbar
   und treffbar (Trefferzonen aus 017 anpassen), Fahrzeug kann nicht fahren, bis es
   ausfährt.
4. **Tarnnetze:** über Stellungen/Fahrzeugen; verbergen vor Luftaufklärung (033) und
   verringern Sichtweite des Gegners, solange nicht gefeuert wird.
5. **Bunker und MG-Nester** am Grabenrand als baubare Stellungen (Modelle aus 041).
6. **KI** baut Stellungen an Engstellen, wenn sie verteidigt.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Eingraben (nur Turm trifft), Tarnnetz und
   Sperrlinien; Golden-Hash begründet.
2. `just selftest`, `just editor-selftest`, `just gallery-check` grün.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
