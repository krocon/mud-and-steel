# Prompt 054 – Explosionen Stufe 2 und schwere Feuerschläge

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Effekte, indirektes Feuer, Licht).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Große Explosionen sollen wuchtig aussehen und spielerisch wirken: Druckwelle, Blitz, der
den Boden beleuchtet, Munitionsexplosionen, die Soldaten umwerfen. Als spätes
Spielelement kommt ein schwerer Feuerschlag mit Vorwarnung, passend zur Epoche.

## 2. Umfang

1. **Druckwelle:** weiße, sich ausdehnende Kuppel/Ring bei großen Explosionen
   (Kondensation), Staubring am Boden, Bildschirmwackeln nach Entfernung.
2. **Blitzlicht:** kurzes Licht, das Gelände und Gebäude beleuchtet (bei Nacht aus 040
   besonders), Budget an Lichtquellen.
3. **Munitionsexplosion** eines Panzers: Turm fliegt ab (Darstellung), Druck wirft
   Infanterie im Radius um (Regel aus 048), Splitter.
4. **Schwerer Feuerschlag** [provisional, WW2-passend statt moderner Waffen]: z. B.
   Eisenbahngeschütz oder Raketenwerfer-Batterie, im späten Spiel freischaltbar,
   Abschussstellung auf der Karte, die der Gegner in einer Frist zerstören kann; farbiger
   Rauch markiert den Zielpunkt, Sirenen-/Warnton, dann ein Flächenschlag mit Kratern.
5. Alles über die Effekte aus 019 und Sound aus 029/053.

## 3. Akzeptanzkriterien

1. `just check` grün (Feuerschlag als Sim-Regel getestet); Golden-Hash begründet.
2. `just selftest` grün; Schlacht-Benchmark hält das Budget.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
