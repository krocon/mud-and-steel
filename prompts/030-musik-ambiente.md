# Prompt 030 – Musik und Umgebungsklang

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Audio aus Prompt 029, Licht/Biome, Menüs).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Das Spiel bekommt eine Klangkulisse: Wind, Vögel, Insekten, Regen (mit 040), Wasser an
Flüssen, fernes Grollen; dazu Musik in Menü, Ladebildschirm, Spiel und Ergebnis.

## 2. Umfang

1. **Ambiente** nach Biom (Wiese/Wald, Wüste, Winter) und nach Kamerahöhe gemischt
   (nah: Details wie Vögel und Wasser; fern: Wind). Wasserflächen und Wälder auf der
   Karte als Klangquellen, ohne einen Player pro Baum (Rasterabfrage an der Hörmitte).
   Vögel verstummen eine Weile nach Gefechtslärm in der Nähe.
2. **Musik:** wenige ruhige Stücke ohne Pathos, Wechsel zwischen „ruhig“ und „Gefecht“
   nach Kampfintensität mit Überblendung, eigene Stücke für Menü und Ergebnis
   (Sieg/Niederlage). Herkunft: selbst komponiert/erzeugt, CC0 oder vom Nutzer
   bereitgestellt (auch KI-generiert), jeweils mit Lizenz in `docs/SOURCES.md`.
   Keine Musik aus anderen Spielen oder Filmen, keine historischen Aufnahmen mit
   unklaren Rechten.
3. **Einstellungen:** Busse `Music` und `Ambience` mit eigenen Reglern (029 legt sie an).
4. **Selbsttest:** jede Biom-Kulisse und jedes Stück lädt; Wechsel ruhig/Gefecht
   passiert in der Demo-Schlacht.

## 3. Akzeptanzkriterien

1. `just check` grün, Golden-Hash unverändert.
2. `just selftest` grün, Musik und Ambiente laufen headless ohne Fehler.
3. Speicherbedarf der Musik gestreamt, nicht vollständig im RAM.
4. Doku in `docs/ARCHITECTURE.md` (Audio) und Quellen in `docs/SOURCES.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
