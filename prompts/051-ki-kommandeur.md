# Prompt 051 – KI-Kommandeur Stufe 2

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/AI.md`, `docs/GAME_DESIGN.md`, `docs/ARCHITECTURE.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Die KI (024) erinnert sich an Gesehenes, sammelt, bündelt Feuer und zieht Angeschlagene
zurück. Nach diesem Prompt führt sie wie ein Kommandeur: Sie klärt auf, sammelt ihre
Armee, greift gemeinsam an, zieht sich bei Unterlegenheit zurück und kommt von einer
anderen Seite wieder. Sie sieht weiterhin nur, was ihre Einheiten gesehen haben.

## 2. Umfang

1. **Aufklärung:** schnelle Einheiten oder Aufklärer (033) klären gezielt auf; Wissen
   veraltet mit der Zeit.
2. **Lagebild:** Gefahrenkarte aus Verlusten und gesehenen Waffen („hier haben wir
   Männer verloren“), Kräfteverhältnis pro Front; Rückzug bei klarer Unterlegenheit,
   neuer Angriff über eine andere Achse.
3. **Konter:** Bau- und Produktionswahl nach gesehener Armee (Panzer → Pak, Infanterie →
   MG/Mörser, Flugzeuge → Flak).
4. **Versorgung:** kämpft um Versorgungspunkte, baut näher an der Front (Depots),
   schützt eigenen Nachschub und überfällt den des Gegners (036).
5. **Nutzt neue Systeme:** Angriffsbewegung und Formationen (046), Feuer und Bewegung
   (047), Artillerie/Rauch (032), Luftunterstützung (033), Transport (034), Pioniere (035),
   Teams (037).
6. **Aufgeben:** kapituliert, wenn die Lage aussichtslos ist (Schwelle begründen).
7. Turnier misst den Fortschritt gegen die alte KI (`Hard` alt vs. neu).

## 3. Akzeptanzkriterien

1. `just check` grün; neue KI gewinnt im Turnier klar gegen die alte `Hard`.
2. `just selftest`, `just lan-selftest` grün; Rechenzeit der KI pro Tick gemessen.
3. `docs/AI.md` aktualisiert.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
