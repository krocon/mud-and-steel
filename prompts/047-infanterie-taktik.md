# Prompt 047 – Infanterie-Taktik: Feuer und Bewegung

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/ARCHITECTURE.md` (Infanterie, Kampf, indirektes Feuer).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Trupps bewegen sich bisher als Block und suchen Deckung erst am Ziel (022). Nach diesem
Prompt kämpfen sie wie eine Infanteriegruppe: Sie teilen sich in zwei Feuertrupps, einer
gibt Feuerschutz, der andere springt vor, von Deckung zu Deckung.

## 2. Umfang

1. **Feuertrupps:** Trupp teilt sich automatisch (z. B. MG-Gruppe und Schützen),
   abwechselnd Feuerschutz und Sprung, sobald Feindkontakt besteht oder Angriffsbewegung
   (046) befohlen ist.
2. **Deckung zu Deckung:** Wegpunkte entlang Deckungspunkten (Mauern, Krater, Gräben,
   Ruinen) statt der kürzesten Linie; Bewegung unter Beschuss geduckt/kriechend.
3. **Kämpfen unterwegs:** auf dem Marsch erwidern sie Feuer, statt stur weiterzulaufen.
4. **Nahe Feuergefechte liegend**, Blindfeuer um Ecken und über Deckung (geringe
   Treffgenauigkeit, Niederhalten).
5. **Niederhalten** (Suppression): Beschuss senkt Genauigkeit und Bewegung, Trupp geht
   zu Boden; wichtig für Feuer und Bewegung. Spielregel mit Begründung.
6. **Mörsertrupps** bauen selbständig auf und bekämpfen erkannte Ziele (032).
7. **Ladehemmung** als seltene Spielregel (optional, abschaltbar), aus `sim::Rng`.
8. Darstellung über vorhandene Haltungen und Animationen; neue Animationen kommen in 048.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests: Trupp springt abwechselnd vor, nutzt Deckung,
   Niederhalten wirkt, gleicher Hash; Golden-Hash begründet.
2. `just selftest`, `just ai-tournament` grün; Infanterie-Benchmark aus 016 hält.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
