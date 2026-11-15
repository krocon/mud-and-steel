# Prompt 035 – Pioniere, Minen, Hindernisse und Stellungen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Kartenobjekte, Zerstörung, Krater, Basisbau, Infanterie),
> `docs/MAPS.md`, `docs/GAME_DESIGN.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Minen, Stacheldraht, Panzersperren und Gräben gibt es als Modelle (007) und als
Kartenobjekte, aber niemand kann sie im Spiel anlegen oder räumen. Nach diesem Prompt
legen Pioniere Minenfelder und Hindernisse, heben Schützenlöcher und Gräben aus,
räumen Minen und reparieren oder sprengen Brücken.

## 2. Umfang

1. **Minen in der Sim:** Panzer- und Schützenminen als verdeckte Objekte (nur für den
   Besitzer sichtbar, für den Gegner erst nach Entdeckung durch Pioniere oder
   Auslösung), Auslösung nach Masse, Schaden an Ketten (Fahrzeug bewegungsunfähig) oder
   Infanterie. Im State-Hash, aber nicht im `PlayerView` des Gegners.
2. **Pionierbefehle:** `Lay` (Mine, Draht, Panzersperre an Position/Linie), `Clear`,
   `Dig` (Schützenloch, Graben: Deckung für 022, über Höhenfeld aus 021 oder als
   Objekt), `Repair`/`Demolish` (Brücken aus 014). Bauzeiten und Kosten (023).
   Mauern und Minen als baubare Objekte stehen in `GAME_DESIGN.md` unter „Open“.
3. **Wegfindung:** bekannte Minenfelder meiden (pro Spieler!), ohne das gemeinsame
   `PathGrid` spielerabhängig zu machen; Lösung entscheiden und begründen.
4. **KI:** legt Minen an Engstellen, räumt sie mit Pionieren.
5. **Darstellung:** Minenmarker nur für den Besitzer/Entdecker, Grabungseffekt, Kette
   abgesprungen am Fahrzeug.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests: Mine löst nur nach Masse aus, Gegner sieht sie nicht
   vorher, Räumen entfernt sie, Graben gibt Deckung, Brücke repariert/gesprengt
   ändert Wege; gleicher Hash; Golden-Hash bewusst aktualisiert.
2. `just selftest`, `just lan-selftest` (Minen dürfen keinen Desync auslösen),
   `just editor-selftest` (Minenfelder im Editor platzierbar) grün.
3. Doku aktualisiert, `GAME_DESIGN.md` „Open“ bereinigt.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
